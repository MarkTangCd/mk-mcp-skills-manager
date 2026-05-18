use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::domain::{AgentKind, ChangeOperation, FilePatch, McpServer, McpTransport, ScopeType};

use super::common::{
    command_version, display_path, duplicate_name_warnings, env_ref_keys, first_existing, home_dir,
    summary,
};
use super::traits::{
    AdapterError, AdapterResult, AgentAdapter, ChangeIntent, ChangePlanDraft, DetectionResult,
    ScanContext, ScanOutcome, ScopeLocation,
};

pub struct OpencodeAdapter;

#[derive(Debug, Deserialize, Serialize)]
struct OpencodeConfig {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    mcp: BTreeMap<String, OpencodeMcpConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpencodeMcpConfig {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    mcp_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<CommandValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    environment: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    env: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum CommandValue {
    String(String),
    Array(Vec<String>),
}

impl OpencodeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn config_candidates(&self, ctx: &ScanContext, scope_type: ScopeType) -> Vec<PathBuf> {
        if let Some(root) = &ctx.fixture_root {
            return vec![root.join("opencode.json")];
        }

        match scope_type {
            ScopeType::Global => home_dir()
                .map(|home| {
                    vec![
                        home.join(".config/opencode/opencode.json"),
                        home.join(".opencode.json"),
                    ]
                })
                .unwrap_or_default(),
            ScopeType::Project => ctx
                .project_path
                .as_ref()
                .map(|project| {
                    vec![
                        project.join("opencode.json"),
                        project.join(".opencode.json"),
                        project.join(".config/opencode/opencode.json"),
                    ]
                })
                .unwrap_or_default(),
        }
    }

    fn parse_config(&self, path: &PathBuf, scope_type: ScopeType) -> AdapterResult<ScanOutcome> {
        let raw = fs::read_to_string(path)?;
        let parsed: OpencodeConfig =
            serde_json::from_str(&raw).map_err(|err| AdapterError::Parse(err.to_string()))?;
        let warnings = duplicate_name_warnings("opencode", parsed.mcp.keys().cloned());
        let mcp_servers = parsed
            .mcp
            .into_iter()
            .map(|(name, config)| self.to_mcp_server(scope_type, name, config))
            .collect::<Vec<_>>();
        Ok(self.outcome(
            vec![ScopeLocation {
                scope_type,
                config_path: path.clone(),
                writable: false,
            }],
            mcp_servers,
            warnings,
        ))
    }

    fn to_mcp_server(
        &self,
        scope_type: ScopeType,
        name: String,
        config: OpencodeMcpConfig,
    ) -> McpServer {
        let (command, args) = match config.command {
            Some(CommandValue::String(command)) => (Some(command), vec![]),
            Some(CommandValue::Array(mut parts)) => {
                let command = if parts.is_empty() {
                    None
                } else {
                    Some(parts.remove(0))
                };
                (command, parts)
            }
            None => (None, vec![]),
        };
        let mut env = config.environment;
        env.extend(config.env);
        let transport = match config.mcp_type.as_deref() {
            Some("remote") => McpTransport::Http,
            _ if config.url.is_some() => McpTransport::Http,
            _ => McpTransport::Stdio,
        };
        McpServer {
            id: format!("opencode:{}:{}", scope_type_label(scope_type), name.trim()),
            name: name.trim().to_string(),
            transport,
            command,
            args,
            url: config.url,
            env_refs: env_ref_keys(&env),
            enabled: config.enabled.unwrap_or(true),
        }
    }

