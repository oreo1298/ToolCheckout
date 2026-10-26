//! Loans: checkout, problem reports, drop-off, return inspection, lost/found,
//! and the lists behind the home screen.
//!
//! Who acts:
//! * the borrower (proven by a PIN token) checks out, reports problems and
//!   drops off;
//! * a technician inspects returns, marks tools lost/found.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::audit::{self, Entry};
use crate::borrowers::{self, Borrower, BorrowerPick};
use crate::clock::DAY;
use crate::error::{CoreError, Result, invalid, not_found};
use crate::incidents::{self, Incident, NewIncident};
use crate::model::{
    Actor, CheckoutState, Condition, ConditionSource, IncidentKind, Outcome, ToolStatus,
};
use crate::status;
use crate::tools::{self, ToolFilter, ToolRow, record_condition, set_status};

pub const MAX_TOOLS_PER_CHECKOUT: usize = 25;
pub const MAX_LOAN: i64 = 90 * DAY;

/// A checkout is overdue once it is past its due-back time and still out.
pub fn is_overdue(state: CheckoutState, due_at: i64, now: i64) -> bool {
    state == CheckoutState::Out && now > due_at
}

/// How late a loan is (or was): for returned loans, time between due and
/// return; for loans still out, time since due. None when on time.
pub fn lateness(due_at: i64, returned_at: Option<i64>, now: i64) -> Option<i64> {
    let end = returned_at.unwrap_or(now);
    (end > due_at).then_some(end - due_at)
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoanRow {
    pub id: i64,
    pub batch_id: String,
    pub tool_id: i64,
    pub asset_tag: String,
    pub tool_name: String,
    pub tool_photo_id: Option<i64>,
    pub tool_status: ToolStatus,
    pub borrower_id: i64,
    pub borrower_name: String,
    pub checked_out_at: i64,
    pub due_at: i64,
    pub checkout_condition: Condition,
    pub authorized_by: Option<String>,
    pub state: CheckoutState,
    pub returned_at: Option<i64>,
    pub dropoff_note: String,
    pub inspected_at: Option<i64>,
    pub inspected_by: Option<String>,
    pub return_condition: Option<Condition>,
    pub return_notes: String,
    pub return_photo_id: Option<i64>,
    /// Derived at query time.
    pub overdue: bool,
    pub late_ms: Option<i64>,
}

const LOAN_SELECT: &str =
    "SELECT c.id, c.batch_id, c.tool_id, t.asset_tag, t.name, t.photo_id, t.status,
       c.borrower_id, b.name, c.checked_out_at, c.due_at, c.checkout_condition, at.name,
       c.state, c.returned_at, c.dropoff_note, c.inspected_at, it.name,
       c.return_condition, c.return_notes, c.return_photo_id
  FROM checkouts c
  JOIN tools t ON t.id = c.tool_id
  JOIN borrowers b ON b.id = c.borrower_id
  LEFT JOIN technicians at ON at.id = c.authorized_by_tech
  LEFT JOIN technicians it ON it.id = c.inspected_by_tech";

fn map_loan(r: &rusqlite::Row<'_>, now: i64) -> rusqlite::Result<LoanRow> {
    let state: CheckoutState = r.get(13)?;
    let due_at: i64 = r.get(10)?;
    let returned_at: Option<i64> = r.get(14)?;
    Ok(LoanRow {
        id: r.get(0)?,
        batch_id: r.get(1)?,
        tool_id: r.get(2)?,
        asset_tag: r.get(3)?,
        tool_name: r.get(4)?,
        tool_photo_id: r.get(5)?,
        tool_status: r.get(6)?,
        borrower_id: r.get(7)?,
        borrower_name: r.get(8)?,
        checked_out_at: r.get(9)?,
        due_at,
        checkout_condition: r.get(11)?,
        authorized_by: r.get(12)?,
        state,
        returned_at,
        dropoff_note: r.get(15)?,
        inspected_at: r.get(16)?,
        inspected_by: r.get(17)?,
        return_condition: r.get(18)?,
        return_notes: r.get(19)?,
        return_photo_id: r.get(20)?,
        overdue: is_overdue(state, due_at, now),
        late_ms: if state == CheckoutState::Lost {
            None
        } else {
            lateness(due_at, returned_at, now)
        },
    })
}

fn query_loans(
    conn: &Connection,
    now: i64,
    where_: &str,
    p: impl rusqlite::Params,
) -> Result<Vec<LoanRow>> {
    let mut st = conn.prepare(&format!("{LOAN_SELECT} {where_}"))?;
    let rows = st
        .query_map(p, |r| map_loan(r, now))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn get_loan(conn: &Connection, now: i64, id: i64) -> Result<LoanRow> {
    query_loans(conn, now, "WHERE c.id = ?1", [id])?
        .pop()
        .ok_or_else(|| not_found("Checkout"))
}

/// The open (out or dropped-off) loan for a tool, if any.
pub fn open_loan_for_tool(conn: &Connection, now: i64, tool_id: i64) -> Result<Option<LoanRow>> {
    Ok(query_loans(
        conn,
        now,
        "WHERE c.tool_id = ?1 AND c.state IN ('out', 'awaiting_inspection')",
        [tool_id],
    )?
    .pop())
}

fn random_id() -> Result<String> {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).map_err(|e| CoreError::Invalid(format!("no randomness: {e}")))?;
    Ok(hex::encode(b))
}

// ---------------------------------------------------------------- checkout

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutItem {
    pub tool_id: i64,
    /// None → now + the tool's default loan period.
    pub due_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutReceipt {
    pub batch_id: String,
    pub borrower: Borrower,
    pub loans: Vec<LoanRow>,
}

/// Lends tools to a borrower who has confirmed their PIN and the condition of
/// each tool. `tech` is the technician authorizing it (required for
/// tech-only tools and when the shop requires a tech PIN for every checkout).
pub fn checkout(
    conn: &Connection,
    now: i64,
    borrower: &Borrower,
    items: &[CheckoutItem],
    tech: Option<&Actor>,
) -> Result<CheckoutReceipt> {
    if items.is_empty() {
        return invalid("Add at least one tool");
    }
    if items.len() > MAX_TOOLS_PER_CHECKOUT {
        return invalid(format!(
            "At most {MAX_TOOLS_PER_CHECKOUT} tools per checkout"
        ));
    }
    let mut seen = std::collections::HashSet::new();
    if !items.iter().all(|i| seen.insert(i.tool_id)) {
        return invalid("The same tool is listed twice");
    }
    if !borrower.active {
        return invalid(format!("{} is inactive", borrower.name));
    }
    let batch_id = random_id()?;
    let actor = borrower.actor();
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let tool = tools::get(conn, item.tool_id)?;
        if tool.status != ToolStatus::Available {
            return Err(CoreError::Conflict(format!(
                "{} {} is {}, so it can't be checked out",
                tool.asset_tag,
                tool.name,
                tool.status.label().to_lowercase()
            )));
        }
        if tool.tech_only && tech.is_none() {
            return Err(CoreError::Conflict(format!(
                "{} {} is tech-only. A technician must authorize this checkout.",
                tool.asset_tag, tool.name
            )));
        }
        let due_at = item
            .due_at
            .unwrap_or(now + tool.default_loan_minutes * 60_000);
        if due_at <= now {
            return invalid("The due-back time must be in the future");
        }
        if due_at > now + MAX_LOAN {
            return invalid("Loans can be at most 90 days");
        }
        set_status(
            conn,
            now,
            tool.id,
            ToolStatus::Available,
            ToolStatus::CheckedOut,
        )?;
        conn.execute(
            "INSERT INTO checkouts (batch_id, tool_id, borrower_id, checked_out_at, due_at, checkout_condition,
                                    authorized_by_tech, state)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'out')",
            params![
                batch_id,
                tool.id,
                borrower.id,
                now,
                due_at,
                tool.condition,
                tech.and_then(Actor::tech_id)
            ],
        )?;
        let id = conn.last_insert_rowid();
        audit::append(
            conn,
            now,
            &actor,
            Entry::new("checkout.create", "checkout", Some(id))
                .before(json!({ "status": ToolStatus::Available }))
                .after(json!({
                    "tool": tool.asset_tag,
                    "toolName": tool.name,
                    "status": ToolStatus::CheckedOut,
                    "dueAt": due_at,
                    "conditionConfirmed": tool.condition,
                    "authorizedBy": tech.map(Actor::name),
                })),
        )?;
        ids.push(id);
    }
    let loans = ids
        .into_iter()
        .map(|id| get_loan(conn, now, id))
        .collect::<Result<Vec<_>>>()?;
    Ok(CheckoutReceipt {
        batch_id,
        borrower: borrower.clone(),
        loans,
    })
}

