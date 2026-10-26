//! In-memory authorization state for the running app:
//!
//! * the technician unlock window (expires after N seconds of inactivity), and
//! * borrower confirmation tokens: proof that a borrower entered their PIN a
//!   moment ago, so the multi-step checkout UI can't skip the PIN check.
//!
//! Nothing here is persisted: restarting the app locks tech mode.

use std::collections::HashMap;

use serde::Serialize;

use crate::clock::MINUTE;
use crate::error::{CoreError, Result};

/// How long a borrower token stays valid after the PIN was entered.
pub const BORROWER_TOKEN_TTL: i64 = 3 * MINUTE;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TechSession {
    pub tech_id: i64,
    pub tech_name: String,
    pub unlocked_at: i64,
    pub last_activity: i64,
    /// When the session will lock if nothing else happens.
    pub expires_at: i64,
}

#[derive(Clone, Debug)]
struct BorrowerToken {
    borrower_id: i64,
    issued_at: i64,
}

#[derive(Default)]
pub struct Sessions {
    tech: Option<TechSession>,
    tokens: HashMap<String, BorrowerToken>,
}

impl Sessions {
    pub fn unlock(
        &mut self,
        tech_id: i64,
        tech_name: String,
        now: i64,
        timeout: i64,
    ) -> TechSession {
        let s = TechSession {
            tech_id,
            tech_name,
            unlocked_at: now,
            last_activity: now,
            expires_at: now + timeout,
        };
        self.tech = Some(s.clone());
        s
    }

    pub fn lock(&mut self) {
        self.tech = None;
    }

    /// The unlocked technician, or None if locked or timed out (which locks).
    pub fn active_tech(&mut self, now: i64, timeout: i64) -> Option<&TechSession> {
        if let Some(s) = &self.tech
            && now - s.last_activity >= timeout
        {
            self.tech = None;
        }
        self.tech.as_ref()
    }

    /// Records user activity, extending the unlock window.
    pub fn touch(&mut self, now: i64, timeout: i64) -> Option<TechSession> {
        self.active_tech(now, timeout)?;
        let s = self.tech.as_mut()?;
        s.last_activity = now;
        s.expires_at = now + timeout;
        Some(s.clone())
    }

    pub fn issue_borrower_token(&mut self, borrower_id: i64, now: i64) -> Result<String> {
        self.tokens
            .retain(|_, t| now - t.issued_at < BORROWER_TOKEN_TTL);
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes)
            .map_err(|e| CoreError::Invalid(format!("no randomness available: {e}")))?;
        let token = hex::encode(bytes);
        self.tokens.insert(
            token.clone(),
            BorrowerToken {
                borrower_id,
                issued_at: now,
            },
        );
        Ok(token)
    }

    /// The borrower a valid token stands for. Tokens can be used for several
    /// steps of one flow until they expire or are revoked.
    pub fn borrower_for_token(&self, token: &str, now: i64) -> Result<i64> {
        match self.tokens.get(token) {
            Some(t) if now - t.issued_at < BORROWER_TOKEN_TTL => Ok(t.borrower_id),
            _ => Err(CoreError::AuthExpired),
        }
    }

    pub fn revoke_token(&mut self, token: &str) {
        self.tokens.remove(token);
    }

    pub fn clear(&mut self) {
        self.tech = None;
        self.tokens.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: i64 = 2 * MINUTE;

    #[test]
    fn tech_session_expires_after_inactivity() {
        let mut s = Sessions::default();
        s.unlock(1, "Ana".into(), 0, TIMEOUT);
        assert!(s.active_tech(TIMEOUT - 1, TIMEOUT).is_some());
        assert!(s.active_tech(TIMEOUT, TIMEOUT).is_none());
        // and stays locked
        assert!(s.active_tech(0, TIMEOUT).is_none());
    }

    #[test]
    fn activity_extends_the_window() {
        let mut s = Sessions::default();
        s.unlock(1, "Ana".into(), 0, TIMEOUT);
        let t = s.touch(MINUTE + 59_000, TIMEOUT).unwrap();
        assert_eq!(t.expires_at, MINUTE + 59_000 + TIMEOUT);
        assert!(s.active_tech(3 * MINUTE, TIMEOUT).is_some());
        assert!(s.touch(10 * MINUTE, TIMEOUT).is_none());
    }

    #[test]
    fn manual_lock() {
        let mut s = Sessions::default();
        s.unlock(1, "Ana".into(), 0, TIMEOUT);
        s.lock();
        assert!(s.active_tech(1, TIMEOUT).is_none());
    }

    #[test]
    fn borrower_tokens_expire_and_can_be_revoked() {
        let mut s = Sessions::default();
        let t = s.issue_borrower_token(7, 0).unwrap();
        assert_eq!(t.len(), 32);
        assert_eq!(s.borrower_for_token(&t, BORROWER_TOKEN_TTL - 1).unwrap(), 7);
        assert_eq!(
            s.borrower_for_token(&t, BORROWER_TOKEN_TTL)
                .unwrap_err()
                .code(),
            "auth_expired"
        );
        let t2 = s.issue_borrower_token(8, 0).unwrap();
        s.revoke_token(&t2);
        assert!(s.borrower_for_token(&t2, 1).is_err());
        assert!(s.borrower_for_token("forged", 1).is_err());
    }
}
