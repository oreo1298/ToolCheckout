//! The single entry point the desktop shell talks to. It owns the database
//! connection and the in-memory sessions, enforces "technician PIN required"
//! and borrower confirmation, and wraps each change in one transaction
//! together with its audit entries.
//!
//! A future shared/server mode would host this same type behind a network API.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{Connection, Transaction};
use serde::Serialize;
use serde_json::json;

use crate::audit::{self, AuditQuery, AuditRow, Entry, VerifyReport};
use crate::clock::Clock;
use crate::db;
use crate::error::{CoreError, Result};
use crate::model::Actor;
use crate::session::{Sessions, TechSession};
use crate::settings::{self, Settings};
use crate::technicians::{self, Technician};

pub struct Service {
    pub(crate) conn: Connection,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) sessions: Sessions,
    pub(crate) settings: Settings,
    pub(crate) db_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub set_up: bool,
    pub demo: bool,
    pub settings: Settings,
    pub session: Option<TechSession>,
    pub schema_version: u32,
    pub db_path: Option<String>,
    pub now: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupResult {
    pub technician: Technician,
    /// Shown exactly once; only its hash is stored.
    pub recovery_code: String,
    pub session: TechSession,
}

impl Service {
    pub fn open(path: &Path, clock: Arc<dyn Clock>) -> Result<Self> {
        let conn = db::open(path)?;
        Self::from_conn(conn, clock, Some(path.to_path_buf()))
    }

    pub fn open_in_memory(clock: Arc<dyn Clock>) -> Result<Self> {
        Self::from_conn(db::open_in_memory()?, clock, None)
    }

    fn from_conn(
        conn: Connection,
        clock: Arc<dyn Clock>,
        db_path: Option<PathBuf>,
    ) -> Result<Self> {
        let settings = settings::load(&conn)?;
        Ok(Service {
            conn,
            clock,
            sessions: Sessions::default(),
            settings,
            db_path,
        })
    }

    pub fn now(&self) -> i64 {
        self.clock.now_ms()
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn db_path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }

    pub(crate) fn lock_timeout(&self) -> i64 {
        self.settings.auto_lock_seconds * 1000
    }

    /// Runs `f` in one transaction; nothing is written unless it succeeds.
    pub(crate) fn tx<T>(
        &mut self,
        f: impl FnOnce(&Transaction<'_>, i64) -> Result<T>,
    ) -> Result<T> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        let out = f(&tx, now)?;
        tx.commit()?;
        Ok(out)
    }

    /// The unlocked technician as an actor (refreshing the inactivity timer),
    /// or `TechRequired`.
    pub(crate) fn require_tech(&mut self) -> Result<Actor> {
        let (now, timeout) = (self.now(), self.lock_timeout());
        let s = self
            .sessions
            .touch(now, timeout)
            .ok_or(CoreError::TechRequired)?;
        Ok(Actor::Technician {
            id: s.tech_id,
            name: s.tech_name,
        })
    }

    // ------------------------------------------------------------ status

    pub fn is_set_up(&self) -> Result<bool> {
        Ok(technicians::count(&self.conn)? > 0)
    }

    pub fn is_demo(&self) -> Result<bool> {
        Ok(settings::get_sys(&self.conn, "demo")?.as_deref() == Some("1"))
    }

    pub fn status(&mut self) -> Result<AppStatus> {
        Ok(AppStatus {
            set_up: self.is_set_up()?,
            demo: self.is_demo()?,
            settings: self.settings.clone(),
            session: self.session(),
            schema_version: db::user_version(&self.conn)?,
            db_path: self.db_path.as_ref().map(|p| p.display().to_string()),
            now: self.now(),
        })
    }

    // ------------------------------------------------------------ first run

    /// Creates the shop's first technician. Only allowed once.
    pub fn setup(&mut self, shop_name: &str, tech_name: &str, pin: &str) -> Result<SetupResult> {
        if self.is_set_up()? {
            return Err(CoreError::Conflict("ToolCheckout is already set up".into()));
        }
        let mut s = self.settings.clone();
        s.shop_name = shop_name.to_string();
        s.validate()?;
        let (technician, recovery_code) = self.tx(|tx, now| {
            settings::save_all(tx, &s)?;
            settings::set_sys(tx, "setup_at", &now.to_string())?;
            let t = technicians::create(tx, now, &Actor::System, tech_name, pin)?;
            audit::append(
                tx,
                now,
                &t.actor(),
                Entry::new("setup.complete", "shop", None)
                    .after(json!({ "shopName": s.shop_name })),
            )?;
            let code = technicians::new_recovery_code(tx, now, &t.actor())?;
            Ok((t, code))
        })?;
        self.settings = s;
        let (now, timeout) = (self.now(), self.lock_timeout());
        let session = self
            .sessions
            .unlock(technician.id, technician.name.clone(), now, timeout);
        Ok(SetupResult {
            technician,
            recovery_code,
            session,
        })
    }

