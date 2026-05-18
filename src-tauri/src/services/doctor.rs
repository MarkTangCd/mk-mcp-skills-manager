// DoctorService: runs read-only health checks against indexed resources
// and persists findings to `doctor_issues`.
//
// Rules are intentionally read-only. Auto-fix is out of scope for Phase 5.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use chrono::Utc;
use rusqlite::params;
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};
use crate::domain::{AgentKind, DoctorIssue, DoctorTargetRef, IssueSeverity};
use crate::services::resources::{ResourceError, ResourceRecord, ResourceService};

#[derive(Debug, Error)]
pub enum DoctorError {
    #[error("db error: {0}")]
    Db(#[from] DbError),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("resource error: {0}")]
    Resource(#[from] ResourceError),
}

pub type DoctorResult<T> = Result<T, DoctorError>;

/// Context passed to every rule during a doctor run.
pub struct RuleContext {
    pub db: Arc<Database>,
    pub resources: Vec<ResourceRecord>,
    pub project_id: Option<String>,
    pub agent_kinds: Vec<AgentKind>,
}

impl RuleContext {
    pub fn resources_by_type(
        &self,
        resource_type: crate::domain::ResourceType,
    ) -> Vec<&ResourceRecord> {
        self.resources
            .iter()
            .filter(|r| r.resource_type == resource_type)
            .collect()
    }

    pub fn resources_for_agent(&self, agent_kind: AgentKind) -> Vec<&ResourceRecord> {
        self.resources
            .iter()
            .filter(|r| r.agent_kind == Some(agent_kind))
            .collect()
    }
}

/// A single read-only doctor rule.
pub trait DoctorRule: Send + Sync {
    fn name(&self) -> &'static str;
    fn category(&self) -> &'static str;
    fn check(&self, ctx: &RuleContext) -> Vec<RawIssue>;
}

/// Pre-serialization issue emitted by a rule.
#[derive(Debug, Clone)]
pub struct RawIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub target_ref: Option<DoctorTargetRef>,
    pub fixable: bool,
}

#[derive(Clone)]
pub struct DoctorService {
    db: Arc<Database>,
    resources: ResourceService,
    rules: Vec<Arc<dyn DoctorRule>>,
}

impl DoctorService {
    pub fn new(db: Arc<Database>, resources: ResourceService) -> Self {
        let rules: Vec<Arc<dyn DoctorRule>> = vec![];
        Self {
            db,
            resources,
            rules,
        }
    }

    pub fn with_rules(mut self, rules: Vec<Arc<dyn DoctorRule>>) -> Self {
        self.rules = rules;
        self
    }

    /// Run all registered rules against every project, plus global state.
    pub fn run_all(&self) -> DoctorResult<Vec<DoctorIssue>> {
        let projects = self.list_projects()?;
        let mut all_issues = Vec::new();
        for project in projects {
            let issues = self.run_for_project(Some(&project.id))?;
            all_issues.extend(issues);
        }
        // Run global checks (no project_id)
        let global_issues = self.run_for_project(None)?;
        all_issues.extend(global_issues);
        Ok(all_issues)
    }

    /// Run rules scoped to a single project (or global when `project_id` is None).
    pub fn run_for_project(&self, project_id: Option<&str>) -> DoctorResult<Vec<DoctorIssue>> {
        let now = Utc::now().to_rfc3339();
        let resources = self.resources.list(None)?;
        let ctx = RuleContext {
            db: self.db.clone(),
            resources,
            project_id: project_id.map(|s| s.to_string()),
            agent_kinds: vec![
                AgentKind::ClaudeCode,
                AgentKind::Codex,
                AgentKind::Opencode,
                AgentKind::Pi,
            ],
        };

        let mut raw_issues: Vec<RawIssue> = Vec::new();
        for rule in &self.rules {
            raw_issues.extend(rule.check(&ctx));
        }

        // Deduplicate by content hash before persisting.
        let mut seen = std::collections::HashSet::new();
        let mut issues: Vec<DoctorIssue> = Vec::new();
        for raw in raw_issues {
            let key = issue_key(&raw, project_id);
            if seen.insert(key) {
                issues.push(DoctorIssue {
                    id: Uuid::new_v4().to_string(),
                    severity: raw.severity,
                    category: "general".into(),
                    message: raw.message,
                    target_ref: raw.target_ref,
                    fixable: raw.fixable,
                });
            }
        }

        // Mark existing unresolved issues for this scope as resolved, then insert new ones.
        self.db.with_conn_mut(|conn| -> DoctorResult<()> {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE doctor_issues
                 SET resolved = 1, updated_at = ?1
                 WHERE resolved = 0
                   AND ((?2 IS NULL AND project_id IS NULL)
                     OR (?2 IS NOT NULL AND project_id = ?2))",
                params![&now, project_id],
            )?;

            for issue in &issues {
                let target_json = issue
                    .target_ref
                    .as_ref()
                    .map(|t| serde_json::to_string(t).unwrap_or_default())
                    .unwrap_or_default();
                let agent_kind_str = issue
                    .target_ref
                    .as_ref()
                    .and_then(|t| t.agent_kind)
                    .map(|k| k.as_str().to_string());
                tx.execute(
                    "INSERT INTO doctor_issues
                     (id, severity, category, message, target_ref_json, project_id, agent_kind, fixable, resolved, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?9)",
                    params![
                        &issue.id,
                        severity_str(issue.severity),
                        &issue.category,
                        &issue.message,
                        target_json,
                        project_id,
                        agent_kind_str,
                        if issue.fixable { 1 } else { 0 },
                        &now
                    ],
                )?;
            }
            tx.commit()?;
            Ok(())
        })?;

