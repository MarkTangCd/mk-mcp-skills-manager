use tauri::State;

use crate::domain::Agent;
use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn agents_list(_state: State<'_, AppState>) -> CommandResult<Vec<Agent>> {
    Ok(vec![])
}