    // ------------------------------------------------------------ tech mode

    pub fn session(&mut self) -> Option<TechSession> {
        let (now, timeout) = (self.now(), self.lock_timeout());
        self.sessions.active_tech(now, timeout).cloned()
    }

    pub fn unlock(&mut self, tech_id: i64, pin: &str) -> Result<TechSession> {
        let now = self.now();
        let t = technicians::check_pin(&self.conn, now, tech_id, pin)?;
        self.tx(|tx, now| {
            audit::append(
                tx,
                now,
                &t.actor(),
                Entry::new("tech.unlock", "technician", Some(t.id)),
            )
        })?;
        let timeout = self.lock_timeout();
        Ok(self.sessions.unlock(t.id, t.name, now, timeout))
    }

    pub fn lock(&mut self) {
        self.sessions.lock();
    }

    /// Called on user activity while unlocked.
    pub fn touch(&mut self) -> Option<TechSession> {
        let (now, timeout) = (self.now(), self.lock_timeout());
        self.sessions.touch(now, timeout)
    }

    /// Active technicians (names only) for the unlock picker. Public.
    pub fn technician_names(&self) -> Result<Vec<Technician>> {
        technicians::list(&self.conn, false)
    }

    pub fn list_technicians(&mut self) -> Result<Vec<Technician>> {
        self.require_tech()?;
        technicians::list(&self.conn, true)
    }

    pub fn add_technician(&mut self, name: &str, pin: &str) -> Result<Technician> {
        let actor = self.require_tech()?;
        self.tx(|tx, now| technicians::create(tx, now, &actor, name, pin))
    }

    pub fn update_technician(&mut self, id: i64, name: &str, active: bool) -> Result<Technician> {
        let actor = self.require_tech()?;
        let t = self.tx(|tx, now| technicians::update(tx, now, &actor, id, name, active))?;
        // Deactivating the unlocked technician ends their session.
        if !t.active && actor.id() == Some(id) {
            self.sessions.lock();
        }
        Ok(t)
    }

    pub fn set_technician_pin(&mut self, id: i64, pin: &str) -> Result<()> {
        let actor = self.require_tech()?;
        self.tx(|tx, now| technicians::set_pin(tx, now, &actor, id, pin, "technician.pin_reset"))
    }

    /// "Forgot PIN": the recovery code sets a new PIN for `tech_id`.
    /// Returns the replacement recovery code to show once.
    pub fn recover(&mut self, code: &str, tech_id: i64, new_pin: &str) -> Result<String> {
        let now = self.now();
        technicians::use_recovery_code(&self.conn, now, code, tech_id, new_pin)
    }

    pub fn regenerate_recovery_code(&mut self) -> Result<String> {
        let actor = self.require_tech()?;
        self.tx(|tx, now| technicians::new_recovery_code(tx, now, &actor))
    }

    // ------------------------------------------------------------ settings

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn update_settings(&mut self, mut new: Settings) -> Result<Settings> {
        let actor = self.require_tech()?;
        new.validate()?;
        let old = self.settings.clone();
        self.tx(|tx, now| {
            let (before, after) = settings::save(tx, &old, &new)?;
            if after.as_object().is_some_and(|o| !o.is_empty()) {
                audit::append(
                    tx,
                    now,
                    &actor,
                    Entry::new("settings.update", "settings", None)
                        .before(before)
                        .after(after),
                )?;
            }
            Ok(())
        })?;
        self.settings = new.clone();
        Ok(new)
    }

    // ------------------------------------------------------------ audit

    pub fn verify_audit(&mut self) -> Result<VerifyReport> {
        self.require_tech()?;
        audit::verify(&self.conn)
    }