        Ok(issues)
    }

    /// List open (unresolved) issues with optional filters.
    pub fn list_issues(
        &self,
        severity: Option<IssueSeverity>,
        category: Option<&str>,
        project_id: Option<&str>,
    ) -> DoctorResult<Vec<DoctorIssue>> {
        let rows = self
            .db
            .with_conn(|c| -> rusqlite::Result<Vec<DoctorIssue>> {
                let sql = "SELECT id, severity, category, message, target_ref_json, fixable
                     FROM doctor_issues
                     WHERE resolved = 0
                       AND (?1 IS NULL OR severity = ?1)
                       AND (?2 IS NULL OR category = ?2)
                       AND (?3 IS NULL OR project_id = ?3)
                     ORDER BY
                       CASE severity
                         WHEN 'critical' THEN 1
                         WHEN 'warning' THEN 2
                         WHEN 'info' THEN 3
                         ELSE 4
                       END,
                       created_at DESC";
                let mut stmt = c.prepare(sql)?;
                let iter = stmt.query_map(
                    params![severity.map(severity_str), category, project_id],
                    |row| {
                        let severity_raw: String = row.get(1)?;
                        let target_json: Option<String> = row.get(4)?;
                        let target_ref = target_json.and_then(|s| serde_json::from_str(&s).ok());
                        Ok(DoctorIssue {
                            id: row.get(0)?,
                            severity: parse_severity(&severity_raw),
                            category: row.get(2)?,
                            message: row.get(3)?,
                            target_ref,
                            fixable: row.get::<_, i64>(5)? != 0,
                        })
                    },
                )?;
                let mut out = Vec::new();
                for r in iter {
                    out.push(r?);
                }
                Ok(out)
            })
            .map_err(DbError::from)?;
        Ok(rows)
    }

    /// Return a summary of open issues grouped by severity.
    pub fn issue_summary(&self) -> DoctorResult<IssueSummary> {
        let (critical, warning, info) = self
            .db
            .with_conn(|c| -> rusqlite::Result<(u32, u32, u32)> {
                let mut critical = 0u32;
                let mut warning = 0u32;
                let mut info = 0u32;
                let mut stmt = c.prepare(
                    "SELECT severity, COUNT(*) FROM doctor_issues WHERE resolved = 0 GROUP BY severity",
                )?;
                let rows = stmt.query_map([], |row| {
                    let sev: String = row.get(0)?;
                    let count: i64 = row.get(1)?;
                    Ok((sev, count))
                })?;
                for r in rows {
                    let (sev, count) = r?;
                    let count = count as u32;
                    match sev.as_str() {
                        "critical" => critical = count,
                        "warning" => warning = count,
                        "info" => info = count,
                        _ => {}
                    }
                }
                Ok((critical, warning, info))
            })
            .map_err(DbError::from)?;
        Ok(IssueSummary {
            critical,
            warning,
            info,
            total: critical + warning + info,
        })
    }

    fn list_projects(&self) -> DoctorResult<Vec<crate::domain::Project>> {
        let rows = self
            .db
            .with_conn(|c| -> rusqlite::Result<Vec<crate::domain::Project>> {
                let mut stmt = c.prepare(
                    "SELECT id, name, path, created_at, updated_at FROM projects ORDER BY name",
                )?;
                let iter = stmt.query_map([], |r| {
                    Ok(crate::domain::Project {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        path: r.get(2)?,
                        created_at: r.get(3)?,
                        updated_at: r.get(4)?,
                    })
                })?;
                let mut out = Vec::new();
                for p in iter {
                    out.push(p?);
                }
                Ok(out)
            })
            .map_err(DbError::from)?;
        Ok(rows)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueSummary {
    pub total: u32,
    pub critical: u32,
    pub warning: u32,
    pub info: u32,
}

fn severity_str(s: IssueSeverity) -> &'static str {
    match s {
        IssueSeverity::Info => "info",
        IssueSeverity::Warning => "warning",
        IssueSeverity::Critical => "critical",
    }
}

fn parse_severity(raw: &str) -> IssueSeverity {
    match raw {
        "critical" => IssueSeverity::Critical,
        "warning" => IssueSeverity::Warning,
        _ => IssueSeverity::Info,
    }
}

fn issue_key(raw: &RawIssue, project_id: Option<&str>) -> u64 {
    let mut hasher = DefaultHasher::new();
    raw.message.hash(&mut hasher);
    project_id.hash(&mut hasher);
    if let Some(target) = &raw.target_ref {
        target.resource_id.hash(&mut hasher);
        target.agent_kind.map(|k| k.as_str()).hash(&mut hasher);
        target.project_id.hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_and_svc() -> (Arc<Database>, DoctorService) {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let resources = ResourceService::new(db.clone());
        let svc = DoctorService::new(db.clone(), resources);
        (db, svc)
    }

    #[test]
    fn empty_run_produces_no_issues() {
        let (_, svc) = db_and_svc();
        let issues = svc.run_for_project(None).unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn issues_are_persisted_and_listed() {
        let (db, svc) = db_and_svc();

        struct DummyRule;
        impl DoctorRule for DummyRule {
            fn name(&self) -> &'static str {
                "dummy"
            }
            fn category(&self) -> &'static str {
                "agent"
            }
            fn check(&self, _ctx: &RuleContext) -> Vec<RawIssue> {
                vec![RawIssue {
                    severity: IssueSeverity::Warning,
                    message: "Test warning".into(),
                    target_ref: None,
                    fixable: false,
                }]
            }
        }

        let svc = svc.with_rules(vec![Arc::new(DummyRule)]);
        let issues = svc.run_for_project(None).unwrap();
        assert_eq!(issues.len(), 1);

        let listed = svc.list_issues(None, None, None).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].message, "Test warning");

