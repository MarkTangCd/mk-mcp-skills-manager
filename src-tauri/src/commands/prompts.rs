use tauri::State;

use crate::domain::PromptTemplate;
use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn prompts_list(_state: State<'_, AppState>) -> CommandResult<Vec<PromptTemplate>> {
    Ok(vec![])
}
