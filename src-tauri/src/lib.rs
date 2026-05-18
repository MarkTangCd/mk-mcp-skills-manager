pub mod adapters;
pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod security;
pub mod services;
pub mod state;

use tauri::Manager;

use crate::commands::{
    agents_list, app_get_dashboard, backups_list, backups_restore, changes_apply,
    changes_apply_plan, changes_create_plan, changes_get_plan, changes_list, changes_transition,
    doctor_issue_summary, doctor_list_issues, doctor_run, doctor_run_all, library_create,
    library_delete, library_get, library_list, library_update, projects_add, projects_get,
    projects_get_matrix, projects_latest_scans, projects_list, projects_remove, projects_rescan,
    prompts_create, prompts_delete, prompts_get, prompts_list, prompts_render, prompts_update,
    resources_list, skills_create, skills_delete, skills_disable, skills_enable, skills_get,
    skills_import, skills_list, skills_update, sub_agent_templates, sub_agents_create,
    sub_agents_delete, sub_agents_disable, sub_agents_enable, sub_agents_get, sub_agents_list,
    sub_agents_update,
};
use crate::db::Database;
use crate::services::{append_app_log, AppDataService};
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
            let _ = append_app_log(app_data.layout().logs.join("agenthub.log"), "app startup");
            let db = Database::open(app_data.layout().database_path.clone())
                .map_err(|e| format!("failed to open database: {e}"))?;
            app.manage(AppState::new(app_data, db));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_get_dashboard,
            agents_list,
            projects_list,
            projects_add,
            projects_get,
            projects_remove,
            projects_rescan,
            projects_latest_scans,
            projects_get_matrix,
            doctor_list_issues,
            doctor_run,
            doctor_run_all,
            doctor_issue_summary,
            changes_list,
            changes_get_plan,
            changes_transition,
            changes_create_plan,
            changes_apply,
            changes_apply_plan,
            backups_list,
            backups_restore,
            prompts_list,
            prompts_create,
            prompts_get,
            prompts_update,
            prompts_delete,
            prompts_render,
            resources_list,
            library_list,
            library_create,
            library_get,
            library_update,
            library_delete,
            skills_list,
            skills_create,
            skills_import,
            skills_get,
            skills_update,
            skills_delete,
            skills_enable,
            skills_disable,
            sub_agents_list,
            sub_agents_create,
            sub_agents_get,
            sub_agents_update,
            sub_agents_delete,
            sub_agents_enable,
            sub_agents_disable,
            sub_agent_templates,
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
