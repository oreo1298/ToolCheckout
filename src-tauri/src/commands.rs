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

use crate::state::{AppState, Inner};

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

// ---------------------------------------------------------------- loans & incidents

use toolcheckout_core::borrowers::BorrowerDetail;
use toolcheckout_core::incidents::{Incident, IncidentFilter};
use toolcheckout_core::loans::{
    CheckoutItem, CheckoutReceipt, Home, InspectionResult, LoanFilter, LoanRow, ScanResult,
};
use toolcheckout_core::model::Outcome;

#[tauri::command(async)]
pub fn home(state: State<'_, AppState>) -> CmdResult<Home> {
    read(&state, |s| s.home())
}

#[tauri::command(async)]
pub fn resolve_scan(state: State<'_, AppState>, code: String) -> CmdResult<ScanResult> {
    read(&state, |s| s.resolve_scan(&code))
}

#[tauri::command(async)]
pub fn open_loan_for_tool(state: State<'_, AppState>, tool_id: i64) -> CmdResult<Option<LoanRow>> {
    read(&state, |s| s.open_loan_for_tool(tool_id))
}

#[tauri::command(async)]
pub fn checkout(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
    items: Vec<CheckoutItem>,
) -> CmdResult<CheckoutReceipt> {
    write(&app, &state, |s| s.checkout(&token, &items))
}

#[tauri::command(async)]
pub fn report_problem(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
    tool_id: i64,
    notes: String,
    photo_id: Option<i64>,
) -> CmdResult<Incident> {
    write(&app, &state, |s| {
        s.report_problem(&token, tool_id, &notes, photo_id)
    })
}

#[tauri::command(async)]
pub fn drop_off(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
    checkout_ids: Vec<i64>,
    note: String,
) -> CmdResult<Vec<LoanRow>> {
    write(&app, &state, |s| s.drop_off(&token, &checkout_ids, &note))
}

#[tauri::command(async)]
pub fn inspect_return(
    app: AppHandle,
    state: State<'_, AppState>,
    checkout_id: i64,
    condition: Condition,
    notes: String,
    photo_id: Option<i64>,
) -> CmdResult<InspectionResult> {
    write(&app, &state, |s| {
        s.inspect_return(checkout_id, condition, &notes, photo_id)
    })
}

#[tauri::command(async)]
pub fn inspect_tool(
    app: AppHandle,
    state: State<'_, AppState>,
    tool_id: i64,
    condition: Condition,
    notes: String,
    photo_id: Option<i64>,
) -> CmdResult<InspectionResult> {
    write(&app, &state, |s| {
        s.inspect_tool(tool_id, condition, &notes, photo_id)
    })
}

#[tauri::command(async)]
pub fn mark_lost(
    app: AppHandle,
    state: State<'_, AppState>,
    tool_id: i64,
    note: String,
) -> CmdResult<Incident> {
    write(&app, &state, |s| s.mark_lost(tool_id, &note))
}

#[tauri::command(async)]
pub fn mark_found(
    app: AppHandle,
    state: State<'_, AppState>,
    tool_id: i64,
    condition: Condition,
    notes: String,
    photo_id: Option<i64>,
) -> CmdResult<InspectionResult> {
    write(&app, &state, |s| {
        s.mark_found(tool_id, condition, &notes, photo_id)
    })
}

#[tauri::command(async)]
pub fn loan_history(state: State<'_, AppState>, filter: LoanFilter) -> CmdResult<Vec<LoanRow>> {
    read(&state, |s| s.loan_history(&filter))
}

#[tauri::command(async)]
pub fn list_incidents(
    state: State<'_, AppState>,
    filter: IncidentFilter,
) -> CmdResult<Vec<Incident>> {
    read(&state, |s| s.list_incidents(&filter))
}

#[tauri::command(async)]
pub fn resolve_incident(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    outcome: Outcome,
    note: String,
) -> CmdResult<Incident> {
    write(&app, &state, |s| s.resolve_incident(id, outcome, &note))
}

#[tauri::command(async)]
pub fn borrower_detail(state: State<'_, AppState>, id: i64) -> CmdResult<BorrowerDetail> {
    read(&state, |s| s.borrower_detail(id))
}

// ---------------------------------------------------------------- reports

use tauri_plugin_dialog::DialogExt;
use toolcheckout_core::reports::{BorrowerSummary, ReportFilter, ToolSummary};

#[tauri::command(async)]
pub fn report_borrowers(
    state: State<'_, AppState>,
    filter: ReportFilter,
) -> CmdResult<Vec<BorrowerSummary>> {
    read(&state, |s| s.report_borrowers(&filter))
}

#[tauri::command(async)]
pub fn report_tools(
    state: State<'_, AppState>,
    filter: ReportFilter,
) -> CmdResult<Vec<ToolSummary>> {
    read(&state, |s| s.report_tools(&filter))
}

