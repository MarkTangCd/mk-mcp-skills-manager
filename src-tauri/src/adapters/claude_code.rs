use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::domain::{
    AgentKind, ChangeOperation, FilePatch, McpServer, McpTransport, ScopeType, Skill, SubAgent,
};

use super::common::{
    command_version, display_path, duplicate_name_warnings, env_ref_keys, first_existing, home_dir,
    summary,
};
use super::traits::{
    AdapterError, AdapterResult, AgentAdapter, ChangeIntent, ChangePlanDraft, DetectionResult,
    ScanContext, ScanOutcome, ScopeLocation,
};

pub struct ClaudeCodeAdapter;

#[derive(Debug, Deserialize, Serialize)]
struct ClaudeConfig {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: BTreeMap<String, ClaudeMcpConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    skills: BTreeMap<String, ClaudeSkillConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    agents: BTreeMap<String, ClaudeAgentConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ClaudeMcpConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    env: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ClaudeSkillConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ClaudeAgentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skills: Vec<String>,
}

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn config_candidates(&self, ctx: &ScanContext, scope_type: ScopeType) -> Vec<PathBuf> {
        if let Some(root) = &ctx.fixture_root {
            return vec![root.join(".claude.json")];
        }

        match scope_type {
            ScopeType::Global => home_dir()
                .map(|home| vec![home.join(".claude.json")])
                .unwrap_or_default(),
            ScopeType::Project => ctx
                .project_path
                .as_ref()
                .map(|project| vec![project.join(".claude.json")])
                .unwrap_or_default(),
        }
    }

    fn parse_config(&self, path: &PathBuf, scope_type: ScopeType) -> AdapterResult<ScanOutcome> {
        let raw = fs::read_to_string(path)?;
        let parsed: ClaudeConfig =
            serde_json::from_str(&raw).map_err(|err| AdapterError::Parse(err.to_string()))?;
        let errors = duplicate_name_warnings("Claude Code", parsed.mcp_servers.keys().cloned());
        let mcp_servers = parsed
            .mcp_servers
            .into_iter()
            .map(|(name, config)| self.to_mcp_server(scope_type, name, config))
            .collect::<Vec<_>>();
        let skills = parsed
            .skills
            .into_iter()
            .map(|(slug, config)| self.to_skill(scope_type, slug, config))
            .collect::<Vec<_>>();
        let sub_agents = parsed
            .agents
            .into_iter()
            .map(|(slug, config)| self.to_sub_agent(scope_type, slug, config))
            .collect::<Vec<_>>();
        Ok(self.outcome(
            vec![ScopeLocation {
                scope_type,
                config_path: path.clone(),
                writable: false,
            }],
            mcp_servers,
            skills,
            sub_agents,
            errors,
        ))
    }

    fn to_mcp_server(
        &self,
        scope_type: ScopeType,
        name: String,
        config: ClaudeMcpConfig,
    ) -> McpServer {
        McpServer {
            id: format!(
                "claude-code:{}:{}",
                scope_type_label(scope_type),
                name.trim()
            ),
            name: name.trim().to_string(),
            transport: if config.url.is_some() {
                McpTransport::Http
            } else {
                McpTransport::Stdio
            },
            command: config.command,
            args: config.args,
            url: config.url,
            env_refs: env_ref_keys(&config.env),
            enabled: config.enabled.unwrap_or(true),
        }
    }

    fn to_skill(&self, scope_type: ScopeType, slug: String, config: ClaudeSkillConfig) -> Skill {
        Skill {
            id: format!(
                "claude-code:{}:skill:{}",
                scope_type_label(scope_type),
                slug.trim()
            ),
            slug: slug.trim().to_string(),
            title: config.title.unwrap_or_else(|| slug.trim().to_string()),
            description: config.description,
            tags: config.tags,
            status: "active".into(),
            source_path: config.path,
        }
    }

    fn to_sub_agent(
        &self,
        scope_type: ScopeType,
        slug: String,
        config: ClaudeAgentConfig,
    ) -> SubAgent {
        SubAgent {
            id: format!(
                "claude-code:{}:sub-agent:{}",
                scope_type_label(scope_type),
                slug.trim()
            ),
            slug: slug.trim().to_string(),
            role: config
                .role
                .or(config.description)
                .unwrap_or_else(|| slug.trim().to_string()),
            agent_kinds: vec![AgentKind::ClaudeCode],
            bound_mcp_ids: config.tools,
            bound_skill_ids: config.skills,
        }
    }

    fn outcome(
        &self,
        scopes: Vec<ScopeLocation>,
        mcp_servers: Vec<McpServer>,
        skills: Vec<Skill>,
        sub_agents: Vec<SubAgent>,
        errors: Vec<String>,
    ) -> ScanOutcome {
        ScanOutcome {
            agent_kind_str: AgentKind::ClaudeCode.as_str().to_string(),
            summary: summary(
                mcp_servers.len(),
                skills.len(),
                sub_agents.len(),
                0,
                errors.clone(),
            ),
            scopes,
            mcp_servers,
            skills,
            sub_agents,
            pi_resources: vec![],
            errors,
        }
    }

    fn resolve_target_path(
        &self,
        ctx: &ScanContext,
        scope_type: ScopeType,
    ) -> AdapterResult<PathBuf> {
        let candidates = self.config_candidates(ctx, scope_type);
        if let Some(path) = first_existing(&candidates) {
            return Ok(path);
        }
        candidates.into_iter().next().ok_or_else(|| {
            AdapterError::Invalid(format!("no config candidates for scope {scope_type:?}"))
        })
    }

    fn build_change_plan_inner(
        &self,
        ctx: &ScanContext,
        intent: &ChangeIntent,
    ) -> AdapterResult<ChangePlanDraft> {
        let target_path = self.resolve_target_path(ctx, intent.target_scope)?;

        let (existing_content, before_hash) = if target_path.exists() {
            let content = fs::read_to_string(&target_path)?;
            let hash = sha256_str(&content);
            (content, Some(hash))
        } else {
            ("{}".to_string(), None)
        };

        let mut config: ClaudeConfig = serde_json::from_str(&existing_content)
            .map_err(|err| AdapterError::Parse(err.to_string()))?;

        let mut warnings = Vec::new();

        match intent.kind.as_str() {
            "createMcp" => {
                let name = extract_name(&intent.payload)?;
                if config.mcp_servers.contains_key(&name) {
                    warnings.push(format!(
                        "MCP server '{}' already exists and will be overwritten",
                        name
                    ));
                }
                let mcp_config = parse_mcp_config(&intent.payload)?;
                config.mcp_servers.insert(name, mcp_config);
            }
            "updateMcp" => {
                let name = extract_name(&intent.payload)?;
                let Some(mut existing) = config.mcp_servers.remove(&name) else {
                    return Err(AdapterError::Invalid(format!(
                        "MCP server '{}' not found",
                        name
                    )));
                };
                update_mcp_config(&intent.payload, &mut existing)?;
                config.mcp_servers.insert(name, existing);
            }
            "deleteMcp" => {
                let name = extract_name(&intent.payload)?;
                if config.mcp_servers.remove(&name).is_none() {
                    return Err(AdapterError::Invalid(format!(
                        "MCP server '{}' not found",
                        name
                    )));
                }
            }
            "enableMcp" => {
                let name = extract_name(&intent.payload)?;
                let Some(mcp) = config.mcp_servers.get_mut(&name) else {
                    return Err(AdapterError::Invalid(format!(
                        "MCP server '{}' not found",
                        name
                    )));
                };
                mcp.enabled = Some(true);
            }
            "disableMcp" => {
                let name = extract_name(&intent.payload)?;
                let Some(mcp) = config.mcp_servers.get_mut(&name) else {
                    return Err(AdapterError::Invalid(format!(
                        "MCP server '{}' not found",
                        name
                    )));
                };
                mcp.enabled = Some(false);
            }
            "enableSkill" => {
                let slug = extract_slug(&intent.payload)?;
                if config.skills.contains_key(&slug) {
                    warnings.push(format!(
                        "Skill '{}' already exists and will be updated",
                        slug
                    ));
                }
                let skill_config = parse_skill_config(&intent.payload)?;
                config.skills.insert(slug, skill_config);
            }
            "disableSkill" | "deleteSkill" => {
                let slug = extract_slug(&intent.payload)?;
                if config.skills.remove(&slug).is_none() {
                    return Err(AdapterError::Invalid(format!("Skill '{}' not found", slug)));
                }
            }
            "enableSubAgent" => {
                let slug = extract_slug(&intent.payload)?;
                if config.agents.contains_key(&slug) {
                    warnings.push(format!(
                        "Sub-agent '{}' already exists and will be overwritten",
                        slug
                    ));
                }
                let agent_config = parse_agent_config(&intent.payload)?;
                config.agents.insert(slug, agent_config);
            }
            "disableSubAgent" | "deleteSubAgent" => {
                let slug = extract_slug(&intent.payload)?;
                if config.agents.remove(&slug).is_none() {
                    return Err(AdapterError::Invalid(format!(
                        "Sub-agent '{}' not found",
                        slug
                    )));
                }
            }
            _ => {
                return Err(AdapterError::Unsupported(format!(
                    "unsupported change intent kind: {}",
                    intent.kind
                )));
            }
        }

        let payload =
            serde_json::to_value(&config).map_err(|err| AdapterError::Parse(err.to_string()))?;
        let new_content = serde_json::to_string_pretty(&payload)
            .map_err(|err| AdapterError::Parse(err.to_string()))?;
        let after_hash = sha256_str(&new_content);
        let diff = make_diff(&existing_content, &new_content);

        let patch = FilePatch {
            path: target_path.to_string_lossy().to_string(),
            before_hash,
            after_hash: Some(after_hash),
            diff,
        };

        Ok(ChangePlanDraft {
            operations: vec![ChangeOperation {
                kind: "writeJson".to_string(),
                target: target_path.to_string_lossy().to_string(),
                payload,
            }],
            target_files: vec![target_path],
            warnings,
            patches: vec![patch],
        })
    }
}

