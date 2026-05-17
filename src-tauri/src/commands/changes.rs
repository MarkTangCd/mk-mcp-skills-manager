use tauri::State;

use crate::domain::{ChangeIntent, ChangePlan, ChangeSet, ChangeStatus};
use crate::error::CommandResult;
use crate::services::ChangeService;
use crate::state::AppState;

#[tauri::command]
pub fn changes_list(state: State<'_, AppState>) -> CommandResult<Vec<ChangeSet>> {
    let svc = ChangeService::new(state.db.clone());
    let list = svc.list()?;
    Ok(list)
}

#[tauri::command]
pub fn changes_get_plan(state: State<'_, AppState>, id: String) -> CommandResult<ChangePlan> {
    let svc = ChangeService::new(state.db.clone());
    let plan = svc.get_plan(&id)?;
    Ok(plan)
}

#[tauri::command]
pub fn changes_transition(
    state: State<'_, AppState>,
    id: String,
    status: ChangeStatus,
) -> CommandResult<ChangePlan> {
    let svc = ChangeService::new(state.db.clone());
    let plan = svc.transition(&id, status)?;
    Ok(plan)
}

#[tauri::command]
pub fn changes_create_plan(
    state: State<'_, AppState>,
    intent: ChangeIntent,
) -> CommandResult<ChangePlan> {
    let svc = ChangeService::new(state.db.clone());
    let plan = svc.create_plan_from_intent(&intent, &state.registry)?;
    Ok(plan)
}

#[tauri::command]
pub fn changes_apply(state: State<'_, AppState>, id: String) -> CommandResult<ChangePlan> {
    let svc = ChangeService::new(state.db.clone());
    let plan = svc.apply(
        &id,
        &state.backups,
        state.app_data.guard(),
        &state.registry,
    )?;
    Ok(plan)
}
