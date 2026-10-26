//! ToolCheckout core: every business rule, independent of the UI.
//!
//! The desktop shell (src-tauri) only calls [`Service`] methods. Status
//! transitions, PIN checks and lockouts, the audit log, overdue math and
//! reports all live here so they are unit-tested and can't be bypassed.

pub mod audit;
pub mod clock;
pub mod db;
pub mod error;
pub mod model;
pub mod paths;
pub mod pin;
pub mod service;
pub mod session;
pub mod settings;
pub mod status;
pub mod technicians;

pub use clock::{Clock, ManualClock, SystemClock};
pub use error::{CoreError, Result};
pub use service::Service;