impl Default for ClaudeCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for ClaudeCodeAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::ClaudeCode
    }

    fn detect_installation(&self, ctx: &ScanContext) -> AdapterResult<DetectionResult> {
        let config_exists = self
            .locate_global_config(ctx)?
            .or(self.locate_project_config(ctx)?)
            .is_some();
        let version = command_version("claude", &["--version"]);
        Ok(DetectionResult {
            installed: config_exists || version.is_some(),
            version,
            notes: if config_exists {
                vec!["Claude Code config path detected".into()]
            } else {
                vec![]
            },
        })
    }

    fn locate_global_config(&self, ctx: &ScanContext) -> AdapterResult<Option<ScopeLocation>> {
        Ok(
            first_existing(&self.config_candidates(ctx, ScopeType::Global)).map(|config_path| {
                ScopeLocation {
                    scope_type: ScopeType::Global,
                    config_path,
                    writable: false,
                }
            }),
        )
    }

    fn locate_project_config(&self, ctx: &ScanContext) -> AdapterResult<Option<ScopeLocation>> {
        Ok(
            first_existing(&self.config_candidates(ctx, ScopeType::Project)).map(|config_path| {
                ScopeLocation {
                    scope_type: ScopeType::Project,
                    config_path,
                    writable: false,
                }
            }),
        )
    }

    fn scan(&self, ctx: &ScanContext) -> AdapterResult<ScanOutcome> {
        if let Some(fixture) = &ctx.fixture_root {
            let path = fixture.join(".claude.json");
            if !path.exists() {
                return Ok(self.outcome(vec![], vec![], vec![], vec![], vec![]));
            }
            return self.parse_config(&path, ScopeType::Global);
        }

        let mut scopes = Vec::new();
        let mut mcp_servers = Vec::new();
        let mut skills = Vec::new();
        let mut sub_agents = Vec::new();
        let mut errors = Vec::new();
        for scope_type in [ScopeType::Global, ScopeType::Project] {
            if let Some(location) = match scope_type {
                ScopeType::Global => self.locate_global_config(ctx)?,
                ScopeType::Project => self.locate_project_config(ctx)?,
            } {
                match self.parse_config(&location.config_path, scope_type) {
                    Ok(outcome) => {
                        scopes.extend(outcome.scopes);
                        mcp_servers.extend(outcome.mcp_servers);
                        skills.extend(outcome.skills);
                        sub_agents.extend(outcome.sub_agents);
                        errors.extend(outcome.errors);
                    }
                    Err(err) => {
                        errors.push(format!("{}: {err}", display_path(&location.config_path)))
                    }
                }
            }
        }
        Ok(self.outcome(scopes, mcp_servers, skills, sub_agents, errors))
    }

    fn build_change_plan(
        &self,
        ctx: &ScanContext,
        intent: &ChangeIntent,
    ) -> AdapterResult<ChangePlanDraft> {
        self.build_change_plan_inner(ctx, intent)
    }
}