/// The borrower saw a problem before taking the tool: flag it for a
/// technician instead of lending it.
pub fn report_problem(
    conn: &Connection,
    now: i64,
    borrower: &Borrower,
    tool_id: i64,
    notes: &str,
    photo_id: Option<i64>,
) -> Result<Incident> {
    let tool = tools::get(conn, tool_id)?;
    let notes = notes.trim();
    if notes.is_empty() {
        return invalid("Say briefly what's wrong");
    }
    if notes.chars().count() > 1000 {
        return invalid("That note is too long");
    }
    if tool.status != ToolStatus::Available {
        return Err(CoreError::InvalidTransition {
            from: tool.status,
            to: ToolStatus::AwaitingInspection,
        });
    }
    set_status(
        conn,
        now,
        tool.id,
        ToolStatus::Available,
        ToolStatus::AwaitingInspection,
    )?;
    let actor = borrower.actor();
    audit::append(
        conn,
        now,
        &actor,
        Entry::new("checkout.problem_report", "tool", Some(tool.id))
            .before(json!({ "status": ToolStatus::Available, "condition": tool.condition }))
            .after(json!({ "status": ToolStatus::AwaitingInspection, "notes": notes })),
    )?;
    incidents::create(
        conn,
        now,
        &actor,
        NewIncident {
            kind: IncidentKind::ProblemReport,
            tool_id: tool.id,
            borrower_id: Some(borrower.id),
            checkout_id: None,
            before: Some(tool.condition),
            after: None,
            notes,
            photo_ids: photo_id.into_iter().collect(),
        },
    )
}

