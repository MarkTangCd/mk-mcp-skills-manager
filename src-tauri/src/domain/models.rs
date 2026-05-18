use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::enums::{
    AgentKind, ChangeStatus, HealthStatus, IssueSeverity, McpTransport, PiResourceKind,
    ResourceType, ScopeType,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub kind: AgentKind,
    pub display_name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub health_status: HealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigScope {
    pub agent_kind: AgentKind,
    pub scope_type: ScopeType,
    pub project_id: Option<String>,
    pub config_path: String,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub transport: McpTransport,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env_refs: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub status: String,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubAgent {
    pub id: String,
    pub slug: String,
    pub role: String,
    pub agent_kinds: Vec<AgentKind>,
    pub bound_mcp_ids: Vec<String>,
    pub bound_skill_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiResource {
    pub id: String,
    pub resource_type: PiResourceKind,
    pub source: String,
    pub path: Option<String>,
    pub enabled: bool,
    pub trusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplate {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub category: Option<String>,
    pub body: String,
    pub variables: Vec<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBinding {
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub agent_kind: AgentKind,
    pub project_id: Option<String>,
    pub scope_type: ScopeType,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub total_resources: u32,
    pub mcp_count: u32,
    pub skill_count: u32,
    pub sub_agent_count: u32,
    pub pi_resource_count: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSnapshot {
    pub id: String,
    pub project_id: Option<String>,
    pub agent_kind: Option<AgentKind>,
    pub summary: ScanSummary,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorTargetRef {
    pub resource_type: Option<ResourceType>,
    pub resource_id: Option<String>,
    pub agent_kind: Option<AgentKind>,
    pub project_id: Option<String>,
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorIssue {
    pub id: String,
    pub severity: IssueSeverity,
    pub category: String,
    pub message: String,
    pub target_ref: Option<DoctorTargetRef>,
    pub fixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePatch {
    pub path: String,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSummary {
    pub files_changed: u32,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeOperation {
    pub kind: String,
    pub target: String,
    pub payload: JsonValue,
}

// ------------------------------------------------------------------
// ChangeIntent: what the user wants to do.
// ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeIntent {
    pub id: String,
    pub change_type: String,
    pub agent_kind: Option<AgentKind>,
    pub project_id: Option<String>,
    pub scope_type: Option<ScopeType>,
    pub resource_id: Option<String>,
    pub payload: JsonValue,
    pub created_at: String,
}

// ------------------------------------------------------------------
// ChangePlan: a rich preview produced by an adapter before any file
// is touched.  It is the serializable contract between backend and
// frontend during the diff-preview phase.
// ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePlan {
    pub id: String,
    pub intent_id: String,
    pub status: ChangeStatus,
    pub agent_kind: Option<AgentKind>,
    pub target_files: Vec<String>,
    pub operations: Vec<ChangeOperation>,
    pub patches: Vec<FilePatch>,
    pub diff_summary: DiffSummary,
    pub risks: Vec<String>,
    pub validation_errors: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ChangePlan {
    /// Returns true when the plan can move from Draft -> Previewed.
    pub fn can_preview(&self) -> bool {
        matches!(self.status, ChangeStatus::Draft)
    }

    /// Returns true when the plan can be confirmed by the user.
    /// A plan with validation errors cannot be confirmed.
    pub fn can_confirm(&self) -> bool {
        matches!(self.status, ChangeStatus::Previewed) && self.validation_errors.is_empty()
    }

    /// Returns true when the plan is ready for apply.
    pub fn can_apply(&self) -> bool {
        matches!(self.status, ChangeStatus::Confirmed)
    }

    /// Transition the plan to a new status, enforcing the allowed
    /// state machine:
    ///   Draft <-> Previewed -> Confirmed -> Applied
    ///   Confirmed -> Failed
    ///   Applied | Failed -> Restored
    pub fn transition_to(&mut self, new_status: ChangeStatus) -> Result<(), String> {
        let allowed = matches!(
            (self.status, new_status),
            (ChangeStatus::Draft, ChangeStatus::Previewed)
                | (ChangeStatus::Previewed, ChangeStatus::Draft)
                | (ChangeStatus::Previewed, ChangeStatus::Confirmed)
                | (ChangeStatus::Confirmed, ChangeStatus::Applied)
                | (ChangeStatus::Confirmed, ChangeStatus::AppliedWithWarning)
                | (ChangeStatus::Confirmed, ChangeStatus::Failed)
                | (ChangeStatus::Applied, ChangeStatus::Restored)
                | (ChangeStatus::AppliedWithWarning, ChangeStatus::Restored)
                | (ChangeStatus::Failed, ChangeStatus::Restored)
        );
        if !allowed {
            return Err(format!(
                "invalid transition from {:?} to {:?}",
                self.status, new_status
            ));
        }
        self.status = new_status;
        self.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }
}

// ------------------------------------------------------------------
// ChangeSet: the persisted record of a ChangePlan.
// ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSet {
    pub id: String,
    pub intent_id: Option<String>,
    pub status: ChangeStatus,
    pub target_files: Vec<String>,
    pub operations: Vec<ChangeOperation>,
    pub patches: Vec<FilePatch>,
    pub diff_summary: DiffSummary,
    pub risks: Vec<String>,
    pub validation_errors: Vec<String>,
    pub backup_id: Option<String>,
    pub project_id: Option<String>,
    pub agent_kind: Option<AgentKind>,
    pub created_at: String,
    pub updated_at: String,
}

impl ChangeSet {
    /// Build a ChangeSet from an approved ChangePlan, optionally
    /// attaching a backup id.
    pub fn from_plan(plan: &ChangePlan, backup_id: Option<String>) -> Self {
        Self {
            id: plan.id.clone(),
            intent_id: Some(plan.intent_id.clone()),
            status: plan.status,
            target_files: plan.target_files.clone(),
            operations: plan.operations.clone(),
            patches: plan.patches.clone(),
            diff_summary: plan.diff_summary.clone(),
            risks: plan.risks.clone(),
            validation_errors: plan.validation_errors.clone(),
            backup_id,
            project_id: plan.operations.iter().find_map(|op| {
                if op.kind == "setProjectId" {
                    op.payload.as_str().map(|s| s.to_string())
                } else {
                    None
                }
            }),
            agent_kind: None,
            created_at: plan.created_at.clone(),
            updated_at: plan.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub id: String,
    pub change_set_id: String,
    pub manifest_path: String,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_serializes_camel_case() {
        let agent = Agent {
            id: "a1".into(),
            kind: AgentKind::Codex,
            display_name: "Codex".into(),
            installed: true,
            version: Some("1.0.0".into()),
            health_status: HealthStatus::Ok,
        };
        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("\"displayName\":\"Codex\""));
        assert!(json.contains("\"healthStatus\":\"ok\""));
        assert!(json.contains("\"kind\":\"codex\""));
    }

    #[test]
    fn change_set_round_trip() {
        let cs = ChangeSet {
            id: "cs1".into(),
            intent_id: None,
            status: ChangeStatus::Draft,
            target_files: vec![],
            operations: vec![],
            patches: vec![],
            diff_summary: DiffSummary {
                files_changed: 0,
                additions: 0,
                deletions: 0,
            },
            risks: vec![],
            validation_errors: vec![],
            backup_id: None,
            project_id: None,
            agent_kind: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&cs).unwrap();
        let back: ChangeSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "cs1");
        assert_eq!(back.status, ChangeStatus::Draft);
    }

    #[test]
    fn change_plan_state_machine() {
        let mut plan = ChangePlan {
            id: "p1".into(),
            intent_id: "i1".into(),
            status: ChangeStatus::Draft,
            agent_kind: None,
            target_files: vec!["a.json".into()],
            operations: vec![],
            patches: vec![],
            diff_summary: DiffSummary {
                files_changed: 0,
                additions: 0,
                deletions: 0,
            },
            risks: vec![],
            validation_errors: vec![],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };

        assert!(plan.can_preview());
        plan.transition_to(ChangeStatus::Previewed).unwrap();
        assert!(plan.can_confirm());
        plan.transition_to(ChangeStatus::Confirmed).unwrap();
        assert!(plan.can_apply());

        // Cannot jump from Draft -> Confirmed.
        let mut plan2 = ChangePlan {
            id: "p2".into(),
            intent_id: "i2".into(),
            status: ChangeStatus::Draft,
            agent_kind: None,
            target_files: vec![],
            operations: vec![],
            patches: vec![],
            diff_summary: DiffSummary {
                files_changed: 0,
                additions: 0,
                deletions: 0,
            },
            risks: vec![],
            validation_errors: vec![],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        assert!(plan2.transition_to(ChangeStatus::Confirmed).is_err());
    }

    #[test]
    fn change_plan_with_validation_errors_cannot_confirm() {
        let plan = ChangePlan {
            id: "p3".into(),
            intent_id: "i3".into(),
            status: ChangeStatus::Previewed,
            agent_kind: None,
            target_files: vec![],
            operations: vec![],
            patches: vec![],
            diff_summary: DiffSummary {
                files_changed: 0,
                additions: 0,
                deletions: 0,
            },
            risks: vec![],
            validation_errors: vec!["missing command".into()],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        assert!(!plan.can_confirm());
    }

    #[test]
    fn change_set_from_plan_maps_fields() {
        let plan = ChangePlan {
            id: "p4".into(),
            intent_id: "i4".into(),
            status: ChangeStatus::Confirmed,
            agent_kind: None,
            target_files: vec!["config.json".into()],
            operations: vec![ChangeOperation {
                kind: "setProjectId".into(),
                target: "project".into(),
                payload: serde_json::json!("proj-1"),
            }],
            patches: vec![FilePatch {
                path: "config.json".into(),
                before_hash: None,
                after_hash: None,
                diff: "+ foo".into(),
            }],
            diff_summary: DiffSummary {
                files_changed: 1,
                additions: 1,
                deletions: 0,
            },
            risks: vec!["overwrites global config".into()],
            validation_errors: vec![],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let cs = ChangeSet::from_plan(&plan, Some("b1".into()));
        assert_eq!(cs.id, "p4");
        assert_eq!(cs.intent_id, Some("i4".into()));
        assert_eq!(cs.project_id, Some("proj-1".into()));
        assert_eq!(cs.risks.len(), 1);
        assert_eq!(cs.backup_id, Some("b1".into()));
    }
}