#[tauri::command(async)]
pub fn report_loans(state: State<'_, AppState>, filter: ReportFilter) -> CmdResult<Vec<LoanRow>> {
    read(&state, |s| s.report_loans(&filter))
}

#[tauri::command(async)]
pub fn report_incidents(
    state: State<'_, AppState>,
    filter: ReportFilter,
    open_only: bool,
) -> CmdResult<Vec<Incident>> {
    read(&state, |s| s.report_incidents(&filter, open_only))
}

/// Builds the CSV in the backend, asks where to save it, writes it.
/// Returns the saved path, or None if the user cancelled.
#[tauri::command(async)]
pub fn export_report_csv(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    filter: ReportFilter,
    file_name: String,
) -> CmdResult<Option<String>> {
    let csv = read(&state, |s| s.report_csv(&kind, &filter))?;
    let Some(chosen) = app
        .dialog()
        .file()
        .set_title("Save report as CSV")
        .set_file_name(&file_name)
        .add_filter("CSV spreadsheet", &["csv"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let mut path = chosen
        .into_path()
        .map_err(|e| CoreError::Invalid(format!("Can't save there: {e}")))?;
    if path.extension().is_none() {
        path.set_extension("csv");
    }
    std::fs::write(&path, csv)?;
    Ok(Some(path.display().to_string()))
}

// ---------------------------------------------------------------- backups

use std::path::PathBuf;
use toolcheckout_core::backup::{BackupCheck, BackupInfo, BackupStatus};
use toolcheckout_core::paths;

fn picked(p: Option<tauri_plugin_dialog::FilePath>) -> CmdResult<Option<PathBuf>> {
    p.map(|f| {
        f.into_path()
            .map_err(|e| CoreError::Invalid(format!("Can't use that location: {e}")))
    })
    .transpose()
}

#[tauri::command(async)]
pub fn backup_status(state: State<'_, AppState>) -> CmdResult<BackupStatus> {
    read(&state, |s| s.backup_status())
}

/// Manual backup into the backup folder (never pruned).
#[tauri::command(async)]
pub fn backup_now(app: AppHandle, state: State<'_, AppState>) -> CmdResult<String> {
    write(&app, &state, |s| {
        let dest = s.backup_dir().join(s.default_backup_name());
        s.backup_to(&dest)?;
        Ok(dest.display().to_string())
    })
}

/// Manual backup to a file the technician chooses (e.g. a USB stick).
#[tauri::command(async)]
pub fn save_backup_as(app: AppHandle, state: State<'_, AppState>) -> CmdResult<Option<String>> {
    let name = read(&state, |s| {
        s.ensure_tech()?; // before showing a dialog
        Ok(s.default_backup_name())
    })?;
    let Some(dest) = picked(
        app.dialog()
            .file()
            .set_title("Save a ToolCheckout backup")
            .set_file_name(&name)
            .add_filter("ToolCheckout backup", &["db"])
            .blocking_save_file(),
    )?
    else {
        return Ok(None);
    };
    write(&app, &state, |s| s.backup_to(&dest))?;
    Ok(Some(dest.display().to_string()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedBackup {
    pub path: String,
    pub info: BackupInfo,
}

/// Step 1 of restore: choose a file and look inside it.
#[tauri::command(async)]
pub fn pick_backup_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<Option<PickedBackup>> {
    let dir = read(&state, |s| {
        s.ensure_tech()?;
        Ok(s.backup_dir())
    })?;
    let Some(path) = picked(
        app.dialog()
            .file()
            .set_title("Choose a backup to restore")
            .set_directory(dir)
            .add_filter("ToolCheckout backup", &["db"])
            .blocking_pick_file(),
    )?
    else {
        return Ok(None);
    };
    let info = read(&state, |s| s.inspect_backup(&path))?;
    Ok(Some(PickedBackup {
        path: path.display().to_string(),
        info,
    }))
}

/// Step 2 of restore, after the technician confirmed.
#[tauri::command(async)]
pub fn restore_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CmdResult<String> {
    write(&app, &state, |s| s.restore_from(&PathBuf::from(path))).map(|p| p.display().to_string())
}

#[tauri::command(async)]
pub fn verify_against_backups(state: State<'_, AppState>) -> CmdResult<Vec<BackupCheck>> {
    read(&state, |s| s.verify_against_backups())
}

#[tauri::command(async)]
pub fn pick_folder(app: AppHandle, title: String) -> CmdResult<Option<String>> {
    Ok(
        picked(app.dialog().file().set_title(&title).blocking_pick_folder())?
            .map(|p| p.display().to_string()),
    )
}

// ---------------------------------------------------------------- data folder

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderInfo {
    pub path: String,
    pub existing: Option<BackupInfo>,
    pub writable: bool,
}

/// Changing which data this PC uses needs a technician of the current data,
/// unless there's nothing to protect yet: it isn't set up, or it couldn't be
/// opened (e.g. a shared folder on a missing drive). Never from the demo.
fn guard_data_change(g: &mut Inner) -> CmdResult<()> {
    let Some(svc) = g.svc.as_mut() else {
        return Ok(());
    };
    if svc.is_demo()? {
        return Err(CoreError::Conflict("Leave the demo first".into()));
    }
    if svc.is_set_up()? {
        svc.ensure_tech()?;
    }
    Ok(())
}

/// What's in a folder we might move the data to.
#[tauri::command(async)]
pub fn inspect_data_folder(state: State<'_, AppState>, path: String) -> CmdResult<FolderInfo> {
    guard_data_change(&mut state.lock())?;
    let dir = PathBuf::from(&path);
    if !dir.is_absolute() {
        return Err(CoreError::Invalid("Choose a full folder path".into()));
    }
    let db = dir.join(paths::DB_FILE);
    let existing = if db.exists() {
        Some(toolcheckout_core::backup::inspect(&db)?)
    } else {
        None
    };
    let probe = dir.join(".toolcheckout-write-test");
    let writable = std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(probe);
    Ok(FolderInfo {
        path,
        existing,
        writable,
    })
}

/// Moves (copies) the data to `path`, or switches to the data already there,
/// then remembers the folder and reopens. `path = None` goes back to the
/// default per-user folder.
#[tauri::command(async)]
pub fn relocate_data(
    app: AppHandle,
    state: State<'_, AppState>,
    path: Option<String>,
    copy: bool,
) -> CmdResult<DataLocation> {
    let mut g = state.lock();
    let loc = g.location()?;
    if matches!(
        loc.mode,
        paths::LocationMode::Portable | paths::LocationMode::Environment
    ) {
        return Err(CoreError::Conflict(
            "The data folder is fixed in this mode (portable build or TOOLCHECKOUT_DATA_DIR)"
                .into(),
        ));
    }
    let target = path
        .map(PathBuf::from)
        .unwrap_or_else(|| loc.default_dir.clone());
    if target == loc.dir {
        return Ok(loc);
    }
    guard_data_change(&mut g)?;
    if copy {
        let svc = g.svc.as_mut().ok_or(CoreError::NotSetUp)?;
        svc.copy_database_to(&target, paths::DB_FILE)?;
    } else if target != loc.default_dir && !target.join(paths::DB_FILE).exists() {
        return Err(CoreError::Invalid(format!(
            "There is no ToolCheckout data in {}",
            target.display()
        )));
    }
    let custom = (target != loc.default_dir).then_some(target.as_path());
    paths::write_pointer(&loc.default_dir, custom)?;
    let new_loc = paths::resolve(None, None, &loc.default_dir);
    g.reopen(new_loc.clone())?;
    drop(g);
    let _ = app.emit(DATA_CHANGED, ());
    Ok(new_loc)
}

/// Tries to open the data again, e.g. after the drive holding a shared data
/// folder was reconnected.
#[tauri::command(async)]
pub fn retry_open(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    let loc = crate::state::resolve_location().map_err(CoreError::Invalid)?;
    state.lock().reopen(loc)?;
    let _ = app.emit(DATA_CHANGED, ());
    Ok(())
}

// ---------------------------------------------------------------- demo

#[tauri::command(async)]
pub fn enter_demo(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    let mut g = state.lock();
    if let Some(svc) = g.svc.as_mut()
        && svc.is_set_up()?
        && !svc.is_demo()?
    {
        svc.ensure_tech()?; // leaving real data for the demo needs a technician
    }
    let loc = g.location()?;
    std::fs::create_dir_all(&loc.dir)?;
    g.svc = None;
    toolcheckout_core::demo::create(&loc.demo_db_path(), chrono::Utc::now().timestamp_millis())?;
    std::fs::write(
        loc.demo_marker(),
        "Demo mode: ToolCheckout is showing sample data.\n",
    )?;
    g.reopen(loc)?;
    drop(g);
    let _ = app.emit(DATA_CHANGED, ());
    Ok(())
}

/// Leaves demo mode: deletes the demo database and reopens the real data.
#[tauri::command(async)]
pub fn exit_demo(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    let mut g = state.lock();
    let loc = g.location()?;
    g.svc = None;
    let _ = std::fs::remove_file(loc.demo_marker());
    for ext in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{ext}", loc.demo_db_path().display()));
    }
    g.reopen(loc)?;
    drop(g);
    let _ = app.emit(DATA_CHANGED, ());
    Ok(())
}