    pub fn audit_log(&mut self, q: &AuditQuery) -> Result<Vec<AuditRow>> {
        self.require_tech()?;
        audit::list(&self.conn, q)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::clock::{MINUTE, ManualClock};

    pub const T0: i64 = 1_767_225_600_000; // 2026-01-01T00:00:00Z

    pub fn new_service() -> (Service, Arc<ManualClock>) {
        let clock = Arc::new(ManualClock::new(T0));
        (Service::open_in_memory(clock.clone()).unwrap(), clock)
    }

    /// A set-up shop with technician "Ana" (PIN 123456), unlocked.
    pub fn ready() -> (Service, Arc<ManualClock>, SetupResult) {
        let (mut s, c) = new_service();
        let r = s.setup("Test Shop", "Ana", "123456").unwrap();
        (s, c, r)
    }

    #[test]
    fn first_run_setup() {
        let (mut s, _c) = new_service();
        assert!(!s.status().unwrap().set_up);
        let r = s.setup("Fix Bench", "Ana", "123456").unwrap();
        assert_eq!(r.recovery_code.len(), 19);
        let st = s.status().unwrap();
        assert!(st.set_up);
        assert_eq!(st.settings.shop_name, "Fix Bench");
        assert_eq!(st.session.unwrap().tech_name, "Ana");
        // Only once.
        assert_eq!(s.setup("X", "Y", "123456").unwrap_err().code(), "conflict");
        assert!(audit::verify(s.conn()).unwrap().ok);
    }

    #[test]
    fn setup_validates_before_writing() {
        let (mut s, _c) = new_service();
        assert!(s.setup("Shop", "Ana", "12").is_err());
        assert!(!s.is_set_up().unwrap());
        assert_eq!(audit::verify(s.conn()).unwrap().entries, 0);
    }

    #[test]
    fn tech_only_actions_need_an_unlocked_session() {
        let (mut s, c, r) = ready();
        s.lock();
        assert_eq!(
            s.add_technician("Bo", "1111").unwrap_err().code(),
            "tech_required"
        );
        assert_eq!(s.verify_audit().unwrap_err().code(), "tech_required");
        let mut new = s.settings().clone();
        new.shop_name = "Hacked".into();
        assert_eq!(s.update_settings(new).unwrap_err().code(), "tech_required");

        s.unlock(r.technician.id, "123456").unwrap();
        s.add_technician("Bo", "1111").unwrap();
        // Inactivity auto-lock (default 2 minutes)
        c.advance(2 * MINUTE);
        assert!(s.session().is_none());
        assert_eq!(
            s.add_technician("Cy", "2222").unwrap_err().code(),
            "tech_required"
        );
    }

    #[test]
    fn activity_keeps_tech_mode_unlocked() {
        let (mut s, c, _r) = ready();
        for _ in 0..5 {
            c.advance(MINUTE + 30_000);
            assert!(s.touch().is_some());
        }
        c.advance(2 * MINUTE);
        assert!(s.touch().is_none());
    }

    #[test]
    fn every_action_is_attributed_to_the_unlocked_technician() {
        let (mut s, _c, _r) = ready();
        let bo = s.add_technician("Bo", "1111").unwrap();
        s.lock();
        s.unlock(bo.id, "1111").unwrap();
        let mut new = s.settings().clone();
        new.auto_lock_seconds = 300;
        s.update_settings(new).unwrap();
        let log = s.audit_log(&AuditQuery::default()).unwrap();
        let settings_entry = log.iter().find(|e| e.action == "settings.update").unwrap();
        assert_eq!(settings_entry.actor_name, "Bo");
        assert_eq!(
            settings_entry.after_json.as_deref(),
            Some(r#"{"autoLockSeconds":300}"#)
        );
        let create = log
            .iter()
            .find(|e| e.action == "technician.create" && e.entity_id == Some(bo.id))
            .unwrap();
        assert_eq!(create.actor_name, "Ana");
    }

    #[test]
    fn auto_lock_timeout_follows_settings() {
        let (mut s, c, _r) = ready();
        let mut new = s.settings().clone();
        new.auto_lock_seconds = 600;
        s.update_settings(new).unwrap();
        c.advance(9 * MINUTE);
        assert!(s.session().is_some());
    }

    #[test]
    fn recovery_flow_through_service() {
        let (mut s, _c, r) = ready();
        s.lock();
        let next = s
            .recover(&r.recovery_code, r.technician.id, "777777")
            .unwrap();
        assert_ne!(next, r.recovery_code);
        s.unlock(r.technician.id, "777777").unwrap();
    }
}