        let count: i64 = db
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM doctor_issues", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn subsequent_run_refreshes_issues() {
        let (_, svc) = db_and_svc();

        struct DummyRule;
        impl DoctorRule for DummyRule {
            fn name(&self) -> &'static str {
                "dummy"
            }
            fn category(&self) -> &'static str {
                "agent"
            }
            fn check(&self, _ctx: &RuleContext) -> Vec<RawIssue> {
                vec![RawIssue {
                    severity: IssueSeverity::Critical,
                    message: "A".into(),
                    target_ref: None,
                    fixable: false,
                }]
            }
        }

        let svc = svc.with_rules(vec![Arc::new(DummyRule)]);
        svc.run_for_project(None).unwrap();
        let first = svc.list_issues(None, None, None).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].message, "A");

        // Re-running should resolve old and insert new (same dedup key -> no duplicate)
        svc.run_for_project(None).unwrap();
        let second = svc.list_issues(None, None, None).unwrap();
        assert_eq!(second.len(), 1);

        // Verify that resolved count is 1 (the old one was marked resolved)
        let resolved: i64 = svc
            .db
            .with_conn(|c| {
                c.query_row(
                    "SELECT COUNT(*) FROM doctor_issues WHERE resolved = 1",
                    [],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(resolved, 1);
    }

    #[test]
    fn severity_filter_works() {
        let (_, svc) = db_and_svc();

        struct MultiRule;
        impl DoctorRule for MultiRule {
            fn name(&self) -> &'static str {
                "multi"
            }
            fn category(&self) -> &'static str {
                "mcp"
            }
            fn check(&self, _ctx: &RuleContext) -> Vec<RawIssue> {
                vec![
                    RawIssue {
                        severity: IssueSeverity::Critical,
                        message: "C1".into(),
                        target_ref: None,
                        fixable: false,
                    },
                    RawIssue {
                        severity: IssueSeverity::Info,
                        message: "I1".into(),
                        target_ref: None,
                        fixable: false,
                    },
                ]
            }
        }

        let svc = svc.with_rules(vec![Arc::new(MultiRule)]);
        svc.run_for_project(None).unwrap();

        let critical_only = svc
            .list_issues(Some(IssueSeverity::Critical), None, None)
            .unwrap();
        assert_eq!(critical_only.len(), 1);
        assert_eq!(critical_only[0].message, "C1");
    }

    #[test]
    fn issue_summary_aggregates_correctly() {
        let (_, svc) = db_and_svc();

        struct MultiRule;
        impl DoctorRule for MultiRule {
            fn name(&self) -> &'static str {
                "multi"
            }
            fn category(&self) -> &'static str {
                "mcp"
            }
            fn check(&self, _ctx: &RuleContext) -> Vec<RawIssue> {
                vec![
                    RawIssue {
                        severity: IssueSeverity::Critical,
                        message: "C1".into(),
                        target_ref: None,
                        fixable: false,
                    },
                    RawIssue {
                        severity: IssueSeverity::Critical,
                        message: "C2".into(),
                        target_ref: None,
                        fixable: false,
                    },
                    RawIssue {
                        severity: IssueSeverity::Warning,
                        message: "W1".into(),
                        target_ref: None,
                        fixable: false,
                    },
                ]
            }
        }

        let svc = svc.with_rules(vec![Arc::new(MultiRule)]);
        svc.run_for_project(None).unwrap();

        let summary = svc.issue_summary().unwrap();
        assert_eq!(summary.critical, 2);
        assert_eq!(summary.warning, 1);
        assert_eq!(summary.info, 0);
        assert_eq!(summary.total, 3);
    }
}
