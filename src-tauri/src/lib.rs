pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod security;
pub mod services;
pub mod state;

use tauri::Manager;

use crate::commands::{
    agents_list, app_get_dashboard, backups_list, changes_list, doctor_list_issues, projects_list,
    prompts_list,
};
use crate::db::Database;
use crate::services::AppDataService;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_root = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
            let app_data = AppDataService::initialize(&app_data_root)
                .map_err(|e| format!("failed to initialize app data: {e}"))?;
            let db = Database::open(app_data.layout().database_path.clone())
                .map_err(|e| format!("failed to open database: {e}"))?;
            app.manage(AppState::new(app_data, db));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_get_dashboard,
            agents_list,
            projects_list,
            doctor_list_issues,
            changes_list,
            backups_list,
            prompts_list,
            ping,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Health-check command kept from Phase 0 to verify the Rust <-> JS bridge.
#[tauri::command]
fn ping() -> &'static str {
    "pong"
}
