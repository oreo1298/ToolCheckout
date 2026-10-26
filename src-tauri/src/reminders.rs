//! Background job: delivers overdue and end-of-day desktop notifications.
//! When to notify is decided in `toolcheckout_core::reminders`.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use toolcheckout_core::reminders::ReminderState;

use crate::state::AppState;

const FIRST_CHECK: Duration = Duration::from_secs(10);
const EVERY: Duration = Duration::from_secs(30);

pub fn start(app: AppHandle) {
    let _ = std::thread::Builder::new()
        .name("reminders".into())
        .spawn(move || {
            let mut st = ReminderState::default();
            std::thread::sleep(FIRST_CHECK);
            loop {
                let notices = app
                    .state::<AppState>()
                    .with(|svc| svc.due_notices(&mut st))
                    .unwrap_or_default();
                for n in notices {
                    if let Err(e) = app
                        .notification()
                        .builder()
                        .title(&n.title)
                        .body(&n.body)
                        .show()
                    {
                        eprintln!("[reminders] notification failed: {e}");
                    }
                    // Also surfaced in the app, in case OS notifications are off.
                    let _ = app.emit("reminder", &n);
                }
                std::thread::sleep(EVERY);
            }
        });
}
