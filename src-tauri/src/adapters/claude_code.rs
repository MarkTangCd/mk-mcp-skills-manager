use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::domain::{AgentKind, McpServer, McpTransport, ScopeType, Skill, SubAgent};

use super::common::{
    command_version, display_path, duplicate_name_warnings, env_ref_keys, first_existing, summary,
};
use super::traits::{
    AdapterError, AdapterResult, AgentAdapter, DetectionResult, ScanContext, ScanOutcome,
    ScopeLocation,
};

pub struct ClaudeCodeAdapter;

#[derive(Debug, Deserialize)]
struct ClaudeConfig {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: BTreeMap<String, ClaudeMcpConfig>,
    #[serde(default)]
    skills: BTreeMap<String, ClaudeSkillConfig>,
    #[serde(default)]
    agents: BTreeMap<String, ClaudeAgentConfig>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMcpConfig {
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    url: Option<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ClaudeSkillConfig {
    path: Option<String>,
    description: Option<String>,
    title: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeAgentConfig {
    description: Option<String>,
    role: Option<String>,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
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
            ScopeType::Global => vec![],
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
        if let Some(location) = self.locate_project_config(ctx)? {
            match self.parse_config(&location.config_path, ScopeType::Project) {
                Ok(outcome) => {
                    scopes.extend(outcome.scopes);
                    mcp_servers.extend(outcome.mcp_servers);
                    skills.extend(outcome.skills);
                    sub_agents.extend(outcome.sub_agents);
                    errors.extend(outcome.errors);
                }
                Err(err) => errors.push(format!("{}: {err}", display_path(&location.config_path))),
            }
        }
        Ok(self.outcome(scopes, mcp_servers, skills, sub_agents, errors))
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
}
