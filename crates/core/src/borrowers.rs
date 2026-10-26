//! Borrowers (sales associates and other staff): who may take tools out.
//! Borrowers are never deleted, only deactivated, so history stays intact.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::audit::{self, Entry};
use crate::clock::DAY;
use crate::error::{CoreError, Result, invalid, not_found};
use crate::model::Actor;
use crate::pin;
use crate::technicians::{clean_name, finish_attempt};
use crate::tools::normalize_tag;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Borrower {
    pub id: i64,
    pub name: String,
    pub employee_id: String,
    pub badge_code: Option<String>,
    pub has_pin: bool,
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Borrower {
    pub fn actor(&self) -> Actor {
        Actor::Borrower {
            id: self.id,
            name: self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BorrowerInput {
    pub name: String,
    pub employee_id: String,
    pub badge_code: Option<String>,
}

const COLS: &str = "b.id, b.name, b.employee_id, b.badge_code, b.pin_hash IS NOT NULL, b.active, b.created_at, b.updated_at";

fn map(r: &rusqlite::Row<'_>) -> rusqlite::Result<Borrower> {
    Ok(Borrower {
        id: r.get(0)?,
        name: r.get(1)?,
        employee_id: r.get(2)?,
        badge_code: r.get(3)?,
        has_pin: r.get(4)?,
        active: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
    })
}

pub fn subject(id: i64) -> String {
    format!("borrower:{id}")
}

/// Badge codes are stored trimmed and upper-cased so scanner case doesn't matter.
pub fn normalize_badge(code: &str) -> String {
    code.trim().to_uppercase()
}

/// True for codes that look like a tool asset tag (T-0001), which would make
/// a scan ambiguous.
fn looks_like_tool_tag(code: &str) -> bool {
    let n = normalize_tag(code);
    n.len() >= 2 && n.starts_with('T') && n[1..].chars().all(|c| c.is_ascii_digit())
}

impl BorrowerInput {
    fn clean(&mut self, conn: &Connection, id: Option<i64>) -> Result<()> {
        self.name = clean_name(&self.name, "Name")?;
        self.employee_id = self.employee_id.trim().to_string();
        if self.employee_id.chars().count() > 40 {
            return invalid("Employee ID is too long");
        }
        self.badge_code = self
            .badge_code
            .as_deref()
            .map(normalize_badge)
            .filter(|s| !s.is_empty());
        if let Some(code) = &self.badge_code {
            if code.chars().count() > 64 {
                return invalid("Badge code is too long");
            }
            if looks_like_tool_tag(code) {
                return invalid("Badge codes can't look like tool tags (T-0001)");
            }
            let taken: Option<String> = conn
                .query_row(
                    "SELECT name FROM borrowers WHERE badge_code = ?1 AND id IS NOT ?2",
                    params![code, id],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(other) = taken {
                return Err(CoreError::Conflict(format!(
                    "That badge is already assigned to {other}"
                )));
            }
        }
        Ok(())
    }
}

pub fn get(conn: &Connection, id: i64) -> Result<Borrower> {
    conn.query_row(
        &format!("SELECT {COLS} FROM borrowers b WHERE b.id = ?1"),
        [id],
        map,
    )
    .optional()?
    .ok_or_else(|| not_found("Borrower"))
}

/// Finds an active or inactive borrower by badge code or employee ID.
pub fn find_by_code(conn: &Connection, code: &str) -> Result<Option<Borrower>> {
    let c = normalize_badge(code);
    if c.is_empty() {
        return Ok(None);
    }
    Ok(conn
        .query_row(
            &format!(
                "SELECT {COLS} FROM borrowers b
                 WHERE b.badge_code = ?1 OR (b.employee_id != '' AND upper(b.employee_id) = ?1)
                 ORDER BY b.badge_code = ?1 DESC LIMIT 1"
            ),
            [c],
            map,
        )
        .optional()?)
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BorrowerFilter {
    pub search: Option<String>,
    pub include_inactive: bool,
}

/// A borrower with loan counts, for the management list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowerRow {
    #[serde(flatten)]
    pub borrower: Borrower,
    pub open_loans: i64,
    pub overdue_loans: i64,
    pub last_checkout_at: Option<i64>,
}

pub fn list(conn: &Connection, now: i64, f: &BorrowerFilter) -> Result<Vec<BorrowerRow>> {
    let like = f
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase().replace(['%', '_'], "")));
    let mut st = conn.prepare(&format!(
        "SELECT {COLS},
                (SELECT count(*) FROM checkouts c WHERE c.borrower_id = b.id AND c.state IN ('out', 'awaiting_inspection')),
                (SELECT count(*) FROM checkouts c WHERE c.borrower_id = b.id AND c.state = 'out' AND c.due_at < ?3),
                (SELECT max(checked_out_at) FROM checkouts c WHERE c.borrower_id = b.id)
         FROM borrowers b
         WHERE (?1 IS NULL OR lower(b.name) LIKE ?1 OR lower(b.employee_id) LIKE ?1 OR lower(coalesce(b.badge_code, '')) LIKE ?1)
           AND (?2 OR b.active = 1)
         ORDER BY b.active DESC, b.name COLLATE NOCASE"
    ))?;
    let rows = st
        .query_map(params![like, f.include_inactive, now], |r| {
            Ok(BorrowerRow {
                borrower: map(r)?,
                open_loans: r.get(8)?,
                overdue_loans: r.get(9)?,
                last_checkout_at: r.get(10)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// What the checkout borrower picker shows: active borrowers, the ones who
/// borrowed most recently first (last 7 days), then the most frequent
/// (last 90 days), then everyone else alphabetically.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowerPick {
    pub id: i64,
    pub name: String,
    pub employee_id: String,
    pub has_pin: bool,
    pub last_checkout_at: Option<i64>,
    pub recent_checkouts: i64,
}

pub fn picker(conn: &Connection, now: i64) -> Result<Vec<BorrowerPick>> {
    let mut st = conn.prepare(
        "SELECT b.id, b.name, b.employee_id, b.pin_hash IS NOT NULL,
                (SELECT max(checked_out_at) FROM checkouts c WHERE c.borrower_id = b.id) AS last_at,
                (SELECT count(DISTINCT batch_id) FROM checkouts c WHERE c.borrower_id = b.id AND c.checked_out_at >= ?1) AS freq
         FROM borrowers b WHERE b.active = 1
         ORDER BY (last_at IS NOT NULL AND last_at >= ?2) DESC,
                  CASE WHEN last_at >= ?2 THEN last_at END DESC,
                  freq DESC, b.name COLLATE NOCASE",
    )?;
    let rows = st
        .query_map(params![now - 90 * DAY, now - 7 * DAY], |r| {
            Ok(BorrowerPick {
                id: r.get(0)?,
                name: r.get(1)?,
                employee_id: r.get(2)?,
                has_pin: r.get(3)?,
                last_checkout_at: r.get(4)?,
                recent_checkouts: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn snapshot(b: &Borrower) -> Value {
    json!({ "name": b.name, "employeeId": b.employee_id, "badgeCode": b.badge_code, "active": b.active })
}

pub fn create(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    mut input: BorrowerInput,
    pin_: Option<&str>,
    pin_required: bool,
) -> Result<Borrower> {
    input.clean(conn, None)?;
    let hash = match pin_.filter(|p| !p.is_empty()) {
        Some(p) => {
            pin::validate(p, pin::BORROWER_PIN_LEN)?;
            Some(pin::hash(p)?)
        }
        None if pin_required => return invalid("Choose a 4-digit PIN for this borrower"),
        None => None,
    };
    conn.execute(
        "INSERT INTO borrowers (name, employee_id, badge_code, pin_hash, active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
        params![input.name, input.employee_id, input.badge_code, hash, now],
    )?;
    let b = get(conn, conn.last_insert_rowid())?;
    let mut after = snapshot(&b);
    after["pinSet"] = json!(b.has_pin);
    audit::append(
        conn,
        now,
        actor,
        Entry::new("borrower.create", "borrower", Some(b.id)).after(after),
    )?;
    Ok(b)
}

pub fn update(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    mut input: BorrowerInput,
    active: bool,
) -> Result<Borrower> {
    let before = get(conn, id)?;
    input.clean(conn, Some(id))?;
    if before.active && !active {
        let open: i64 = conn.query_row(
            "SELECT count(*) FROM checkouts WHERE borrower_id = ?1 AND state = 'out'",
            [id],
            |r| r.get(0),
        )?;
        if open > 0 {
            return Err(CoreError::Conflict(format!(
                "{} still has {open} tool(s) out. Return them first.",
                before.name
            )));
        }
    }
    conn.execute(
        "UPDATE borrowers SET name = ?1, employee_id = ?2, badge_code = ?3, active = ?4, updated_at = ?5 WHERE id = ?6",
        params![input.name, input.employee_id, input.badge_code, active, now, id],
    )?;
    let after = get(conn, id)?;
    let (b, a) = (snapshot(&before), snapshot(&after));
    if b != a {
        audit::append(
            conn,
            now,
            actor,
            Entry::new("borrower.update", "borrower", Some(id))
                .before(b)
                .after(a),
        )?;
    }
    Ok(after)
}

/// Technician sets or resets a borrower's PIN (also clears a lockout).
pub fn set_pin(conn: &Connection, now: i64, actor: &Actor, id: i64, new_pin: &str) -> Result<()> {
    let b = get(conn, id)?;
    pin::validate(new_pin, pin::BORROWER_PIN_LEN)?;
    conn.execute(
        "UPDATE borrowers SET pin_hash = ?1, updated_at = ?2 WHERE id = ?3",
        params![pin::hash(new_pin)?, now, id],
    )?;
    pin::save(conn, &subject(id), &pin::LockState::default())?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("borrower.pin_reset", "borrower", Some(id))
            .after(json!({ "name": b.name, "hadPin": b.has_pin })),
    )?;
    Ok(())
}

/// Checks a borrower's PIN with lockout. Commits the attempt even if wrong.
pub fn check_pin(conn: &Connection, now: i64, id: i64, pin_: &str) -> Result<Borrower> {
    let b = get(conn, id)?;
    if !b.active {
        return invalid(format!("{} is inactive. Ask a technician.", b.name));
    }
    let hash: Option<String> =
        conn.query_row("SELECT pin_hash FROM borrowers WHERE id = ?1", [id], |r| {
            r.get(0)
        })?;
    let Some(hash) = hash else {
        return Err(CoreError::Conflict(format!(
            "{} has no PIN yet. A technician can set one in Borrowers.",
            b.name
        )));
    };
    let tx = conn.unchecked_transaction()?;
    let attempt = pin::attempt(&tx, &subject(id), now, || pin::verify(pin_, &hash))?;
    let res = finish_attempt(&tx, now, attempt, "borrower", Some(id), &b.name);
    tx.commit()?;
    res.map(|_| b)
}

// ---------------------------------------------------------------- service API

/// Proof that a borrower just confirmed with their PIN (or that PINs are off).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowerAuth {
    pub token: String,
    pub borrower: Borrower,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowerDetail {
    pub row: BorrowerRow,
    pub loans: Vec<crate::loans::LoanRow>,
    pub incidents: Vec<crate::incidents::Incident>,
}

impl crate::Service {
    pub fn borrower_detail(&mut self, id: i64) -> Result<BorrowerDetail> {
        self.require_tech()?;
        let now = self.now();
        let row = list(
            &self.conn,
            now,
            &BorrowerFilter {
                include_inactive: true,
                ..Default::default()
            },
        )?
        .into_iter()
        .find(|r| r.borrower.id == id)
        .ok_or_else(|| not_found("Borrower"))?;
        Ok(BorrowerDetail {
            row,
            loans: crate::loans::history(
                &self.conn,
                now,
                &crate::loans::LoanFilter {
                    borrower_id: Some(id),
                    limit: Some(200),
                    ..Default::default()
                },
            )?,
            incidents: crate::incidents::list(
                &self.conn,
                &crate::incidents::IncidentFilter {
                    borrower_id: Some(id),
                    ..Default::default()
                },
            )?,
        })
    }

    /// Names for the checkout picker (public).
    pub fn borrower_picker(&self) -> Result<Vec<BorrowerPick>> {
        picker(&self.conn, self.now())
    }

    pub fn list_borrowers(&mut self, f: &BorrowerFilter) -> Result<Vec<BorrowerRow>> {
        self.require_tech()?;
        list(&self.conn, self.now(), f)
    }

    pub fn get_borrower(&mut self, id: i64) -> Result<Borrower> {
        self.require_tech()?;
        get(&self.conn, id)
    }

    pub fn create_borrower(&mut self, input: BorrowerInput, pin: Option<&str>) -> Result<Borrower> {
        let required = self.settings.borrower_pin_required;
        self.tech_tx(|tx, now, actor| create(tx, now, actor, input, pin, required))
    }

    pub fn update_borrower(
        &mut self,
        id: i64,
        input: BorrowerInput,
        active: bool,
    ) -> Result<Borrower> {
        self.tech_tx(|tx, now, actor| update(tx, now, actor, id, input, active))
    }

    pub fn set_borrower_pin(&mut self, id: i64, pin: &str) -> Result<()> {
        self.tech_tx(|tx, now, actor| set_pin(tx, now, actor, id, pin))
    }

    /// Step 1 of checkout / drop-off: the borrower confirms it's them.
    /// Returns a short-lived token the following steps must present.
    pub fn authenticate_borrower(&mut self, id: i64, pin: Option<&str>) -> Result<BorrowerAuth> {
        let now = self.now();
        let borrower = if self.settings.borrower_pin_required {
            let p = pin
                .filter(|p| !p.is_empty())
                .ok_or_else(|| CoreError::Invalid("Enter your 4-digit PIN".into()))?;
            check_pin(&self.conn, now, id, p)?
        } else {
            let b = get(&self.conn, id)?;
            if !b.active {
                return invalid(format!("{} is inactive. Ask a technician.", b.name));
            }
            b
        };
        let token = self.sessions.issue_borrower_token(borrower.id, now)?;
        Ok(BorrowerAuth { token, borrower })
    }

    /// The borrower behind a still-valid confirmation token.
    pub(crate) fn borrower_from_token(&self, token: &str) -> Result<Borrower> {
        let id = self.sessions.borrower_for_token(token, self.now())?;
        let b = get(&self.conn, id)?;
        if !b.active {
            return invalid(format!("{} is inactive", b.name));
        }
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::MINUTE;
    use crate::service::tests::ready;

    fn input(name: &str, badge: Option<&str>) -> BorrowerInput {
        BorrowerInput {
            name: name.into(),
            employee_id: String::new(),
            badge_code: badge.map(Into::into),
        }
    }

    #[test]
    fn create_requires_pin_when_setting_is_on() {
        let (mut s, _c, _r) = ready();
        assert!(s.create_borrower(input("Sam", None), None).is_err());
        assert!(
            s.create_borrower(input("Sam", None), Some("12345"))
                .is_err()
        );
        let b = s.create_borrower(input("Sam", None), Some("4321")).unwrap();
        assert!(b.has_pin);
        let mut st = s.settings().clone();
        st.borrower_pin_required = false;
        s.update_settings(st).unwrap();
        let nopin = s.create_borrower(input("Kim", None), None).unwrap();
        assert!(!nopin.has_pin);
    }

    #[test]
    fn badge_codes_are_normalized_unique_and_not_tool_like() {
        let (mut s, _c, _r) = ready();
        let a = s
            .create_borrower(input("Sam", Some("  emp-77a ")), Some("1111"))
            .unwrap();
        assert_eq!(a.badge_code.as_deref(), Some("EMP-77A"));
        assert_eq!(
            s.create_borrower(input("Kim", Some("EMP-77a")), Some("2222"))
                .unwrap_err()
                .code(),
            "conflict"
        );
        assert!(
            s.create_borrower(input("Kim", Some("t-0042")), Some("2222"))
                .is_err()
        );
        assert_eq!(find_by_code(s.conn(), "emp-77A").unwrap().unwrap().id, a.id);
        // Employee IDs scan too.
        let mut i = input("Lee", None);
        i.employee_id = "E1009".into();
        let lee = s.create_borrower(i, Some("3333")).unwrap();
        assert_eq!(find_by_code(s.conn(), "e1009").unwrap().unwrap().id, lee.id);
    }

    #[test]
    fn authenticate_issues_token_and_enforces_lockout() {
        let (mut s, c, _r) = ready();
        let b = s.create_borrower(input("Sam", None), Some("4321")).unwrap();
        s.lock(); // borrowers authenticate without tech mode
        assert_eq!(
            s.authenticate_borrower(b.id, None).unwrap_err().code(),
            "invalid"
        );
        let auth = s.authenticate_borrower(b.id, Some("4321")).unwrap();
        assert_eq!(s.borrower_from_token(&auth.token).unwrap().id, b.id);
        for _ in 0..4 {
            assert_eq!(
                s.authenticate_borrower(b.id, Some("0000"))
                    .unwrap_err()
                    .code(),
                "wrong_pin"
            );
        }
        assert_eq!(
            s.authenticate_borrower(b.id, Some("0000"))
                .unwrap_err()
                .code(),
            "locked_out"
        );
        assert_eq!(
            s.authenticate_borrower(b.id, Some("4321"))
                .unwrap_err()
                .code(),
            "locked_out"
        );
        c.advance(MINUTE);
        assert!(s.authenticate_borrower(b.id, Some("4321")).is_ok());
    }

    #[test]
    fn tech_can_reset_pin_which_clears_lockout() {
        let (mut s, _c, _r) = ready();
        let b = s.create_borrower(input("Sam", None), Some("4321")).unwrap();
        for _ in 0..5 {
            let _ = s.authenticate_borrower(b.id, Some("0000"));
        }
        s.set_borrower_pin(b.id, "9876").unwrap();
        assert!(s.authenticate_borrower(b.id, Some("9876")).is_ok());
        let log = audit::list(s.conn(), &Default::default()).unwrap();
        assert!(
            log.iter()
                .any(|e| e.action == "borrower.pin_reset" && e.actor_name == "Ana")
        );
        assert!(
            !log.iter()
                .any(|e| e.after_json.as_deref().unwrap_or("").contains("argon2"))
        );
    }

    #[test]
    fn inactive_borrowers_cannot_authenticate() {
        let (mut s, _c, _r) = ready();
        let b = s.create_borrower(input("Sam", None), Some("4321")).unwrap();
        s.update_borrower(b.id, input("Sam", None), false).unwrap();
        assert!(s.authenticate_borrower(b.id, Some("4321")).is_err());
        assert!(s.borrower_picker().unwrap().is_empty());
    }

    #[test]
    fn management_needs_tech_mode() {
        let (mut s, _c, _r) = ready();
        s.lock();
        assert_eq!(
            s.create_borrower(input("Sam", None), Some("4321"))
                .unwrap_err()
                .code(),
            "tech_required"
        );
        assert_eq!(
            s.list_borrowers(&Default::default()).unwrap_err().code(),
            "tech_required"
        );
        assert!(s.borrower_picker().is_ok());
    }
}
