use tauri::State;

use crate::domain::Backup;
use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn backups_list(_state: State<'_, AppState>) -> CommandResult<Vec<Backup>> {
    Ok(vec![])
}
