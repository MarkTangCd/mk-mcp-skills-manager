use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;
use uuid::Uuid;

use crate::adapters::{ScanOutcome, ScopeLocation};
use crate::db::{Database, DbError};
use crate::domain::{AgentKind, PiResourceKind, ResourceType, ScopeType};

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("db error: {0}")]
    Db(#[from] DbError),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type ResourceResult<T> = Result<T, ResourceError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBindingRecord {
    pub id: String,
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub agent_kind: AgentKind,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub scope_type: ScopeType,
    pub enabled: bool,
    pub status: String,
    pub config_path: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRecord {
    pub id: String,
    pub resource_type: ResourceType,
    pub name: String,
    pub slug: Option<String>,
    pub agent_kind: Option<AgentKind>,
    pub source_path: Option<String>,
    pub status: String,
    pub payload: JsonValue,
    pub updated_at: String,
    pub bindings: Vec<ResourceBindingRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSource {
    pub resource_id: String,
    pub resource_name: String,
    pub scope_type: ScopeType,
    pub project_id: Option<String>,
    pub config_path: Option<String>,
    pub source_path: Option<String>,
    pub enabled: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixCell {
    pub agent_kind: AgentKind,
    pub status: String,
    pub sources: Vec<MatrixSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixRow {
    pub key: String,
    pub name: String,
    pub resource_type: ResourceType,
    pub cells: Vec<MatrixCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiResourceKindSummary {
    pub resource_type: PiResourceKind,
    pub total: u32,
    pub enabled: u32,
    pub disabled: u32,
    pub missing: u32,
    pub untrusted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PiResourceSummary {
    pub total: u32,
    pub enabled: u32,
    pub disabled: u32,
    pub missing: u32,
    pub untrusted: u32,
    pub by_kind: Vec<PiResourceKindSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMatrix {
    pub project_id: String,
    pub agents: Vec<AgentKind>,
    pub mcp_matrix: Vec<MatrixRow>,
    pub skills_matrix: Vec<MatrixRow>,
    pub sub_agent_matrix: Vec<MatrixRow>,
    pub pi_resource_summary: PiResourceSummary,
}

#[derive(Clone)]
pub struct ResourceIndexer {
    db: Arc<Database>,
}

impl ResourceIndexer {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn index_scan(
        &self,
        project_id: Option<&str>,
        agent_kind: AgentKind,
        outcome: &ScanOutcome,
    ) -> ResourceResult<()> {
        let now = Utc::now().to_rfc3339();
        let scope_paths = scope_paths(&outcome.scopes);
        self.db.with_conn_mut(|conn| -> ResourceResult<()> {
            let tx = conn.transaction()?;

            for scope in &outcome.scopes {
                tx.execute(
                    "INSERT INTO config_scopes
                         (id, agent_kind, scope_type, project_id, config_path, writable)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                         ON CONFLICT(agent_kind, scope_type, project_id, config_path)
                         DO UPDATE SET writable = excluded.writable",
                    params![
                        Uuid::new_v4().to_string(),
                        agent_kind.as_str(),
                        scope_type_str(scope.scope_type),
                        project_id,
                        scope.config_path.to_string_lossy().to_string(),
                        scope.writable
                    ],
                )?;
            }

            tx.execute(
                "UPDATE resource_bindings
                     SET status = 'missing', updated_at = ?1
                     WHERE agent_kind = ?2
                       AND ((?3 IS NULL AND project_id IS NULL)
                         OR (?3 IS NOT NULL AND project_id = ?3))",
                params![now, agent_kind.as_str(), project_id],
            )?;

            for mcp in &outcome.mcp_servers {
                upsert_resource(
                    &tx,
                    project_id,
                    agent_kind,
                    ResourceType::Mcp,
                    &mcp.id,
                    &mcp.name,
                    None,
                    mcp.enabled,
                    source_path_for(&scope_paths, infer_scope(agent_kind, &mcp.id, project_id)),
                    source_path_for(&scope_paths, infer_scope(agent_kind, &mcp.id, project_id)),
                    infer_scope(agent_kind, &mcp.id, project_id),
                    serde_json::to_value(mcp)?,
                    &now,
                )?;
            }

            for skill in &outcome.skills {
                let scope_type = infer_scope(agent_kind, &skill.id, project_id);
                upsert_resource(
                    &tx,
                    project_id,
                    agent_kind,
                    ResourceType::Skill,
                    &skill.id,
                    &skill.title,
                    Some(&skill.slug),
                    skill.status != "disabled",
                    skill
                        .source_path
                        .clone()
                        .or_else(|| source_path_for(&scope_paths, scope_type)),
                    source_path_for(&scope_paths, scope_type),
                    scope_type,
                    serde_json::to_value(skill)?,
                    &now,
                )?;
            }

            for sub_agent in &outcome.sub_agents {
                let scope_type = infer_scope(agent_kind, &sub_agent.id, project_id);
                upsert_resource(
                    &tx,
                    project_id,
                    agent_kind,
                    ResourceType::SubAgent,
                    &sub_agent.id,
                    &sub_agent.slug,
                    Some(&sub_agent.slug),
                    true,
                    source_path_for(&scope_paths, scope_type),
                    source_path_for(&scope_paths, scope_type),
                    scope_type,
                    serde_json::to_value(sub_agent)?,
                    &now,
                )?;
            }

            for pi_resource in &outcome.pi_resources {
                let scope_type = infer_scope(agent_kind, &pi_resource.id, project_id);
                upsert_resource(
                    &tx,
                    project_id,
                    agent_kind,
                    ResourceType::PiResource,
                    &pi_resource.id,
                    &pi_resource.source,
                    Some(&pi_resource.source),
                    pi_resource.enabled,
                    pi_resource
                        .path
                        .clone()
                        .or_else(|| source_path_for(&scope_paths, scope_type)),
                    source_path_for(&scope_paths, scope_type),
                    scope_type,
                    serde_json::to_value(pi_resource)?,
                    &now,
                )?;
            }

            tx.execute(
                "UPDATE resources
                     SET status = CASE
                       WHEN EXISTS (
                         SELECT 1 FROM resource_bindings b
                         WHERE b.resource_id = resources.id AND b.status = 'active'
                       )
                       THEN 'active'
                       ELSE 'missing'
                     END",
                [],
            )?;

            tx.commit()?;
            Ok(())
        })?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct ResourceService {
    db: Arc<Database>,
}

impl ResourceService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list(&self, resource_type: Option<ResourceType>) -> ResourceResult<Vec<ResourceRecord>> {
        let rows = self
            .db
            .with_conn(|conn| -> rusqlite::Result<Vec<ResourceRecord>> {
                let mut stmt = conn.prepare(
                    "SELECT id, resource_type, name, slug, agent_kind, source_path,
                            json_payload, status, updated_at
                     FROM resources
                     WHERE (?1 IS NULL OR resource_type = ?1)
                     ORDER BY resource_type, lower(name), agent_kind",
                )?;
                let iter =
                    stmt.query_map(params![resource_type.map(resource_type_str)], |row| {
                        let resource_type_raw: String = row.get(1)?;
                        let agent_kind_raw: Option<String> = row.get(4)?;
                        let payload_raw: String = row.get(6)?;
                        Ok(ResourceRecord {
                            id: row.get(0)?,
                            resource_type: parse_resource_type(&resource_type_raw),
                            name: row.get(2)?,
                            slug: row.get(3)?,
                            agent_kind: agent_kind_raw.as_deref().map(parse_agent_kind),
                            source_path: row.get(5)?,
                            payload: serde_json::from_str(&payload_raw).unwrap_or(JsonValue::Null),
                            status: row.get(7)?,
                            updated_at: row.get(8)?,
                            bindings: Vec::new(),
                        })
                    })?;
                let mut resources = Vec::new();
                for row in iter {
                    resources.push(row?);
                }
                Ok(resources)
            })
            .map_err(DbError::from)?;

        let mut records = rows;
        for record in &mut records {
            record.bindings = self.bindings_for(&record.id)?;
        }
        Ok(records)
    }

    pub fn project_matrix(&self, project_id: &str) -> ResourceResult<ProjectMatrix> {
        let resources = self.list(None)?;
        let project_resources = resources
            .into_iter()
            .filter(|resource| {
                resource
                    .bindings
                    .iter()
                    .any(|binding| binding.project_id.as_deref() == Some(project_id))
            })
            .collect::<Vec<_>>();
        let agents = vec![
            AgentKind::ClaudeCode,
            AgentKind::Codex,
            AgentKind::Opencode,
            AgentKind::Pi,
        ];
        Ok(ProjectMatrix {
            project_id: project_id.to_string(),
            agents: agents.clone(),
            mcp_matrix: build_matrix(&project_resources, ResourceType::Mcp, &agents, project_id),
            skills_matrix: build_matrix(
                &project_resources,
                ResourceType::Skill,
                &agents,
                project_id,
            ),
            sub_agent_matrix: build_matrix(
                &project_resources,
                ResourceType::SubAgent,
                &agents,
                project_id,
            ),
            pi_resource_summary: build_pi_summary(&project_resources, project_id),
        })
    }

    fn bindings_for(&self, resource_id: &str) -> ResourceResult<Vec<ResourceBindingRecord>> {
        let bindings = self
            .db
            .with_conn(|conn| -> rusqlite::Result<Vec<ResourceBindingRecord>> {
                let mut stmt = conn.prepare(
                    "SELECT b.id, b.resource_type, b.resource_id, b.agent_kind, b.project_id,
                            p.name, b.scope_type, b.enabled, b.status, b.config_path, b.updated_at
                     FROM resource_bindings b
                     LEFT JOIN projects p ON p.id = b.project_id
                     WHERE b.resource_id = ?1
                     ORDER BY b.agent_kind, b.scope_type, p.name",
                )?;
                let iter = stmt.query_map(params![resource_id], |row| {
                    let resource_type_raw: String = row.get(1)?;
                    let agent_kind_raw: String = row.get(3)?;
                    let scope_type_raw: String = row.get(6)?;
                    Ok(ResourceBindingRecord {
                        id: row.get(0)?,
                        resource_type: parse_resource_type(&resource_type_raw),
                        resource_id: row.get(2)?,
                        agent_kind: parse_agent_kind(&agent_kind_raw),
                        project_id: row.get(4)?,
                        project_name: row.get(5)?,
                        scope_type: parse_scope_type(&scope_type_raw),
                        enabled: row.get(7)?,
                        status: row.get(8)?,
                        config_path: row.get(9)?,
                        updated_at: row.get(10)?,
                    })
                })?;
                let mut out = Vec::new();
                for binding in iter {
                    out.push(binding?);
                }
                Ok(out)
            })
            .map_err(DbError::from)?;
        Ok(bindings)
    }
}

fn upsert_resource(
    tx: &rusqlite::Transaction<'_>,
    project_id: Option<&str>,
    agent_kind: AgentKind,
    resource_type: ResourceType,
    resource_id: &str,
    name: &str,
    slug: Option<&str>,
    enabled: bool,
    source_path: Option<String>,
    config_path: Option<String>,
    scope_type: ScopeType,
    payload: JsonValue,
    now: &str,
) -> ResourceResult<()> {
    let resource_type_str = resource_type_str(resource_type);
    let payload_json = serde_json::to_string(&payload)?;
    tx.execute(
        "INSERT INTO resources
         (id, resource_type, name, slug, agent_kind, source_path, json_payload, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8, ?8)
         ON CONFLICT(id) DO UPDATE SET
           resource_type = excluded.resource_type,
           name = excluded.name,
           slug = excluded.slug,
           agent_kind = excluded.agent_kind,
           source_path = excluded.source_path,
           json_payload = excluded.json_payload,
           status = 'active',
           updated_at = excluded.updated_at",
        params![
            resource_id,
            resource_type_str,
            name,
            slug,
            agent_kind.as_str(),
            source_path,
            payload_json,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO resource_bindings
         (id, resource_type, resource_id, agent_kind, project_id, scope_type,
          enabled, created_at, updated_at, config_path, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9, 'active')
         ON CONFLICT(resource_type, resource_id, agent_kind, project_id, scope_type)
         DO UPDATE SET
           enabled = excluded.enabled,
           updated_at = excluded.updated_at,
           config_path = excluded.config_path,
           status = 'active'",
        params![
            Uuid::new_v4().to_string(),
            resource_type_str,
            resource_id,
            agent_kind.as_str(),
            project_id,
            scope_type_str(scope_type),
            enabled,
            now,
            config_path
        ],
    )?;
    Ok(())
}

fn build_matrix(
    resources: &[ResourceRecord],
    resource_type: ResourceType,
    agents: &[AgentKind],
    project_id: &str,
) -> Vec<MatrixRow> {
    let mut grouped: BTreeMap<String, (String, Vec<&ResourceRecord>)> = BTreeMap::new();
    for resource in resources
        .iter()
        .filter(|resource| resource.resource_type == resource_type)
    {
        let key = resource
            .slug
            .clone()
            .unwrap_or_else(|| resource.name.clone())
            .trim()
            .to_ascii_lowercase();
        grouped
            .entry(key.clone())
            .or_insert_with(|| (resource.name.clone(), Vec::new()))
            .1
            .push(resource);
    }

    grouped
        .into_iter()
        .map(|(key, (name, group))| MatrixRow {
            key,
            name,
            resource_type,
            cells: agents
                .iter()
                .copied()
                .map(|agent_kind| build_cell(&group, agent_kind, project_id))
                .collect(),
        })
        .collect()
}

fn build_cell(
    resources: &[&ResourceRecord],
    agent_kind: AgentKind,
    project_id: &str,
) -> MatrixCell {
    let mut sources = Vec::new();
    for resource in resources {
        for binding in resource.bindings.iter().filter(|binding| {
            binding.agent_kind == agent_kind && binding.project_id.as_deref() == Some(project_id)
        }) {
            sources.push(MatrixSource {
                resource_id: resource.id.clone(),
                resource_name: resource.name.clone(),
                scope_type: binding.scope_type,
                project_id: binding.project_id.clone(),
                config_path: binding.config_path.clone(),
                source_path: resource.source_path.clone(),
                enabled: binding.enabled,
                status: binding.status.clone(),
            });
        }
    }
    let status = cell_status(&sources);
    MatrixCell {
        agent_kind,
        status,
        sources,
    }
}

fn cell_status(sources: &[MatrixSource]) -> String {
    if sources.is_empty() {
        return "unknown".into();
    }
    if sources
        .iter()
        .any(|source| source.status == "active" && source.enabled)
    {
        return "enabled".into();
    }
    if sources.iter().any(|source| source.status == "active") {
        return "disabled".into();
    }
    "missing".into()
}

fn build_pi_summary(resources: &[ResourceRecord], project_id: &str) -> PiResourceSummary {
    let mut summary = PiResourceSummary {
        total: 0,
        enabled: 0,
        disabled: 0,
        missing: 0,
        untrusted: 0,
        by_kind: Vec::new(),
    };
    let mut by_kind: BTreeMap<String, PiResourceKindSummary> = BTreeMap::new();

    for resource in resources
        .iter()
        .filter(|resource| resource.resource_type == ResourceType::PiResource)
    {
        let Some(kind) = pi_kind_from_payload(&resource.payload) else {
            continue;
        };
        let project_bindings = resource
            .bindings
            .iter()
            .filter(|binding| binding.project_id.as_deref() == Some(project_id))
            .collect::<Vec<_>>();
        if project_bindings.is_empty() {
            continue;
        }
        let is_missing = project_bindings
            .iter()
            .all(|binding| binding.status == "missing");
        let is_enabled = project_bindings
            .iter()
            .any(|binding| binding.status == "active" && binding.enabled);
        let is_untrusted = resource
            .payload
            .get("trusted")
            .and_then(|value| value.as_bool())
            .map(|trusted| !trusted)
            .unwrap_or(false);

        summary.total += 1;
        if is_missing {
            summary.missing += 1;
        } else if is_enabled {
            summary.enabled += 1;
        } else {
            summary.disabled += 1;
        }
        if is_untrusted {
            summary.untrusted += 1;
        }

        let key = pi_resource_kind_str(kind).to_string();
        let entry = by_kind.entry(key).or_insert(PiResourceKindSummary {
            resource_type: kind,
            total: 0,
            enabled: 0,
            disabled: 0,
            missing: 0,
            untrusted: 0,
        });
        entry.total += 1;
        if is_missing {
            entry.missing += 1;
        } else if is_enabled {
            entry.enabled += 1;
        } else {
            entry.disabled += 1;
        }
        if is_untrusted {
            entry.untrusted += 1;
        }
    }

    summary.by_kind = by_kind.into_values().collect();
    summary
}

fn scope_paths(scopes: &[ScopeLocation]) -> HashMap<ScopeType, String> {
    let mut paths = HashMap::new();
    for scope in scopes {
        paths
            .entry(scope.scope_type)
            .or_insert_with(|| scope.config_path.to_string_lossy().to_string());
    }
    paths
}

fn infer_scope(agent_kind: AgentKind, resource_id: &str, project_id: Option<&str>) -> ScopeType {
    let prefix = format!("{}:project:", agent_kind.as_str());
    if resource_id.starts_with(&prefix) {
        return ScopeType::Project;
    }
    let prefix = format!("{}:global:", agent_kind.as_str());
    if resource_id.starts_with(&prefix) {
        return ScopeType::Global;
    }
    if project_id.is_some() {
        ScopeType::Project
    } else {
        ScopeType::Global
    }
}

fn source_path_for(paths: &HashMap<ScopeType, String>, scope_type: ScopeType) -> Option<String> {
    paths.get(&scope_type).cloned()
}

fn parse_agent_kind(raw: &str) -> AgentKind {
    match raw {
        "claude-code" => AgentKind::ClaudeCode,
        "codex" => AgentKind::Codex,
        "opencode" => AgentKind::Opencode,
        "pi" => AgentKind::Pi,
        _ => AgentKind::Codex,
    }
}

fn parse_resource_type(raw: &str) -> ResourceType {
    match raw {
        "mcp" => ResourceType::Mcp,
        "skill" => ResourceType::Skill,
        "sub-agent" => ResourceType::SubAgent,
        "pi-resource" => ResourceType::PiResource,
        _ => ResourceType::Mcp,
    }
}

fn parse_scope_type(raw: &str) -> ScopeType {
    match raw {
        "project" => ScopeType::Project,
        _ => ScopeType::Global,
    }
}

fn resource_type_str(resource_type: ResourceType) -> &'static str {
    match resource_type {
        ResourceType::Mcp => "mcp",
        ResourceType::Skill => "skill",
        ResourceType::SubAgent => "sub-agent",
        ResourceType::PiResource => "pi-resource",
    }
}

fn scope_type_str(scope_type: ScopeType) -> &'static str {
    match scope_type {
        ScopeType::Global => "global",
        ScopeType::Project => "project",
    }
}

fn pi_kind_from_payload(payload: &JsonValue) -> Option<PiResourceKind> {
    let raw = payload.get("resourceType")?.as_str()?;
    Some(match raw {
        "skill" => PiResourceKind::Skill,
        "prompt-template" => PiResourceKind::PromptTemplate,
        "extension" => PiResourceKind::Extension,
        "package" => PiResourceKind::Package,
        "theme" => PiResourceKind::Theme,
        "setting" => PiResourceKind::Setting,
        _ => return None,
    })
}

fn pi_resource_kind_str(kind: PiResourceKind) -> &'static str {
    match kind {
        PiResourceKind::Skill => "skill",
        PiResourceKind::PromptTemplate => "prompt-template",
        PiResourceKind::Extension => "extension",
        PiResourceKind::Package => "package",
        PiResourceKind::Theme => "theme",
        PiResourceKind::Setting => "setting",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::ScanOutcome;
    use crate::domain::{McpServer, McpTransport, ScanSummary, Skill};

    fn service() -> (Arc<Database>, ResourceIndexer, ResourceService) {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, path, created_at, updated_at)
                 VALUES ('p1', 'Project One', '/tmp/project-one', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        });
        (
            db.clone(),
            ResourceIndexer::new(db.clone()),
            ResourceService::new(db),
        )
    }

    fn outcome(mcp_servers: Vec<McpServer>, skills: Vec<Skill>) -> ScanOutcome {
        ScanOutcome {
            agent_kind_str: AgentKind::Codex.as_str().to_string(),
            scopes: vec![ScopeLocation {
                scope_type: ScopeType::Project,
                config_path: "/tmp/project/.codex/config.toml".into(),
                writable: false,
            }],
            mcp_servers,
            skills,
            sub_agents: vec![],
            pi_resources: vec![],
            summary: ScanSummary::default(),
            errors: vec![],
        }
    }

    fn mcp(enabled: bool) -> McpServer {
        McpServer {
            id: "codex:project:github".into(),
            name: "github".into(),
            transport: McpTransport::Stdio,
            command: Some("github-mcp".into()),
            args: vec![],
            url: None,
            env_refs: vec!["GITHUB_TOKEN".into()],
            enabled,
        }
    }

    #[test]
    fn indexing_is_idempotent() {
        let (db, indexer, _resources) = service();
        indexer
            .index_scan(
                Some("p1"),
                AgentKind::Codex,
                &outcome(vec![mcp(true)], vec![]),
            )
            .unwrap();
        indexer
            .index_scan(
                Some("p1"),
                AgentKind::Codex,
                &outcome(vec![mcp(true)], vec![]),
            )
            .unwrap();

        let counts: (i64, i64) = db
            .with_conn(|conn| {
                Ok::<_, rusqlite::Error>((
                    conn.query_row("SELECT COUNT(*) FROM resources", [], |row| row.get(0))?,
                    conn.query_row("SELECT COUNT(*) FROM resource_bindings", [], |row| {
                        row.get(0)
                    })?,
                ))
            })
            .unwrap();
        assert_eq!(counts, (1, 1));
    }

    #[test]
    fn rescan_marks_missing_bindings() {
        let (_db, indexer, resources) = service();
        indexer
            .index_scan(
                Some("p1"),
                AgentKind::Codex,
                &outcome(vec![mcp(true)], vec![]),
            )
            .unwrap();
        indexer
            .index_scan(Some("p1"), AgentKind::Codex, &outcome(vec![], vec![]))
            .unwrap();

        let records = resources.list(Some(ResourceType::Mcp)).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].status, "missing");
        assert_eq!(records[0].bindings[0].status, "missing");
    }

    #[test]
    fn matrix_groups_by_name_and_exposes_cell_status() {
        let (_db, indexer, resources) = service();
        indexer
            .index_scan(
                Some("p1"),
                AgentKind::Codex,
                &outcome(vec![mcp(false)], vec![]),
            )
            .unwrap();
        let matrix = resources.project_matrix("p1").unwrap();
        assert_eq!(matrix.mcp_matrix.len(), 1);
        let codex_cell = matrix.mcp_matrix[0]
            .cells
            .iter()
            .find(|cell| cell.agent_kind == AgentKind::Codex)
            .unwrap();
        assert_eq!(codex_cell.status, "disabled");
        assert_eq!(codex_cell.sources.len(), 1);
    }
}
