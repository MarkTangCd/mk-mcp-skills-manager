use tauri::State;

use crate::domain::ChangeSet;
use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn changes_list(_state: State<'_, AppState>) -> CommandResult<Vec<ChangeSet>> {
    Ok(vec![])
}
