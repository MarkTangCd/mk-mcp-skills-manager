use tauri::State;

use crate::domain::ResourceType;
use crate::error::{CommandError, CommandResult};
use crate::services::ResourceRecord;
use crate::state::AppState;

#[tauri::command]
pub fn resources_list(
    state: State<'_, AppState>,
    resource_type: Option<ResourceType>,
) -> CommandResult<Vec<ResourceRecord>> {
    state
        .resources
        .list(resource_type)
        .map_err(|e| CommandError::new("resources_error", e.to_string()))
}
