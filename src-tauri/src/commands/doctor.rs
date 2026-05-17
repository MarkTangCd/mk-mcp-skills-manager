use tauri::State;

use crate::domain::{DoctorIssue, IssueSeverity};
use crate::error::CommandResult;
use crate::services::IssueSummary;
use crate::state::AppState;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorRunResponse {
    pub issues: Vec<DoctorIssue>,
    pub summary: IssueSummary,
}

/// List unresolved doctor issues with optional filters.
#[tauri::command]
pub fn doctor_list_issues(
    state: State<'_, AppState>,
    severity: Option<String>,
    category: Option<String>,
    project_id: Option<String>,
) -> CommandResult<Vec<DoctorIssue>> {
    let sev = severity.and_then(|s| match s.as_str() {
        "critical" => Some(IssueSeverity::Critical),
        "warning" => Some(IssueSeverity::Warning),
        "info" => Some(IssueSeverity::Info),
        _ => None,
    });
    let issues = state
        .doctor
        .list_issues(sev, category.as_deref(), project_id.as_deref())
        .map_err(|e| crate::error::CommandError::new("doctor_error", e.to_string()))?;
    Ok(issues)
}

/// Run doctor checks for a specific project (or globally when project_id is omitted).
#[tauri::command]
pub fn doctor_run(
    state: State<'_, AppState>,
    project_id: Option<String>,
) -> CommandResult<DoctorRunResponse> {
    let issues = state
        .doctor
        .run_for_project(project_id.as_deref())
        .map_err(|e| crate::error::CommandError::new("doctor_error", e.to_string()))?;
    let summary = state
        .doctor
        .issue_summary()
        .map_err(|e| crate::error::CommandError::new("doctor_error", e.to_string()))?;
    Ok(DoctorRunResponse { issues, summary })
}

/// Run doctor checks across all projects and global state.
#[tauri::command]
pub fn doctor_run_all(state: State<'_, AppState>) -> CommandResult<DoctorRunResponse> {
    let issues = state
        .doctor
        .run_all()
        .map_err(|e| crate::error::CommandError::new("doctor_error", e.to_string()))?;
    let summary = state
        .doctor
        .issue_summary()
        .map_err(|e| crate::error::CommandError::new("doctor_error", e.to_string()))?;
    Ok(DoctorRunResponse { issues, summary })
}

/// Return a lightweight summary of open issues for the Dashboard.
#[tauri::command]
pub fn doctor_issue_summary(state: State<'_, AppState>) -> CommandResult<IssueSummary> {
    let summary = state
        .doctor
        .issue_summary()
        .map_err(|e| crate::error::CommandError::new("doctor_error", e.to_string()))?;
    Ok(summary)
}
