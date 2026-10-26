use serde::Serialize;
use serde::ser::SerializeStruct;

use crate::model::ToolStatus;

pub type Result<T> = std::result::Result<T, CoreError>;

/// Every failure the UI can see. Serializes to
/// `{ code, message, attemptsLeft?, lockedUntil? }` so the frontend can react
/// to specific cases (wrong PIN, lockout, tech PIN required) without parsing
/// text.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("{0} not found")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("A technician PIN is required for this")]
    TechRequired,
    #[error("Wrong PIN")]
    WrongPin { attempts_left: u32 },
    #[error("Too many wrong PINs. Try again later.")]
    LockedOut { until: i64 },
    #[error("Can't change a tool from {} to {}", from.label(), to.label())]
    InvalidTransition { from: ToolStatus, to: ToolStatus },
    #[error("PIN confirmation expired. Enter the PIN again.")]
    AuthExpired,
    #[error("ToolCheckout hasn't been set up yet")]
    NotSetUp,
    #[error(
        "This data was saved by a newer version of ToolCheckout (schema {found}; this app understands up to {supported}). Update the app."
    )]
    SchemaTooNew { found: u32, supported: u32 },
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::Db(_) => "db",
            CoreError::Io(_) => "io",
            CoreError::Invalid(_) => "invalid",
            CoreError::NotFound(_) => "not_found",
            CoreError::Conflict(_) => "conflict",
            CoreError::TechRequired => "tech_required",
            CoreError::WrongPin { .. } => "wrong_pin",
            CoreError::LockedOut { .. } => "locked_out",
            CoreError::InvalidTransition { .. } => "invalid_transition",
            CoreError::AuthExpired => "auth_expired",
            CoreError::NotSetUp => "not_set_up",
            CoreError::SchemaTooNew { .. } => "schema_too_new",
        }
    }
}

impl Serialize for CoreError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("CoreError", 4)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        match self {
            CoreError::WrongPin { attempts_left } => {
                st.serialize_field("attemptsLeft", attempts_left)?
            }
            CoreError::LockedOut { until } => st.serialize_field("lockedUntil", until)?,
            _ => st.skip_field("attemptsLeft")?,
        }
        st.end()
    }
}

pub fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(CoreError::Invalid(msg.into()))
}

pub fn not_found(what: impl Into<String>) -> CoreError {
    CoreError::NotFound(what.into())
}