    fn outcome(
        &self,
        scopes: Vec<ScopeLocation>,
        mcp_servers: Vec<McpServer>,
        errors: Vec<String>,
    ) -> ScanOutcome {
        ScanOutcome {
            agent_kind_str: AgentKind::Opencode.as_str().to_string(),
            summary: summary(mcp_servers.len(), 0, 0, 0, errors.clone()),
            scopes,
            mcp_servers,
            skills: vec![],
            sub_agents: vec![],
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

        let mut config: OpencodeConfig = serde_json::from_str(&existing_content)
            .map_err(|err| AdapterError::Parse(err.to_string()))?;

        let mut warnings = Vec::new();

        match intent.kind.as_str() {
            "createMcp" => {
                let name = extract_name(&intent.payload)?;
                if config.mcp.contains_key(&name) {
                    warnings.push(format!(
                        "MCP server '{}' already exists and will be overwritten",
                        name
                    ));
                }
                let mcp_config = parse_mcp_config(&intent.payload)?;
                config.mcp.insert(name, mcp_config);
            }
            "updateMcp" => {
                let name = extract_name(&intent.payload)?;
                let Some(mut existing) = config.mcp.remove(&name) else {
                    return Err(AdapterError::Invalid(format!(
                        "MCP server '{}' not found",
                        name
                    )));
                };
                update_mcp_config(&intent.payload, &mut existing)?;
                config.mcp.insert(name, existing);
            }
            "deleteMcp" => {
                let name = extract_name(&intent.payload)?;
                if config.mcp.remove(&name).is_none() {
                    return Err(AdapterError::Invalid(format!(
                        "MCP server '{}' not found",
                        name
                    )));
                }
            }
            "enableMcp" => {
                let name = extract_name(&intent.payload)?;
                let Some(mcp) = config.mcp.get_mut(&name) else {
                    return Err(AdapterError::Invalid(format!(
                        "MCP server '{}' not found",
                        name
                    )));
                };
                mcp.enabled = Some(true);
            }
            "disableMcp" => {
                let name = extract_name(&intent.payload)?;
                let Some(mcp) = config.mcp.get_mut(&name) else {
                    return Err(AdapterError::Invalid(format!(
                        "MCP server '{}' not found",
                        name
                    )));
                };
                mcp.enabled = Some(false);
            }
            "enableSkill" | "disableSkill" | "deleteSkill" => {
                return Err(AdapterError::Unsupported(
                    "opencode skill sync is not supported".to_string(),
                ));
            }
            "enableSubAgent" | "disableSubAgent" | "deleteSubAgent" => {
                return Err(AdapterError::Unsupported(
                    "opencode sub-agent sync is not supported".to_string(),
                ));
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

impl Default for OpencodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for OpencodeAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Opencode
    }

    fn detect_installation(&self, ctx: &ScanContext) -> AdapterResult<DetectionResult> {
        let config_exists = self
            .locate_global_config(ctx)?
            .or(self.locate_project_config(ctx)?)
            .is_some();
        let version = command_version("opencode", &["--version"]);
        Ok(DetectionResult {
            installed: config_exists || version.is_some(),
            version,
            notes: if config_exists {
                vec!["opencode config path detected".into()]
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
            let path = fixture.join("opencode.json");
            if !path.exists() {
                return Ok(self.outcome(vec![], vec![], vec![]));
            }
            return self.parse_config(&path, ScopeType::Global);
        }

        let mut scopes = Vec::new();
        let mut mcp_servers = Vec::new();
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
                        errors.extend(outcome.errors);
                    }
                    Err(err) => {
                        errors.push(format!("{}: {err}", display_path(&location.config_path)))
                    }
                }
            }
        }
        Ok(self.outcome(scopes, mcp_servers, errors))
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

fn parse_mcp_config(payload: &JsonValue) -> AdapterResult<OpencodeMcpConfig> {
    serde_json::from_value(payload.clone())
        .map_err(|err| AdapterError::Invalid(format!("invalid MCP config: {err}")))
}

fn update_mcp_config(payload: &JsonValue, existing: &mut OpencodeMcpConfig) -> AdapterResult<()> {
    if let Some(ty) = payload.get("type").and_then(|v| v.as_str()) {
        existing.mcp_type = Some(ty.to_string());
    }
    if let Some(cmd) = payload.get("command") {
        existing.command = serde_json::from_value(cmd.clone())
            .map_err(|err| AdapterError::Invalid(format!("invalid command: {err}")))?;
    }
    if let Some(url) = payload.get("url").and_then(|v| v.as_str()) {
        existing.url = Some(url.to_string());
    }
    if let Some(env) = payload.get("environment") {
        existing.environment = serde_json::from_value(env.clone())
            .map_err(|err| AdapterError::Invalid(format!("invalid environment: {err}")))?;
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
            .join("../fixtures/agents/opencode")
            .join(name)
    }

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
    fn parses_valid_opencode_fixture() {
        let adapter = OpencodeAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("valid-global")))
            .unwrap();
        assert_eq!(out.mcp_servers.len(), 2);
        assert!(out.mcp_servers.iter().any(|mcp| !mcp.enabled));
        assert_eq!(out.summary.mcp_count, 2);
    }

    #[test]
    fn empty_opencode_fixture_returns_empty_scan() {
        let adapter = OpencodeAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("empty")))
            .unwrap();
        assert_eq!(out.summary.total_resources, 0);
    }

