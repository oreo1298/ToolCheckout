//! Shop settings, stored one JSON value per key in the `settings` table.
//! Internal bookkeeping keys are prefixed `sys.` and never exposed as settings.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Result, invalid};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub shop_name: String,
    /// Loan period pre-filled for new tools.
    pub default_loan_minutes: i64,
    /// Borrowers confirm checkouts and drop-offs with their 4-digit PIN.
    pub borrower_pin_required: bool,
    /// Checkouts also need an unlocked technician.
    pub tech_pin_for_checkout: bool,
    /// Tech mode locks after this much inactivity.
    pub auto_lock_seconds: i64,
    /// "system" | "light" | "dark"
    pub theme: String,
    pub overdue_notifications: bool,
    /// Repeat reminder while anything is overdue.
    pub overdue_reminder_minutes: i64,
    pub end_of_day_reminder: bool,
    /// Local time "HH:MM" for the end-of-day reminder and "due end of day".
    pub closing_time: String,
    pub auto_backup: bool,
    /// None → `<data folder>/backups`.
    pub backup_dir: Option<String>,
    pub backup_keep: i64,
    pub currency_symbol: String,
    /// Default label stock for asset tag sheets.
    pub label_stock: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            shop_name: "Repair Shop".into(),
            default_loan_minutes: 4 * 60,
            borrower_pin_required: true,
            tech_pin_for_checkout: false,
            auto_lock_seconds: 120,
            theme: "system".into(),
            overdue_notifications: true,
            overdue_reminder_minutes: 60,
            end_of_day_reminder: true,
            closing_time: "17:30".into(),
            auto_backup: true,
            backup_dir: None,
            backup_keep: 30,
            currency_symbol: "$".into(),
            label_stock: "avery5160".into(),
        }
    }
}

pub const LABEL_STOCKS: &[&str] = &["avery5160", "avery5167", "avery_l7160", "avery_l7651"];

impl Settings {
    pub fn validate(&mut self) -> Result<()> {
        self.shop_name = self.shop_name.trim().to_string();
        if self.shop_name.is_empty() || self.shop_name.chars().count() > 80 {
            return invalid("Shop name must be 1–80 characters");
        }
        if !(5..=60 * 24 * 90).contains(&self.default_loan_minutes) {
            return invalid("Default loan period must be between 5 minutes and 90 days");
        }
        if !(30..=3600).contains(&self.auto_lock_seconds) {
            return invalid("Auto-lock must be between 30 seconds and 60 minutes");
        }
        if !["system", "light", "dark"].contains(&self.theme.as_str()) {
            return invalid("Theme must be System, Light or Dark");
        }
        if !(15..=24 * 60).contains(&self.overdue_reminder_minutes) {
            return invalid("Overdue reminders must repeat every 15 minutes to 24 hours");
        }
        if parse_hhmm(&self.closing_time).is_none() {
            return invalid("Closing time must look like 17:30");
        }
        if !(1..=365).contains(&self.backup_keep) {
            return invalid("Keep between 1 and 365 backups");
        }
        if let Some(dir) = &self.backup_dir {
            let d = dir.trim();
            self.backup_dir = (!d.is_empty()).then(|| d.to_string());
        }
        self.currency_symbol = self.currency_symbol.trim().to_string();
        if self.currency_symbol.chars().count() > 4 {
            return invalid("Currency symbol is too long");
        }
        if !LABEL_STOCKS.contains(&self.label_stock.as_str()) {
            return invalid("Unknown label stock");
        }
        Ok(())
    }
}

pub fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.split_once(':')?;
    let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (h < 24 && m < 60 && s.len() == 5).then_some((h, m))
}

pub fn load(conn: &Connection) -> Result<Settings> {
    let Value::Object(mut obj) = serde_json::to_value(Settings::default()).unwrap() else {
        unreachable!("Settings serializes to an object")
    };
    let mut st = conn.prepare("SELECT key, value FROM settings WHERE key NOT LIKE 'sys.%'")?;
    for row in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (k, raw) = row?;
        // Overlay one key at a time; a malformed or invalid value keeps its
        // default instead of stopping the shop from starting.
        let Ok(v) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let mut candidate = obj.clone();
        candidate.insert(k.clone(), v.clone());
        if let Ok(mut s) = serde_json::from_value::<Settings>(Value::Object(candidate))
            && s.validate().is_ok()
        {
            obj.insert(k, v);
        }
    }
    let mut s: Settings = serde_json::from_value(Value::Object(obj)).unwrap_or_default();
    let _ = s.validate();
    Ok(s)
}

