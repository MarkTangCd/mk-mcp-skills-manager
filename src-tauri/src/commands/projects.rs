use tauri::State;

use crate::adapters::ScanContext;
use crate::domain::{AgentKind, Project, ScanSnapshot};
use crate::error::{CommandError, CommandResult};
use crate::services::{ProjectError, ProjectMatrix};
use crate::state::AppState;

impl From<ProjectError> for CommandError {
    fn from(err: ProjectError) -> Self {
        let (code, recoverable) = match &err {
            ProjectError::NotFound(_) => ("project_not_found", true),
            ProjectError::NotDirectory(_) => ("project_not_directory", true),
            ProjectError::Duplicate(_) => ("project_duplicate", true),
            ProjectError::UnknownId(_) => ("project_unknown_id", true),
            ProjectError::Io(_) => ("io_error", true),
            ProjectError::Db(_) => ("db_error", false),
        };
        let mut e = CommandError::new(code, err.to_string());
        if !recoverable {
            e = e.fatal();
        }
        e
    }
}

#[tauri::command]
pub fn projects_list(state: State<'_, AppState>) -> CommandResult<Vec<Project>> {
    Ok(state.projects.list()?)
}

#[tauri::command]
pub fn projects_add(
    state: State<'_, AppState>,
    path: String,
    name: Option<String>,
) -> CommandResult<Project> {
    let project = state.projects.add(&path, name.as_deref())?;
    state.app_data.guard().allow(std::path::Path::new(&project.path));
    Ok(project)
}

#[tauri::command]
pub fn projects_get(state: State<'_, AppState>, id: String) -> CommandResult<Project> {
    Ok(state.projects.get(&id)?)
}

#[tauri::command]
pub fn projects_remove(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    state.projects.remove(&id)?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectScanReport {
    pub snapshots: Vec<ScanSnapshot>,
    pub adapter_errors: Vec<AdapterErrorEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterErrorEntry {
    pub agent_kind: AgentKind,
    pub message: String,
}

#[tauri::command]
pub fn projects_rescan(state: State<'_, AppState>, id: String) -> CommandResult<ProjectScanReport> {
    let project = state.projects.get(&id)?;
    let ctx = ScanContext::for_project(std::path::PathBuf::from(&project.path));
    let report = state
        .scans
        .run(Some(&project.id), &ctx)
        .map_err(|e| CommandError::new("scan_error", e.to_string()))?;
    Ok(ProjectScanReport {
        snapshots: report.snapshots,
        adapter_errors: report
            .adapter_errors
            .into_iter()
            .map(|(k, m)| AdapterErrorEntry {
                agent_kind: k,
                message: m,
            })
            .collect(),
    })
}

#[tauri::command]
pub fn projects_latest_scans(
    state: State<'_, AppState>,
    id: String,
) -> CommandResult<Vec<ScanSnapshot>> {
    state
        .scans
        .latest_snapshots(Some(&id))
        .map_err(|e| CommandError::new("scan_error", e.to_string()))
}

#[tauri::command]
pub fn projects_get_matrix(state: State<'_, AppState>, id: String) -> CommandResult<ProjectMatrix> {
    state.projects.get(&id)?;
    state
        .resources
        .project_matrix(&id)
        .map_err(|e| CommandError::new("matrix_error", e.to_string()))
}
