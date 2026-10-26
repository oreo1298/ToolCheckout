//! PIN hashing (Argon2id, per-PIN random salt) and escalating lockouts.
//!
//! Lockout policy: the first 4 wrong PINs in a row only count. The 5th locks
//! the account for 30 s. While the streak continues, each further wrong PIN
//! locks again at the next step: 1 min, 5 min, 15 min, then 1 h for every
//! wrong PIN after that. A correct PIN resets everything, and so does a
//! quiet day (no lockout for 24 h).

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use rusqlite::{Connection, OptionalExtension, params};

use crate::clock::{DAY, MINUTE, SECOND};
use crate::error::{CoreError, Result, invalid};

pub const MAX_ATTEMPTS: u32 = 5;
const DELAYS: [i64; 5] = [30 * SECOND, MINUTE, 5 * MINUTE, 15 * MINUTE, 60 * MINUTE];
const RESET_AFTER: i64 = DAY;

/// Technician PINs: 4–8 digits (6 recommended). Borrower PINs: exactly 4.
pub const TECH_PIN_LEN: (usize, usize) = (4, 8);
pub const BORROWER_PIN_LEN: (usize, usize) = (4, 4);

pub fn validate(pin: &str, (min, max): (usize, usize)) -> Result<()> {
    if !pin.chars().all(|c| c.is_ascii_digit()) {
        return invalid("PINs can only contain digits");
    }
    if pin.len() < min || pin.len() > max {
        return if min == max {
            invalid(format!("PIN must be {min} digits"))
        } else {
            invalid(format!("PIN must be {min}–{max} digits"))
        };
    }
    Ok(())
}

pub fn hash(secret: &str) -> Result<String> {
    Argon2::default()
        .hash_password(secret.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| CoreError::Invalid(format!("could not hash PIN: {e}")))
}

