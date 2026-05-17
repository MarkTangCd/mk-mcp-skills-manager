// AgentAdapter trait and supporting types.
//
// The trait deliberately separates *intent* (ChangeIntent) from
// *plan* (ChangePlanDraft). The plan is what ChangeService later
// turns into FilePatch + Backup + atomic apply. Adapters never
// perform I/O writes themselves.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::domain::{
    AgentKind, ChangeOperation, DoctorIssue, FilePatch, McpServer, PiResource, ResourceType,
    ScanSummary, ScopeType, Skill, SubAgent,
};

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("config not found at {0}")]
    NotFound(String),
    #[error("config parse failed: {0}")]
    Parse(String),
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
}

pub type AdapterResult<T> = Result<T, AdapterError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub installed: bool,
    pub version: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeLocation {
    pub scope_type: ScopeType,
    pub config_path: PathBuf,
    pub writable: bool,
}

/// Context passed to `scan`. Adapters resolve concrete config paths
/// from this context — e.g. project root, override fixture root, or
/// real user home in production.
#[derive(Debug, Clone)]
pub struct ScanContext {
    pub project_path: Option<PathBuf>,
    /// When set, adapters MUST use this root instead of the real user
    /// home / OS-level locations. Used by fixtures and tests.
    pub fixture_root: Option<PathBuf>,
}

impl ScanContext {
    pub fn for_project(path: PathBuf) -> Self {
        Self {
            project_path: Some(path),
            fixture_root: None,
        }
    }

    pub fn with_fixture(mut self, root: PathBuf) -> Self {
        self.fixture_root = Some(root);
        self
    }

    pub fn empty() -> Self {
        Self {
            project_path: None,
            fixture_root: None,
        }
    }
}

/// Resources surfaced by a scan, grouped by kind. Adapters only output
/// the kinds they support — e.g. Pi never produces `sub_agents`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOutcome {
    pub agent_kind_str: String,
    pub scopes: Vec<ScopeLocation>,
    pub mcp_servers: Vec<McpServer>,
    pub skills: Vec<Skill>,
    pub sub_agents: Vec<SubAgent>,
    pub pi_resources: Vec<PiResource>,
    pub summary: ScanSummary,
    pub errors: Vec<String>,
}

/// High-level intent issued by the UI ("add MCP", "disable skill", ...).
/// Adapters translate this into a `ChangePlanDraft` that ChangeService
/// later materializes as a `ChangePlan` with concrete FilePatches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeIntent {
    pub kind: String,
    pub resource_type: ResourceType,
    pub target_scope: ScopeType,
    pub project_id: Option<String>,
    pub payload: JsonValue,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePlanDraft {
    pub operations: Vec<ChangeOperation>,
    pub target_files: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub patches: Vec<FilePatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationOutcome {
    pub ok: bool,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub issues: Vec<DoctorIssue>,
}

/// Read-only adapter surface for a single agent product.
///
/// Phase 2 only implements `kind`, `detect_installation`, `locate_*`,
/// `scan`, plus stub `build_change_plan` / `validate_applied_change` /
/// `run_doctor`. Real write logic lands in later phases.
pub trait AgentAdapter: Send + Sync {
    fn kind(&self) -> AgentKind;

    fn detect_installation(&self, ctx: &ScanContext) -> AdapterResult<DetectionResult>;

    fn locate_global_config(&self, ctx: &ScanContext) -> AdapterResult<Option<ScopeLocation>>;

    fn locate_project_config(&self, ctx: &ScanContext) -> AdapterResult<Option<ScopeLocation>>;

    fn scan(&self, ctx: &ScanContext) -> AdapterResult<ScanOutcome>;

    fn build_change_plan(
        &self,
        _ctx: &ScanContext,
        intent: &ChangeIntent,
    ) -> AdapterResult<ChangePlanDraft> {
        Err(AdapterError::Unsupported(format!(
            "build_change_plan not implemented for {:?}",
            intent.resource_type
        )))
    }

    fn validate_applied_change(&self, ctx: &ScanContext) -> AdapterResult<ValidationOutcome> {
        let _ = ctx;
        Ok(ValidationOutcome {
            ok: true,
            messages: vec![],
        })
    }

    fn run_doctor(&self, ctx: &ScanContext) -> AdapterResult<DoctorReport> {
        let _ = ctx;
        Ok(DoctorReport::default())
    }
}
