//! Background job: makes the daily automatic backup.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::DATA_CHANGED;
use crate::state::AppState;

const FIRST_CHECK: Duration = Duration::from_secs(60);
const EVERY: Duration = Duration::from_secs(30 * 60);

pub fn start(app: AppHandle) {
    let _ = std::thread::Builder::new()
        .name("backups".into())
        .spawn(move || {
            std::thread::sleep(FIRST_CHECK);
            loop {
                let made = app.state::<AppState>().with(|svc| {
                    if svc.auto_backup_due()? {
                        svc.run_auto_backup().map(Some)
                    } else {
                        Ok(None)
                    }
                });
                match made {
                    Ok(Some(path)) => {
                        eprintln!("[backups] wrote {}", path.display());
                        let _ = app.emit(DATA_CHANGED, ());
                    }
                    Ok(None) => {}
                    Err(e) => eprintln!("[backups] automatic backup failed: {e}"),
                }
                std::thread::sleep(EVERY);
            }
        });
}
