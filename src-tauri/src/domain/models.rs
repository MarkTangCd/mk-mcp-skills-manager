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
    pub body: String,
    pub variables: Vec<String>,
    pub tags: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSet {
    pub id: String,
    pub status: ChangeStatus,
    pub operations: Vec<ChangeOperation>,
    pub patches: Vec<FilePatch>,
    pub diff_summary: DiffSummary,
    pub backup_id: Option<String>,
    pub created_at: String,
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
            status: ChangeStatus::Draft,
            operations: vec![],
            patches: vec![],
            diff_summary: DiffSummary {
                files_changed: 0,
                additions: 0,
                deletions: 0,
            },
            backup_id: None,
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&cs).unwrap();
        let back: ChangeSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "cs1");
        assert_eq!(back.status, ChangeStatus::Draft);
    }
}
