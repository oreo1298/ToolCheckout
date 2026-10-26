//! Process-wide state: the open core service and where its data lives.

use std::sync::{Arc, Mutex, MutexGuard};

use toolcheckout_core::paths::{self, DataLocation};
use toolcheckout_core::{CoreError, Result, Service, SystemClock};

pub struct Inner {
    pub svc: Option<Service>,
    pub location: Option<DataLocation>,
    /// Why the database could not be opened (shown instead of the app).
    pub startup_error: Option<String>,
}

pub struct AppState(pub Mutex<Inner>);

impl AppState {
    pub fn new(location: std::result::Result<DataLocation, String>) -> Self {
        let mut inner = Inner {
            svc: None,
            location: None,
            startup_error: None,
        };
        match location {
            Ok(loc) => {
                match open_service(&loc) {
                    Ok(svc) => inner.svc = Some(svc),
                    Err(e) => {
                        inner.startup_error = Some(format!(
                            "Could not open the ToolCheckout data in {}: {e}",
                            loc.dir.display()
                        ))
                    }
                }
                inner.location = Some(loc);
            }
            Err(e) => inner.startup_error = Some(e),
        }
        AppState(Mutex::new(inner))
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock leaves the data consistent (SQLite
        // transactions roll back), so keep serving instead of poisoning forever.
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Runs `f` against the open service.
    pub fn with<T>(&self, f: impl FnOnce(&mut Service) -> Result<T>) -> Result<T> {
        let mut g = self.lock();
        let msg = g.startup_error.clone();
        let svc = g.svc.as_mut().ok_or_else(|| {
            CoreError::Invalid(msg.unwrap_or_else(|| "The database is not open".into()))
        })?;
        f(svc)
    }
}

impl Inner {
    /// Closes the current database and opens the one `loc` points at
    /// (switching demo/real data or after moving the data folder).
    pub fn reopen(&mut self, loc: DataLocation) -> Result<()> {
        self.svc = None; // close first so files can be replaced
        match open_service(&loc) {
            Ok(svc) => {
                self.svc = Some(svc);
                self.startup_error = None;
                self.location = Some(loc);
                Ok(())
            }
            Err(e) => {
                self.startup_error = Some(format!(
                    "Could not open the ToolCheckout data in {}: {e}",
                    loc.dir.display()
                ));
                self.location = Some(loc);
                Err(e)
            }
        }
    }

    pub fn location(&self) -> Result<DataLocation> {
        self.location
            .clone()
            .ok_or_else(|| CoreError::Invalid("The data folder is unknown".into()))
    }
}

/// Opens the demo database while demo mode is on, otherwise the real one.
pub fn open_service(loc: &DataLocation) -> Result<Service> {
    std::fs::create_dir_all(&loc.dir)?;
    let path = if loc.demo_marker().exists() {
        loc.demo_db_path()
    } else {
        loc.db_path()
    };
    Service::open(&path, Arc::new(SystemClock))
}

pub fn resolve_location() -> std::result::Result<DataLocation, String> {
    paths::resolve_current().map_err(|e| e.to_string())
}
