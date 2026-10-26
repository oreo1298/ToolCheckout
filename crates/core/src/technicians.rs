//! Technicians (admins), their PINs, and the one-time recovery code.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::json;

use crate::audit::{self, Entry};
use crate::error::{CoreError, Result, invalid, not_found};
use crate::model::Actor;
use crate::pin::{self, Attempt};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Technician {
    pub id: i64,
    pub name: String,
    pub active: bool,
    pub created_at: i64,
}

impl Technician {
    pub fn actor(&self) -> Actor {
        Actor::Technician {
            id: self.id,
            name: self.name.clone(),
        }
    }
}

pub fn subject(id: i64) -> String {
    format!("technician:{id}")
}

pub fn clean_name(name: &str, what: &str) -> Result<String> {
    let n = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if n.is_empty() {
        return invalid(format!("{what} is required"));
    }
    if n.chars().count() > 80 {
        return invalid(format!("{what} is too long"));
    }
    Ok(n)
}

fn map(r: &rusqlite::Row<'_>) -> rusqlite::Result<Technician> {
    Ok(Technician {
        id: r.get(0)?,
        name: r.get(1)?,
        active: r.get(2)?,
        created_at: r.get(3)?,
    })
}

pub fn count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT count(*) FROM technicians", [], |r| r.get(0))?)
}

pub fn list(conn: &Connection, include_inactive: bool) -> Result<Vec<Technician>> {
    let mut st = conn.prepare(
        "SELECT id, name, active, created_at FROM technicians
         WHERE ?1 OR active = 1 ORDER BY name COLLATE NOCASE",
    )?;
    Ok(st
        .query_map([include_inactive], map)?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn get(conn: &Connection, id: i64) -> Result<Technician> {
    conn.query_row(
        "SELECT id, name, active, created_at FROM technicians WHERE id = ?1",
        [id],
        map,
    )
    .optional()?
    .ok_or_else(|| not_found("Technician"))
}

pub fn create(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    name: &str,
    pin_: &str,
) -> Result<Technician> {
    let name = clean_name(name, "Name")?;
    pin::validate(pin_, pin::TECH_PIN_LEN)?;
    let hash = pin::hash(pin_)?;
    conn.execute(
        "INSERT INTO technicians (name, pin_hash, active, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?3)",
        params![name, hash, now],
    )?;
    let t = get(conn, conn.last_insert_rowid())?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("technician.create", "technician", Some(t.id)).after(json!({ "name": t.name })),
    )?;
    Ok(t)
}

pub fn update(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    name: &str,
    active: bool,
) -> Result<Technician> {
    let before = get(conn, id)?;
    let name = clean_name(name, "Name")?;
    if before.active && !active {
        let others: i64 = conn.query_row(
            "SELECT count(*) FROM technicians WHERE active = 1 AND id != ?1",
            [id],
            |r| r.get(0),
        )?;
        if others == 0 {
            return Err(CoreError::Conflict(
                "At least one technician must stay active".into(),
            ));
        }
    }
    conn.execute(
        "UPDATE technicians SET name = ?1, active = ?2, updated_at = ?3 WHERE id = ?4",
        params![name, active, now, id],
    )?;
    let after = get(conn, id)?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new("technician.update", "technician", Some(id))
            .before(json!({ "name": before.name, "active": before.active }))
            .after(json!({ "name": after.name, "active": after.active })),
    )?;
    Ok(after)
}

/// Sets a new PIN and clears any lockout. Hashes never go in the audit log.
pub fn set_pin(
    conn: &Connection,
    now: i64,
    actor: &Actor,
    id: i64,
    new_pin: &str,
    action: &str,
) -> Result<()> {
    get(conn, id)?;
    pin::validate(new_pin, pin::TECH_PIN_LEN)?;
    let hash = pin::hash(new_pin)?;
    conn.execute(
        "UPDATE technicians SET pin_hash = ?1, updated_at = ?2 WHERE id = ?3",
        params![hash, now, id],
    )?;
    pin::save(conn, &subject(id), &pin::LockState::default())?;
    audit::append(conn, now, actor, Entry::new(action, "technician", Some(id)))?;
    Ok(())
}