/// No technician free: the borrower leaves the tools for inspection. They stay
/// attributed to the borrower until a technician inspects them.
pub fn drop_off(
    conn: &Connection,
    now: i64,
    borrower: &Borrower,
    checkout_ids: &[i64],
    note: &str,
) -> Result<Vec<LoanRow>> {
    if checkout_ids.is_empty() {
        return invalid("Choose the tools you're returning");
    }
    let note = note.trim();
    if note.chars().count() > 1000 {
        return invalid("That note is too long");
    }
    let actor = borrower.actor();
    let mut out = Vec::new();
    for &id in checkout_ids {
        let loan = get_loan(conn, now, id)?;
        if loan.borrower_id != borrower.id {
            return Err(CoreError::Conflict(format!(
                "{} is checked out to {}, not {}",
                loan.asset_tag, loan.borrower_name, borrower.name
            )));
        }
        if loan.state != CheckoutState::Out {
            return Err(CoreError::Conflict(format!(
                "{} was already returned",
                loan.asset_tag
            )));
        }
        set_status(
            conn,
            now,
            loan.tool_id,
            ToolStatus::CheckedOut,
            ToolStatus::AwaitingInspection,
        )?;
        conn.execute(
            "UPDATE checkouts SET state = 'awaiting_inspection', returned_at = ?1, dropoff_note = ?2
             WHERE id = ?3 AND state = 'out'",
            params![now, note, id],
        )?;
        audit::append(
            conn,
            now,
            &actor,
            Entry::new("checkout.dropoff", "checkout", Some(id))
                .before(json!({ "status": ToolStatus::CheckedOut }))
                .after(json!({
                    "tool": loan.asset_tag,
                    "status": ToolStatus::AwaitingInspection,
                    "note": note,
                    "lateMs": lateness(loan.due_at, Some(now), now),
                })),
        )?;
        out.push(get_loan(conn, now, id)?);
    }
    Ok(out)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionResult {
    pub loan: Option<LoanRow>,
    pub tool: ToolRow,
    /// A damage incident, when the tool came back worse than it left.
    pub incident: Option<Incident>,
}

/// A technician inspects a returned tool (handed back now, or dropped off
/// earlier). Worse than at checkout → damage incident against the borrower.
pub fn inspect_return(
    conn: &Connection,
    now: i64,
    tech: &Actor,
    checkout_id: i64,
    condition: Condition,
    notes: &str,
    photo_id: Option<i64>,
) -> Result<InspectionResult> {
    let loan = get_loan(conn, now, checkout_id)?;
    let from = match loan.state {
        CheckoutState::Out => ToolStatus::CheckedOut,
        CheckoutState::AwaitingInspection => ToolStatus::AwaitingInspection,
        CheckoutState::Closed | CheckoutState::Lost => {
            return Err(CoreError::Conflict(format!(
                "{} was already {}",
                loan.asset_tag,
                if loan.state == CheckoutState::Lost {
                    "marked lost"
                } else {
                    "inspected"
                }
            )));
        }
    };
    let notes = notes.trim();
    let returned_at = loan.returned_at.unwrap_or(now);
    let to = status::after_inspection(condition);
    set_status(conn, now, loan.tool_id, from, to)?;
    conn.execute(
        "UPDATE checkouts SET state = 'closed', returned_at = ?1, inspected_at = ?2, inspected_by_tech = ?3,
                return_condition = ?4, return_notes = ?5, return_photo_id = ?6
         WHERE id = ?7",
        params![returned_at, now, tech.tech_id(), condition, notes, photo_id, checkout_id],
    )?;
    record_condition(
        conn,
        now,
        loan.tool_id,
        condition,
        notes,
        photo_id,
        &format!("Returned by {}", loan.borrower_name),
        ConditionSource::ReturnInspection,
        Some(checkout_id),
        tech.tech_id(),
    )?;
    audit::append(
        conn,
        now,
        tech,
        Entry::new("checkout.inspect", "checkout", Some(checkout_id))
            .before(json!({ "status": from, "condition": loan.checkout_condition }))
            .after(json!({
                "tool": loan.asset_tag,
                "borrower": loan.borrower_name,
                "status": to,
                "condition": condition,
                "notes": notes,
                "returnedAt": returned_at,
                "lateMs": lateness(loan.due_at, Some(returned_at), now),
            })),
    )?;
    let incident = if condition.is_downgrade_from(loan.checkout_condition) {
        Some(incidents::create(
            conn,
            now,
            tech,
            NewIncident {
                kind: IncidentKind::Damage,
                tool_id: loan.tool_id,
                borrower_id: Some(loan.borrower_id),
                checkout_id: Some(checkout_id),
                before: Some(loan.checkout_condition),
                after: Some(condition),
                notes,
                photo_ids: photo_id.into_iter().collect(),
            },
        )?)
    } else {
        None
    };
    Ok(InspectionResult {
        loan: Some(get_loan(conn, now, checkout_id)?),
        tool: tool_row(conn, loan.tool_id)?,
        incident,
    })
}

fn tool_row(conn: &Connection, id: i64) -> Result<ToolRow> {
    tools::list(
        conn,
        &ToolFilter {
            id: Some(id),
            ..Default::default()
        },
    )?
    .pop()
    .ok_or_else(|| not_found("Tool"))
}

/// Inspects a tool flagged by a problem report (no loan attached). Settles the
/// report; if the tool is worse than its recorded condition, opens a damage
/// incident (not attributed to anyone: the damage happened between loans).
pub fn inspect_flagged_tool(
    conn: &Connection,
    now: i64,
    tech: &Actor,
    tool_id: i64,
    condition: Condition,
    notes: &str,
    photo_id: Option<i64>,
) -> Result<InspectionResult> {
    let tool = tools::get(conn, tool_id)?;
    if tool.status != ToolStatus::AwaitingInspection {
        return Err(CoreError::Conflict(format!(
            "{} isn't waiting for inspection",
            tool.asset_tag
        )));
    }
    if let Some(loan) = open_loan_for_tool(conn, now, tool_id)? {
        return inspect_return(conn, now, tech, loan.id, condition, notes, photo_id);
    }
    let notes = notes.trim();
    let to = status::after_inspection(condition);
    set_status(conn, now, tool_id, ToolStatus::AwaitingInspection, to)?;
    record_condition(
        conn,
        now,
        tool_id,
        condition,
        notes,
        photo_id,
        "Inspected after a reported problem",
        ConditionSource::Inspection,
        None,
        tech.tech_id(),
    )?;
    audit::append(
        conn,
        now,
        tech,
        Entry::new("inspection.complete", "tool", Some(tool_id))
            .before(
                json!({ "status": ToolStatus::AwaitingInspection, "condition": tool.condition }),
            )
            .after(json!({ "status": to, "condition": condition, "notes": notes })),
    )?;
    let note = format!(
        "Inspected: {}{}",
        condition.label(),
        if notes.is_empty() {
            String::new()
        } else {
            format!(" — {notes}")
        }
    );
    incidents::close_open_for_tool(
        conn,
        now,
        tech,
        tool_id,
        IncidentKind::ProblemReport,
        Outcome::Inspected,
        &note,
    )?;
    let incident = if condition.is_downgrade_from(tool.condition) {
        let last = last_borrower_name(conn, tool_id)?;
        let context = match last {
            Some(name) => format!("Found at checkout; last borrowed by {name}. {notes}"),
            None => format!("Found at checkout. {notes}"),
        };
        Some(incidents::create(
            conn,
            now,
            tech,
            NewIncident {
                kind: IncidentKind::Damage,
                tool_id,
                borrower_id: None,
                checkout_id: None,
                before: Some(tool.condition),
                after: Some(condition),
                notes: context.trim(),
                photo_ids: photo_id.into_iter().collect(),
            },
        )?)
    } else {
        None
    };
    Ok(InspectionResult {
        loan: None,
        tool: tool_row(conn, tool_id)?,
        incident,
    })
}

fn last_borrower_name(conn: &Connection, tool_id: i64) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT b.name FROM checkouts c JOIN borrowers b ON b.id = c.borrower_id
             WHERE c.tool_id = ?1 ORDER BY c.checked_out_at DESC LIMIT 1",
            [tool_id],
            |r| r.get(0),
        )
        .optional()?)
}

