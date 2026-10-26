//! Incidents: damage (condition got worse on loan), lost tools, and problems a
//! borrower reported before taking a tool. Technicians resolve them with an
//! outcome and a note; nothing is ever deleted.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::audit::{self, Entry};
use crate::error::{CoreError, Result, invalid, not_found};
use crate::model::{Actor, Condition, IncidentKind, IncidentStatus, Outcome};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Incident {
    pub id: i64,
    pub kind: IncidentKind,
    pub tool_id: i64,
    pub asset_tag: String,
    pub tool_name: String,
    pub replacement_cost_cents: Option<i64>,
    pub borrower_id: Option<i64>,
    pub borrower_name: Option<String>,
    pub checkout_id: Option<i64>,
    pub before_condition: Option<Condition>,
    pub after_condition: Option<Condition>,
    pub notes: String,
    pub photo_ids: Vec<i64>,
    pub created_at: i64,
    pub created_by_kind: String,
    pub created_by_name: Option<String>,
    pub status: IncidentStatus,
    pub outcome: Option<Outcome>,
    pub resolution_note: Option<String>,
    pub resolved_at: Option<i64>,
    pub resolved_by_name: Option<String>,
}

const SELECT: &str = "SELECT i.id, i.kind, i.tool_id, t.asset_tag, t.name, t.replacement_cost_cents,
       i.borrower_id, b.name, i.checkout_id, i.before_condition, i.after_condition, i.notes, i.photo_ids,
       i.created_at, i.created_by_kind,
       CASE i.created_by_kind WHEN 'technician' THEN ct.name WHEN 'borrower' THEN cb.name ELSE NULL END,
       i.status, i.outcome, i.resolution_note, i.resolved_at, rt.name
  FROM incidents i
  JOIN tools t ON t.id = i.tool_id
  LEFT JOIN borrowers b ON b.id = i.borrower_id
  LEFT JOIN technicians ct ON i.created_by_kind = 'technician' AND ct.id = i.created_by_id
  LEFT JOIN borrowers cb ON i.created_by_kind = 'borrower' AND cb.id = i.created_by_id
  LEFT JOIN technicians rt ON rt.id = i.resolved_by_tech";

fn map(r: &rusqlite::Row<'_>) -> rusqlite::Result<Incident> {
    let photos: String = r.get(12)?;
    Ok(Incident {
        id: r.get(0)?,
        kind: r.get(1)?,
        tool_id: r.get(2)?,
        asset_tag: r.get(3)?,
        tool_name: r.get(4)?,
        replacement_cost_cents: r.get(5)?,
        borrower_id: r.get(6)?,
        borrower_name: r.get(7)?,
        checkout_id: r.get(8)?,
        before_condition: r.get(9)?,
        after_condition: r.get(10)?,
        notes: r.get(11)?,
        photo_ids: serde_json::from_str(&photos).unwrap_or_default(),
        created_at: r.get(13)?,
        created_by_kind: r.get(14)?,
        created_by_name: r.get(15)?,
        status: r.get(16)?,
        outcome: r.get(17)?,
        resolution_note: r.get(18)?,
        resolved_at: r.get(19)?,
        resolved_by_name: r.get(20)?,
    })
}

pub fn get(conn: &Connection, id: i64) -> Result<Incident> {
    conn.query_row(&format!("{SELECT} WHERE i.id = ?1"), [id], map)
        .optional()?
        .ok_or_else(|| not_found("Incident"))
}

/// What a new incident is about.
pub struct NewIncident<'a> {
    pub kind: IncidentKind,
    pub tool_id: i64,
    pub borrower_id: Option<i64>,
    pub checkout_id: Option<i64>,
    pub before: Option<Condition>,
    pub after: Option<Condition>,
    pub notes: &'a str,
    pub photo_ids: Vec<i64>,
}