    #[test]
    fn invalid_opencode_fixture_returns_parse_error() {
        let adapter = OpencodeAdapter::new();
        let err = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("invalid")))
            .unwrap_err();
        assert!(matches!(err, AdapterError::Parse(_)));
    }

    #[test]
    fn duplicate_opencode_fixture_returns_warning() {
        let adapter = OpencodeAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("duplicate-mcp")))
            .unwrap();
        assert_eq!(out.mcp_servers.len(), 2);
        assert_eq!(out.errors.len(), 1);
    }

    // ------------------------------------------------------------------
    // build_change_plan tests
    // ------------------------------------------------------------------

    #[test]
    fn create_mcp_on_empty_config() {
        let dir = tempdir().unwrap();
        let adapter = OpencodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent(
                    "createMcp",
                    serde_json::json!({
                        "name": "new-server",
                        "type": "local",
                        "command": ["echo", "hello"],
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
    fn create_mcp_on_existing_config() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("opencode.json"),
            r#"{"mcp":{"existing":{"type":"local","command":["echo","hi"],"enabled":true}}}"#,
        )
        .unwrap();

        let adapter = OpencodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent(
                    "createMcp",
                    serde_json::json!({
                        "name": "new-server",
                        "type": "local",
                        "command": ["echo", "hello"],
                        "enabled": true
                    }),
                ),
            )
            .unwrap();

        assert_eq!(plan.operations.len(), 1);
        assert!(plan.patches[0].before_hash.is_some());
        assert!(plan.patches[0].after_hash.is_some());
        // Diff should show both old and new lines
        assert!(plan.patches[0].diff.contains("existing"));
        assert!(plan.patches[0].diff.contains("new-server"));
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn create_mcp_duplicate_name_warns() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("opencode.json"),
            r#"{"mcp":{"dup":{"type":"local","command":["echo","hi"],"enabled":true}}}"#,
        )
        .unwrap();

        let adapter = OpencodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent(
                    "createMcp",
                    serde_json::json!({
                        "name": "dup",
                        "type": "local",
                        "command": ["echo", "hello"],
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
            dir.path().join("opencode.json"),
            r#"{"mcp":{"srv":{"type":"local","command":["echo","old"],"enabled":false}}}"#,
        )
        .unwrap();

        let adapter = OpencodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent(
                    "updateMcp",
                    serde_json::json!({
                        "name": "srv",
                        "command": ["echo", "new"],
                        "enabled": true
                    }),
                ),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        let mcp = payload.get("mcp").unwrap().get("srv").unwrap();
        assert_eq!(mcp.get("command").unwrap().as_array().unwrap()[1], "new");
        assert!(mcp.get("enabled").unwrap().as_bool().unwrap());
        // Type should be preserved
        assert_eq!(mcp.get("type").unwrap().as_str().unwrap(), "local");
    }

    #[test]
    fn delete_mcp_removes_entry() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("opencode.json"),
            r#"{"mcp":{"keep":{"type":"local","command":["echo","keep"],"enabled":true},"remove":{"type":"local","command":["echo","remove"],"enabled":true}}}"#,
        )
        .unwrap();

        let adapter = OpencodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("deleteMcp", serde_json::json!({"name": "remove"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        let mcp = payload.get("mcp").unwrap();
        assert!(mcp.get("remove").is_none());
        assert!(mcp.get("keep").is_some());
    }

    #[test]
    fn enable_mcp_sets_enabled_true() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("opencode.json"),
            r#"{"mcp":{"srv":{"type":"local","command":["echo","hi"],"enabled":false}}}"#,
        )
        .unwrap();

        let adapter = OpencodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("enableMcp", serde_json::json!({"name": "srv"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        let enabled = payload
            .get("mcp")
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
            dir.path().join("opencode.json"),
            r#"{"mcp":{"srv":{"type":"local","command":["echo","hi"],"enabled":true}}}"#,
        )
        .unwrap();

        let adapter = OpencodeAdapter::new();
        let ctx = ScanContext::empty().with_fixture(dir.path().to_path_buf());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("disableMcp", serde_json::json!({"name": "srv"})),
            )
            .unwrap();

        let payload = &plan.operations[0].payload;
        let enabled = payload
            .get("mcp")
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
        fs::write(dir.path().join("opencode.json"), r#"{ "mcp": { "broken": "#).unwrap();

        let adapter = OpencodeAdapter::new();
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
}
