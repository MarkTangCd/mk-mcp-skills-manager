use tauri::State;

use crate::domain::DoctorIssue;
use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn doctor_list_issues(_state: State<'_, AppState>) -> CommandResult<Vec<DoctorIssue>> {
    Ok(vec![])
}
