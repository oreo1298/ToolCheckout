//! Connection setup and schema migrations.

use std::path::Path;

use rusqlite::Connection;

use crate::error::{CoreError, Result};

/// Ordered schema migrations. Never edit a shipped entry; append a new one.
/// `PRAGMA user_version` records how many have been applied.
pub const MIGRATIONS: &[&str] = &[include_str!("../migrations/0001_initial.sql")];

pub fn schema_version() -> u32 {
    MIGRATIONS.len() as u32
}

/// Opens (creating if needed) the database file, applies pragmas and migrations.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    // WAL lets the UI read while a backup or write is in progress.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&mut conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

pub fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(())
}

pub fn user_version(conn: &Connection) -> Result<u32> {
    Ok(conn.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))?)
}

pub fn migrate(conn: &mut Connection) -> Result<u32> {
    migrate_with(conn, MIGRATIONS)
}

/// Applies each pending migration in its own transaction.
pub fn migrate_with(conn: &mut Connection, migrations: &[&str]) -> Result<u32> {
    let current = user_version(conn)?;
    let target = migrations.len() as u32;
    if current > target {
        return Err(CoreError::SchemaTooNew {
            found: current,
            supported: target,
        });
    }
    for (i, sql) in migrations.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as u32)?;
        tx.commit()?;
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables(conn: &Connection) -> Vec<String> {
        let mut st = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        st.query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn fresh_database_gets_full_schema() {
        let conn = open_in_memory().unwrap();
        assert_eq!(user_version(&conn).unwrap(), schema_version());
        let t = tables(&conn);
        for want in [
            "audit_log",
            "borrowers",
            "checkouts",
            "condition_events",
            "incidents",
            "photos",
            "pin_lockouts",
            "recovery_codes",
            "settings",
            "technicians",
            "tools",
        ] {
            assert!(t.contains(&want.to_string()), "missing table {want}");
        }
    }

    #[test]
    fn migrating_twice_is_a_no_op() {
        let mut conn = open_in_memory().unwrap();
        assert_eq!(migrate(&mut conn).unwrap(), schema_version());
        assert_eq!(user_version(&conn).unwrap(), schema_version());
    }

    #[test]
    fn upgrades_apply_only_pending_migrations_in_order() {
        let mut conn = Connection::open_in_memory().unwrap();
        let v1 = ["CREATE TABLE a (x INTEGER);"];
        migrate_with(&mut conn, &v1).unwrap();
        conn.execute("INSERT INTO a VALUES (1)", []).unwrap();
        let v2 = [
            v1[0],
            "ALTER TABLE a ADD COLUMN y TEXT NOT NULL DEFAULT 'z';",
        ];
        assert_eq!(migrate_with(&mut conn, &v2).unwrap(), 2);
        let y: String = conn.query_row("SELECT y FROM a", [], |r| r.get(0)).unwrap();
        assert_eq!(y, "z"); // existing data kept, new column added
    }

    #[test]
    fn failed_migration_rolls_back_and_keeps_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        let bad = [
            "CREATE TABLE ok (x);",
            "CREATE TABLE b (x); THIS IS NOT SQL;",
        ];
        assert!(migrate_with(&mut conn, &bad).is_err());
        assert_eq!(user_version(&conn).unwrap(), 1);
        assert!(!tables(&conn).contains(&"b".to_string()));
    }

    #[test]
    fn refuses_databases_from_newer_versions() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        let err = migrate(&mut conn).unwrap_err();
        assert_eq!(err.code(), "schema_too_new");
    }

    #[test]
    fn file_database_uses_wal_and_foreign_keys() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("sub/tc.db")).unwrap();
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        let fk: i64 = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
        // Foreign keys are enforced.
        let err = conn.execute(
            "INSERT INTO checkouts (batch_id, tool_id, borrower_id, checked_out_at, due_at, checkout_condition, state)
             VALUES ('b', 999, 999, 1, 2, 'good', 'out')",
            [],
        );
        assert!(err.is_err());
    }

    #[test]
    fn append_only_tables_reject_updates_and_deletes() {
        let conn = open_in_memory().unwrap();
        conn.execute_batch(
            "INSERT INTO audit_log (ts, actor_kind, actor_name, action, entity_kind, prev_hash, hash)
             VALUES (1, 'system', 'System', 'x', 'y', 'p', 'h');",
        )
        .unwrap();
        assert!(
            conn.execute("UPDATE audit_log SET action = 'z'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM audit_log", []).is_err());
    }
}
