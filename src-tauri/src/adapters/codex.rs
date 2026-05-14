use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::domain::{AgentKind, McpServer, McpTransport, ScopeType, Skill, SubAgent};

use super::common::{
    command_version, display_path, duplicate_name_warnings, env_ref_keys, first_existing, home_dir,
    summary,
};
use super::traits::{
    AdapterError, AdapterResult, AgentAdapter, DetectionResult, ScanContext, ScanOutcome,
    ScopeLocation,
};

pub struct CodexAdapter;

#[derive(Debug, Deserialize)]
struct CodexConfig {
    #[serde(default)]
    mcp_servers: BTreeMap<String, CodexMcpConfig>,
    #[serde(default)]
    skills: Vec<CodexSkillConfig>,
    #[serde(default)]
    custom_agents: Vec<CodexAgentConfig>,
}

#[derive(Debug, Deserialize)]
struct CodexMcpConfig {
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    url: Option<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CodexSkillConfig {
    slug: String,
    path: Option<String>,
    description: Option<String>,
    title: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CodexAgentConfig {
    slug: String,
    description: Option<String>,
    role: Option<String>,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
}

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }

    fn config_candidates(&self, ctx: &ScanContext, scope_type: ScopeType) -> Vec<PathBuf> {
        if let Some(root) = &ctx.fixture_root {
            return vec![root.join("config.toml")];
        }

        match scope_type {
            ScopeType::Global => home_dir()
                .map(|home| vec![home.join(".codex/config.toml")])
                .unwrap_or_default(),
            ScopeType::Project => ctx
                .project_path
                .as_ref()
                .map(|project| {
                    vec![
                        project.join(".codex/config.toml"),
                        project.join("config.toml"),
                    ]
                })
                .unwrap_or_default(),
        }
    }

    fn parse_config(&self, path: &PathBuf, scope_type: ScopeType) -> AdapterResult<ScanOutcome> {
        let raw = fs::read_to_string(path)?;
        let parsed: CodexConfig =
            toml::from_str(&raw).map_err(|err| AdapterError::Parse(err.to_string()))?;
        let errors = duplicate_name_warnings("Codex", parsed.mcp_servers.keys().cloned());
        let mcp_servers = parsed
            .mcp_servers
            .into_iter()
            .map(|(name, config)| self.to_mcp_server(scope_type, name, config))
            .collect::<Vec<_>>();
        let skills = parsed
            .skills
            .into_iter()
            .map(|config| self.to_skill(scope_type, config))
            .collect::<Vec<_>>();
        let sub_agents = parsed
            .custom_agents
            .into_iter()
            .map(|config| self.to_sub_agent(scope_type, config))
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
        config: CodexMcpConfig,
    ) -> McpServer {
        McpServer {
            id: format!("codex:{}:{}", scope_type_label(scope_type), name.trim()),
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

    fn to_skill(&self, scope_type: ScopeType, config: CodexSkillConfig) -> Skill {
        let slug = config.slug.trim().to_string();
        Skill {
            id: format!("codex:{}:skill:{slug}", scope_type_label(scope_type)),
            title: config.title.unwrap_or_else(|| slug.clone()),
            slug,
            description: config.description,
            tags: config.tags,
            status: "active".into(),
            source_path: config.path,
        }
    }

    fn to_sub_agent(&self, scope_type: ScopeType, config: CodexAgentConfig) -> SubAgent {
        let slug = config.slug.trim().to_string();
        SubAgent {
            id: format!("codex:{}:custom-agent:{slug}", scope_type_label(scope_type)),
            slug: slug.clone(),
            role: config.role.or(config.description).unwrap_or(slug),
            agent_kinds: vec![AgentKind::Codex],
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
            agent_kind_str: AgentKind::Codex.as_str().to_string(),
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

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for CodexAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn detect_installation(&self, ctx: &ScanContext) -> AdapterResult<DetectionResult> {
        let config_exists = self
            .locate_global_config(ctx)?
            .or(self.locate_project_config(ctx)?)
            .is_some();
        let version = command_version("codex", &["--version"]);
        Ok(DetectionResult {
            installed: config_exists || version.is_some(),
            version,
            notes: if config_exists {
                vec!["Codex config path detected".into()]
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
            let path = fixture.join("config.toml");
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
            .join("../fixtures/agents/codex")
            .join(name)
    }

    #[test]
    fn parses_valid_codex_fixture() {
        let adapter = CodexAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("valid-global")))
            .unwrap();
        assert_eq!(out.mcp_servers.len(), 1);
        assert_eq!(out.skills.len(), 1);
        assert_eq!(out.sub_agents.len(), 1);
    }

    #[test]
    fn invalid_codex_fixture_returns_parse_error() {
        let adapter = CodexAdapter::new();
        let err = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("invalid")))
            .unwrap_err();
        assert!(matches!(err, AdapterError::Parse(_)));
    }

    #[test]
    fn duplicate_codex_fixture_returns_warning() {
        let adapter = CodexAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("duplicate-mcp")))
            .unwrap();
        assert_eq!(out.errors.len(), 1);
    }
}
