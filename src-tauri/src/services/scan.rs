// ScanService: drives `AgentAdapter::scan` across the registry and
// persists a per-run ScanSnapshot to SQLite.
//
// Errors from individual adapters are isolated — one adapter's failure
// must never prevent other adapters from completing. Each adapter run
// produces its own `ScanSnapshot` row keyed by `(project_id, agent_kind)`.

use std::sync::Arc;

use chrono::Utc;
use rusqlite::params;
use thiserror::Error;
use uuid::Uuid;

use crate::adapters::{AdapterRegistry, ScanContext};
use crate::db::{Database, DbError};
use crate::domain::{AgentKind, ScanSnapshot, ScanSummary};

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("db error: {0}")]
    Db(#[from] DbError),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type ScanResult<T> = Result<T, ScanError>;

#[derive(Debug, Clone)]
pub struct ScanRunReport {
    pub snapshots: Vec<ScanSnapshot>,
    pub adapter_errors: Vec<(AgentKind, String)>,
}

#[derive(Clone)]
pub struct ScanService {
    db: Arc<Database>,
    registry: Arc<AdapterRegistry>,
}

impl ScanService {
    pub fn new(db: Arc<Database>, registry: Arc<AdapterRegistry>) -> Self {
        Self { db, registry }
    }

    pub fn registry(&self) -> &AdapterRegistry {
        &self.registry
    }

    /// Run every registered adapter against `ctx` and persist snapshots.
    /// Adapter-level errors are captured per-adapter; they do not abort
    /// the run for sibling adapters.
    pub fn run(&self, project_id: Option<&str>, ctx: &ScanContext) -> ScanResult<ScanRunReport> {
        let mut snapshots = Vec::new();
        let mut adapter_errors = Vec::new();
        let now = Utc::now().to_rfc3339();

        for adapter in self.registry.all() {
            let kind = adapter.kind();
            let (summary, errors) = match adapter.scan(ctx) {
                Ok(outcome) => {
                    let mut s = outcome.summary;
                    if s.total_resources == 0 {
                        s.total_resources =
                            s.mcp_count + s.skill_count + s.sub_agent_count + s.pi_resource_count;
                    }
                    let errors = outcome.errors;
                    (s, errors)
                }
                Err(err) => {
                    let msg = err.to_string();
                    adapter_errors.push((kind, msg.clone()));
                    (
                        ScanSummary {
                            total_resources: 0,
                            mcp_count: 0,
                            skill_count: 0,
                            sub_agent_count: 0,
                            pi_resource_count: 0,
                            errors: vec![msg.clone()],
                        },
                        vec![msg],
                    )
                }
            };

            let snapshot = ScanSnapshot {
                id: Uuid::new_v4().to_string(),
                project_id: project_id.map(|s| s.to_string()),
                agent_kind: Some(kind),
                summary: ScanSummary {
                    errors: errors.clone(),
                    ..summary
                },
                created_at: now.clone(),
            };

            self.persist_snapshot(&snapshot)?;
            snapshots.push(snapshot);
        }

        Ok(ScanRunReport {
            snapshots,
            adapter_errors,
        })
    }

    fn persist_snapshot(&self, snapshot: &ScanSnapshot) -> ScanResult<()> {
        let summary_json = serde_json::to_string(&snapshot.summary)?;
        let agent_kind_str = snapshot.agent_kind.map(|k| k.as_str().to_string());
        self.db
            .with_conn(|c| -> rusqlite::Result<()> {
                c.execute(
                    "INSERT INTO scan_snapshots (id, project_id, agent_kind, summary_json, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        snapshot.id,
                        snapshot.project_id,
                        agent_kind_str,
                        summary_json,
                        snapshot.created_at,
                    ],
                )?;
                Ok(())
            })
            .map_err(DbError::from)?;
        Ok(())
    }

    /// Return the latest snapshot per agent for the given project (or
    /// global scans when `project_id` is `None`).
    pub fn latest_snapshots(&self, project_id: Option<&str>) -> ScanResult<Vec<ScanSnapshot>> {
        let rows = self
            .db
            .with_conn(|c| -> rusqlite::Result<Vec<ScanSnapshot>> {
                let mut stmt = c.prepare(
                    "SELECT s.id, s.project_id, s.agent_kind, s.summary_json, s.created_at
                     FROM scan_snapshots s
                     JOIN (
                       SELECT agent_kind, MAX(created_at) AS max_created
                       FROM scan_snapshots
                       WHERE (?1 IS NULL AND project_id IS NULL)
                          OR (?1 IS NOT NULL AND project_id = ?1)
                       GROUP BY agent_kind
                     ) latest
                     ON s.agent_kind IS latest.agent_kind
                     AND s.created_at = latest.max_created
                     WHERE (?1 IS NULL AND s.project_id IS NULL)
                        OR (?1 IS NOT NULL AND s.project_id = ?1)
                     ORDER BY s.agent_kind",
                )?;
                let iter = stmt.query_map(params![project_id], |row| {
                    let id: String = row.get(0)?;
                    let pid: Option<String> = row.get(1)?;
                    let kind_str: Option<String> = row.get(2)?;
                    let summary_json: String = row.get(3)?;
                    let created_at: String = row.get(4)?;
                    Ok((id, pid, kind_str, summary_json, created_at))
                })?;
                let mut out = Vec::new();
                for r in iter {
                    let (id, pid, kind_str, summary_json, created_at) = r?;
                    let summary: ScanSummary =
                        serde_json::from_str(&summary_json).unwrap_or(ScanSummary {
                            total_resources: 0,
                            mcp_count: 0,
                            skill_count: 0,
                            sub_agent_count: 0,
                            pi_resource_count: 0,
                            errors: vec!["corrupt summary".into()],
                        });
                    let agent_kind = kind_str.as_deref().and_then(parse_kind);
                    out.push(ScanSnapshot {
                        id,
                        project_id: pid,
                        agent_kind,
                        summary,
                        created_at,
                    });
                }
                Ok(out)
            })
            .map_err(DbError::from)?;
        Ok(rows)
    }
}