/// Technician marks a tool lost. If it was on loan, a Lost incident is opened
/// against the borrower.
pub fn mark_lost(
    conn: &Connection,
    now: i64,
    tech: &Actor,
    tool_id: i64,
    note: &str,
) -> Result<Incident> {
    let tool = tools::get(conn, tool_id)?;
    let note = note.trim();
    if note.is_empty() {
        return invalid("Add a note (where was it last seen?)");
    }
    let loan = open_loan_for_tool(conn, now, tool_id)?;
    set_status(conn, now, tool_id, tool.status, ToolStatus::Lost)?;
    if let Some(l) = &loan {
        conn.execute("UPDATE checkouts SET state = 'lost' WHERE id = ?1", [l.id])?;
    }
    audit::append(
        conn,
        now,
        tech,
        Entry::new("tool.lost", "tool", Some(tool_id))
            .before(json!({ "status": tool.status }))
            .after(json!({
                "status": ToolStatus::Lost,
                "note": note,
                "borrower": loan.as_ref().map(|l| l.borrower_name.clone()),
            })),
    )?;
    incidents::create(
        conn,
        now,
        tech,
        NewIncident {
            kind: IncidentKind::Lost,
            tool_id,
            borrower_id: loan.as_ref().map(|l| l.borrower_id),
            checkout_id: loan.as_ref().map(|l| l.id),
            before: Some(
                loan.as_ref()
                    .map_or(tool.condition, |l| l.checkout_condition),
            ),
            after: None,
            notes: note,
            photo_ids: vec![],
        },
    )
}

/// A lost tool turned up. Its open Lost incidents are resolved as Found; if it
/// came back worse than when it was lent, a damage incident is opened against
/// that borrower.
pub fn mark_found(
    conn: &Connection,
    now: i64,
    tech: &Actor,
    tool_id: i64,
    condition: Condition,
    notes: &str,
    photo_id: Option<i64>,
) -> Result<InspectionResult> {
    let tool = tools::get(conn, tool_id)?;
    if tool.status != ToolStatus::Lost {
        return Err(CoreError::Conflict(format!(
            "{} isn't marked lost",
            tool.asset_tag
        )));
    }
    let notes = notes.trim();
    let to = status::after_inspection(condition);
    set_status(conn, now, tool_id, ToolStatus::Lost, to)?;
    record_condition(
        conn,
        now,
        tool_id,
        condition,
        notes,
        photo_id,
        "Found",
        ConditionSource::Found,
        None,
        tech.tech_id(),
    )?;
    audit::append(
        conn,
        now,
        tech,
        Entry::new("tool.found", "tool", Some(tool_id))
            .before(json!({ "status": ToolStatus::Lost }))
            .after(json!({ "status": to, "condition": condition, "notes": notes })),
    )?;
    let closed = incidents::close_open_for_tool(
        conn,
        now,
        tech,
        tool_id,
        IncidentKind::Lost,
        Outcome::Found,
        &format!("Found by {}", tech.name()),
    )?;
    let mut incident = None;
    if let Some(lost) = closed.iter().find(|i| i.borrower_id.is_some())
        && let Some(before) = lost.before_condition
        && condition.is_downgrade_from(before)
    {
        incident = Some(incidents::create(
            conn,
            now,
            tech,
            NewIncident {
                kind: IncidentKind::Damage,
                tool_id,
                borrower_id: lost.borrower_id,
                checkout_id: lost.checkout_id,
                before: Some(before),
                after: Some(condition),
                notes: &format!("Found damaged after being lost. {notes}"),
                photo_ids: photo_id.into_iter().collect(),
            },
        )?);
    }
    Ok(InspectionResult {
        loan: None,
        tool: tool_row(conn, tool_id)?,
        incident,
    })
}

// ---------------------------------------------------------------- home lists

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaitingItem {
    /// "dropoff" or "problem_report"
    pub kind: &'static str,
    pub tool_id: i64,
    pub asset_tag: String,
    pub tool_name: String,
    pub tool_photo_id: Option<i64>,
    pub checkout_id: Option<i64>,
    pub incident_id: Option<i64>,
    pub borrower_id: Option<i64>,
    pub borrower_name: Option<String>,
    pub since: i64,
    pub note: String,
    /// Condition when it was lent (drop-off) or when it was flagged (report).
    pub condition_before: Condition,
    pub returned_late: bool,
}

pub fn awaiting_inspection(conn: &Connection, now: i64) -> Result<Vec<AwaitingItem>> {
    let mut items: Vec<AwaitingItem> = query_loans(
        conn,
        now,
        "WHERE c.state = 'awaiting_inspection' ORDER BY c.returned_at",
        [],
    )?
    .into_iter()
    .map(|l| AwaitingItem {
        kind: "dropoff",
        tool_id: l.tool_id,
        asset_tag: l.asset_tag,
        tool_name: l.tool_name,
        tool_photo_id: l.tool_photo_id,
        checkout_id: Some(l.id),
        incident_id: None,
        borrower_id: Some(l.borrower_id),
        borrower_name: Some(l.borrower_name),
        since: l.returned_at.unwrap_or(l.checked_out_at),
        note: l.dropoff_note,
        condition_before: l.checkout_condition,
        returned_late: l.late_ms.is_some(),
    })
    .collect();
    let mut st = conn.prepare(
        "SELECT t.id, t.asset_tag, t.name, t.photo_id, i.id, i.borrower_id, b.name, i.created_at, i.notes,
                coalesce(i.before_condition, t.condition)
         FROM tools t
         JOIN incidents i ON i.tool_id = t.id AND i.kind = 'problem_report' AND i.status = 'open'
         LEFT JOIN borrowers b ON b.id = i.borrower_id
         WHERE t.status = 'awaiting_inspection'
           AND NOT EXISTS (SELECT 1 FROM checkouts c WHERE c.tool_id = t.id AND c.state = 'awaiting_inspection')
         ORDER BY i.created_at",
    )?;
    let reports = st.query_map([], |r| {
        Ok(AwaitingItem {
            kind: "problem_report",
            tool_id: r.get(0)?,
            asset_tag: r.get(1)?,
            tool_name: r.get(2)?,
            tool_photo_id: r.get(3)?,
            checkout_id: None,
            incident_id: Some(r.get(4)?),
            borrower_id: r.get(5)?,
            borrower_name: r.get(6)?,
            since: r.get(7)?,
            note: r.get(8)?,
            condition_before: r.get(9)?,
            returned_late: false,
        })
    })?;
    for r in reports {
        items.push(r?);
    }
    items.sort_by_key(|i| i.since);
    Ok(items)
}

