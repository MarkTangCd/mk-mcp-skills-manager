use tauri::State;

use crate::domain::Backup;
use crate::error::CommandResult;
use crate::services::ChangeService;
use crate::state::AppState;

#[tauri::command]
pub fn backups_list(state: State<'_, AppState>) -> CommandResult<Vec<Backup>> {
    let list = state.backups.list()?;
    Ok(list)
}

#[tauri::command]
pub fn backups_restore(state: State<'_, AppState>, id: String) -> CommandResult<Backup> {
    // 1. Restore files from the backup.
    state.backups.restore_change_set(&id)?;

    // 2. Find the linked change set and transition it to Restored.
    let change_svc = ChangeService::new(state.db.clone());
    let backup = state
        .backups
        .list()?
        .into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| crate::error::CommandError::new("backup_not_found", id.clone()))?;

    // Transition the linked change set to restored if it exists.
    let _: Result<_, _> =
        change_svc.transition(&backup.change_set_id, crate::domain::ChangeStatus::Restored);

    Ok(backup)
}