pub(crate) fn create(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    n: NewIncident<'_>,
) -> Result<Incident> {
    conn.execute(
        "INSERT INTO incidents (kind, tool_id, borrower_id, checkout_id, before_condition, after_condition,
                                notes, photo_ids, created_at, created_by_kind, created_by_id, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'open')",
        params![
            n.kind,
            n.tool_id,
            n.borrower_id,
            n.checkout_id,
            n.before,
            n.after,
            n.notes.trim(),
            serde_json::to_string(&n.photo_ids).unwrap(),
            now,
            actor.kind(),
            actor.id()
        ],
    )?;
    let inc = get(conn, conn.last_insert_rowid())?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("incident.create", "incident", Some(inc.id)).after(json!({
            "kind": inc.kind,
            "tool": inc.asset_tag,
            "borrower": inc.borrower_name,
            "checkoutId": inc.checkout_id,
            "before": inc.before_condition,
            "after": inc.after_condition,
            "notes": inc.notes,
        })),
    )?;
    Ok(inc)
}

pub(crate) fn close(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    outcome: Outcome,
    note: &str,
) -> Result<Incident> {
    let before = get(conn, id)?;
    if before.status != IncidentStatus::Open {
        return Err(CoreError::Conflict(
            "This incident is already resolved".into(),
        ));
    }
    conn.execute(
        "UPDATE incidents SET status = 'resolved', outcome = ?1, resolution_note = ?2, resolved_at = ?3,
                resolved_by_tech = ?4
         WHERE id = ?5 AND status = 'open'",
        params![outcome, note, now, actor.tech_id(), id],
    )?;
    let after = get(conn, id)?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("incident.resolve", "incident", Some(id))
            .before(json!({ "status": before.status }))
            .after(json!({ "status": after.status, "outcome": outcome, "note": note })),
    )?;
    Ok(after)
}

/// Technician resolves an incident with an outcome and a note.
pub fn resolve(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    outcome: Outcome,
    note: &str,
) -> Result<Incident> {
    let note = note.split_whitespace().collect::<Vec<_>>().join(" ");
    if note.is_empty() {
        return invalid("Add a short note explaining the outcome");
    }
    if note.chars().count() > 1000 {
        return invalid("Note is too long");
    }
    close(conn, now, actor, id, outcome, &note)
}

/// Resolves every open incident of `kind` for a tool (e.g. when a lost tool
/// turns up). Returns how many were closed.
pub(crate) fn close_open_for_tool(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    tool_id: i64,
    kind: IncidentKind,
    outcome: Outcome,
    note: &str,
) -> Result<Vec<Incident>> {
    let ids: Vec<i64> = conn
        .prepare("SELECT id FROM incidents WHERE tool_id = ?1 AND kind = ?2 AND status = 'open' ORDER BY id")?
        .query_map(params![tool_id, kind], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    ids.into_iter()
        .map(|id| close(conn, now, actor, id, outcome, note))
        .collect()
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct IncidentFilter {
    pub status: Option<IncidentStatus>,
    pub kind: Option<IncidentKind>,
    pub borrower_id: Option<i64>,
    pub tool_id: Option<i64>,
    /// created_at >= from
    pub from: Option<i64>,
    /// created_at < to
    pub to: Option<i64>,
}

pub fn list(conn: &Connection, f: &IncidentFilter) -> Result<Vec<Incident>> {
    let mut st = conn.prepare(&format!(
        "{SELECT}
         WHERE (?1 IS NULL OR i.status = ?1) AND (?2 IS NULL OR i.kind = ?2)
           AND (?3 IS NULL OR i.borrower_id = ?3) AND (?4 IS NULL OR i.tool_id = ?4)
           AND (?5 IS NULL OR i.created_at >= ?5) AND (?6 IS NULL OR i.created_at < ?6)
         ORDER BY i.status = 'open' DESC, i.created_at DESC"
    ))?;
    let rows = st
        .query_map(
            params![f.status, f.kind, f.borrower_id, f.tool_id, f.from, f.to],
            map,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

impl crate::Service {
    pub fn list_incidents(&mut self, f: &IncidentFilter) -> Result<Vec<Incident>> {
        self.require_tech()?;
        list(&self.conn, f)
    }

    pub fn resolve_incident(&mut self, id: i64, outcome: Outcome, note: &str) -> Result<Incident> {
        self.tech_tx(|tx, now, actor| resolve(tx, now, actor, id, outcome, note))
    }

    pub fn open_incident_count(&self) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT count(*) FROM incidents WHERE status = 'open'",
            [],
            |r| r.get(0),
        )?)
    }
}
