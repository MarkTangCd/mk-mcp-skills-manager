use serde::Serialize;
use tauri::State;

use crate::domain::{Agent, ChangeSet, DoctorIssue, ScanSnapshot};
use crate::error::CommandResult;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub agents: Vec<Agent>,
    pub recent_scans: Vec<ScanSnapshot>,
    pub open_issues: Vec<DoctorIssue>,
    pub recent_changes: Vec<ChangeSet>,
    pub bootstrap: BootstrapInfo,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapInfo {
    pub data_dir: String,
    pub database_path: String,
    pub schema_version: u32,
}

/// Read-only Dashboard payload. Returns real on-disk state so the frontend
/// wiring is exercised end-to-end.
#[tauri::command]
pub fn app_get_dashboard(state: State<'_, AppState>) -> CommandResult<DashboardSnapshot> {
    let layout = state.app_data.layout();
    let schema_version: u32 = state
        .db
        .with_conn(|c| c.query_row("PRAGMA user_version", [], |r| r.get(0)))
        .unwrap_or(0);
    let recent_scans = state
        .scans
        .latest_snapshots(None)
        .map_err(|e| crate::error::CommandError::new("scan_error", e.to_string()))?;
    let open_issues = state
        .doctor
        .list_issues(None, None, None)
        .map_err(|e| crate::error::CommandError::new("doctor_error", e.to_string()))?;

    Ok(DashboardSnapshot {
        agents: state.agents.list(),
        recent_scans,
        open_issues,
        recent_changes: vec![],
        bootstrap: BootstrapInfo {
            data_dir: layout.root.to_string_lossy().to_string(),
            database_path: layout.database_path.to_string_lossy().to_string(),
            schema_version,
        },
    })
}
