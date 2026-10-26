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

// ---------------------------------------------------------------- tools

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use toolcheckout_core::model::Condition;
use toolcheckout_core::tools::{Tool, ToolDetail, ToolFilter, ToolInput, ToolRow};

#[tauri::command(async)]
pub fn list_tools(state: State<'_, AppState>, filter: ToolFilter) -> CmdResult<Vec<ToolRow>> {
    read(&state, |s| s.list_tools(&filter))
}

#[tauri::command(async)]
pub fn tool_row(state: State<'_, AppState>, id: i64) -> CmdResult<ToolRow> {
    read(&state, |s| s.tool_row(id))
}

#[tauri::command(async)]
pub fn tool_categories(state: State<'_, AppState>) -> CmdResult<Vec<String>> {
    read(&state, |s| s.tool_categories())
}

#[tauri::command(async)]
pub fn tool_detail(state: State<'_, AppState>, id: i64) -> CmdResult<ToolDetail> {
    read(&state, |s| s.tool_detail(id))
}

#[tauri::command(async)]
pub fn create_tool(
    app: AppHandle,
    state: State<'_, AppState>,
    input: ToolInput,
) -> CmdResult<Tool> {
    write(&app, &state, |s| s.create_tool(input))
}

#[tauri::command(async)]
pub fn update_tool(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    input: ToolInput,
) -> CmdResult<Tool> {
    write(&app, &state, |s| s.update_tool(id, input))
}

#[tauri::command(async)]
pub fn update_tool_condition(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    condition: Condition,
    notes: String,
    photo_id: Option<i64>,
    reason: String,
) -> CmdResult<Tool> {
    write(&app, &state, |s| {
        s.update_tool_condition(id, condition, &notes, photo_id, &reason)
    })
}

#[tauri::command(async)]
pub fn send_tool_to_repair(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    note: String,
) -> CmdResult<Tool> {
    write(&app, &state, |s| s.send_tool_to_repair(id, &note))
}

#[tauri::command(async)]
pub fn tool_back_from_repair(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    condition: Condition,
    notes: String,
    photo_id: Option<i64>,
) -> CmdResult<Tool> {
    write(&app, &state, |s| {
        s.tool_back_from_repair(id, condition, &notes, photo_id)
    })
}

#[tauri::command(async)]
pub fn retire_tool(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    reason: String,
) -> CmdResult<Tool> {
    write(&app, &state, |s| s.retire_tool(id, &reason))
}

#[tauri::command(async)]
pub fn reinstate_tool(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    condition: Condition,
    notes: String,
) -> CmdResult<Tool> {
    write(&app, &state, |s| s.reinstate_tool(id, condition, &notes))
}

// ---------------------------------------------------------------- photos

/// Accepts a `data:image/...;base64,` URL (or bare base64) from the UI.
#[tauri::command(async)]
pub fn save_photo(state: State<'_, AppState>, data: String) -> CmdResult<i64> {
    let b64 = data.split_once(',').map_or(data.as_str(), |(_, b)| b);
    let bytes = B64
        .decode(b64.trim())
        .map_err(|_| CoreError::Invalid("That photo couldn't be read".into()))?;
    read(&state, |s| s.save_photo(&bytes))
}

#[tauri::command(async)]
pub fn get_photo(state: State<'_, AppState>, id: i64) -> CmdResult<String> {
    let (mime, bytes) = read(&state, |s| s.photo(id))?;
    Ok(format!("data:{mime};base64,{}", B64.encode(bytes)))
}

// ---------------------------------------------------------------- borrowers

use toolcheckout_core::borrowers::{
    Borrower, BorrowerAuth, BorrowerFilter, BorrowerInput, BorrowerPick, BorrowerRow,
};

#[tauri::command(async)]
pub fn borrower_picker(state: State<'_, AppState>) -> CmdResult<Vec<BorrowerPick>> {
    read(&state, |s| s.borrower_picker())
}

#[tauri::command(async)]
pub fn list_borrowers(
    state: State<'_, AppState>,
    filter: BorrowerFilter,
) -> CmdResult<Vec<BorrowerRow>> {
    read(&state, |s| s.list_borrowers(&filter))
}

#[tauri::command(async)]
pub fn get_borrower(state: State<'_, AppState>, id: i64) -> CmdResult<Borrower> {
    read(&state, |s| s.get_borrower(id))
}

#[tauri::command(async)]
pub fn create_borrower(
    app: AppHandle,
    state: State<'_, AppState>,
    input: BorrowerInput,
    pin: Option<String>,
) -> CmdResult<Borrower> {
    write(&app, &state, |s| s.create_borrower(input, pin.as_deref()))
}

#[tauri::command(async)]
pub fn update_borrower(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    input: BorrowerInput,
    active: bool,
) -> CmdResult<Borrower> {
    write(&app, &state, |s| s.update_borrower(id, input, active))
}

#[tauri::command(async)]
pub fn set_borrower_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    pin: String,
) -> CmdResult<()> {
    write(&app, &state, |s| s.set_borrower_pin(id, &pin))
}

#[tauri::command(async)]
pub fn authenticate_borrower(
    state: State<'_, AppState>,
    id: i64,
    pin: Option<String>,
) -> CmdResult<BorrowerAuth> {
    read(&state, |s| s.authenticate_borrower(id, pin.as_deref()))
}