/// Constant-time verification against a stored PHC string.
pub fn verify(secret: &str, stored: &str) -> bool {
    PasswordHash::new(stored)
        .map(|h| {
            Argon2::default()
                .verify_password(secret.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false)
}

pub fn delay_for_level(level: u32) -> i64 {
    let i = (level.max(1) as usize - 1).min(DELAYS.len() - 1);
    DELAYS[i]
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LockState {
    pub failed_attempts: u32,
    pub lockout_level: u32,
    pub locked_until: Option<i64>,
}

impl LockState {
    /// Some(until) while the account is locked.
    pub fn locked(&self, now: i64) -> Option<i64> {
        self.locked_until.filter(|&until| until > now)
    }

    /// Forget an old streak once a full day has passed since the last lockout.
    pub fn decay(&mut self, now: i64) {
        if let Some(until) = self.locked_until
            && now - until >= RESET_AFTER
        {
            *self = LockState::default();
        }
    }

    /// Records a wrong PIN. Returns Some(until) if this attempt locked the account.
    pub fn register_failure(&mut self, now: i64) -> Option<i64> {
        self.decay(now);
        self.failed_attempts += 1;
        if self.failed_attempts >= MAX_ATTEMPTS {
            self.lockout_level += 1;
            let until = now + delay_for_level(self.lockout_level);
            self.locked_until = Some(until);
            Some(until)
        } else {
            None
        }
    }

    /// Wrong PINs left before the next lockout.
    pub fn attempts_left(&self) -> u32 {
        MAX_ATTEMPTS.saturating_sub(self.failed_attempts)
    }
}

pub fn load(conn: &Connection, subject: &str) -> Result<LockState> {
    Ok(conn
        .query_row(
            "SELECT failed_attempts, lockout_level, locked_until FROM pin_lockouts WHERE subject = ?1",
            [subject],
            |r| {
                Ok(LockState {
                    failed_attempts: r.get(0)?,
                    lockout_level: r.get(1)?,
                    locked_until: r.get(2)?,
                })
            },
        )
        .optional()?
        .unwrap_or_default())
}

pub fn save(conn: &Connection, subject: &str, s: &LockState) -> Result<()> {
    if *s == LockState::default() {
        conn.execute("DELETE FROM pin_lockouts WHERE subject = ?1", [subject])?;
    } else {
        conn.execute(
            "INSERT INTO pin_lockouts (subject, failed_attempts, lockout_level, locked_until)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (subject) DO UPDATE SET failed_attempts = ?2, lockout_level = ?3, locked_until = ?4",
            params![subject, s.failed_attempts, s.lockout_level, s.locked_until],
        )?;
    }
    Ok(())
}

/// Outcome of a checked attempt, before it is turned into an error.
#[derive(Debug, PartialEq, Eq)]
pub enum Attempt {
    Ok,
    Wrong {
        attempts_left: u32,
    },
    /// This attempt triggered a lockout (log it).
    LockedNow {
        until: i64,
        level: u32,
    },
    /// Already locked; the PIN was not even checked.
    StillLocked {
        until: i64,
    },
}

/// Runs one PIN attempt for `subject` and persists the lockout state.
/// The caller must commit this even when the attempt fails.
pub fn attempt(
    conn: &Connection,
    subject: &str,
    now: i64,
    check: impl FnOnce() -> bool,
) -> Result<Attempt> {
    let mut st = load(conn, subject)?;
    if let Some(until) = st.locked(now) {
        return Ok(Attempt::StillLocked { until });
    }
    if check() {
        save(conn, subject, &LockState::default())?;
        return Ok(Attempt::Ok);
    }
    let locked = st.register_failure(now);
    save(conn, subject, &st)?;
    Ok(match locked {
        Some(until) => Attempt::LockedNow {
            until,
            level: st.lockout_level,
        },
        None => Attempt::Wrong {
            attempts_left: st.attempts_left(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn validates_length_and_digits() {
        assert!(validate("123456", TECH_PIN_LEN).is_ok());
        assert!(validate("1234", TECH_PIN_LEN).is_ok());
        assert!(validate("12345678", TECH_PIN_LEN).is_ok());
        assert!(validate("123", TECH_PIN_LEN).is_err());
        assert!(validate("123456789", TECH_PIN_LEN).is_err());
        assert!(validate("12a4", TECH_PIN_LEN).is_err());
        assert!(validate("1234", BORROWER_PIN_LEN).is_ok());
        assert_eq!(
            validate("12345", BORROWER_PIN_LEN).unwrap_err().to_string(),
            "PIN must be 4 digits"
        );
    }

    #[test]
    fn hashes_are_salted_argon2id_and_verify() {
        let a = hash("1234").unwrap();
        let b = hash("1234").unwrap();
        assert!(a.starts_with("$argon2id$"));
        assert_ne!(a, b, "each hash has its own random salt");
        assert!(verify("1234", &a));
        assert!(verify("1234", &b));
        assert!(!verify("1235", &a));
        assert!(!verify("1234", "not a hash"));
    }

    #[test]
    fn escalating_lockout_schedule() {
        let mut s = LockState::default();
        let now = 1_000_000;
        for i in 1..MAX_ATTEMPTS {
            assert_eq!(s.register_failure(now), None, "attempt {i}");
        }
        assert_eq!(s.attempts_left(), 1);
        // 5th wrong PIN: 30 s
        assert_eq!(s.register_failure(now), Some(now + 30 * SECOND));
        assert_eq!(s.locked(now + 29 * SECOND), Some(now + 30 * SECOND));
        assert_eq!(s.locked(now + 30 * SECOND), None);
        // Each further wrong PIN escalates: 1 m, 5 m, 15 m, 1 h, 1 h …
        let t = now + MINUTE;
        assert_eq!(s.register_failure(t), Some(t + MINUTE));
        assert_eq!(s.register_failure(t), Some(t + 5 * MINUTE));
        assert_eq!(s.register_failure(t), Some(t + 15 * MINUTE));
        assert_eq!(s.register_failure(t), Some(t + 60 * MINUTE));
        assert_eq!(s.register_failure(t), Some(t + 60 * MINUTE));
    }

    #[test]
    fn streak_decays_after_a_quiet_day() {
        let mut s = LockState::default();
        for _ in 0..MAX_ATTEMPTS {
            s.register_failure(0);
        }
        assert_eq!(s.lockout_level, 1);
        // A day after the lockout ended, one typo doesn't lock again.
        assert_eq!(s.register_failure(30 * SECOND + DAY), None);
        assert_eq!(s.failed_attempts, 1);
        assert_eq!(s.lockout_level, 0);
    }

    #[test]
    fn attempt_persists_state_and_success_resets() {
        let conn = db::open_in_memory().unwrap();
        let now = 5_000;
        for left in (1..MAX_ATTEMPTS).rev() {
            assert_eq!(
                attempt(&conn, "borrower:1", now, || false).unwrap(),
                Attempt::Wrong {
                    attempts_left: left
                }
            );
        }
        let until = now + 30 * SECOND;
        assert_eq!(
            attempt(&conn, "borrower:1", now, || false).unwrap(),
            Attempt::LockedNow { until, level: 1 }
        );
        // While locked, even the right PIN is refused without being checked.
        let mut checked = false;
        assert_eq!(
            attempt(&conn, "borrower:1", now + SECOND, || {
                checked = true;
                true
            })
            .unwrap(),
            Attempt::StillLocked { until }
        );
        assert!(!checked);
        // Other accounts are unaffected.
        assert_eq!(
            attempt(&conn, "borrower:2", now, || true).unwrap(),
            Attempt::Ok
        );
        // After the lock expires, the right PIN works and clears the streak.
        assert_eq!(
            attempt(&conn, "borrower:1", until, || true).unwrap(),
            Attempt::Ok
        );
        assert_eq!(load(&conn, "borrower:1").unwrap(), LockState::default());
    }
}