pub fn open_loans(conn: &Connection, now: i64) -> Result<Vec<LoanRow>> {
    query_loans(
        conn,
        now,
        "WHERE c.state = 'out' ORDER BY c.due_at, c.id",
        [],
    )
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LoanFilter {
    pub tool_id: Option<i64>,
    pub borrower_id: Option<i64>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<i64>,
}

/// Loan history, newest first.
pub fn history(conn: &Connection, now: i64, f: &LoanFilter) -> Result<Vec<LoanRow>> {
    query_loans(
        conn,
        now,
        "WHERE (?1 IS NULL OR c.tool_id = ?1) AND (?2 IS NULL OR c.borrower_id = ?2)
           AND (?3 IS NULL OR c.checked_out_at >= ?3) AND (?4 IS NULL OR c.checked_out_at < ?4)
         ORDER BY c.checked_out_at DESC, c.id DESC LIMIT ?5",
        params![
            f.tool_id,
            f.borrower_id,
            f.from,
            f.to,
            f.limit.unwrap_or(1000)
        ],
    )
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Home {
    pub overdue: Vec<LoanRow>,
    pub out: Vec<LoanRow>,
    pub awaiting: Vec<AwaitingItem>,
    pub open_incidents: i64,
    pub available_tools: i64,
    pub now: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScanResult {
    Tool {
        tool: Box<ToolRow>,
        loan: Option<Box<LoanRow>>,
    },
    Borrower {
        borrower: BorrowerPick,
    },
    Unknown {
        code: String,
        message: String,
    },
}

pub fn resolve_scan(conn: &Connection, now: i64, code: &str) -> Result<ScanResult> {
    let code = code.trim();
    if let Some(t) = tools::find_by_tag(conn, code)? {
        return Ok(ScanResult::Tool {
            tool: Box::new(tool_row(conn, t.id)?),
            loan: open_loan_for_tool(conn, now, t.id)?.map(Box::new),
        });
    }
    if let Some(b) = borrowers::find_by_code(conn, code)? {
        if !b.active {
            return Ok(ScanResult::Unknown {
                code: code.into(),
                message: format!("{} is inactive. Ask a technician.", b.name),
            });
        }
        let pick = borrowers::picker(conn, now)?
            .into_iter()
            .find(|p| p.id == b.id)
            .ok_or_else(|| not_found("Borrower"))?;
        return Ok(ScanResult::Borrower { borrower: pick });
    }
    Ok(ScanResult::Unknown {
        code: code.into(),
        message: format!("Nothing matches “{code}”"),
    })
}

// ---------------------------------------------------------------- service API

impl crate::Service {
    pub fn home(&self) -> Result<Home> {
        let now = self.now();
        let out = open_loans(&self.conn, now)?;
        let (overdue, out): (Vec<_>, Vec<_>) = out.into_iter().partition(|l| l.overdue);
        Ok(Home {
            overdue,
            out,
            awaiting: awaiting_inspection(&self.conn, now)?,
            open_incidents: self.open_incident_count()?,
            available_tools: self.conn.query_row(
                "SELECT count(*) FROM tools WHERE status = 'available'",
                [],
                |r| r.get(0),
            )?,
            now,
        })
    }

    pub fn resolve_scan(&self, code: &str) -> Result<ScanResult> {
        resolve_scan(&self.conn, self.now(), code)
    }

    pub fn open_loan_for_tool(&self, tool_id: i64) -> Result<Option<LoanRow>> {
        open_loan_for_tool(&self.conn, self.now(), tool_id)
    }

    /// Final checkout step. The token proves the borrower entered their PIN.
    pub fn checkout(&mut self, token: &str, items: &[CheckoutItem]) -> Result<CheckoutReceipt> {
        let borrower = self.borrower_from_token(token)?;
        let mut needs_tech = self.settings.tech_pin_for_checkout;
        for i in items {
            needs_tech |= tools::get(&self.conn, i.tool_id)?.tech_only;
        }
        let tech = if needs_tech {
            Some(self.require_tech()?)
        } else {
            None
        };
        let receipt = self.tx(|tx, now| checkout(tx, now, &borrower, items, tech.as_ref()))?;
        self.sessions.revoke_token(token);
        Ok(receipt)
    }

    pub fn report_problem(
        &mut self,
        token: &str,
        tool_id: i64,
        notes: &str,
        photo_id: Option<i64>,
    ) -> Result<Incident> {
        let borrower = self.borrower_from_token(token)?;
        self.tx(|tx, now| report_problem(tx, now, &borrower, tool_id, notes, photo_id))
    }

    pub fn drop_off(
        &mut self,
        token: &str,
        checkout_ids: &[i64],
        note: &str,
    ) -> Result<Vec<LoanRow>> {
        let borrower = self.borrower_from_token(token)?;
        let out = self.tx(|tx, now| drop_off(tx, now, &borrower, checkout_ids, note))?;
        self.sessions.revoke_token(token);
        Ok(out)
    }

    pub fn inspect_return(
        &mut self,
        checkout_id: i64,
        condition: Condition,
        notes: &str,
        photo_id: Option<i64>,
    ) -> Result<InspectionResult> {
        self.tech_tx(|tx, now, actor| {
            inspect_return(tx, now, actor, checkout_id, condition, notes, photo_id)
        })
    }

    pub fn inspect_tool(
        &mut self,
        tool_id: i64,
        condition: Condition,
        notes: &str,
        photo_id: Option<i64>,
    ) -> Result<InspectionResult> {
        self.tech_tx(|tx, now, actor| {
            inspect_flagged_tool(tx, now, actor, tool_id, condition, notes, photo_id)
        })
    }

    pub fn mark_lost(&mut self, tool_id: i64, note: &str) -> Result<Incident> {
        self.tech_tx(|tx, now, actor| mark_lost(tx, now, actor, tool_id, note))
    }

    pub fn mark_found(
        &mut self,
        tool_id: i64,
        condition: Condition,
        notes: &str,
        photo_id: Option<i64>,
    ) -> Result<InspectionResult> {
        self.tech_tx(|tx, now, actor| {
            mark_found(tx, now, actor, tool_id, condition, notes, photo_id)
        })
    }

    pub fn loan_history(&mut self, f: &LoanFilter) -> Result<Vec<LoanRow>> {
        self.require_tech()?;
        history(&self.conn, self.now(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::borrowers::BorrowerInput;
    use crate::clock::{HOUR, MINUTE};
    use crate::incidents::IncidentFilter;
    use crate::model::IncidentStatus;
    use crate::service::tests::ready;
    use crate::tools::ToolInput;
    use crate::{ManualClock, Service, audit};
    use std::sync::Arc;

    struct Shop {
        s: Service,
        c: Arc<ManualClock>,
        driver: i64,
        meter: i64,
        iron: i64,
        sam: Borrower,
        kim: Borrower,
    }

    fn tool(s: &mut Service, name: &str, cond: Condition, tech_only: bool) -> i64 {
        s.create_tool(ToolInput {
            name: name.into(),
            default_loan_minutes: 240,
            condition: Some(cond),
            tech_only,
            replacement_cost_cents: Some(5000),
            ..Default::default()
        })
        .unwrap()
        .id
    }

    fn shop() -> Shop {
        let (mut s, c, _r) = ready();
        let driver = tool(&mut s, "P5 driver", Condition::Good, false);
        let meter = tool(&mut s, "Multimeter", Condition::New, false);
        let iron = tool(&mut s, "Hot air station", Condition::Good, true);
        let mk = |s: &mut Service, n: &str, p: &str| {
            s.create_borrower(
                BorrowerInput {
                    name: n.into(),
                    ..Default::default()
                },
                Some(p),
            )
            .unwrap()
        };
        let sam = mk(&mut s, "Sam", "1111");
        let kim = mk(&mut s, "Kim", "2222");
        s.lock();
        Shop {
            s,
            c,
            driver,
            meter,
            iron,
            sam,
            kim,
        }
    }

    fn token(s: &mut Service, b: &Borrower, pin: &str) -> String {
        s.authenticate_borrower(b.id, Some(pin)).unwrap().token
    }

    fn item(id: i64) -> CheckoutItem {
        CheckoutItem {
            tool_id: id,
            due_at: None,
        }
    }

    fn unlock(s: &mut Service) {
        s.unlock(1, "123456").unwrap();
    }

    #[test]
    fn multi_tool_checkout_without_tech() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        let r =
            sh.s.checkout(&t, &[item(sh.driver), item(sh.meter)])
                .unwrap();
        assert_eq!(r.loans.len(), 2);
        assert!(
            r.loans
                .iter()
                .all(|l| l.batch_id == r.batch_id && l.state == CheckoutState::Out)
        );
        assert_eq!(r.loans[0].due_at, r.loans[0].checked_out_at + 4 * HOUR);
        assert_eq!(r.loans[1].checkout_condition, Condition::New);
        assert_eq!(
            tools::get(sh.s.conn(), sh.driver).unwrap().status,
            ToolStatus::CheckedOut
        );
        // Token is single-checkout.
        assert_eq!(
            sh.s.checkout(&t, &[item(sh.driver)]).unwrap_err().code(),
            "auth_expired"
        );
        // Audit: borrower is the actor.
        let log = audit::list(sh.s.conn(), &Default::default()).unwrap();
        assert_eq!(log[0].action, "checkout.create");
        assert_eq!(log[0].actor_name, "Sam");
        assert_eq!(log[0].actor_kind, "borrower");
    }

    #[test]
    fn checkout_requires_valid_token() {
        let mut sh = shop();
        assert_eq!(
            sh.s.checkout("nope", &[item(sh.driver)])
                .unwrap_err()
                .code(),
            "auth_expired"
        );
        let t = token(&mut sh.s, &sh.sam, "1111");
        sh.c.advance(3 * MINUTE);
        assert_eq!(
            sh.s.checkout(&t, &[item(sh.driver)]).unwrap_err().code(),
            "auth_expired"
        );
    }

    #[test]
    fn tools_already_out_cannot_be_checked_out_again() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        sh.s.checkout(&t, &[item(sh.driver)]).unwrap();
        let t2 = token(&mut sh.s, &sh.kim, "2222");
        let e =
            sh.s.checkout(&t2, &[item(sh.meter), item(sh.driver)])
                .unwrap_err();
        assert_eq!(e.code(), "conflict");
        // All-or-nothing: the meter was not lent either.
        assert_eq!(
            tools::get(sh.s.conn(), sh.meter).unwrap().status,
            ToolStatus::Available
        );
    }

    #[test]
    fn tech_only_tools_need_a_technician() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        assert_eq!(
            sh.s.checkout(&t, &[item(sh.iron)]).unwrap_err().code(),
            "tech_required"
        );
        unlock(&mut sh.s);
        let r = sh.s.checkout(&t, &[item(sh.iron)]).unwrap();
        assert_eq!(r.loans[0].authorized_by.as_deref(), Some("Ana"));
    }

    #[test]
    fn setting_can_require_tech_for_every_checkout() {
        let mut sh = shop();
        unlock(&mut sh.s);
        let mut st = sh.s.settings().clone();
        st.tech_pin_for_checkout = true;
        sh.s.update_settings(st).unwrap();
        sh.s.lock();
        let t = token(&mut sh.s, &sh.sam, "1111");
        assert_eq!(
            sh.s.checkout(&t, &[item(sh.driver)]).unwrap_err().code(),
            "tech_required"
        );
        unlock(&mut sh.s);
        assert!(sh.s.checkout(&t, &[item(sh.driver)]).is_ok());
    }

    #[test]
    fn custom_due_times_are_validated() {
        let mut sh = shop();
        let now = sh.s.now();
        let t = token(&mut sh.s, &sh.sam, "1111");
        let past = CheckoutItem {
            tool_id: sh.driver,
            due_at: Some(now - 1),
        };
        assert!(sh.s.checkout(&t, &[past]).is_err());
        let far = CheckoutItem {
            tool_id: sh.driver,
            due_at: Some(now + 91 * DAY),
        };
        assert!(sh.s.checkout(&t, &[far]).is_err());
        let ok = CheckoutItem {
            tool_id: sh.driver,
            due_at: Some(now + 30 * MINUTE),
        };
        assert_eq!(
            sh.s.checkout(&t, &[ok]).unwrap().loans[0].due_at,
            now + 30 * MINUTE
        );
    }

    #[test]
    fn overdue_calculation() {
        assert!(!is_overdue(CheckoutState::Out, 100, 100));
        assert!(is_overdue(CheckoutState::Out, 100, 101));
        assert!(!is_overdue(CheckoutState::AwaitingInspection, 100, 500));
        assert!(!is_overdue(CheckoutState::Closed, 100, 500));
        assert_eq!(lateness(100, Some(90), 500), None);
        assert_eq!(lateness(100, Some(160), 500), Some(60));
        assert_eq!(lateness(100, None, 130), Some(30));

        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        sh.s.checkout(&t, &[item(sh.driver)]).unwrap();
        let h = sh.s.home().unwrap();
        assert_eq!((h.out.len(), h.overdue.len()), (1, 0));
        sh.c.advance(4 * HOUR);
        assert_eq!(
            sh.s.home().unwrap().overdue.len(),
            0,
            "due exactly now is not overdue yet"
        );
        sh.c.advance(1);
        let h = sh.s.home().unwrap();
        assert_eq!((h.out.len(), h.overdue.len()), (0, 1));
        assert_eq!(h.overdue[0].late_ms, Some(1));
    }

    #[test]
    fn return_inspected_same_condition_no_incident() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        let loan =
            sh.s.checkout(&t, &[item(sh.driver)])
                .unwrap()
                .loans
                .remove(0);
        assert_eq!(
            sh.s.inspect_return(loan.id, Condition::Good, "", None)
                .unwrap_err()
                .code(),
            "tech_required"
        );
        unlock(&mut sh.s);
        let r =
            sh.s.inspect_return(loan.id, Condition::Good, "fine", None)
                .unwrap();
        assert!(r.incident.is_none());
        assert_eq!(r.tool.tool.status, ToolStatus::Available);
        let l = r.loan.unwrap();
        assert_eq!(l.state, CheckoutState::Closed);
        assert_eq!(l.inspected_by.as_deref(), Some("Ana"));
        // Improvement isn't a downgrade either.
        let t = token(&mut sh.s, &sh.sam, "1111");
        let loan =
            sh.s.checkout(&t, &[item(sh.driver)])
                .unwrap()
                .loans
                .remove(0);
        assert!(
            sh.s.inspect_return(loan.id, Condition::New, "cleaned", None)
                .unwrap()
                .incident
                .is_none()
        );
    }

    #[test]
    fn condition_downgrade_creates_damage_incident() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        let loan =
            sh.s.checkout(&t, &[item(sh.meter)])
                .unwrap()
                .loans
                .remove(0);
        unlock(&mut sh.s);
        let photo = sh.s.save_photo(&[0xFF, 0xD8, 0xFF, 0xE0, 1, 2]).unwrap();
        let r =
            sh.s.inspect_return(loan.id, Condition::Worn, "scratched screen", Some(photo))
                .unwrap();
        let inc = r.incident.expect("damage incident");
        assert_eq!(inc.kind, IncidentKind::Damage);
        assert_eq!(inc.borrower_name.as_deref(), Some("Sam"));
        assert_eq!(
            (inc.before_condition, inc.after_condition),
            (Some(Condition::New), Some(Condition::Worn))
        );
        assert_eq!(inc.photo_ids, vec![photo]);
        assert_eq!(inc.checkout_id, Some(loan.id));
        // Worn still works → back on the shelf; condition history updated.
        assert_eq!(r.tool.tool.status, ToolStatus::Available);
        let h = tools::condition_history(sh.s.conn(), sh.meter).unwrap();
        assert_eq!(h[0].source, ConditionSource::ReturnInspection);
        assert_eq!(h[0].checkout_id, Some(loan.id));
    }

    #[test]
    fn broken_on_return_goes_to_repair() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        let loan =
            sh.s.checkout(&t, &[item(sh.driver)])
                .unwrap()
                .loans
                .remove(0);
        unlock(&mut sh.s);
        let r =
            sh.s.inspect_return(loan.id, Condition::Broken, "tip snapped", None)
                .unwrap();
        assert_eq!(r.tool.tool.status, ToolStatus::OutForRepair);
        assert!(r.incident.is_some());
        // Can't inspect twice.
        assert_eq!(
            sh.s.inspect_return(loan.id, Condition::Good, "", None)
                .unwrap_err()
                .code(),
            "conflict"
        );
    }

    #[test]
    fn drop_off_keeps_attribution_until_inspected() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        let loans =
            sh.s.checkout(&t, &[item(sh.driver), item(sh.meter)])
                .unwrap()
                .loans;
        sh.c.advance(5 * HOUR); // an hour late
        // Kim can't drop off Sam's tools.
        let tk = token(&mut sh.s, &sh.kim, "2222");
        assert_eq!(
            sh.s.drop_off(&tk, &[loans[0].id], "").unwrap_err().code(),
            "conflict"
        );
        let t = token(&mut sh.s, &sh.sam, "1111");
        let dropped =
            sh.s.drop_off(&t, &[loans[0].id, loans[1].id], "left on bench 2")
                .unwrap();
        assert!(
            dropped
                .iter()
                .all(|l| l.state == CheckoutState::AwaitingInspection && !l.overdue)
        );
        assert_eq!(dropped[0].late_ms, Some(HOUR));
        let home = sh.s.home().unwrap();
        assert_eq!(home.awaiting.len(), 2);
        assert_eq!(home.awaiting[0].borrower_name.as_deref(), Some("Sam"));
        assert!(home.awaiting[0].returned_late);
        assert!(home.overdue.is_empty() && home.out.is_empty());
        assert_eq!(
            tools::get(sh.s.conn(), sh.driver).unwrap().status,
            ToolStatus::AwaitingInspection
        );

        // Tech inspects later: return time stays the drop-off time.
        sh.c.advance(2 * HOUR);
        unlock(&mut sh.s);
        let r =
            sh.s.inspect_return(loans[1].id, Condition::Damaged, "cracked case", None)
                .unwrap();
        let l = r.loan.unwrap();
        assert_eq!(l.returned_at, dropped[1].returned_at);
        assert_eq!(l.late_ms, Some(HOUR));
        assert_eq!(r.incident.unwrap().borrower_name.as_deref(), Some("Sam"));
        assert_eq!(sh.s.home().unwrap().awaiting.len(), 1);
    }

    #[test]
    fn problem_report_flags_tool_instead_of_lending() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.kim, "2222");
        let inc =
            sh.s.report_problem(&t, sh.driver, "tip is chipped", None)
                .unwrap();
        assert_eq!(inc.kind, IncidentKind::ProblemReport);
        assert_eq!(inc.borrower_name.as_deref(), Some("Kim"));
        // The same token still works for the rest of the checkout.
        sh.s.checkout(&t, &[item(sh.meter)]).unwrap();
        assert_eq!(
            tools::get(sh.s.conn(), sh.driver).unwrap().status,
            ToolStatus::AwaitingInspection
        );
        let home = sh.s.home().unwrap();
        assert_eq!(home.awaiting.len(), 1);
        assert_eq!(home.awaiting[0].kind, "problem_report");
        // A flagged tool can't be lent.
        let t2 = token(&mut sh.s, &sh.sam, "1111");
        assert!(sh.s.checkout(&t2, &[item(sh.driver)]).is_err());

        // Tech inspects: report settled, damage noted but not pinned on anyone.
        unlock(&mut sh.s);
        let r =
            sh.s.inspect_tool(sh.driver, Condition::Damaged, "chipped tip confirmed", None)
                .unwrap();
        assert_eq!(r.tool.tool.status, ToolStatus::OutForRepair);
        let dmg = r.incident.unwrap();
        assert_eq!(dmg.borrower_id, None);
        let report = crate::incidents::get(sh.s.conn(), inc.id).unwrap();
        assert_eq!(report.status, IncidentStatus::Resolved);
        assert_eq!(report.outcome, Some(Outcome::Inspected));
        assert!(sh.s.home().unwrap().awaiting.is_empty());
    }

    #[test]
    fn lost_and_found() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        let loan =
            sh.s.checkout(&t, &[item(sh.meter)])
                .unwrap()
                .loans
                .remove(0);
        unlock(&mut sh.s);
        assert!(sh.s.mark_lost(sh.meter, " ").is_err());
        let inc = sh.s.mark_lost(sh.meter, "Sam can't find it").unwrap();
        assert_eq!(inc.kind, IncidentKind::Lost);
        assert_eq!(inc.borrower_name.as_deref(), Some("Sam"));
        assert_eq!(
            get_loan(sh.s.conn(), sh.s.now(), loan.id).unwrap().state,
            CheckoutState::Lost
        );
        assert!(
            sh.s.home().unwrap().out.is_empty(),
            "lost loans aren't 'out'"
        );
        // Lost tools can't be lent or inspected.
        assert!(
            sh.s.inspect_return(loan.id, Condition::Good, "", None)
                .is_err()
        );
        // Found, scratched: lost incident resolved, damage incident against Sam.
        let r =
            sh.s.mark_found(sh.meter, Condition::Fair, "behind the bench", None)
                .unwrap();
        assert_eq!(r.tool.tool.status, ToolStatus::Available);
        let lost = crate::incidents::get(sh.s.conn(), inc.id).unwrap();
        assert_eq!(lost.outcome, Some(Outcome::Found));
        assert_eq!(r.incident.unwrap().borrower_name.as_deref(), Some("Sam"));
        // A tool lost from the shelf has no borrower.
        let inc2 = sh.s.mark_lost(sh.driver, "not in drawer").unwrap();
        assert_eq!(inc2.borrower_id, None);
    }

    #[test]
    fn resolving_incidents() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        let loan =
            sh.s.checkout(&t, &[item(sh.meter)])
                .unwrap()
                .loans
                .remove(0);
        unlock(&mut sh.s);
        let inc =
            sh.s.inspect_return(loan.id, Condition::Fair, "", None)
                .unwrap()
                .incident
                .unwrap();
        assert!(
            sh.s.resolve_incident(inc.id, Outcome::NoFault, "  ")
                .is_err()
        );
        let done =
            sh.s.resolve_incident(inc.id, Outcome::NoFault, "normal wear")
                .unwrap();
        assert_eq!(done.resolved_by_name.as_deref(), Some("Ana"));
        assert_eq!(
            sh.s.resolve_incident(inc.id, Outcome::Replaced, "x")
                .unwrap_err()
                .code(),
            "conflict"
        );
        let open =
            sh.s.list_incidents(&IncidentFilter {
                status: Some(IncidentStatus::Open),
                ..Default::default()
            })
            .unwrap();
        assert!(open.is_empty());
        sh.s.lock();
        assert_eq!(
            sh.s.resolve_incident(inc.id, Outcome::Other, "x")
                .unwrap_err()
                .code(),
            "tech_required"
        );
    }

    #[test]
    fn scan_resolution() {
        let mut sh = shop();
        unlock(&mut sh.s);
        sh.s.update_borrower(
            sh.sam.id,
            BorrowerInput {
                name: "Sam".into(),
                badge_code: Some("B-100".into()),
                ..Default::default()
            },
            true,
        )
        .unwrap();
        match sh.s.resolve_scan("t-0001").unwrap() {
            ScanResult::Tool { tool, loan } => {
                assert_eq!(tool.tool.id, sh.driver);
                assert!(loan.is_none());
            }
            other => panic!("{other:?}"),
        }
        match sh.s.resolve_scan(" b-100\n").unwrap() {
            ScanResult::Borrower { borrower } => assert_eq!(borrower.id, sh.sam.id),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            sh.s.resolve_scan("zzz").unwrap(),
            ScanResult::Unknown { .. }
        ));
        let t = token(&mut sh.s, &sh.sam, "1111");
        sh.s.checkout(&t, &[item(sh.driver)]).unwrap();
        match sh.s.resolve_scan("T-0001").unwrap() {
            ScanResult::Tool { loan, .. } => assert_eq!(loan.unwrap().borrower_name, "Sam"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn whole_day_keeps_the_audit_chain_intact() {
        let mut sh = shop();
        let t = token(&mut sh.s, &sh.sam, "1111");
        let loans =
            sh.s.checkout(&t, &[item(sh.driver), item(sh.meter)])
                .unwrap()
                .loans;
        let tk = token(&mut sh.s, &sh.kim, "2222");
        sh.s.report_problem(&tk, sh.iron, "fan noisy", None)
            .unwrap();
        sh.c.advance(6 * HOUR);
        let t = token(&mut sh.s, &sh.sam, "1111");
        sh.s.drop_off(&t, &[loans[0].id], "").unwrap();
        unlock(&mut sh.s);
        sh.s.inspect_return(loans[0].id, Condition::Good, "", None)
            .unwrap();
        sh.s.inspect_return(loans[1].id, Condition::Worn, "", None)
            .unwrap();
        sh.s.inspect_tool(sh.iron, Condition::Good, "fan cleaned", None)
            .unwrap();
        let report = sh.s.verify_audit().unwrap();
        assert!(report.ok, "{report:?}");
        assert!(report.entries > 15);
    }
}
