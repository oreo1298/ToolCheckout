//! Backups and restore.
//!
//! * Backups are consistent single-file snapshots made with `VACUUM INTO`
//!   (safe while the app is running; photos are inside the database).
//! * Automatic: once per day, into the backup folder, keeping the newest N
//!   automatic backups. Manual backups are never pruned.
//! * Restore validates the file first (SQLite integrity, it is a ToolCheckout
//!   database, not from a newer version, audit chain intact), saves a safety
//!   copy of the current data, then copies the backup in.
//! * Backups double as tamper evidence: the live audit log must extend the
//!   chain stored in each backup.

use std::path::{Path, PathBuf};

use chrono::{Local, TimeZone};
use rusqlite::{Connection, MAIN_DB, OpenFlags};
use serde::Serialize;
use serde_json::json;

use crate::audit::{self, Entry};
use crate::error::{CoreError, Result, invalid};
use crate::model::Actor;
use crate::{db, settings};

pub const AUTO_PREFIX: &str = "toolcheckout-auto-";
pub const MANUAL_PREFIX: &str = "toolcheckout-backup-";
pub const SAFETY_PREFIX: &str = "toolcheckout-before-restore-";

fn stamp(now: i64) -> String {
    Local
        .timestamp_millis_opt(now)
        .single()
        .map(|d| d.format("%Y%m%d-%H%M%S").to_string())
        .unwrap_or_else(|| now.to_string())
}

fn local_day(ms: i64) -> Option<chrono::NaiveDate> {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.date_naive())
}

/// Writes a consistent snapshot of `conn` to `dest` (replacing it).
pub fn snapshot(conn: &Connection, dest: &Path) -> Result<()> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // VACUUM INTO refuses to overwrite; write beside, then move into place.
    let tmp = dest.with_extension("db-partial");
    let _ = std::fs::remove_file(&tmp);
    conn.execute("VACUUM INTO ?1", [tmp.to_string_lossy()])?;
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: i64,
    pub automatic: bool,
}

/// Backups in a folder, newest first.
pub fn list(dir: &Path) -> Result<Vec<BackupFile>> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(out);
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !(name.starts_with("toolcheckout-") && name.ends_with(".db")) {
            continue;
        }
        let meta = e.metadata()?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        out.push(BackupFile {
            path: e.path().display().to_string(),
            automatic: name.starts_with(AUTO_PREFIX),
            name,
            size: meta.len(),
            modified,
        });
    }
    // Names embed a sortable timestamp; fall back to mtime for others.
    out.sort_by(|a, b| b.modified.cmp(&a.modified).then(b.name.cmp(&a.name)));
    Ok(out)
}

/// Deletes the oldest automatic backups beyond `keep`.
pub fn prune(dir: &Path, keep: usize) -> Result<usize> {
    let mut autos: Vec<_> = list(dir)?.into_iter().filter(|b| b.automatic).collect();
    autos.sort_by(|a, b| b.name.cmp(&a.name));
    let mut removed = 0;
    for old in autos.iter().skip(keep) {
        std::fs::remove_file(&old.path)?;
        removed += 1;
    }
    Ok(removed)
}

