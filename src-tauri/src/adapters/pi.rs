use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::domain::{AgentKind, ChangeOperation, FilePatch, PiResource, PiResourceKind, ScopeType};

use super::common::{command_version, first_existing, home_dir, summary};
use super::traits::{
    AdapterError, AdapterResult, AgentAdapter, ChangeIntent, ChangePlanDraft, DetectionResult,
    ScanContext, ScanOutcome, ScopeLocation,
};

pub struct PiAdapter;

#[derive(Debug, Default, Deserialize, Serialize)]
struct PiSettings {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    resource_paths: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skills: Vec<PiSkillConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    prompt_templates: Vec<PiPromptConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    extensions: Vec<PiExtensionConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    packages: Vec<PiPackageConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    themes: Vec<PiThemeConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PiSkillConfig {
    slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PiPromptConfig {
    slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PiExtensionConfig {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trusted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PiPackageConfig {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PiThemeConfig {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

    fn resolve_target_path(
        &self,
        ctx: &ScanContext,
        scope_type: ScopeType,
    ) -> AdapterResult<PathBuf> {
        let candidates = self.config_candidates(ctx, scope_type);
        if let Some(path) = first_existing(&candidates) {
            return Ok(path);
        }
        candidates
            .into_iter()
            .next()
            .ok_or_else(|| AdapterError::Invalid(format!("no config candidates for scope {scope_type:?}")))
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
            ("".to_string(), None)
        };

        let mut settings: PiSettings = if existing_content.trim().is_empty() {
            PiSettings::default()
        } else {
            serde_yaml::from_str(&existing_content)
                .map_err(|err| AdapterError::Parse(err.to_string()))?
        };

        let mut warnings = Vec::new();

        match intent.kind.as_str() {
            "enableSkill" => {
                let library_path = ctx
                    .app_data_path
                    .as_ref()
                    .map(|p| p.join("library").join("skills").to_string_lossy().to_string())
                    .or_else(|| {
                        intent.payload.get("libraryPath").and_then(|v| v.as_str()).map(|s| s.to_string())
                    })
                    .ok_or_else(|| AdapterError::Invalid(
                        "app_data_path or libraryPath required for Pi enableSkill".to_string()
                    ))?;

                if settings.resource_paths.contains_key("skills") {
                    warnings.push(
                        "resource_paths.skills already exists and will be overwritten".to_string()
                    );
                }
                settings.resource_paths.insert("skills".to_string(), library_path);
            }
            "disableSkill" | "deleteSkill" => {
                let library_path = ctx
                    .app_data_path
                    .as_ref()
                    .map(|p| p.join("library").join("skills").to_string_lossy().to_string());

                if let Some(current) = settings.resource_paths.get("skills") {
                    if let Some(ref expected) = library_path {
                        if current == expected {
                            settings.resource_paths.remove("skills");
                        } else {
                            warnings.push(format!(
                                "resource_paths.skills points to '{}' which is not the AgentHub library; leaving unchanged",
                                current
                            ));
                        }
                    } else {
                        settings.resource_paths.remove("skills");
                    }
                } else {
                    return Err(AdapterError::Invalid(
                        "resource_paths.skills not found".to_string()
                    ));
                }
            }
            "enableSubAgent" | "disableSubAgent" | "deleteSubAgent" => {
                return Err(AdapterError::Unsupported(
                    "Pi does not support sub-agents".to_string()
                ));
            }
            _ => {
                return Err(AdapterError::Unsupported(format!(
                    "unsupported change intent kind: {}",
                    intent.kind
                )));
            }
        }

        let new_content = serde_yaml::to_string(&settings)
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
                kind: "writeText".to_string(),
                target: target_path.to_string_lossy().to_string(),
                payload: JsonValue::String(new_content),
            }],
            target_files: vec![target_path],
            warnings,
            patches: vec![patch],
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
    fn empty_pi_fixture_returns_empty_scan() {
        let adapter = PiAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("empty")))
            .unwrap();
        assert_eq!(out.summary.total_resources, 0);
        assert!(out.pi_resources.is_empty());
        assert!(out.sub_agents.is_empty());
    }