/// Saves `new` and returns (before, after) objects holding only changed keys.
pub fn save(conn: &Connection, old: &Settings, new: &Settings) -> Result<(Value, Value)> {
    let old_v = serde_json::to_value(old).unwrap();
    let new_v = serde_json::to_value(new).unwrap();
    let (mut before, mut after) = (Map::new(), Map::new());
    if let (Value::Object(o), Value::Object(n)) = (&old_v, &new_v) {
        for (k, nv) in n {
            let ov = o.get(k).cloned().unwrap_or(Value::Null);
            if &ov != nv {
                conn.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)
                     ON CONFLICT (key) DO UPDATE SET value = ?2",
                    params![k, nv.to_string()],
                )?;
                before.insert(k.clone(), ov);
                after.insert(k.clone(), nv.clone());
            }
        }
    }
    Ok((Value::Object(before), Value::Object(after)))
}

/// Writes every setting (used at first-run so the DB is self-describing).
pub fn save_all(conn: &Connection, s: &Settings) -> Result<()> {
    if let Value::Object(o) = serde_json::to_value(s).unwrap() {
        for (k, v) in o {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = ?2",
                params![k, v.to_string()],
            )?;
        }
    }
    Ok(())
}

pub fn get_sys(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [format!("sys.{key}")],
            |r| r.get(0),
        )
        .optional()?)
}

pub fn set_sys(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = ?2",
        params![format!("sys.{key}"), value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn defaults_when_empty() {
        let conn = db::open_in_memory().unwrap();
        let s = load(&conn).unwrap();
        assert_eq!(s, Settings::default());
        assert!(s.borrower_pin_required);
        assert!(!s.tech_pin_for_checkout);
        assert_eq!(s.auto_lock_seconds, 120);
    }

    #[test]
    fn save_returns_only_changed_keys_and_round_trips() {
        let conn = db::open_in_memory().unwrap();
        let old = load(&conn).unwrap();
        let mut new = old.clone();
        new.shop_name = "Fix-It Bench".into();
        new.auto_lock_seconds = 300;
        let (before, after) = save(&conn, &old, &new).unwrap();
        assert_eq!(
            after,
            serde_json::json!({"shopName": "Fix-It Bench", "autoLockSeconds": 300})
        );
        assert_eq!(
            before,
            serde_json::json!({"shopName": "Repair Shop", "autoLockSeconds": 120})
        );
        assert_eq!(load(&conn).unwrap(), new);
    }

    #[test]
    fn validation() {
        let mut s = Settings {
            closing_time: "25:00".into(),
            ..Settings::default()
        };
        assert!(s.validate().is_err());
        s.closing_time = "09:05".into();
        s.backup_dir = Some("   ".into());
        s.validate().unwrap();
        assert_eq!(s.backup_dir, None);
        s.theme = "purple".into();
        assert!(s.validate().is_err());
    }

    #[test]
    fn corrupt_values_fall_back_per_key() {
        let conn = db::open_in_memory().unwrap();
        conn.execute_batch(
            r#"INSERT INTO settings VALUES ('shopName', '"Bench"'), ('autoLockSeconds', '"oops"');"#,
        )
        .unwrap();
        let s = load(&conn).unwrap();
        assert_eq!(s.shop_name, "Bench");
        assert_eq!(s.auto_lock_seconds, 120);
    }

    #[test]
    fn sys_keys_are_hidden_from_settings() {
        let conn = db::open_in_memory().unwrap();
        set_sys(&conn, "demo", "1").unwrap();
        assert_eq!(get_sys(&conn, "demo").unwrap().as_deref(), Some("1"));
        assert_eq!(load(&conn).unwrap(), Settings::default());
    }
}