fn scope_type_label(scope_type: ScopeType) -> &'static str {
    match scope_type {
        ScopeType::Global => "global",
        ScopeType::Project => "project",
    }
}

fn sha256_str(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

fn make_diff(old: &str, new: &str) -> String {
    use similar::TextDiff;
    let diff = TextDiff::from_lines(old, new);
    let mut out = String::new();
    for group in diff.grouped_ops(3) {
        for op in group {
            for change in diff.iter_changes(&op) {
                let sign = match change.tag() {
                    similar::ChangeTag::Delete => '-',
                    similar::ChangeTag::Insert => '+',
                    similar::ChangeTag::Equal => ' ',
                };
                out.push_str(&format!("{}{}", sign, change.value()));
                if change.missing_newline() {
                    out.push('\n');
                }
            }
        }
    }
    out
}

fn extract_name(payload: &JsonValue) -> AdapterResult<String> {
    payload
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AdapterError::Invalid("payload missing 'name' field".to_string()))
}

fn extract_slug(payload: &JsonValue) -> AdapterResult<String> {
    payload
        .get("slug")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AdapterError::Invalid("payload missing 'slug' field".to_string()))
}

fn parse_mcp_config(payload: &JsonValue) -> AdapterResult<ClaudeMcpConfig> {
    serde_json::from_value(payload.clone())
        .map_err(|err| AdapterError::Invalid(format!("invalid MCP config: {err}")))
}