    #[test]
    fn duplicate_pi_package_fixture_parses_without_crash() {
        let adapter = PiAdapter::new();
        let out = adapter
            .scan(&ScanContext::empty().with_fixture(fixture("duplicate-mcp")))
            .unwrap();
        assert_eq!(out.pi_resources.len(), 2);
        assert!(out
            .pi_resources
            .iter()
            .any(|r| r.resource_type == PiResourceKind::Package));
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

    // ------------------------------------------------------------------
    // build_change_plan tests
    // ------------------------------------------------------------------

    fn intent(kind: &str, payload: JsonValue) -> ChangeIntent {
        ChangeIntent {
            kind: kind.to_string(),
            resource_type: crate::domain::ResourceType::Skill,
            target_scope: ScopeType::Global,
            project_id: None,
            payload,
        }
    }

    #[test]
    fn enable_skill_sets_resource_paths_skills() {
        let dir = tempdir().unwrap();
        let adapter = PiAdapter::new();
        let ctx = ScanContext::empty()
            .with_fixture(dir.path().to_path_buf())
            .with_app_data("/agenthub".into());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("enableSkill", serde_json::json!({})),
            )
            .unwrap();

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].kind, "writeText");
        assert_eq!(plan.patches.len(), 1);
        assert!(plan.patches[0].before_hash.is_none());
        assert!(plan.patches[0].after_hash.is_some());

        let content = plan.operations[0].payload.as_str().unwrap();
        assert!(content.contains("resource_paths"));
        assert!(content.contains("skills"));
        assert!(content.contains("/agenthub/library/skills"));
    }

    #[test]
    fn enable_skill_overwrite_warns() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("settings.yaml"),
            "resource_paths:\n  skills: /old/path\n",
        )
        .unwrap();

        let adapter = PiAdapter::new();
        let ctx = ScanContext::empty()
            .with_fixture(dir.path().to_path_buf())
            .with_app_data("/agenthub".into());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("enableSkill", serde_json::json!({})),
            )
            .unwrap();

        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("already exists"));

        let content = plan.operations[0].payload.as_str().unwrap();
        assert!(content.contains("/agenthub/library/skills"));
        assert!(!content.contains("/old/path"));
    }

    #[test]
    fn disable_skill_removes_library_path() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("settings.yaml"),
            "resource_paths:\n  skills: /agenthub/library/skills\n  other: /other\n",
        )
        .unwrap();

        let adapter = PiAdapter::new();
        let ctx = ScanContext::empty()
            .with_fixture(dir.path().to_path_buf())
            .with_app_data("/agenthub".into());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("disableSkill", serde_json::json!({})),
            )
            .unwrap();

        let content = plan.operations[0].payload.as_str().unwrap();
        assert!(!content.contains("/agenthub/library/skills"));
        assert!(content.contains("/other"));
    }

    #[test]
    fn disable_skill_non_library_warns() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("settings.yaml"),
            "resource_paths:\n  skills: /custom/skills\n",
        )
        .unwrap();

        let adapter = PiAdapter::new();
        let ctx = ScanContext::empty()
            .with_fixture(dir.path().to_path_buf())
            .with_app_data("/agenthub".into());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("disableSkill", serde_json::json!({})),
            )
            .unwrap();

        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("not the AgentHub library"));

        let content = plan.operations[0].payload.as_str().unwrap();
        assert!(content.contains("/custom/skills"));
    }

    #[test]
    fn disable_missing_skill_returns_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("settings.yaml"), "resource_paths:\n  other: /other\n").unwrap();

        let adapter = PiAdapter::new();
        let ctx = ScanContext::empty()
            .with_fixture(dir.path().to_path_buf())
            .with_app_data("/agenthub".into());
        let err = adapter
            .build_change_plan(
                &ctx,
                &intent("disableSkill", serde_json::json!({})),
            )
            .unwrap_err();

        assert!(matches!(err, AdapterError::Invalid(_)));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn delete_skill_removes_library_path() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("settings.yaml"),
            "resource_paths:\n  skills: /agenthub/library/skills\n",
        )
        .unwrap();

        let adapter = PiAdapter::new();
        let ctx = ScanContext::empty()
            .with_fixture(dir.path().to_path_buf())
            .with_app_data("/agenthub".into());
        let plan = adapter
            .build_change_plan(
                &ctx,
                &intent("deleteSkill", serde_json::json!({})),
            )
            .unwrap();

        let content = plan.operations[0].payload.as_str().unwrap();
        assert!(!content.contains("skills"));
    }
}
