use tauri::State;

use crate::adapters::ScanContext;
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
    let ctx = if let Some(ref project_id) = intent.project_id {
        match state.projects.get(project_id) {
            Ok(project) => ScanContext::for_project(project.path.into()),
            Err(_) => ScanContext::empty(),
        }
    } else {
        ScanContext::empty()
    };
    let plan = svc.create_plan_from_intent(&intent, &state.registry, &ctx)?;
    Ok(plan)
}

#[tauri::command]
pub fn changes_apply_plan(state: State<'_, AppState>, plan_id: String) -> CommandResult<ChangePlan> {
    let svc = ChangeService::new(state.db.clone());
    let plan = svc.apply(
        &plan_id,
        &state.backups,
        state.app_data.guard(),
        &state.registry,
    )?;
    Ok(plan)
}

#[tauri::command]
pub fn changes_apply(state: State<'_, AppState>, id: String) -> CommandResult<ChangePlan> {
    changes_apply_plan(state, id)
}
