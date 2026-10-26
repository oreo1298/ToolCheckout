//! Tools: CRUD, asset tags, condition history and technician status actions.
//! Loans (checkout/return/lost) live in `loans`.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::audit::{self, Entry};
use crate::error::{CoreError, Result, invalid, not_found};
use crate::model::{Actor, Condition, ConditionSource, ToolStatus};
use crate::{photos, settings, status};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub id: i64,
    pub asset_tag: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub serial: String,
    pub model: String,
    pub photo_id: Option<i64>,
    pub replacement_cost_cents: Option<i64>,
    pub home_location: String,
    pub default_loan_minutes: i64,
    pub tech_only: bool,
    pub condition: Condition,
    pub condition_notes: String,
    pub status: ToolStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

pub(crate) const COLS: &str =
    "t.id, t.asset_tag, t.name, t.category, t.description, t.serial, t.model,
    t.photo_id, t.replacement_cost_cents, t.home_location, t.default_loan_minutes, t.tech_only,
    t.condition, t.condition_notes, t.status, t.created_at, t.updated_at";
pub(crate) const NCOLS: usize = 17;

pub(crate) fn map(r: &rusqlite::Row<'_>) -> rusqlite::Result<Tool> {
    Ok(Tool {
        id: r.get(0)?,
        asset_tag: r.get(1)?,
        name: r.get(2)?,
        category: r.get(3)?,
        description: r.get(4)?,
        serial: r.get(5)?,
        model: r.get(6)?,
        photo_id: r.get(7)?,
        replacement_cost_cents: r.get(8)?,
        home_location: r.get(9)?,
        default_loan_minutes: r.get(10)?,
        tech_only: r.get(11)?,
        condition: r.get(12)?,
        condition_notes: r.get(13)?,
        status: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
    })
}