fn parse_skill_config(payload: &JsonValue) -> AdapterResult<ClaudeSkillConfig> {
    serde_json::from_value(payload.clone())
        .map_err(|err| AdapterError::Invalid(format!("invalid skill config: {err}")))
}

fn parse_agent_config(payload: &JsonValue) -> AdapterResult<ClaudeAgentConfig> {
    let role = payload
        .get("role")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let tools = payload
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let skills = payload
        .get("skills")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(ClaudeAgentConfig {
        description: description.or(role.clone()),
        role,
        tools,
        skills,
    })
}

fn update_mcp_config(payload: &JsonValue, existing: &mut ClaudeMcpConfig) -> AdapterResult<()> {
    if let Some(cmd) = payload.get("command").and_then(|v| v.as_str()) {
        existing.command = Some(cmd.to_string());
    }
    if let Some(args) = payload.get("args") {
        existing.args = serde_json::from_value(args.clone())
            .map_err(|err| AdapterError::Invalid(format!("invalid args: {err}")))?;
    }
    if let Some(url) = payload.get("url").and_then(|v| v.as_str()) {
        existing.url = Some(url.to_string());
    }
    if let Some(env) = payload.get("env") {
        existing.env = serde_json::from_value(env.clone())
            .map_err(|err| AdapterError::Invalid(format!("invalid env: {err}")))?;
    }
    if let Some(enabled) = payload.get("enabled").and_then(|v| v.as_bool()) {
        existing.enabled = Some(enabled);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/agents/claude-code")
            .join(name)
    }

    #[test]
    fn parses_valid_claude_fixture() {
        let adapter = ClaudeCodeAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("valid-global")))
            .unwrap();
        assert_eq!(out.mcp_servers.len(), 1);
        assert_eq!(out.skills.len(), 1);
        assert_eq!(out.sub_agents.len(), 1);
        assert_eq!(out.summary.total_resources, 3);
    }

    #[test]
    fn empty_claude_fixture_returns_empty_scan() {
        let adapter = ClaudeCodeAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("empty")))
            .unwrap();
        assert_eq!(out.summary.total_resources, 0);
        assert!(out.mcp_servers.is_empty());
        assert!(out.skills.is_empty());
        assert!(out.sub_agents.is_empty());
    }

    #[test]
    fn duplicate_claude_fixture_returns_warning() {
        let adapter = ClaudeCodeAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("duplicate-mcp")))
            .unwrap();
        assert_eq!(out.mcp_servers.len(), 2);
        assert_eq!(out.errors.len(), 1);
        assert!(out.errors[0].contains("duplicate"));
    }

    #[test]
    fn invalid_claude_fixture_returns_parse_error() {
        let adapter = ClaudeCodeAdapter::new();
        let err = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("invalid")))
            .unwrap_err();
        assert!(matches!(err, AdapterError::Parse(_)));
    }

    // ------------------------------------------------------------------
    // build_change_plan tests
    // ------------------------------------------------------------------

    fn intent(kind: &str, payload: JsonValue) -> ChangeIntent {
        ChangeIntent {
            kind: kind.to_string(),
            resource_type: crate::domain::ResourceType::Mcp,
            target_scope: ScopeType::Global,
            project_id: None,
            payload,
        }
    }

    #[test]
    fn create_mcp_on_empty_config() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent(
                    "createMcp",
                    serde_json::json!({
                        "name": "new-server",
                        "command": "echo",
                        "args": ["hello"],
                        "enabled": true
                    }),
                ),
            )
            .unwrap();

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].kind, "writeJson");
        assert_eq!(plan.patches.len(), 1);
        assert!(plan.patches[0].before_hash.is_none());
        assert!(plan.patches[0].after_hash.is_some());
        assert!(plan.patches[0].diff.contains("new-server"));
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn create_mcp_duplicate_warns() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"mcpServers":{"dup":{"command":"echo","args":["hi"],"enabled":true}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent(
                    "createMcp",
                    serde_json::json!({
                        "name": "dup",
                        "command": "echo",
                        "args": ["hello"],
                        "enabled": true
                    }),
                ),
            )
            .unwrap();

        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("already exists"));
    }

    #[test]
    fn update_mcp_changes_fields() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"mcpServers":{"srv":{"command":"old","args":[],"enabled":false}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent(
                    "updateMcp",
                    serde_json::json!({
                        "name": "srv",
                        "command": "new",
                        "args": ["arg1"],
                        "enabled": true
                    }),
                ),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        let mcp = payload.get("mcpServers").unwrap().get("srv").unwrap();
        assert_eq!(mcp.get("command").unwrap().as_str().unwrap(), "new");
        assert_eq!(mcp.get("args").unwrap().as_array().unwrap()[0], "arg1");
        assert!(mcp.get("enabled").unwrap().as_bool().unwrap());
    }

    #[test]
    fn delete_mcp_removes_entry() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"mcpServers":{"keep":{"command":"echo","args":["keep"],"enabled":true},"remove":{"command":"echo","args":["remove"],"enabled":true}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("deleteMcp", serde_json::json!({"name": "remove"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        let mcp_servers = payload.get("mcpServers").unwrap();
        assert!(mcp_servers.get("remove").is_none());
        assert!(mcp_servers.get("keep").is_some());
    }

    #[test]
    fn enable_mcp_sets_enabled_true() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"mcpServers":{"srv":{"command":"echo","args":["hi"],"enabled":false}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("enableMcp", serde_json::json!({"name": "srv"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        let enabled = payload
            .get("mcpServers")
            .unwrap()
            .get("srv")
            .unwrap()
            .get("enabled")
            .unwrap()
            .as_bool()
            .unwrap();
        assert!(enabled);
    }

    #[test]
    fn disable_mcp_sets_enabled_false() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"mcpServers":{"srv":{"command":"echo","args":["hi"],"enabled":true}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("disableMcp", serde_json::json!({"name": "srv"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        let enabled = payload
            .get("mcpServers")
            .unwrap()
            .get("srv")
            .unwrap()
            .get("enabled")
            .unwrap()
            .as_bool()
            .unwrap();
        assert!(!enabled);
    }

    #[test]
    fn invalid_config_returns_recoverable_error() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{ "mcpServers": { "broken": "#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let err = adapter
            .build_change_plan(
                &ctx,
                &intent(
                    "createMcp",
                    serde_json::json!({"name": "srv", "enabled": true}),
                ),
            )
            .unwrap_err();

        assert!(matches!(err, AdapterError::Parse(_)));
    }

    // ------------------------------------------------------------------
    // Skill change plan tests
    // ------------------------------------------------------------------

    fn skill_payload(slug: &str, path: &str) -> JsonValue {
        serde_json::json!({
            "slug": slug,
            "path": path,
            "title": slug.to_string().replace("-", " ").to_title_case(),
            "description": "A test skill",
            "tags": ["test"]
        })
    }

    trait TitleCase {
        fn to_title_case(&self) -> String;
    }

    impl TitleCase for str {
        fn to_title_case(&self) -> String {
            self.split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    #[test]
    fn enable_skill_on_empty_config() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent(
                    "enableSkill",
                    skill_payload("my-skill", "/tmp/library/skills/my-skill"),
                ),
            )
            .unwrap();

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].kind, "writeJson");
        assert_eq!(plan.patches.len(), 1);
        assert!(plan.patches[0].before_hash.is_none());
        assert!(plan.patches[0].after_hash.is_some());

        let payload = &plan.operations[0].payload;
        let skills = payload.get("skills").unwrap();
        assert!(skills.get("my-skill").is_some());
        assert_eq!(
            skills
                .get("my-skill")
                .unwrap()
                .get("path")
                .unwrap()
                .as_str()
                .unwrap(),
            "/tmp/library/skills/my-skill"
        );
    }

    #[test]
    fn enable_skill_duplicate_warns() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"skills":{"dup":{"path":"/old","title":"Old","description":"old","tags":[]}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(&ctx, &intent("enableSkill", skill_payload("dup", "/new")))
            .unwrap();

        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("already exists"));

        let payload = &plan.operations[0].payload;
        let skills = payload.get("skills").unwrap();
        assert_eq!(
            skills
                .get("dup")
                .unwrap()
                .get("path")
                .unwrap()
                .as_str()
                .unwrap(),
            "/new"
        );
    }

    #[test]
    fn disable_skill_removes_entry() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"skills":{"keep":{"path":"/keep","title":"Keep","description":"keep","tags":[]},"remove":{"path":"/remove","title":"Remove","description":"remove","tags":[]}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("disableSkill", serde_json::json!({"slug": "remove"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        let skills = payload.get("skills").unwrap();
        assert!(skills.get("remove").is_none());
        assert!(skills.get("keep").is_some());
    }

    #[test]
    fn delete_skill_removes_entry() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"skills":{"gone":{"path":"/gone","title":"Gone","description":"gone","tags":[]}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("deleteSkill", serde_json::json!({"slug": "gone"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        // When skills map becomes empty, serde skips serializing it.
        if let Some(skills) = payload.get("skills") {
            assert!(skills.get("gone").is_none());
        }
        // Verify the skill is not present by checking raw content.
        let raw = serde_json::to_string(payload).unwrap();
        assert!(!raw.contains("\"gone\""));
    }

    #[test]
    fn disable_missing_skill_returns_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".claude.json"), r#"{"skills":{}}"#).unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let err = adapter
            .build_change_plan(
                &ctx,
                &intent("disableSkill", serde_json::json!({"slug": "missing"})),
            )
            .unwrap_err();

        assert!(matches!(err, AdapterError::Invalid(_)));
        assert!(err.to_string().contains("not found"));
    }

    // ------------------------------------------------------------------
    // Sub-agent change plan tests
    // ------------------------------------------------------------------

    fn agent_payload(slug: &str) -> JsonValue {
        serde_json::json!({
            "slug": slug,
            "role": "A test agent",
            "description": "Detailed description",
            "tools": ["mcp1", "mcp2"],
            "skills": ["skill1"]
        })
    }

    #[test]
    fn enable_sub_agent_on_empty_config() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(&ctx, &intent("enableSubAgent", agent_payload("my-agent")))
            .unwrap();

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].kind, "writeJson");
        assert_eq!(plan.patches.len(), 1);
        assert!(plan.patches[0].before_hash.is_none());
        assert!(plan.patches[0].after_hash.is_some());
        assert!(plan.patches[0].diff.contains("my-agent"));
        assert!(plan.warnings.is_empty());

        let payload = &plan.operations[0].payload;
        let agents = payload.get("agents").unwrap();
        assert!(agents.get("my-agent").is_some());
        assert_eq!(
            agents
                .get("my-agent")
                .unwrap()
                .get("role")
                .unwrap()
                .as_str()
                .unwrap(),
            "A test agent"
        );
    }

    #[test]
    fn enable_sub_agent_duplicate_warns() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"agents":{"dup":{"role":"Old","description":"old","tools":[],"skills":[]}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(&ctx, &intent("enableSubAgent", agent_payload("dup")))
            .unwrap();

        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("already exists"));

        let payload = &plan.operations[0].payload;
        let agents = payload.get("agents").unwrap();
        assert_eq!(
            agents
                .get("dup")
                .unwrap()
                .get("role")
                .unwrap()
                .as_str()
                .unwrap(),
            "A test agent"
        );
    }

    #[test]
    fn disable_sub_agent_removes_entry() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"agents":{"keep":{"role":"Keep","tools":[],"skills":[]},"remove":{"role":"Remove","tools":[],"skills":[]}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("disableSubAgent", serde_json::json!({"slug": "remove"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        if let Some(agents) = payload.get("agents") {
            assert!(agents.get("remove").is_none());
            assert!(agents.get("keep").is_some());
        }
        let raw = serde_json::to_string(payload).unwrap();
        assert!(!raw.contains("\"remove\""));
    }

    #[test]
    fn delete_sub_agent_removes_entry() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".claude.json"),
            r#"{"agents":{"gone":{"role":"Gone","tools":[],"skills":[]}}}"#,
        )
        .unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("deleteSubAgent", serde_json::json!({"slug": "gone"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        if let Some(agents) = payload.get("agents") {
            assert!(agents.get("gone").is_none());
        }
        let raw = serde_json::to_string(payload).unwrap();
        assert!(!raw.contains("\"gone\""));
    }

    #[test]
    fn disable_missing_sub_agent_returns_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".claude.json"), r#"{"agents":{}}"#).unwrap();

        let adapter = ClaudeCodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let err = adapter
            .build_change_plan(
                &ctx,
                &intent("disableSubAgent", serde_json::json!({"slug": "missing"})),
            )
            .unwrap_err();

        assert!(matches!(err, AdapterError::Invalid(_)));
        assert!(err.to_string().contains("not found"));
    }
}
