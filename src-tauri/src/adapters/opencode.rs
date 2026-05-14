use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::domain::{AgentKind, McpServer, McpTransport, ScopeType};

use super::common::{
    command_version, display_path, duplicate_name_warnings, env_ref_keys, first_existing, home_dir,
    summary,
};
use super::traits::{
    AdapterError, AdapterResult, AgentAdapter, DetectionResult, ScanContext, ScanOutcome,
    ScopeLocation,
};

pub struct OpencodeAdapter;

#[derive(Debug, Deserialize)]
struct OpencodeConfig {
    #[serde(default)]
    mcp: BTreeMap<String, OpencodeMcpConfig>,
}

#[derive(Debug, Deserialize)]
struct OpencodeMcpConfig {
    #[serde(rename = "type")]
    mcp_type: Option<String>,
    command: Option<CommandValue>,
    url: Option<String>,
    #[serde(default)]
    environment: HashMap<String, String>,
    #[serde(default)]
    env: HashMap<String, String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
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
}

fn scope_type_label(scope_type: ScopeType) -> &'static str {
    match scope_type {
        ScopeType::Global => "global",
        ScopeType::Project => "project",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/agents/opencode")
            .join(name)
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
}
