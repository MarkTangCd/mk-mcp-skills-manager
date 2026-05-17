use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentKind {
    ClaudeCode,
    Codex,
    Opencode,
    Pi,
}

impl AgentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentKind::ClaudeCode => "claude-code",
            AgentKind::Codex => "codex",
            AgentKind::Opencode => "opencode",
            AgentKind::Pi => "pi",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScopeType {
    Global,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceType {
    Mcp,
    Skill,
    SubAgent,
    PiResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PiResourceKind {
    Skill,
    PromptTemplate,
    Extension,
    Package,
    Theme,
    Setting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    Draft,
    Previewed,
    Confirmed,
    Applied,
    AppliedWithWarning,
    Failed,
    Restored,
}

impl ChangeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeStatus::Draft => "draft",
            ChangeStatus::Previewed => "previewed",
            ChangeStatus::Confirmed => "confirmed",
            ChangeStatus::Applied => "applied",
            ChangeStatus::AppliedWithWarning => "applied_with_warning",
            ChangeStatus::Failed => "failed",
            ChangeStatus::Restored => "restored",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpTransport {
    Stdio,
    Sse,
    Http,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_kind_round_trips_kebab_case() {
        let json = serde_json::to_string(&AgentKind::ClaudeCode).unwrap();
        assert_eq!(json, "\"claude-code\"");
        let back: AgentKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, AgentKind::ClaudeCode);
    }

    #[test]
    fn change_status_uses_snake_case() {
        let json = serde_json::to_string(&ChangeStatus::AppliedWithWarning).unwrap();
        assert_eq!(json, "\"applied_with_warning\"");
    }

    #[test]
    fn severity_round_trip() {
        let parsed: IssueSeverity = serde_json::from_str("\"warning\"").unwrap();
        assert_eq!(parsed, IssueSeverity::Warning);
    }
}
