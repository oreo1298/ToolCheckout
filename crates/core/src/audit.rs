//! Append-only, hash-chained audit log.
//!
//! Each entry stores `hash = SHA-256(domain ‖ fields ‖ prev_hash)`, where every
//! field is length-prefixed so no two different entries can serialize to the
//! same bytes. Changing, inserting or deleting any entry breaks every hash after
//! it, which [`verify`] reports. This makes tampering *evident*, not
//! impossible: someone with write access to the file could rewrite the whole
//! chain from the tampered point on. Comparing the chain head with an older
//! copy (e.g. a backup) catches that; see [`verify_prefix_matches`].

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::model::Actor;

pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const DOMAIN: &[u8] = b"toolcheckout-audit-v1";

/// What to record. `before`/`after` hold only the fields that matter for the
/// action (e.g. changed settings, old/new status).
pub struct Entry<'a> {
    pub action: &'a str,
    pub entity_kind: &'a str,
    pub entity_id: Option<i64>,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

impl<'a> Entry<'a> {
    pub fn new(action: &'a str, entity_kind: &'a str, entity_id: Option<i64>) -> Self {
        Entry {
            action,
            entity_kind,
            entity_id,
            before: None,
            after: None,
        }
    }
    pub fn before(mut self, v: Value) -> Self {
        self.before = Some(v);
        self
    }
    pub fn after(mut self, v: Value) -> Self {
        self.after = Some(v);
        self
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuditRow {
    pub id: i64,
    pub ts: i64,
    pub actor_kind: String,
    pub actor_id: Option<i64>,
    pub actor_name: String,
    pub action: String,
    pub entity_kind: String,
    pub entity_id: Option<i64>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub prev_hash: String,
    pub hash: String,
}

fn put(h: &mut Sha256, field: Option<&[u8]>) {
    match field {
        None => h.update(b"N"),
        Some(bytes) => {
            h.update(b"S");
            h.update((bytes.len() as u64).to_be_bytes());
            h.update(bytes);
        }
    }
}

/// Hash of one entry given its contents and the previous entry's hash.
pub fn compute_hash(r: &AuditRow) -> String {
    let mut h = Sha256::new();
    h.update(DOMAIN);
    let id = r.id.to_string();
    let ts = r.ts.to_string();
    let actor_id = r.actor_id.map(|v| v.to_string());
    let entity_id = r.entity_id.map(|v| v.to_string());
    put(&mut h, Some(id.as_bytes()));
    put(&mut h, Some(ts.as_bytes()));
    put(&mut h, Some(r.actor_kind.as_bytes()));
    put(&mut h, actor_id.as_deref().map(str::as_bytes));
    put(&mut h, Some(r.actor_name.as_bytes()));
    put(&mut h, Some(r.action.as_bytes()));
    put(&mut h, Some(r.entity_kind.as_bytes()));
    put(&mut h, entity_id.as_deref().map(str::as_bytes));
    put(&mut h, r.before_json.as_deref().map(str::as_bytes));
    put(&mut h, r.after_json.as_deref().map(str::as_bytes));
    put(&mut h, Some(r.prev_hash.as_bytes()));
    hex::encode(h.finalize())
}

/// Appends an entry. Call inside the same transaction as the change it
/// describes so the log and the data can never disagree.
pub fn append(conn: &Connection, now: i64, actor: &Actor, e: Entry<'_>) -> Result<AuditRow> {
    let last: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, hash FROM audit_log ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (id, prev_hash) = match last {
        Some((id, hash)) => (id + 1, hash),
        None => (1, GENESIS.to_string()),
    };
    let mut row = AuditRow {
        id,
        ts: now,
        actor_kind: actor.kind().to_string(),
        actor_id: actor.id(),
        actor_name: actor.name().to_string(),
        action: e.action.to_string(),
        entity_kind: e.entity_kind.to_string(),
        entity_id: e.entity_id,
        before_json: e.before.map(|v| v.to_string()),
        after_json: e.after.map(|v| v.to_string()),
        prev_hash,
        hash: String::new(),
    };
    row.hash = compute_hash(&row);
    conn.execute(
        "INSERT INTO audit_log (id, ts, actor_kind, actor_id, actor_name, action, entity_kind,
                                entity_id, before_json, after_json, prev_hash, hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            row.id,
            row.ts,
            row.actor_kind,
            row.actor_id,
            row.actor_name,
            row.action,
            row.entity_kind,
            row.entity_id,
            row.before_json,
            row.after_json,
            row.prev_hash,
            row.hash
        ],
    )?;
    Ok(row)
}

fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<AuditRow> {
    Ok(AuditRow {
        id: r.get(0)?,
        ts: r.get(1)?,
        actor_kind: r.get(2)?,
        actor_id: r.get(3)?,
        actor_name: r.get(4)?,
        action: r.get(5)?,
        entity_kind: r.get(6)?,
        entity_id: r.get(7)?,
        before_json: r.get(8)?,
        after_json: r.get(9)?,
        prev_hash: r.get(10)?,
        hash: r.get(11)?,
    })
}

const COLS: &str = "id, ts, actor_kind, actor_id, actor_name, action, entity_kind, entity_id,
                    before_json, after_json, prev_hash, hash";

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrokenLink {
    pub id: i64,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyReport {
    pub ok: bool,
    pub entries: i64,
    pub head_hash: String,
    /// The first entry that fails verification, if any.
    pub first_broken: Option<BrokenLink>,
}

/// Walks the whole chain in id order and reports the first broken link.
pub fn verify(conn: &Connection) -> Result<VerifyReport> {
    let mut st = conn.prepare(&format!("SELECT {COLS} FROM audit_log ORDER BY id"))?;
    let rows = st.query_map([], map_row)?;
    let mut expected_prev = GENESIS.to_string();
    let mut count = 0;
    for (i, row) in rows.enumerate() {
        let row = row?;
        let expected_id = i as i64 + 1;
        let broken = |reason: String| VerifyReport {
            ok: false,
            entries: count,
            head_hash: expected_prev.clone(),
            first_broken: Some(BrokenLink { id: row.id, reason }),
        };
        if row.id != expected_id {
            return Ok(broken(format!(
                "entry {expected_id} is missing (next entry is {})",
                row.id
            )));
        }
        if row.prev_hash != expected_prev {
            return Ok(broken(
                "does not link to the previous entry (an entry was removed, inserted or altered)"
                    .into(),
            ));
        }
        if compute_hash(&row) != row.hash {
            return Ok(broken("contents were changed after it was written".into()));
        }
        expected_prev = row.hash;
        count += 1;
    }
    Ok(VerifyReport {
        ok: true,
        entries: count,
        head_hash: expected_prev,
        first_broken: None,
    })
}

/// Checks that `other` (e.g. an older backup) is a prefix of the live chain:
/// its last entry must exist here with the same hash. Detects history that was
/// rewritten or truncated since that copy was taken.
pub fn verify_prefix_matches(live: &Connection, other: &Connection) -> Result<Option<String>> {
    let other_head: Option<(i64, String)> = other
        .query_row(
            "SELECT id, hash FROM audit_log ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((id, hash)) = other_head else {
        return Ok(None);
    };
    let live_hash: Option<String> = live
        .query_row("SELECT hash FROM audit_log WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(match live_hash {
        None => Some(format!(
            "entry {id} from the backup is missing from the current log (history was truncated)"
        )),
        Some(h) if h != hash => Some(format!(
            "entry {id} differs from the backup (history was rewritten)"
        )),
        Some(_) => None,
    })
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AuditQuery {
    pub entity_kind: Option<String>,
    pub entity_id: Option<i64>,
    pub before_id: Option<i64>,
    pub limit: Option<i64>,
}

/// Newest-first page of entries.
pub fn list(conn: &Connection, q: &AuditQuery) -> Result<Vec<AuditRow>> {
    let limit = q.limit.unwrap_or(100).clamp(1, 1000);
    let mut st = conn.prepare(&format!(
        "SELECT {COLS} FROM audit_log
         WHERE (?1 IS NULL OR entity_kind = ?1)
           AND (?2 IS NULL OR entity_id = ?2)
           AND (?3 IS NULL OR id < ?3)
         ORDER BY id DESC LIMIT ?4"
    ))?;
    let rows = st
        .query_map(
            params![q.entity_kind, q.entity_id, q.before_id, limit],
            map_row,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use serde_json::json;

    fn tech() -> Actor {
        Actor::Technician {
            id: 1,
            name: "Ana".into(),
        }
    }

    fn seed(conn: &Connection, n: i64) {
        for i in 0..n {
            append(
                conn,
                1_000 + i,
                &tech(),
                Entry::new("tool.update", "tool", Some(i))
                    .before(json!({"name": "a"}))
                    .after(json!({"name": format!("b{i}")})),
            )
            .unwrap();
        }
    }

    /// Simulates an attacker with file access: drop the protective triggers.
    fn drop_triggers(conn: &Connection) {
        conn.execute_batch("DROP TRIGGER audit_log_no_update; DROP TRIGGER audit_log_no_delete;")
            .unwrap();
    }

    #[test]
    fn chain_links_entries() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn, 3);
        let rows = list(&conn, &AuditQuery::default()).unwrap();
        assert_eq!(rows.len(), 3);
        // newest first
        assert_eq!(rows[2].prev_hash, GENESIS);
        assert_eq!(rows[1].prev_hash, rows[2].hash);
        assert_eq!(rows[0].prev_hash, rows[1].hash);
        let report = verify(&conn).unwrap();
        assert!(report.ok);
        assert_eq!(report.entries, 3);
        assert_eq!(report.head_hash, rows[0].hash);
    }

    #[test]
    fn empty_log_verifies() {
        let conn = db::open_in_memory().unwrap();
        let r = verify(&conn).unwrap();
        assert!(r.ok);
        assert_eq!(r.entries, 0);
    }

    #[test]
    fn detects_edited_contents() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn, 5);
        drop_triggers(&conn);
        conn.execute(
            "UPDATE audit_log SET actor_name = 'Mallory' WHERE id = 3",
            [],
        )
        .unwrap();
        let r = verify(&conn).unwrap();
        assert!(!r.ok);
        let b = r.first_broken.unwrap();
        assert_eq!(b.id, 3);
        assert!(b.reason.contains("changed"));
        assert_eq!(r.entries, 2);
    }

    #[test]
    fn detects_edit_with_recomputed_hash() {
        // A smarter attacker recomputes the edited entry's own hash; the next
        // link still breaks.
        let conn = db::open_in_memory().unwrap();
        seed(&conn, 5);
        drop_triggers(&conn);
        let mut row = list(
            &conn,
            &AuditQuery {
                before_id: Some(4),
                limit: Some(1),
                ..Default::default()
            },
        )
        .unwrap()
        .remove(0);
        assert_eq!(row.id, 3);
        row.after_json = Some("{\"name\":\"forged\"}".into());
        row.hash = compute_hash(&row);
        conn.execute(
            "UPDATE audit_log SET after_json = ?1, hash = ?2 WHERE id = 3",
            params![row.after_json, row.hash],
        )
        .unwrap();
        let r = verify(&conn).unwrap();
        assert_eq!(r.first_broken.unwrap().id, 4);
    }

    #[test]
    fn detects_deleted_entry() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn, 5);
        drop_triggers(&conn);
        conn.execute("DELETE FROM audit_log WHERE id = 2", [])
            .unwrap();
        let r = verify(&conn).unwrap();
        let b = r.first_broken.unwrap();
        assert_eq!(b.id, 3);
        assert!(b.reason.contains("missing"));
    }

    #[test]
    fn backup_prefix_detects_truncation_and_rewrite() {
        let live = db::open_in_memory().unwrap();
        let backup = db::open_in_memory().unwrap();
        seed(&live, 4);
        // the backup holds the first 3 entries of the same history
        for row in list(&live, &AuditQuery::default())
            .unwrap()
            .iter()
            .rev()
            .take(3)
        {
            backup
                .execute(
                    &format!("INSERT INTO audit_log ({COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)"),
                    params![row.id, row.ts, row.actor_kind, row.actor_id, row.actor_name, row.action,
                            row.entity_kind, row.entity_id, row.before_json, row.after_json, row.prev_hash, row.hash],
                )
                .unwrap();
        }
        assert_eq!(verify_prefix_matches(&live, &backup).unwrap(), None);

        // Rewrite history from scratch in "live": consistent chain, different hashes.
        drop_triggers(&live);
        live.execute("DELETE FROM audit_log", []).unwrap();
        append(&live, 5, &tech(), Entry::new("x", "y", None)).unwrap();
        assert!(verify(&live).unwrap().ok);
        let problem = verify_prefix_matches(&live, &backup).unwrap().unwrap();
        assert!(problem.contains("truncated"));
    }

    #[test]
    fn hash_fields_are_unambiguous() {
        let base = AuditRow {
            id: 1,
            ts: 1,
            actor_kind: "system".into(),
            actor_id: None,
            actor_name: "ab".into(),
            action: "c".into(),
            entity_kind: "e".into(),
            entity_id: None,
            before_json: None,
            after_json: None,
            prev_hash: GENESIS.into(),
            hash: String::new(),
        };
        let mut shifted = base.clone();
        shifted.actor_name = "a".into();
        shifted.action = "bc".into();
        assert_ne!(compute_hash(&base), compute_hash(&shifted));
        let mut empty_vs_none = base.clone();
        empty_vs_none.before_json = Some(String::new());
        assert_ne!(compute_hash(&base), compute_hash(&empty_vs_none));
    }
}
