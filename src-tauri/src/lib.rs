pub mod commands;
pub mod error;
pub mod services;
pub mod state;

use services::config::ConfigService;
use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let config = ConfigService::new(config_dir.join("settings.json"));
            app.manage(AppState {
                db: Mutex::new(None),
                vault: Mutex::new(None),
                config,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_open,
            commands::doc_open,
            commands::doc_save,
            commands::session_flush,
            commands::session_list,
            commands::session_restore,
            commands::session_discard,
            commands::vault_scan,
            commands::index_file,
            commands::index_reindex,
            commands::search_query,
            commands::attach_paste,
            commands::attach_read,
            commands::render_markdown,
            commands::export_document,
            commands::snapshot_create,
            commands::snapshot_restore,
            commands::snapshot_list,
            commands::window_create,
            commands::config_load,
            commands::config_save,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
