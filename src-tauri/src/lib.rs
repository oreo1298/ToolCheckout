//! ToolCheckout desktop shell. All business rules live in `toolcheckout-core`;
//! this crate wires them to Tauri commands, windows, OS integration and
//! background jobs.

mod appearance;
mod commands;
mod state;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

pub fn run() {
    let location = state::resolve_location();
    // Portable mode keeps the WebView profile beside the exe too.
    let webview_dir = location.as_ref().ok().and_then(|l| l.webview_dir());

    tauri::Builder::default()
        // Must be registered first: a second launch focuses the running window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(appearance::AppearanceState::default())
        .manage(AppState::new(location))
        .invoke_handler(tauri::generate_handler![
            appearance::get_os_appearance,
            commands::bootstrap,
            commands::app_status,
            commands::setup_shop,
            commands::technician_names,
            commands::unlock,
            commands::lock,
            commands::touch,
            commands::session,
            commands::recover,
            commands::list_technicians,
            commands::add_technician,
            commands::update_technician,
            commands::set_technician_pin,
            commands::regenerate_recovery_code,
            commands::update_settings,
            commands::verify_audit,
            commands::audit_log,
        ])
        .setup(move |app| {
            let mut win = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("ToolCheckout")
                .inner_size(1280.0, 820.0)
                .min_inner_size(900.0, 600.0)
                .center();
            if let Some(dir) = webview_dir {
                win = win.data_directory(dir);
            }
            win.build()?;
            appearance::start(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ToolCheckout");
}