/// Turns a lockout-aware attempt into a result, logging new lockouts.
pub(crate) fn finish_attempt(
    conn: &Connection,
    now: i64,
    attempt: Attempt,
    entity_kind: &str,
    entity_id: Option<i64>,
    who: &str,
) -> Result<()> {
    match attempt {
        Attempt::Ok => Ok(()),
        Attempt::Wrong { attempts_left } => Err(CoreError::WrongPin { attempts_left }),
        Attempt::StillLocked { until } => Err(CoreError::LockedOut { until }),
        Attempt::LockedNow { until, level } => {
            audit::append(
                conn,
                now,
                &Actor::System,
                Entry::new("pin.lockout", entity_kind, entity_id).after(json!({
                    "who": who,
                    "level": level,
                    "lockedUntil": until,
                    "seconds": (until - now) / 1000,
                })),
            )?;
            Err(CoreError::LockedOut { until })
        }
    }
}

/// Checks a technician's PIN. The lockout bookkeeping is committed even when
/// the PIN is wrong, so retrying can't erase the failure count.
pub fn check_pin(conn: &Connection, now: i64, id: i64, pin_: &str) -> Result<Technician> {
    let t = get(conn, id)?;
    if !t.active {
        return invalid(format!("{} is no longer an active technician", t.name));
    }
    let hash: String = conn.query_row(
        "SELECT pin_hash FROM technicians WHERE id = ?1",
        [id],
        |r| r.get(0),
    )?;
    let tx = conn.unchecked_transaction()?;
    let attempt = pin::attempt(&tx, &subject(id), now, || pin::verify(pin_, &hash))?;
    let res = finish_attempt(&tx, now, attempt, "technician", Some(id), &t.name);
    tx.commit()?;
    res.map(|_| t)
}

// ---------------------------------------------------------------- recovery

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
pub const RECOVERY_SUBJECT: &str = "recovery";

/// 16 random Crockford base32 characters (80 bits) as XXXX-XXXX-XXXX-XXXX.
pub fn generate_code() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|e| CoreError::Invalid(format!("no randomness available: {e}")))?;
    let chars: Vec<char> = bytes
        .iter()
        .map(|b| CROCKFORD[(b & 31) as usize] as char)
        .collect();
    Ok(chars
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-"))
}

/// Accepts what people actually type: lowercase, spaces, missing dashes, and
/// the letters O/I/L that Crockford maps to digits.
pub fn normalize_code(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect()
}

/// Replaces the active recovery code and returns the new one (shown once).
pub fn new_recovery_code(conn: &Connection, now: i64, actor: &Actor) -> Result<String> {
    let code = generate_code()?;
    conn.execute(
        "UPDATE recovery_codes SET revoked_at = ?1 WHERE used_at IS NULL AND revoked_at IS NULL",
        [now],
    )?;
    conn.execute(
        "INSERT INTO recovery_codes (code_hash, created_at) VALUES (?1, ?2)",
        params![pin::hash(&normalize_code(&code))?, now],
    )?;
    audit::append(
        conn,
        now,
        actor,
        Entry::new(
            "recovery.generate",
            "recovery_code",
            Some(conn.last_insert_rowid()),
        ),
    )?;
    Ok(code)
}

