//! Tauri commands: thin wrappers over `toolcheckout_core::Service`.
//! No business rules live here. Commands that change data emit
//! `data-changed` so every open view refreshes.

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use toolcheckout_core::audit::{AuditQuery, AuditRow, VerifyReport};
use toolcheckout_core::paths::DataLocation;
use toolcheckout_core::service::{AppStatus, SetupResult};
use toolcheckout_core::session::TechSession;
use toolcheckout_core::settings::Settings;
use toolcheckout_core::technicians::Technician;
use toolcheckout_core::{CoreError, Service};

use crate::state::AppState;

pub type CmdResult<T> = Result<T, CoreError>;

pub const DATA_CHANGED: &str = "data-changed";

pub fn read<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&mut Service) -> toolcheckout_core::Result<T>,
) -> CmdResult<T> {
    state.with(f)
}

pub fn write<T>(
    app: &AppHandle,
    state: &State<'_, AppState>,
    f: impl FnOnce(&mut Service) -> toolcheckout_core::Result<T>,
) -> CmdResult<T> {
    let out = state.with(f)?;
    let _ = app.emit(DATA_CHANGED, ());
    Ok(out)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub status: Option<AppStatus>,
    pub location: Option<DataLocation>,
    pub startup_error: Option<String>,
    pub platform: &'static str,
    pub version: &'static str,
}

#[tauri::command(async)]
pub fn bootstrap(state: State<'_, AppState>) -> Bootstrap {
    let mut g = state.lock();
    let status = g.svc.as_mut().and_then(|s| s.status().ok());
    Bootstrap {
        status,
        location: g.location.clone(),
        startup_error: g.startup_error.clone(),
        platform: std::env::consts::OS,
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command(async)]
pub fn app_status(state: State<'_, AppState>) -> CmdResult<AppStatus> {
    read(&state, |s| s.status())
}

#[tauri::command(async)]
pub fn setup_shop(
    app: AppHandle,
    state: State<'_, AppState>,
    shop_name: String,
    tech_name: String,
    pin: String,
) -> CmdResult<SetupResult> {
    write(&app, &state, |s| s.setup(&shop_name, &tech_name, &pin))
}

// ---------------------------------------------------------------- tech mode

#[tauri::command(async)]
pub fn technician_names(state: State<'_, AppState>) -> CmdResult<Vec<Technician>> {
    read(&state, |s| s.technician_names())
}

#[tauri::command(async)]
pub fn unlock(state: State<'_, AppState>, tech_id: i64, pin: String) -> CmdResult<TechSession> {
    read(&state, |s| s.unlock(tech_id, &pin))
}

#[tauri::command(async)]
pub fn lock(state: State<'_, AppState>) -> CmdResult<()> {
    read(&state, |s| {
        s.lock();
        Ok(())
    })
}

#[tauri::command(async)]
pub fn touch(state: State<'_, AppState>) -> CmdResult<Option<TechSession>> {
    read(&state, |s| Ok(s.touch()))
}

#[tauri::command(async)]
pub fn session(state: State<'_, AppState>) -> CmdResult<Option<TechSession>> {
    read(&state, |s| Ok(s.session()))
}

#[tauri::command(async)]
pub fn recover(
    app: AppHandle,
    state: State<'_, AppState>,
    code: String,
    tech_id: i64,
    new_pin: String,
) -> CmdResult<String> {
    write(&app, &state, |s| s.recover(&code, tech_id, &new_pin))
}

// ---------------------------------------------------------------- technicians

#[tauri::command(async)]
pub fn list_technicians(state: State<'_, AppState>) -> CmdResult<Vec<Technician>> {
    read(&state, |s| s.list_technicians())
}

#[tauri::command(async)]
pub fn add_technician(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    pin: String,
) -> CmdResult<Technician> {
    write(&app, &state, |s| s.add_technician(&name, &pin))
}

#[tauri::command(async)]
pub fn update_technician(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    name: String,
    active: bool,
) -> CmdResult<Technician> {
    write(&app, &state, |s| s.update_technician(id, &name, active))
}

#[tauri::command(async)]
pub fn set_technician_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    pin: String,
) -> CmdResult<()> {
    write(&app, &state, |s| s.set_technician_pin(id, &pin))
}

#[tauri::command(async)]
pub fn regenerate_recovery_code(app: AppHandle, state: State<'_, AppState>) -> CmdResult<String> {
    write(&app, &state, |s| s.regenerate_recovery_code())
}

// ---------------------------------------------------------------- settings & audit

#[tauri::command(async)]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> CmdResult<Settings> {
    write(&app, &state, |s| s.update_settings(settings))
}

#[tauri::command(async)]
pub fn verify_audit(state: State<'_, AppState>) -> CmdResult<VerifyReport> {
    read(&state, |s| s.verify_audit())
}

#[tauri::command(async)]
pub fn audit_log(state: State<'_, AppState>, query: AuditQuery) -> CmdResult<Vec<AuditRow>> {
    read(&state, |s| s.audit_log(&query))
}
