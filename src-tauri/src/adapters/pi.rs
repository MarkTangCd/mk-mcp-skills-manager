use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::domain::{AgentKind, PiResource, PiResourceKind, ScopeType};

use super::common::{command_version, first_existing, home_dir, summary};
use super::traits::{
    AdapterError, AdapterResult, AgentAdapter, DetectionResult, ScanContext, ScanOutcome,
    ScopeLocation,
};

pub struct PiAdapter;

#[derive(Debug, Default, Deserialize)]
struct PiSettings {
    #[serde(default)]
    resource_paths: BTreeMap<String, String>,
    #[serde(default)]
    skills: Vec<PiSkillConfig>,
    #[serde(default)]
    prompt_templates: Vec<PiPromptConfig>,
    #[serde(default)]
    extensions: Vec<PiExtensionConfig>,
    #[serde(default)]
    packages: Vec<PiPackageConfig>,
    #[serde(default)]
    themes: Vec<PiThemeConfig>,
}

#[derive(Debug, Deserialize)]
struct PiSkillConfig {
    slug: String,
    path: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PiPromptConfig {
    slug: String,
    title: Option<String>,
    body: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PiExtensionConfig {
    id: String,
    path: Option<String>,
    trusted: Option<bool>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PiPackageConfig {
    id: String,
    version: Option<String>,
    path: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PiThemeConfig {
    id: String,
    path: Option<String>,
    enabled: Option<bool>,
}

impl PiAdapter {
    pub fn new() -> Self {
        Self
    }

    fn config_candidates(&self, ctx: &ScanContext, scope_type: ScopeType) -> Vec<PathBuf> {
        if let Some(root) = &ctx.fixture_root {
            return vec![root.join("settings.yaml")];
        }

        match scope_type {
            ScopeType::Global => home_dir()
                .map(|home| {
                    vec![
                        home.join(".config/pi/settings.yaml"),
                        home.join(".pi/settings.yaml"),
                    ]
                })
                .unwrap_or_default(),
            ScopeType::Project => ctx
                .project_path
                .as_ref()
                .map(|project| {
                    vec![
                        project.join(".pi/settings.yaml"),
                        project.join("settings.yaml"),
                    ]
                })
                .unwrap_or_default(),
        }
    }

    fn parse_settings(&self, path: &PathBuf, scope_type: ScopeType) -> AdapterResult<ScanOutcome> {
        let raw = fs::read_to_string(path)?;
        let settings: PiSettings =
            serde_yaml::from_str(&raw).map_err(|err| AdapterError::Parse(err.to_string()))?;
        let mut warnings = Vec::new();
        let mut resources = Vec::new();

        for (key, value) in settings.resource_paths {
            self.warn_missing_path(&value, &mut warnings);
            resources.push(PiResource {
                id: format!("pi:{}:setting:{key}", scope_type_label(scope_type)),
                resource_type: PiResourceKind::Setting,
                source: key,
                path: Some(value),
                enabled: true,
                trusted: true,
            });
        }

        for skill in settings.skills {
            if let Some(path) = &skill.path {
                self.warn_missing_path(path, &mut warnings);
            }
            resources.push(PiResource {
                id: format!(
                    "pi:{}:skill:{}",
                    scope_type_label(scope_type),
                    skill.slug.trim()
                ),
                resource_type: PiResourceKind::Skill,
                source: skill.slug.trim().to_string(),
                path: skill.path,
                enabled: skill.enabled.unwrap_or(true),
                trusted: true,
            });
        }

        for prompt in settings.prompt_templates {
            resources.push(PiResource {
                id: format!(
                    "pi:{}:prompt:{}",
                    scope_type_label(scope_type),
                    prompt.slug.trim()
                ),
                resource_type: PiResourceKind::PromptTemplate,
                source: prompt
                    .title
                    .or(prompt.body)
                    .unwrap_or_else(|| prompt.slug.trim().to_string()),
                path: None,
                enabled: prompt.enabled.unwrap_or(true),
                trusted: true,
            });
        }

        for extension in settings.extensions {
            if let Some(path) = &extension.path {
                self.warn_missing_path(path, &mut warnings);
            }
            resources.push(PiResource {
                id: format!(
                    "pi:{}:extension:{}",
                    scope_type_label(scope_type),
                    extension.id.trim()
                ),
                resource_type: PiResourceKind::Extension,
                source: extension.id.trim().to_string(),
                path: extension.path,
                enabled: extension.enabled.unwrap_or(true),
                trusted: extension.trusted.unwrap_or(false),
            });
        }

        for package in settings.packages {
            if let Some(path) = &package.path {
                self.warn_missing_path(path, &mut warnings);
            }
            resources.push(PiResource {
                id: format!(
                    "pi:{}:package:{}",
                    scope_type_label(scope_type),
                    package.id.trim()
                ),
                resource_type: PiResourceKind::Package,
                source: package
                    .version
                    .map(|version| format!("{}@{version}", package.id.trim()))
                    .unwrap_or_else(|| package.id.trim().to_string()),
                path: package.path,
                enabled: package.enabled.unwrap_or(true),
                trusted: true,
            });
        }

        for theme in settings.themes {
            if let Some(path) = &theme.path {
                self.warn_missing_path(path, &mut warnings);
            }
            resources.push(PiResource {
                id: format!(
                    "pi:{}:theme:{}",
                    scope_type_label(scope_type),
                    theme.id.trim()
                ),
                resource_type: PiResourceKind::Theme,
                source: theme.id.trim().to_string(),
                path: theme.path,
                enabled: theme.enabled.unwrap_or(true),
                trusted: true,
            });
        }

        Ok(self.outcome(
            vec![ScopeLocation {
                scope_type,
                config_path: path.clone(),
                writable: false,
            }],
            resources,
            warnings,
        ))
    }

    fn warn_missing_path(&self, path: &str, warnings: &mut Vec<String>) {
        if !Path::new(path).exists() {
            warnings.push(format!("Pi resource path does not exist: {path}"));
        }
    }

    fn outcome(
        &self,
        scopes: Vec<ScopeLocation>,
        pi_resources: Vec<PiResource>,
        errors: Vec<String>,
    ) -> ScanOutcome {
        ScanOutcome {
            agent_kind_str: AgentKind::Pi.as_str().to_string(),
            summary: summary(0, 0, 0, pi_resources.len(), errors.clone()),
            scopes,
            mcp_servers: vec![],
            skills: vec![],
            sub_agents: vec![],
            pi_resources,
            errors,
        }
    }
}

impl Default for PiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for PiAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Pi
    }

    fn detect_installation(&self, ctx: &ScanContext) -> AdapterResult<DetectionResult> {
        let config_exists = self
            .locate_global_config(ctx)?
            .or(self.locate_project_config(ctx)?)
            .is_some();
        let version = command_version("pi", &["--version"]);
        Ok(DetectionResult {
            installed: config_exists || version.is_some(),
            version,
            notes: if config_exists {
                vec!["Pi settings path detected".into()]
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
            let path = fixture.join("settings.yaml");
            if !path.exists() {
                return Ok(self.outcome(vec![], vec![], vec![]));
            }
            return self.parse_settings(&path, ScopeType::Global);
        }

        let mut scopes = Vec::new();
        let mut resources = Vec::new();
        let mut errors = Vec::new();
        for scope_type in [ScopeType::Global, ScopeType::Project] {
            if let Some(location) = match scope_type {
                ScopeType::Global => self.locate_global_config(ctx)?,
                ScopeType::Project => self.locate_project_config(ctx)?,
            } {
                match self.parse_settings(&location.config_path, scope_type) {
                    Ok(outcome) => {
                        scopes.extend(outcome.scopes);
                        resources.extend(outcome.pi_resources);
                        errors.extend(outcome.errors);
                    }
                    Err(err) => {
                        errors.push(format!("{}: {err}", location.config_path.to_string_lossy()))
                    }
                }
            }
        }
        Ok(self.outcome(scopes, resources, errors))
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
            .join("../fixtures/agents/pi")
            .join(name)
    }

    #[test]
    fn parses_valid_pi_fixture_without_sub_agents() {
        let adapter = PiAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("valid-global")))
            .unwrap();
        assert_eq!(out.pi_resources.len(), 7);
        assert!(out.sub_agents.is_empty());
        assert!(out
            .pi_resources
            .iter()
            .any(
                |resource| resource.resource_type == PiResourceKind::Extension && !resource.trusted
            ));
    }

    #[test]
    fn invalid_pi_fixture_returns_parse_error() {
        let adapter = PiAdapter::new();
        let err = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("invalid")))
            .unwrap_err();
        assert!(matches!(err, AdapterError::Parse(_)));
    }

    #[test]
    fn missing_pi_resource_paths_return_warnings() {
        let adapter = PiAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("valid-global")))
            .unwrap();
        assert!(out
            .errors
            .iter()
            .any(|warning| warning.contains("does not exist")));
    }
}