/// Checks that `path` is a healthy ToolCheckout database this version can use.
pub fn inspect(path: &Path) -> Result<BackupInfo> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| CoreError::Invalid("That file isn't a ToolCheckout backup".into()))?;
    let ok: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|_| CoreError::Invalid("That file isn't a ToolCheckout backup".into()))?;
    if ok != "ok" {
        return invalid(format!("The backup is damaged ({ok})"));
    }
    let has_tables: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('audit_log', 'tools', 'checkouts', 'technicians')",
        [],
        |r| r.get(0),
    )?;
    if has_tables < 4 {
        return invalid("That file isn't a ToolCheckout backup");
    }
    let version = db::user_version(&conn)?;
    if version > db::schema_version() {
        return Err(CoreError::SchemaTooNew {
            found: version,
            supported: db::schema_version(),
        });
    }
    let chain = audit::verify(&conn)?;
    let count = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0));
    Ok(BackupInfo {
        schema_version: version,
        tools: count("SELECT count(*) FROM tools")?,
        borrowers: count("SELECT count(*) FROM borrowers")?,
        checkouts: count("SELECT count(*) FROM checkouts")?,
        last_activity: conn
            .query_row("SELECT max(ts) FROM audit_log", [], |r| r.get(0))
            .unwrap_or(None),
        shop_name: settings::load(&conn)
            .map(|s| s.shop_name)
            .unwrap_or_default(),
        audit_ok: chain.ok,
        audit_entries: chain.entries,
        demo: crate::demo::is_demo(&conn)?,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub schema_version: u32,
    pub tools: i64,
    pub borrowers: i64,
    pub checkouts: i64,
    pub last_activity: Option<i64>,
    pub shop_name: String,
    pub audit_ok: bool,
    pub audit_entries: i64,
    /// A copy of demo data (never restored over real data).
    pub demo: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCheck {
    pub name: String,
    /// None when the live log extends this backup's chain.
    pub problem: Option<String>,
}

/// Compares the live audit chain with the newest `n` backups in `dir`.
pub fn check_against_backups(conn: &Connection, dir: &Path, n: usize) -> Result<Vec<BackupCheck>> {
    let mut out = Vec::new();
    for b in list(dir)?.into_iter().take(n) {
        let problem = match Connection::open_with_flags(&b.path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
            Ok(other) => audit::verify_prefix_matches(conn, &other)
                .unwrap_or_else(|e| Some(format!("could not read: {e}"))),
            Err(e) => Some(format!("could not open: {e}")),
        };
        out.push(BackupCheck {
            name: b.name,
            problem,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------- service API

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub dir: String,
    pub last_auto_backup: Option<i64>,
    pub files: Vec<BackupFile>,
}

impl crate::Service {
    fn not_in_demo(&self) -> Result<()> {
        if self.is_demo()? {
            return Err(CoreError::Conflict(
                "Not available with demo data. Leave the demo first.".into(),
            ));
        }
        Ok(())
    }

    /// The folder backups go to: the setting, or `<data folder>/backups`.
    pub fn backup_dir(&self) -> PathBuf {
        if let Some(d) = &self.settings.backup_dir {
            return PathBuf::from(d);
        }
        self.db_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.join("backups"))
            .unwrap_or_else(|| PathBuf::from("backups"))
    }

    pub fn backup_status(&mut self) -> Result<BackupStatus> {
        self.require_tech()?;
        let dir = self.backup_dir();
        Ok(BackupStatus {
            last_auto_backup: settings::get_sys(&self.conn, "last_auto_backup")?
                .and_then(|v| v.parse().ok()),
            files: list(&dir)?,
            dir: dir.display().to_string(),
        })
    }

    /// True if automatic backups are on and none was made today.
    pub fn auto_backup_due(&self) -> Result<bool> {
        // Demo data is throwaway; don't mix its backups with the shop's.
        if !self.settings.auto_backup || self.db_path.is_none() || self.is_demo()? {
            return Ok(false);
        }
        let last: Option<i64> =
            settings::get_sys(&self.conn, "last_auto_backup")?.and_then(|v| v.parse().ok());
        Ok(match last {
            None => true,
            Some(t) => local_day(t) != local_day(self.now()),
        })
    }

    /// Makes today's automatic backup and prunes old ones. Returns its path.
    pub fn run_auto_backup(&mut self) -> Result<PathBuf> {
        let now = self.now();
        let dir = self.backup_dir();
        let path = dir.join(format!("{AUTO_PREFIX}{}.db", stamp(now)));
        snapshot(&self.conn, &path)?;
        let keep = self.settings.backup_keep.max(1) as usize;
        let pruned = prune(&dir, keep)?;
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        self.tx(|tx, now| {
            settings::set_sys(tx, "last_auto_backup", &now.to_string())?;
            audit::append(
                tx,
                now,
                &Actor::System,
                Entry::new("backup.create", "backup", None)
                    .after(json!({ "file": name, "automatic": true, "pruned": pruned })),
            )?;
            Ok(())
        })?;
        Ok(path)
    }

    /// Technician saves a backup copy to a chosen file (e.g. a USB stick).
    pub fn backup_to(&mut self, dest: &Path) -> Result<()> {
        let actor = self.require_tech()?;
        self.not_in_demo()?;
        snapshot(&self.conn, dest)?;
        let name = dest.display().to_string();
        self.tx(|tx, now| {
            audit::append(
                tx,
                now,
                &actor,
                Entry::new("backup.create", "backup", None)
                    .after(json!({ "file": name, "automatic": false })),
            )
        })?;
        Ok(())
    }

    pub fn default_backup_name(&self) -> String {
        format!("{MANUAL_PREFIX}{}.db", stamp(self.now()))
    }

    /// Looks inside a backup without changing anything (tech only).
    pub fn inspect_backup(&mut self, path: &Path) -> Result<BackupInfo> {
        self.require_tech()?;
        inspect(path)
    }

    /// Replaces all data with the backup at `path`. A safety copy of the
    /// current data is written to the backup folder first. Ends tech mode
    /// (the restored data may have different technicians).
    pub fn restore_from(&mut self, path: &Path) -> Result<PathBuf> {
        let actor = self.require_tech()?;
        self.not_in_demo()?;
        let info = inspect(path)?;
        if info.demo {
            return invalid(
                "That file is a copy of demo data, not your shop's data. It was not restored.",
            );
        }
        if !info.audit_ok {
            return invalid(
                "That backup's audit log fails verification, so it may have been tampered with. It was not restored.",
            );
        }
        let now = self.now();
        let safety = self
            .backup_dir()
            .join(format!("{SAFETY_PREFIX}{}.db", stamp(now)));
        snapshot(&self.conn, &safety)?;
        let previous_head = audit::verify(&self.conn)?.head_hash;

        self.conn
            .restore(MAIN_DB, path, None::<fn(rusqlite::backup::Progress)>)?;
        db::configure(&self.conn)?;
        db::migrate(&mut self.conn)?;
        self.settings = settings::load(&self.conn)?;
        self.sessions.clear();
        let file = path.display().to_string();
        let safety_name = safety.display().to_string();
        self.tx(|tx, now| {
            audit::append(
                tx,
                now,
                &actor,
                Entry::new("backup.restore", "backup", None).after(json!({
                    "file": file,
                    "safetyCopy": safety_name,
                    "replacedLogHead": previous_head,
                })),
            )
        })?;
        Ok(safety)
    }

    /// Chain check plus comparison with the newest backups.
    pub fn verify_against_backups(&mut self) -> Result<Vec<BackupCheck>> {
        self.require_tech()?;
        check_against_backups(&self.conn, &self.backup_dir(), 3)
    }

    /// Copies the database into `dir` (for moving the data folder).
    pub fn copy_database_to(&mut self, dir: &Path, file_name: &str) -> Result<PathBuf> {
        let actor = self.require_tech()?;
        self.not_in_demo()?;
        let dest = dir.join(file_name);
        if dest.exists() {
            return Err(CoreError::Conflict(format!(
                "{} already has ToolCheckout data",
                dir.display()
            )));
        }
        let to = dir.display().to_string();
        // Log first so the copy carries the record of the move.
        self.tx(|tx, now| {
            audit::append(
                tx,
                now,
                &actor,
                Entry::new("data.relocate", "data", None).after(json!({ "to": to })),
            )
        })?;
        snapshot(&self.conn, &dest)?;
        Ok(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::tests::T0;
    use crate::tools::ToolInput;
    use crate::{ManualClock, Service, model::Condition};
    use std::sync::Arc;

    fn file_service(dir: &Path) -> (Service, Arc<ManualClock>) {
        let clock = Arc::new(ManualClock::new(T0));
        let mut s = Service::open(&dir.join("toolcheckout.db"), clock.clone()).unwrap();
        s.setup("Shop", "Ana", "123456").unwrap();
        (s, clock)
    }

    fn add_tool(s: &mut Service, name: &str) {
        s.create_tool(ToolInput {
            name: name.into(),
            default_loan_minutes: 60,
            condition: Some(Condition::Good),
            ..Default::default()
        })
        .unwrap();
    }

    #[test]
    fn daily_auto_backup_and_pruning() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut s, c) = file_service(tmp.path());
        assert!(s.auto_backup_due().unwrap());
        let p = s.run_auto_backup().unwrap();
        assert!(p.exists());
        assert!(!s.auto_backup_due().unwrap(), "only once per day");
        c.advance(crate::clock::DAY);
        assert!(s.auto_backup_due().unwrap());
        // Keep only 3 automatic backups; manual ones are never pruned.
        let mut st = s.settings().clone();
        st.backup_keep = 3;
        s.unlock(1, "123456").unwrap();
        s.update_settings(st).unwrap();
        s.backup_to(&s.backup_dir().join("toolcheckout-backup-manual.db"))
            .unwrap();
        for _ in 0..5 {
            c.advance(crate::clock::DAY);
            s.run_auto_backup().unwrap();
        }
        let files = list(&s.backup_dir()).unwrap();
        assert_eq!(files.iter().filter(|f| f.automatic).count(), 3);
        assert_eq!(files.iter().filter(|f| !f.automatic).count(), 1);
    }

    #[test]
    fn backup_restore_round_trip_with_safety_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut s, _c) = file_service(tmp.path());
        add_tool(&mut s, "Before");
        let bk = tmp.path().join("bk.db");
        s.backup_to(&bk).unwrap();
        add_tool(&mut s, "After");
        assert_eq!(s.list_tools(&Default::default()).unwrap().len(), 2);

        let info = s.inspect_backup(&bk).unwrap();
        assert_eq!(info.tools, 1);
        assert!(info.audit_ok);

        let safety = s.restore_from(&bk).unwrap();
        assert!(safety.exists());
        assert_eq!(s.list_tools(&Default::default()).unwrap().len(), 1);
        assert!(s.session().is_none(), "restore ends tech mode");
        // The restore itself is on record, and the chain is intact.
        let log = audit::list(s.conn(), &Default::default()).unwrap();
        assert_eq!(log[0].action, "backup.restore");
        assert!(audit::verify(s.conn()).unwrap().ok);
        // The safety copy still has both tools.
        assert_eq!(inspect(&safety).unwrap().tools, 2);
    }

    #[test]
    fn refuses_garbage_and_tampered_backups() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut s, _c) = file_service(tmp.path());
        let junk = tmp.path().join("junk.db");
        std::fs::write(&junk, b"definitely not sqlite").unwrap();
        assert!(s.restore_from(&junk).is_err());

        let bk = tmp.path().join("bk.db");
        s.backup_to(&bk).unwrap();
        {
            let c = Connection::open(&bk).unwrap();
            c.execute_batch("DROP TRIGGER audit_log_no_update; UPDATE audit_log SET actor_name = 'X' WHERE id = 1;")
                .unwrap();
        }
        let e = s.restore_from(&bk).unwrap_err();
        assert!(e.to_string().contains("tampered"));
    }

    #[test]
    fn live_log_is_checked_against_backups() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut s, _c) = file_service(tmp.path());
        s.run_auto_backup().unwrap();
        add_tool(&mut s, "x");
        let checks = s.verify_against_backups().unwrap();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].problem, None);
        // Someone rewrites the live history from scratch.
        s.conn()
            .execute_batch("DROP TRIGGER audit_log_no_delete; DELETE FROM audit_log;")
            .unwrap();
        let checks = s.verify_against_backups().unwrap();
        assert!(checks[0].problem.is_some());
    }

    #[test]
    fn demo_data_stays_out_of_backups() {
        let tmp = tempfile::tempdir().unwrap();
        let demo_path = tmp.path().join("demo.db");
        crate::demo::create(&demo_path, T0).unwrap();
        assert!(inspect(&demo_path).unwrap().demo);
        // A real shop refuses to restore demo data…
        let (mut s, _c) = file_service(&tmp.path().join("real"));
        assert!(
            s.restore_from(&demo_path)
                .unwrap_err()
                .to_string()
                .contains("demo")
        );
        // …and the demo refuses backups and restores.
        let mut d = Service::open(&demo_path, Arc::new(ManualClock::new(T0))).unwrap();
        d.unlock(1, crate::demo::TECH_PIN).unwrap();
        assert_eq!(
            d.backup_to(&tmp.path().join("x.db")).unwrap_err().code(),
            "conflict"
        );
        assert!(!d.auto_backup_due().unwrap());
    }

    #[test]
    fn copy_for_relocation_refuses_to_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let (mut s, _c) = file_service(tmp.path());
        let other = tmp.path().join("shared");
        let dest = s.copy_database_to(&other, "toolcheckout.db").unwrap();
        assert!(inspect(&dest).unwrap().audit_entries > 0);
        assert_eq!(
            s.copy_database_to(&other, "toolcheckout.db")
                .unwrap_err()
                .code(),
            "conflict"
        );
    }
}