fn parse_kind(s: &str) -> Option<AgentKind> {
    match s {
        "claude-code" => Some(AgentKind::ClaudeCode),
        "codex" => Some(AgentKind::Codex),
        "opencode" => Some(AgentKind::Opencode),
        "pi" => Some(AgentKind::Pi),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::MockAdapter;

    fn registry_with(
        adapters: Vec<Arc<dyn crate::adapters::AgentAdapter>>,
    ) -> Arc<AdapterRegistry> {
        let mut reg = AdapterRegistry::new();
        for a in adapters {
            reg.register(a);
        }
        Arc::new(reg)
    }

    #[test]
    fn run_writes_snapshot_per_adapter() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let reg = registry_with(vec![
            Arc::new(MockAdapter::new(AgentKind::Codex)),
            Arc::new(MockAdapter::new(AgentKind::Opencode)),
        ]);
        let svc = ScanService::new(db.clone(), reg);
        let report = svc.run(None, &ScanContext::empty()).unwrap();
        assert_eq!(report.snapshots.len(), 2);
        assert!(report.adapter_errors.is_empty());
        let count: i64 = db
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM scan_snapshots", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn one_failing_adapter_does_not_block_others() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let reg = registry_with(vec![
            Arc::new(MockAdapter::failing_scan(AgentKind::Codex)),
            Arc::new(MockAdapter::new(AgentKind::Opencode)),
        ]);
        let svc = ScanService::new(db.clone(), reg);
        let report = svc.run(None, &ScanContext::empty()).unwrap();
        assert_eq!(report.snapshots.len(), 2);
        assert_eq!(report.adapter_errors.len(), 1);
        assert_eq!(report.adapter_errors[0].0, AgentKind::Codex);
        let failed_snap = report
            .snapshots
            .iter()
            .find(|s| s.agent_kind == Some(AgentKind::Codex))
            .unwrap();
        assert!(!failed_snap.summary.errors.is_empty());
    }

    #[test]
    fn latest_snapshots_returns_one_per_agent() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let reg = registry_with(vec![Arc::new(MockAdapter::new(AgentKind::Codex))]);
        let svc = ScanService::new(db, reg);
        svc.run(None, &ScanContext::empty()).unwrap();
        svc.run(None, &ScanContext::empty()).unwrap();
        let latest = svc.latest_snapshots(None).unwrap();
        assert_eq!(latest.len(), 1);
    }
}
