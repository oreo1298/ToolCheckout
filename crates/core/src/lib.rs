//! ToolCheckout core: every business rule, independent of the UI.
//!
//! The desktop shell (src-tauri) only calls [`Service`] methods. Status
//! transitions, PIN checks and lockouts, the audit log, overdue math and
//! reports all live here so they are unit-tested and can't be bypassed.

pub mod audit;
pub mod backup;
pub mod borrowers;
pub mod clock;
pub mod csv;
pub mod db;
pub mod demo;
pub mod error;
pub mod incidents;
pub mod loans;
pub mod model;
pub mod paths;
pub mod photos;
pub mod pin;
pub mod reminders;
pub mod reports;
pub mod service;
pub mod session;
pub mod settings;
pub mod status;
pub mod technicians;
pub mod tools;

pub use clock::{Clock, ManualClock, SystemClock};
pub use error::{CoreError, Result};
pub use service::Service;
