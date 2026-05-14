use tauri::State;

use crate::domain::Project;
use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn projects_list(_state: State<'_, AppState>) -> CommandResult<Vec<Project>> {
    Ok(vec![])
}