/// A tool plus who has it, for lists and the home screen.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRow {
    #[serde(flatten)]
    pub tool: Tool,
    pub checkout_id: Option<i64>,
    pub borrower_id: Option<i64>,
    pub borrower_name: Option<String>,
    pub checked_out_at: Option<i64>,
    pub due_at: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ToolInput {
    pub name: String,
    pub category: String,
    pub description: String,
    pub serial: String,
    pub model: String,
    pub photo_id: Option<i64>,
    pub replacement_cost_cents: Option<i64>,
    pub home_location: String,
    pub default_loan_minutes: i64,
    pub tech_only: bool,
    /// Required when adding a tool; ignored by edits (use a condition update).
    pub condition: Option<Condition>,
    pub condition_notes: String,
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn limit(s: &str, max: usize, what: &str) -> Result<()> {
    if s.chars().count() > max {
        return invalid(format!("{what} is too long (max {max} characters)"));
    }
    Ok(())
}

impl ToolInput {
    fn clean(&mut self, conn: &Connection) -> Result<()> {
        self.name = squash(&self.name);
        self.category = squash(&self.category);
        self.serial = self.serial.trim().to_string();
        self.model = squash(&self.model);
        self.home_location = squash(&self.home_location);
        self.description = self.description.trim().to_string();
        self.condition_notes = self.condition_notes.trim().to_string();
        if self.name.is_empty() {
            return invalid("Tool name is required");
        }
        limit(&self.name, 120, "Name")?;
        limit(&self.category, 60, "Category")?;
        limit(&self.serial, 80, "Serial")?;
        limit(&self.model, 80, "Model")?;
        limit(&self.home_location, 120, "Home location")?;
        limit(&self.description, 2000, "Description")?;
        limit(&self.condition_notes, 2000, "Condition notes")?;
        if !(5..=60 * 24 * 90).contains(&self.default_loan_minutes) {
            return invalid("Loan period must be between 5 minutes and 90 days");
        }
        if let Some(c) = self.replacement_cost_cents
            && !(0..=100_000_000).contains(&c)
        {
            return invalid("Replacement cost looks wrong");
        }
        if let Some(p) = self.photo_id
            && !photos::exists(conn, p)?
        {
            return invalid("That photo no longer exists");
        }
        Ok(())
    }
}

pub fn get(conn: &Connection, id: i64) -> Result<Tool> {
    conn.query_row(
        &format!("SELECT {COLS} FROM tools t WHERE t.id = ?1"),
        [id],
        map,
    )
    .optional()?
    .ok_or_else(|| not_found("Tool"))
}

/// Normalizes a scanned or typed asset tag ("t-0001", "T0001", "T–0001").
pub fn normalize_tag(code: &str) -> String {
    code.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

pub fn find_by_tag(conn: &Connection, code: &str) -> Result<Option<Tool>> {
    let norm = normalize_tag(code);
    if norm.is_empty() {
        return Ok(None);
    }
    Ok(conn
        .query_row(
            &format!("SELECT {COLS} FROM tools t WHERE replace(upper(t.asset_tag), '-', '') = ?1"),
            [norm],
            map,
        )
        .optional()?)
}

/// Next free tag: T-0001, T-0002, … Tags are never reused, even after retiring.
fn next_asset_tag(conn: &Connection) -> Result<String> {
    let mut n: i64 = settings::get_sys(conn, "next_asset_number")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    loop {
        let tag = format!("T-{n:04}");
        n += 1;
        if find_by_tag(conn, &tag)?.is_none() {
            settings::set_sys(conn, "next_asset_number", &n.to_string())?;
            return Ok(tag);
        }
    }
}

/// Appends to the condition history and updates the tool's current condition.
#[allow(clippy::too_many_arguments)]
pub(crate) fn record_condition(
    conn: &Connection,
    now: i64,
    tool_id: i64,
    condition: Condition,
    notes: &str,
    photo_id: Option<i64>,
    reason: &str,
    source: ConditionSource,
    checkout_id: Option<i64>,
    tech_id: Option<i64>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO condition_events (tool_id, condition, notes, photo_id, reason, source, checkout_id, tech_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![tool_id, condition, notes, photo_id, reason, source, checkout_id, tech_id, now],
    )?;
    let ev = conn.last_insert_rowid();
    conn.execute(
        "UPDATE tools SET condition = ?1, condition_notes = ?2, updated_at = ?3 WHERE id = ?4",
        params![condition, notes, now, tool_id],
    )?;
    Ok(ev)
}

/// Changes status if the transition is allowed and nobody changed it meanwhile.
pub(crate) fn set_status(
    conn: &Connection,
    now: i64,
    tool_id: i64,
    from: ToolStatus,
    to: ToolStatus,
) -> Result<()> {
    status::check(from, to)?;
    let n = conn.execute(
        "UPDATE tools SET status = ?1, updated_at = ?2 WHERE id = ?3 AND status = ?4",
        params![to, now, tool_id, from],
    )?;
    if n == 0 {
        return Err(CoreError::Conflict(
            "This tool was changed by someone else. Refresh and try again.".into(),
        ));
    }
    Ok(())
}

fn input_json(t: &Tool) -> Value {
    json!({
        "name": t.name,
        "category": t.category,
        "description": t.description,
        "serial": t.serial,
        "model": t.model,
        "photoId": t.photo_id,
        "replacementCostCents": t.replacement_cost_cents,
        "homeLocation": t.home_location,
        "defaultLoanMinutes": t.default_loan_minutes,
        "techOnly": t.tech_only,
    })
}

pub fn create(conn: &Connection, now: i64, actor: &Actor, mut input: ToolInput) -> Result<Tool> {
    input.clean(conn)?;
    let Some(condition) = input.condition else {
        return invalid("Choose the tool's current condition");
    };
    let tag = next_asset_tag(conn)?;
    conn.execute(
        "INSERT INTO tools (asset_tag, name, category, description, serial, model, photo_id,
                            replacement_cost_cents, home_location, default_loan_minutes, tech_only,
                            condition, condition_notes, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'available', ?14, ?14)",
        params![
            tag,
            input.name,
            input.category,
            input.description,
            input.serial,
            input.model,
            input.photo_id,
            input.replacement_cost_cents,
            input.home_location,
            input.default_loan_minutes,
            input.tech_only,
            condition,
            input.condition_notes,
            now
        ],
    )?;
    let id = conn.last_insert_rowid();
    record_condition(
        conn,
        now,
        id,
        condition,
        &input.condition_notes,
        input.photo_id,
        "Added to inventory",
        ConditionSource::Initial,
        None,
        actor.tech_id(),
    )?;
    let tool = get(conn, id)?;
    let mut after = input_json(&tool);
    after["assetTag"] = json!(tool.asset_tag);
    after["condition"] = json!(condition);
    audit::append(
        conn,
        now,
        actor,
        Entry::new("tool.create", "tool", Some(id)).after(after),
    )?;
    Ok(tool)
}

/// Edits descriptive fields. Condition and status have their own actions so
/// that each change carries a reason and lands in the history.
pub fn update(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    mut input: ToolInput,
) -> Result<Tool> {
    let before = get(conn, id)?;
    input.clean(conn)?;
    conn.execute(
        "UPDATE tools SET name = ?1, category = ?2, description = ?3, serial = ?4, model = ?5, photo_id = ?6,
                replacement_cost_cents = ?7, home_location = ?8, default_loan_minutes = ?9, tech_only = ?10,
                updated_at = ?11
         WHERE id = ?12",
        params![
            input.name,
            input.category,
            input.description,
            input.serial,
            input.model,
            input.photo_id,
            input.replacement_cost_cents,
            input.home_location,
            input.default_loan_minutes,
            input.tech_only,
            now,
            id
        ],
    )?;
    let after = get(conn, id)?;
    let (b, a) = (input_json(&before), input_json(&after));
    let (mut bm, mut am) = (Map::new(), Map::new());
    for (k, av) in a.as_object().unwrap() {
        let bv = &b[k];
        if bv != av {
            bm.insert(k.clone(), bv.clone());
            am.insert(k.clone(), av.clone());
        }
    }
    if !am.is_empty() {
        audit::append(
            conn,
            now,
            actor,
            Entry::new("tool.update", "tool", Some(id))
                .before(Value::Object(bm))
                .after(Value::Object(am)),
        )?;
    }
    Ok(after)
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ToolFilter {
    pub search: Option<String>,
    pub status: Option<ToolStatus>,
    pub category: Option<String>,
    pub include_retired: bool,
    pub limit: Option<i64>,
    /// Exactly this tool (any status).
    pub id: Option<i64>,
}

pub fn list(conn: &Connection, f: &ToolFilter) -> Result<Vec<ToolRow>> {
    let search = f
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase());
    let like = search
        .as_ref()
        .map(|s| format!("%{}%", s.replace(['%', '_'], "")));
    let tag = search.as_deref().map(normalize_tag);
    let mut st = conn.prepare(&format!(
        "SELECT {COLS}, c.id, b.id, b.name, c.checked_out_at, c.due_at
         FROM tools t
         LEFT JOIN checkouts c ON c.tool_id = t.id AND c.state IN ('out', 'awaiting_inspection')
         LEFT JOIN borrowers b ON b.id = c.borrower_id
         WHERE (?1 IS NULL OR lower(t.name) LIKE ?1 OR lower(t.asset_tag) LIKE ?1 OR lower(t.category) LIKE ?1
                OR lower(t.serial) LIKE ?1 OR lower(t.model) LIKE ?1 OR lower(t.home_location) LIKE ?1
                OR lower(b.name) LIKE ?1 OR replace(upper(t.asset_tag), '-', '') = ?2)
           AND (?3 IS NULL OR t.status = ?3)
           AND (?4 IS NULL OR t.category = ?4)
           AND (?5 OR t.status != 'retired' OR ?3 = 'retired' OR ?7 IS NOT NULL)
           AND (?7 IS NULL OR t.id = ?7)
         ORDER BY (replace(upper(t.asset_tag), '-', '') = ?2) DESC, t.name COLLATE NOCASE, t.asset_tag
         LIMIT ?6"
    ))?;
    let rows = st
        .query_map(
            params![
                like,
                tag,
                f.status,
                f.category,
                f.include_retired,
                f.limit.unwrap_or(500),
                f.id
            ],
            |r| {
                Ok(ToolRow {
                    tool: map(r)?,
                    checkout_id: r.get(NCOLS)?,
                    borrower_id: r.get(NCOLS + 1)?,
                    borrower_name: r.get(NCOLS + 2)?,
                    checked_out_at: r.get(NCOLS + 3)?,
                    due_at: r.get(NCOLS + 4)?,
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn categories(conn: &Connection) -> Result<Vec<String>> {
    let mut st = conn.prepare(
        "SELECT DISTINCT category FROM tools WHERE category != '' ORDER BY category COLLATE NOCASE",
    )?;
    Ok(st
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

fn require_reason(reason: &str) -> Result<String> {
    let r = squash(reason);
    if r.is_empty() {
        return invalid("Please give a reason");
    }
    limit(&r, 500, "Reason")?;
    Ok(r)
}

/// Technician changes a tool's condition from its page (reason required).
#[allow(clippy::too_many_arguments)]
pub fn update_condition(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    condition: Condition,
    notes: &str,
    photo_id: Option<i64>,
    reason: &str,
) -> Result<Tool> {
    let tool = get(conn, id)?;
    let reason = require_reason(reason)?;
    match tool.status {
        ToolStatus::Available | ToolStatus::OutForRepair => {}
        ToolStatus::CheckedOut | ToolStatus::AwaitingInspection => {
            return Err(CoreError::Conflict(
                "This tool is on loan. Its condition is set when it's returned and inspected."
                    .into(),
            ));
        }
        ToolStatus::Lost | ToolStatus::Retired => {
            return Err(CoreError::Conflict(format!(
                "This tool is {}. Mark it found or reinstate it first.",
                tool.status.label().to_lowercase()
            )));
        }
    }
    let notes = notes.trim();
    limit(notes, 2000, "Notes")?;
    record_condition(
        conn,
        now,
        id,
        condition,
        notes,
        photo_id,
        &reason,
        ConditionSource::Manual,
        None,
        actor.tech_id(),
    )?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("tool.condition", "tool", Some(id))
            .before(json!({ "condition": tool.condition, "notes": tool.condition_notes }))
            .after(json!({ "condition": condition, "notes": notes, "reason": reason })),
    )?;
    get(conn, id)
}

pub fn send_to_repair(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    note: &str,
) -> Result<Tool> {
    let tool = get(conn, id)?;
    let note = require_reason(note)?;
    set_status(conn, now, id, tool.status, ToolStatus::OutForRepair)?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("tool.repair_out", "tool", Some(id))
            .before(json!({ "status": tool.status }))
            .after(json!({ "status": ToolStatus::OutForRepair, "note": note })),
    )?;
    get(conn, id)
}

pub fn back_from_repair(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    condition: Condition,
    notes: &str,
    photo_id: Option<i64>,
) -> Result<Tool> {
    let tool = get(conn, id)?;
    if tool.status != ToolStatus::OutForRepair {
        return Err(CoreError::InvalidTransition {
            from: tool.status,
            to: ToolStatus::Available,
        });
    }
    if condition.needs_repair() {
        return invalid("Still damaged or broken? Keep it out for repair, or retire it.");
    }
    set_status(conn, now, id, tool.status, ToolStatus::Available)?;
    record_condition(
        conn,
        now,
        id,
        condition,
        notes.trim(),
        photo_id,
        "Back from repair",
        ConditionSource::Repair,
        None,
        actor.tech_id(),
    )?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("tool.repair_back", "tool", Some(id))
            .before(json!({ "status": tool.status, "condition": tool.condition }))
            .after(json!({ "status": ToolStatus::Available, "condition": condition })),
    )?;
    get(conn, id)
}

pub fn retire(conn: &Connection, now: i64, actor: &Actor, id: i64, reason: &str) -> Result<Tool> {
    let tool = get(conn, id)?;
    let reason = require_reason(reason)?;
    set_status(conn, now, id, tool.status, ToolStatus::Retired)?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("tool.retire", "tool", Some(id))
            .before(json!({ "status": tool.status }))
            .after(json!({ "status": ToolStatus::Retired, "reason": reason })),
    )?;
    get(conn, id)
}

pub fn reinstate(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    condition: Condition,
    notes: &str,
) -> Result<Tool> {
    let tool = get(conn, id)?;
    if condition.needs_repair() {
        return invalid("A damaged or broken tool can't go back into service");
    }
    set_status(conn, now, id, tool.status, ToolStatus::Available)?;
    record_condition(
        conn,
        now,
        id,
        condition,
        notes.trim(),
        None,
        "Reinstated",
        ConditionSource::Manual,
        None,
        actor.tech_id(),
    )?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("tool.reinstate", "tool", Some(id))
            .before(json!({ "status": tool.status }))
            .after(json!({ "status": ToolStatus::Available, "condition": condition })),
    )?;
    get(conn, id)
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionEvent {
    pub id: i64,
    pub tool_id: i64,
    pub condition: Condition,
    pub notes: String,
    pub photo_id: Option<i64>,
    pub reason: String,
    pub source: ConditionSource,
    pub checkout_id: Option<i64>,
    pub tech_name: Option<String>,
    pub created_at: i64,
}

pub fn condition_history(conn: &Connection, tool_id: i64) -> Result<Vec<ConditionEvent>> {
    let mut st = conn.prepare(
        "SELECT e.id, e.tool_id, e.condition, e.notes, e.photo_id, e.reason, e.source, e.checkout_id,
                t.name, e.created_at
         FROM condition_events e LEFT JOIN technicians t ON t.id = e.tech_id
         WHERE e.tool_id = ?1 ORDER BY e.id DESC",
    )?;
    let rows = st
        .query_map([tool_id], |r| {
            Ok(ConditionEvent {
                id: r.get(0)?,
                tool_id: r.get(1)?,
                condition: r.get(2)?,
                notes: r.get(3)?,
                photo_id: r.get(4)?,
                reason: r.get(5)?,
                source: r.get(6)?,
                checkout_id: r.get(7)?,
                tech_name: r.get(8)?,
                created_at: r.get(9)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db;

    pub fn tech() -> Actor {
        Actor::Technician {
            id: 1,
            name: "Ana".into(),
        }
    }

    pub fn conn_with_tech() -> Connection {
        let conn = db::open_in_memory().unwrap();
        crate::technicians::create(&conn, 0, &Actor::System, "Ana", "123456").unwrap();
        conn
    }

    pub fn input(name: &str) -> ToolInput {
        ToolInput {
            name: name.into(),
            category: "Drivers".into(),
            default_loan_minutes: 240,
            condition: Some(Condition::Good),
            replacement_cost_cents: Some(2499),
            ..Default::default()
        }
    }

    #[test]
    fn create_assigns_sequential_tags_and_initial_condition() {
        let conn = conn_with_tech();
        let a = create(&conn, 10, &tech(), input("  iFixit   Pro Tech Kit ")).unwrap();
        let b = create(&conn, 11, &tech(), input("Spudger")).unwrap();
        assert_eq!(a.asset_tag, "T-0001");
        assert_eq!(b.asset_tag, "T-0002");
        assert_eq!(a.name, "iFixit Pro Tech Kit");
        assert_eq!(a.status, ToolStatus::Available);
        let h = condition_history(&conn, a.id).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].source, ConditionSource::Initial);
        assert_eq!(h[0].tech_name.as_deref(), Some("Ana"));
    }

    #[test]
    fn create_requires_condition_and_valid_fields() {
        let conn = conn_with_tech();
        let mut i = input("X");
        i.condition = None;
        assert!(create(&conn, 1, &tech(), i).is_err());
        let mut i = input(" ");
        i.condition = Some(Condition::New);
        assert!(create(&conn, 1, &tech(), i).is_err());
        let mut i = input("X");
        i.default_loan_minutes = 0;
        assert!(create(&conn, 1, &tech(), i).is_err());
        let mut i = input("X");
        i.photo_id = Some(999);
        assert!(create(&conn, 1, &tech(), i).is_err());
    }

    #[test]
    fn tags_are_never_reused_and_lookup_is_forgiving() {
        let conn = conn_with_tech();
        let a = create(&conn, 1, &tech(), input("A")).unwrap();
        retire(&conn, 2, &tech(), a.id, "worn out").unwrap();
        let b = create(&conn, 3, &tech(), input("B")).unwrap();
        assert_eq!(b.asset_tag, "T-0002");
        for code in ["T-0002", "t-0002", "T0002", " t 0002 "] {
            assert_eq!(
                find_by_tag(&conn, code).unwrap().unwrap().id,
                b.id,
                "{code}"
            );
        }
        assert!(find_by_tag(&conn, "T-9999").unwrap().is_none());
    }

    #[test]
    fn update_audits_only_changed_fields_and_keeps_condition() {
        let conn = conn_with_tech();
        let t = create(&conn, 1, &tech(), input("Driver")).unwrap();
        let mut i = input("Driver set");
        i.condition = Some(Condition::Broken); // ignored by edits
        let u = update(&conn, 2, &tech(), t.id, i).unwrap();
        assert_eq!(u.condition, Condition::Good);
        let log = audit::list(&conn, &Default::default()).unwrap();
        assert_eq!(log[0].action, "tool.update");
        assert_eq!(
            log[0].after_json.as_deref(),
            Some(r#"{"name":"Driver set"}"#)
        );
    }

    #[test]
    fn condition_updates_need_reason_and_append_history() {
        let conn = conn_with_tech();
        let t = create(&conn, 1, &tech(), input("Meter")).unwrap();
        assert!(update_condition(&conn, 2, &tech(), t.id, Condition::Worn, "", None, " ").is_err());
        let u = update_condition(
            &conn,
            3,
            &tech(),
            t.id,
            Condition::Worn,
            "probe tips frayed",
            None,
            "Monthly check",
        )
        .unwrap();
        assert_eq!(u.condition, Condition::Worn);
        assert_eq!(u.condition_notes, "probe tips frayed");
        let h = condition_history(&conn, t.id).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].reason, "Monthly check");
        assert_eq!(
            h[1].condition,
            Condition::Good,
            "history is never overwritten"
        );
    }

    #[test]
    fn repair_cycle_and_retirement() {
        let conn = conn_with_tech();
        let t = create(&conn, 1, &tech(), input("Iron")).unwrap();
        let r = send_to_repair(&conn, 2, &tech(), t.id, "tip loose").unwrap();
        assert_eq!(r.status, ToolStatus::OutForRepair);
        assert!(back_from_repair(&conn, 3, &tech(), t.id, Condition::Broken, "", None).is_err());
        let b =
            back_from_repair(&conn, 4, &tech(), t.id, Condition::Good, "new tip", None).unwrap();
        assert_eq!(b.status, ToolStatus::Available);
        let ret = retire(&conn, 5, &tech(), t.id, "replaced").unwrap();
        assert_eq!(ret.status, ToolStatus::Retired);
        // Retired tools can't be repaired or have conditions changed.
        assert_eq!(
            send_to_repair(&conn, 6, &tech(), t.id, "x")
                .unwrap_err()
                .code(),
            "invalid_transition"
        );
        assert_eq!(
            update_condition(&conn, 6, &tech(), t.id, Condition::Good, "", None, "x")
                .unwrap_err()
                .code(),
            "conflict"
        );
        let back = reinstate(&conn, 7, &tech(), t.id, Condition::Fair, "found a use").unwrap();
        assert_eq!(back.status, ToolStatus::Available);
        assert!(audit::verify(&conn).unwrap().ok);
    }

    #[test]
    fn list_filters_and_searches() {
        let conn = conn_with_tech();
        create(&conn, 1, &tech(), input("Phillips #00")).unwrap();
        let mut i = input("Multimeter");
        i.category = "Meters".into();
        i.home_location = "Bench 2, top drawer".into();
        let m = create(&conn, 2, &tech(), i).unwrap();
        let old = create(&conn, 3, &tech(), input("Old spudger")).unwrap();
        retire(&conn, 4, &tech(), old.id, "snapped").unwrap();

        let all = list(&conn, &ToolFilter::default()).unwrap();
        assert_eq!(all.len(), 2, "retired hidden by default");
        let f = |s: &str| {
            list(
                &conn,
                &ToolFilter {
                    search: Some(s.into()),
                    ..Default::default()
                },
            )
            .unwrap()
        };
        assert_eq!(f("bench 2")[0].tool.id, m.id);
        assert_eq!(f("t0002")[0].tool.id, m.id);
        assert_eq!(f("meters").len(), 1);
        let retired = list(
            &conn,
            &ToolFilter {
                status: Some(ToolStatus::Retired),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(retired.len(), 1);
        assert_eq!(categories(&conn).unwrap(), vec!["Drivers", "Meters"]);
    }

    #[test]
    fn status_changes_are_guarded_against_races() {
        let conn = conn_with_tech();
        let t = create(&conn, 1, &tech(), input("A")).unwrap();
        // Someone else already moved it.
        set_status(
            &conn,
            2,
            t.id,
            ToolStatus::Available,
            ToolStatus::OutForRepair,
        )
        .unwrap();
        assert_eq!(
            set_status(&conn, 3, t.id, ToolStatus::Available, ToolStatus::Retired)
                .unwrap_err()
                .code(),
            "conflict"
        );
    }
}

// ---------------------------------------------------------------- service API

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDetail {
    pub row: ToolRow,
    pub conditions: Vec<ConditionEvent>,
    pub loans: Vec<crate::loans::LoanRow>,
    pub incidents: Vec<crate::incidents::Incident>,
}

impl crate::Service {
    /// Tool list/search. Public: the checkout and return pickers use it.
    pub fn list_tools(&self, f: &ToolFilter) -> Result<Vec<ToolRow>> {
        list(&self.conn, f)
    }

    pub fn tool_row(&self, id: i64) -> Result<ToolRow> {
        list(
            &self.conn,
            &ToolFilter {
                id: Some(id),
                ..Default::default()
            },
        )?
        .into_iter()
        .next()
        .ok_or_else(|| not_found("Tool"))
    }

    pub fn tool_categories(&self) -> Result<Vec<String>> {
        categories(&self.conn)
    }

    pub fn tool_detail(&mut self, id: i64) -> Result<ToolDetail> {
        self.require_tech()?;
        let now = self.now();
        Ok(ToolDetail {
            row: self.tool_row(id)?,
            conditions: condition_history(&self.conn, id)?,
            loans: crate::loans::history(
                &self.conn,
                now,
                &crate::loans::LoanFilter {
                    tool_id: Some(id),
                    limit: Some(200),
                    ..Default::default()
                },
            )?,
            incidents: crate::incidents::list(
                &self.conn,
                &crate::incidents::IncidentFilter {
                    tool_id: Some(id),
                    ..Default::default()
                },
            )?,
        })
    }

    pub fn create_tool(&mut self, input: ToolInput) -> Result<Tool> {
        self.tech_tx(|tx, now, actor| create(tx, now, actor, input))
    }

    pub fn update_tool(&mut self, id: i64, input: ToolInput) -> Result<Tool> {
        self.tech_tx(|tx, now, actor| update(tx, now, actor, id, input))
    }

    pub fn update_tool_condition(
        &mut self,
        id: i64,
        condition: Condition,
        notes: &str,
        photo_id: Option<i64>,
        reason: &str,
    ) -> Result<Tool> {
        self.tech_tx(|tx, now, actor| {
            update_condition(tx, now, actor, id, condition, notes, photo_id, reason)
        })
    }

    pub fn send_tool_to_repair(&mut self, id: i64, note: &str) -> Result<Tool> {
        self.tech_tx(|tx, now, actor| send_to_repair(tx, now, actor, id, note))
    }

    pub fn tool_back_from_repair(
        &mut self,
        id: i64,
        condition: Condition,
        notes: &str,
        photo_id: Option<i64>,
    ) -> Result<Tool> {
        self.tech_tx(|tx, now, actor| {
            back_from_repair(tx, now, actor, id, condition, notes, photo_id)
        })
    }

    pub fn retire_tool(&mut self, id: i64, reason: &str) -> Result<Tool> {
        self.tech_tx(|tx, now, actor| retire(tx, now, actor, id, reason))
    }

    pub fn reinstate_tool(&mut self, id: i64, condition: Condition, notes: &str) -> Result<Tool> {
        self.tech_tx(|tx, now, actor| reinstate(tx, now, actor, id, condition, notes))
    }

    /// Stores a photo (tool picture, condition evidence). Anyone may attach
    /// one: borrowers add photos to problem reports.
    pub fn save_photo(&mut self, bytes: &[u8]) -> Result<i64> {
        self.tx(|tx, now| photos::save(tx, now, bytes))
    }

    pub fn photo(&self, id: i64) -> Result<(String, Vec<u8>)> {
        photos::get(&self.conn, id)
    }
}