/// Uses the recovery code to set a technician's PIN. Returns the replacement
/// recovery code, because each code works only once.
pub fn use_recovery_code(
    conn: &Connection,
    now: i64,
    code: &str,
    tech_id: i64,
    new_pin: &str,
) -> Result<String> {
    let tech = get(conn, tech_id)?;
    pin::validate(new_pin, pin::TECH_PIN_LEN)?;
    let active: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, code_hash FROM recovery_codes WHERE used_at IS NULL AND revoked_at IS NULL
             ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((code_id, hash)) = active else {
        return Err(CoreError::Conflict(
            "There is no active recovery code".into(),
        ));
    };
    let normalized = normalize_code(code);
    {
        let tx = conn.unchecked_transaction()?;
        let attempt = pin::attempt(&tx, RECOVERY_SUBJECT, now, || {
            pin::verify(&normalized, &hash)
        })?;
        let res = finish_attempt(
            &tx,
            now,
            attempt,
            "recovery_code",
            Some(code_id),
            "recovery code",
        );
        tx.commit()?;
        res.map_err(|e| match e {
            CoreError::WrongPin { .. } => {
                CoreError::Invalid("That recovery code is not correct".into())
            }
            other => other,
        })?;
    }
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE recovery_codes SET used_at = ?1, used_by_tech = ?2 WHERE id = ?3",
        params![now, tech_id, code_id],
    )?;
    let actor = tech.actor();
    set_pin(&tx, now, &actor, tech_id, new_pin, "recovery.pin_reset")?;
    let next = new_recovery_code(&tx, now, &actor)?;
    tx.commit()?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup() -> (Connection, Technician) {
        let conn = db::open_in_memory().unwrap();
        let t = create(&conn, 1, &Actor::System, "  Ana   Ruiz ", "123456").unwrap();
        (conn, t)
    }

    #[test]
    fn create_normalizes_name_and_hashes_pin() {
        let (conn, t) = setup();
        assert_eq!(t.name, "Ana Ruiz");
        let hash: String = conn
            .query_row("SELECT pin_hash FROM technicians", [], |r| r.get(0))
            .unwrap();
        assert!(hash.starts_with("$argon2id$") && !hash.contains("123456"));
        assert!(create(&conn, 1, &Actor::System, "Bo", "12").is_err());
    }

    #[test]
    fn check_pin_and_lockout_survive_rollback_attempts() {
        let (conn, t) = setup();
        assert_eq!(check_pin(&conn, 10, t.id, "123456").unwrap().id, t.id);
        for _ in 0..4 {
            assert_eq!(
                check_pin(&conn, 10, t.id, "000000").unwrap_err().code(),
                "wrong_pin"
            );
        }
        let e = check_pin(&conn, 10, t.id, "000000").unwrap_err();
        assert!(matches!(e, CoreError::LockedOut { until } if until == 10 + 30_000));
        // Correct PIN while locked is refused.
        assert_eq!(
            check_pin(&conn, 20, t.id, "123456").unwrap_err().code(),
            "locked_out"
        );
        // Lockout was logged.
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM audit_log WHERE action = 'pin.lockout'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        assert!(check_pin(&conn, 30_010, t.id, "123456").is_ok());
    }

    #[test]
    fn last_active_technician_cannot_be_deactivated() {
        let (conn, t) = setup();
        assert_eq!(
            update(&conn, 2, &t.actor(), t.id, "Ana", false)
                .unwrap_err()
                .code(),
            "conflict"
        );
        let b = create(&conn, 2, &t.actor(), "Bo", "2222").unwrap();
        update(&conn, 3, &t.actor(), b.id, "Bo", false).unwrap();
        assert_eq!(
            check_pin(&conn, 4, b.id, "2222").unwrap_err().code(),
            "invalid"
        );
    }

    #[test]
    fn recovery_codes_format_and_normalize() {
        let c = generate_code().unwrap();
        assert_eq!(c.len(), 19);
        assert_eq!(c.matches('-').count(), 3);
        assert!(
            c.chars()
                .all(|ch| ch == '-' || CROCKFORD.contains(&(ch as u8)))
        );
        assert_eq!(normalize_code("ab0o-1il x"), "AB00111X");
    }

    #[test]
    fn recovery_code_resets_pin_once_and_rotates() {
        let (conn, t) = setup();
        let code = new_recovery_code(&conn, 1, &Actor::System).unwrap();
        // Wrong code
        assert_eq!(
            use_recovery_code(&conn, 2, "AAAA-AAAA-AAAA-AAAA", t.id, "654321")
                .unwrap_err()
                .code(),
            "invalid"
        );
        // Lock the technician out, then recover (also clears the lockout).
        for _ in 0..5 {
            let _ = check_pin(&conn, 3, t.id, "999999");
        }
        let next = use_recovery_code(&conn, 4, &code.to_lowercase(), t.id, "654321").unwrap();
        assert_ne!(next, code);
        assert!(check_pin(&conn, 5, t.id, "654321").is_ok());
        // The old code is spent; the new one works.
        assert!(use_recovery_code(&conn, 6, &code, t.id, "111111").is_err());
        use_recovery_code(&conn, 7, &next, t.id, "111111").unwrap();
        assert!(check_pin(&conn, 8, t.id, "111111").is_ok());
        assert!(crate::audit::verify(&conn).unwrap().ok);
    }

    #[test]
    fn generating_a_new_code_revokes_the_old_one() {
        let (conn, t) = setup();
        let first = new_recovery_code(&conn, 1, &t.actor()).unwrap();
        let _second = new_recovery_code(&conn, 2, &t.actor()).unwrap();
        assert!(use_recovery_code(&conn, 3, &first, t.id, "222222").is_err());
    }
}
