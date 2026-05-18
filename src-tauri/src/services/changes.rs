// ChangeService: persistence and state-machine management for ChangePlan
// and ChangeSet.  The actual filesystem writes are handled by
// ChangeService::apply in T06-04; this module focuses on the plan
// lifecycle and database records.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde_json;
use thiserror::Error;

use crate::adapters::{AdapterRegistry, ScanContext, ChangeIntent as AdapterIntent};
use crate::db::{Database, DbError};
use crate::domain::{AgentKind, ChangeOperation, ChangePlan, ChangeSet, ChangeStatus, DiffSummary, ChangeIntent as DomainIntent, ResourceType, ScopeType};
use crate::security::PathGuard;
use crate::services::BackupService;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ChangeError {
    #[error("change set not found: {0}")]
    NotFound(String),
    #[error("invalid state transition from {from:?} to {to:?}")]
    InvalidTransition { from: ChangeStatus, to: ChangeStatus },
    #[error("plan has validation errors and cannot be confirmed")]
    ValidationFailed,
    #[error("path not allowed: {0}")]
    PathNotAllowed(String),
    #[error("backup failed: {0}")]
    BackupFailed(String),
    #[error("apply failed: {0}")]
    ApplyFailed(String),
    #[error("adapter error: {0}")]
    Adapter(String),
    #[error("db error: {0}")]
    Db(#[from] DbError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type ChangeResult<T> = Result<T, ChangeError>;

#[derive(Clone)]
pub struct ChangeService {
    db: Arc<Database>,
}

impl ChangeService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    // ------------------------------------------------------------------
    // Plan lifecycle
    // ------------------------------------------------------------------

    /// Persist a ChangePlan into the `change_sets` table.
    pub fn save_plan(&self, plan: &ChangePlan) -> ChangeResult<String> {
        let id = plan.id.clone();
        let now = Utc::now().to_rfc3339();
        let ops_json = serde_json::to_string(&plan.operations)?;
        let patches_json = serde_json::to_string(&plan.patches)?;
        let diff_json = serde_json::to_string(&plan.diff_summary)?;
        let target_files_json = serde_json::to_string(&plan.target_files)?;
        let risks_json = serde_json::to_string(&plan.risks)?;
        let validation_errors_json = serde_json::to_string(&plan.validation_errors)?;
        let intent_json = serde_json::to_string(&serde_json::json!({
            "intentId": plan.intent_id,
        }))?;

        self.db
            .with_conn(|c| -> rusqlite::Result<()> {
                c.execute(
                    "INSERT INTO change_sets (
                        id, status, operations_json, patches_json, diff_summary_json,
                        backup_id, project_id, agent_kind, created_at, updated_at,
                        intent_json, target_files_json, risks_json, validation_errors_json
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5,
                        ?6, ?7, ?8, ?9, ?10,
                        ?11, ?12, ?13, ?14
                    )
                    ON CONFLICT(id) DO UPDATE SET
                        status = excluded.status,
                        operations_json = excluded.operations_json,
                        patches_json = excluded.patches_json,
                        diff_summary_json = excluded.diff_summary_json,
                        backup_id = excluded.backup_id,
                        project_id = excluded.project_id,
                        agent_kind = excluded.agent_kind,
                        updated_at = excluded.updated_at,
                        intent_json = excluded.intent_json,
                        target_files_json = excluded.target_files_json,
                        risks_json = excluded.risks_json,
                        validation_errors_json = excluded.validation_errors_json",
                    params![
                        id,
                        plan.status.as_str(),
                        ops_json,
                        patches_json,
                        diff_json,
                        None::<String>,
                        None::<String>,
                        plan.agent_kind.map(|k| k.as_str()),
                        plan.created_at.clone(),
                        now,
                        intent_json,
                        target_files_json,
                        risks_json,
                        validation_errors_json,
                    ],
                )?;
                Ok(())
            })
            .map_err(DbError::from)?;
        Ok(id)
    }

    /// Load a ChangeSet by id and reconstruct a ChangePlan from it.
    pub fn get_plan(&self, id: &str) -> ChangeResult<ChangePlan> {
        let row = self
            .db
            .with_conn(|c| -> rusqlite::Result<Option<ChangePlanRow>> {
                c.query_row(
                    "SELECT
                        id, status, operations_json, patches_json, diff_summary_json,
                        backup_id, project_id, agent_kind, created_at, updated_at,
                        intent_json, target_files_json, risks_json, validation_errors_json
                     FROM change_sets WHERE id = ?1",
                    params![id],
                    |r| {
                        Ok(ChangePlanRow {
                            id: r.get(0)?,
                            status: r.get(1)?,
                            operations_json: r.get(2)?,
                            patches_json: r.get(3)?,
                            diff_summary_json: r.get(4)?,
                            backup_id: r.get(5)?,
                            project_id: r.get(6)?,
                            agent_kind: r.get(7)?,
                            created_at: r.get(8)?,
                            updated_at: r.get(9)?,
                            intent_json: r.get(10)?,
                            target_files_json: r.get(11)?,
                            risks_json: r.get(12)?,
                            validation_errors_json: r.get(13)?,
                        })
                    },
                )
                .optional()
            })
            .map_err(DbError::from)?;

        let row = row.ok_or_else(|| ChangeError::NotFound(id.to_string()))?;
        Self::row_to_plan(row)
    }

    /// List persisted change sets, ordered by newest first.
    pub fn list(&self) -> ChangeResult<Vec<ChangeSet>> {
        let rows = self
            .db
            .with_conn(|c| -> rusqlite::Result<Vec<ChangePlanRow>> {
                let mut stmt = c.prepare(
                    "SELECT
                        id, status, operations_json, patches_json, diff_summary_json,
                        backup_id, project_id, agent_kind, created_at, updated_at,
                        intent_json, target_files_json, risks_json, validation_errors_json
                     FROM change_sets ORDER BY updated_at DESC",
                )?;
                let iter = stmt.query_map([], |r| {
                    Ok(ChangePlanRow {
                        id: r.get(0)?,
                        status: r.get(1)?,
                        operations_json: r.get(2)?,
                        patches_json: r.get(3)?,
                        diff_summary_json: r.get(4)?,
                        backup_id: r.get(5)?,
                        project_id: r.get(6)?,
                        agent_kind: r.get(7)?,
                        created_at: r.get(8)?,
                        updated_at: r.get(9)?,
                        intent_json: r.get(10)?,
                        target_files_json: r.get(11)?,
                        risks_json: r.get(12)?,
                        validation_errors_json: r.get(13)?,
                    })
                })?;
                let mut out = Vec::new();
                for r in iter {
                    out.push(r?);
                }
                Ok(out)
            })
            .map_err(DbError::from)?;

        rows.into_iter().map(Self::row_to_set).collect()
    }

    /// Transition a persisted plan to a new status, enforcing the
    /// same state-machine rules as `ChangePlan::transition_to`.
    pub fn transition(&self, id: &str, to: ChangeStatus) -> ChangeResult<ChangePlan> {
        let mut plan = self.get_plan(id)?;

        // Enforce confirm guard: validation errors must be empty.
        if to == ChangeStatus::Confirmed && !plan.validation_errors.is_empty() {
            return Err(ChangeError::ValidationFailed);
        }

        plan.transition_to(to).map_err(|_msg| ChangeError::InvalidTransition {
            from: plan.status,
            to,
        })?;

        self.save_plan(&plan)?;
        Ok(plan)
    }

    // ------------------------------------------------------------------
    // Create plan from intent
    // ------------------------------------------------------------------

    /// Build a ChangePlan from a ChangeIntent by delegating to the
    /// appropriate agent adapter.  The plan is persisted with status
    /// `Draft` and returned so the UI can enter the preview flow.
    pub fn create_plan_from_intent(
        &self,
        intent: &DomainIntent,
        registry: &AdapterRegistry,
        ctx: &ScanContext,
    ) -> ChangeResult<ChangePlan> {
        let agent_kind = intent
            .agent_kind
            .ok_or_else(|| ChangeError::Adapter("agent_kind is required".to_string()))?;

        let adapter = registry
            .get(agent_kind)
            .ok_or_else(|| ChangeError::Adapter(format!("no adapter found for {:?}", agent_kind)))?;

        let resource_type = match intent.change_type.as_str() {
            "createMcp" | "updateMcp" | "deleteMcp" | "enableMcp" | "disableMcp" => {
                ResourceType::Mcp
            }
            "createSkill" | "updateSkill" | "deleteSkill" | "enableSkill" | "disableSkill" => {
                ResourceType::Skill
            }
            "createSubAgent" | "updateSubAgent" | "deleteSubAgent" | "enableSubAgent" | "disableSubAgent" => ResourceType::SubAgent,
            _ => {
                return Err(ChangeError::Adapter(format!(
                    "unsupported change_type: {}",
                    intent.change_type
                )));
            }
        };

        let adapter_intent = AdapterIntent {
            kind: intent.change_type.clone(),
            resource_type,
            target_scope: intent.scope_type.unwrap_or(ScopeType::Global),
            project_id: intent.project_id.clone(),
            payload: intent.payload.clone(),
        };

        let draft = adapter
            .build_change_plan(ctx, &adapter_intent)
            .map_err(|e| ChangeError::Adapter(e.to_string()))?;

        let mut files_changed = 0u32;
        let mut additions = 0u32;
        let mut deletions = 0u32;

        for patch in &draft.patches {
            files_changed += 1;
            for line in patch.diff.lines() {
                if line.starts_with('+') {
                    additions += 1;
                } else if line.starts_with('-') {
                    deletions += 1;
                }
            }
        }

        let plan = ChangePlan {
            id: Uuid::new_v4().to_string(),
            intent_id: intent.id.clone(),
            status: ChangeStatus::Draft,
            agent_kind: Some(agent_kind),
            target_files: draft
                .target_files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            operations: draft.operations,
            patches: draft.patches,
            diff_summary: DiffSummary {
                files_changed,
                additions,
                deletions,
            },
            risks: draft.warnings,
            validation_errors: vec![],
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        self.save_plan(&plan)?;
        Ok(plan)
    }

    // ------------------------------------------------------------------
    // Apply
    // ------------------------------------------------------------------

    /// Apply a confirmed change plan to the filesystem.
    ///
    /// Preconditions enforced before any write:
    ///   1. Plan status is `Confirmed`.
    ///   2. Every target file is within an allowed root (`PathGuard`).
    ///   3. A backup snapshot is created successfully.
    ///
    /// Writes use temp-file + atomic rename.  After each write the
    /// resulting file hash is compared against the patch `after_hash`
    /// when present.  The adapter's `validate_applied_change` is then
    /// invoked; on failure the plan transitions to `Failed` while the
    /// backup is preserved.
    pub fn apply(
        &self,
        plan_id: &str,
        backup_svc: &BackupService,
        guard: &PathGuard,
        registry: &AdapterRegistry,
    ) -> ChangeResult<ChangePlan> {
        let plan = self.get_plan(plan_id)?;

        if !plan.can_apply() {
            return Err(ChangeError::InvalidTransition {
                from: plan.status,
                to: ChangeStatus::Applied,
            });
        }

        // 1. Path guard check.
        for file in &plan.target_files {
            let path = PathBuf::from(file);
            guard
                .ensure_writable(&path)
                .map_err(|e| ChangeError::PathNotAllowed(e.to_string()))?;
        }

        // 2. Create backup.
        let target_paths: Vec<PathBuf> = plan.target_files.iter().map(PathBuf::from).collect();
        let backup = backup_svc
            .create(plan_id, &target_paths)
            .map_err(|e| ChangeError::BackupFailed(e.to_string()))?;

        // 3. Apply each operation using temp file + atomic rename.
        let mut write_results: Vec<(String, String)> = Vec::new();
        for op in &plan.operations {
            let result = Self::apply_operation(op);
            match result {
                Ok((path, hash)) => write_results.push((path, hash)),
                Err(e) => {
                    // Leave backup intact and mark plan as failed.
                    let mut failed_plan = plan.clone();
                    failed_plan.transition_to(ChangeStatus::Failed).ok();
                    failed_plan.updated_at = Utc::now().to_rfc3339();
                    let _ = self.save_plan(&failed_plan);
                    return Err(e);
                }
            }
        }

        // 4. Optional hash verification against patches.
        for patch in &plan.patches {
            if let Some(expected) = &patch.after_hash {
                if let Some((_, actual)) = write_results.iter().find(|(p, _)| p == &patch.path) {
                    if actual != expected {
                        let mut failed_plan = plan.clone();
                        failed_plan.transition_to(ChangeStatus::Failed).ok();
                        failed_plan.updated_at = Utc::now().to_rfc3339();
                        let _ = self.save_plan(&failed_plan);
                        return Err(ChangeError::ApplyFailed(format!(
                            "hash mismatch for {}: expected {expected}, got {actual}",
                            patch.path
                        )));
                    }
                }
            }
        }

        // 5. Adapter validation.
        if let Some(agent_kind) = plan.agent_kind {
            if let Some(adapter) = registry.get(agent_kind) {
                let ctx = ScanContext::empty();
                let validation = adapter
                    .validate_applied_change(&ctx)
                    .map_err(|e| ChangeError::ApplyFailed(e.to_string()))?;
                if !validation.ok {
                    let mut failed_plan = plan.clone();
                    failed_plan.transition_to(ChangeStatus::Failed).ok();
                    failed_plan.updated_at = Utc::now().to_rfc3339();
                    let _ = self.save_plan(&failed_plan);
                    return Err(ChangeError::ApplyFailed(
                        validation.messages.join("; "),
                    ));
                }
            }
        }

        // 6. Transition to Applied and record backup id in the DB row.
        self.transition(plan_id, ChangeStatus::Applied)?;
        let now = Utc::now().to_rfc3339();
        self.db.with_conn(|c| -> rusqlite::Result<()> {
            c.execute(
                "UPDATE change_sets SET backup_id = ?1, updated_at = ?2 WHERE id = ?3",
                params![&backup.id, &now, plan_id],
            )?;
            Ok(())
        }).map_err(DbError::from)?;

        self.get_plan(plan_id)
    }

    fn apply_operation(op: &ChangeOperation) -> ChangeResult<(String, String)> {
        let path = PathBuf::from(&op.target);
        let parent = path
            .parent()
            .ok_or_else(|| ChangeError::ApplyFailed(format!("no parent for {}", op.target)))?;
        std::fs::create_dir_all(parent)
            .map_err(|e| ChangeError::ApplyFailed(e.to_string()))?;

        let content = match op.kind.as_str() {
            "writeText" => op.payload.as_str().unwrap_or("").to_string(),
            "writeJson" => serde_json::to_string_pretty(&op.payload)
                .map_err(|e| ChangeError::ApplyFailed(e.to_string()))?,
            _ => {
                return Err(ChangeError::ApplyFailed(format!(
                    "unsupported operation kind: {}",
                    op.kind
                )));
            }
        };

        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, content).map_err(|e| ChangeError::ApplyFailed(e.to_string()))?;
        std::fs::rename(&temp_path, &path).map_err(|e| ChangeError::ApplyFailed(e.to_string()))?;

        let hash = crate::services::backup::file_sha256(&path)
            .map_err(|e| ChangeError::ApplyFailed(e.to_string()))?;
        Ok((op.target.clone(), hash))
    }

    // ------------------------------------------------------------------
    // Row helpers
    // ------------------------------------------------------------------

    fn row_to_plan(row: ChangePlanRow) -> ChangeResult<ChangePlan> {
        let status: ChangeStatus =
            serde_json::from_value(serde_json::Value::String(row.status))?;
        let operations = serde_json::from_str(&row.operations_json)?;
        let patches = serde_json::from_str(&row.patches_json)?;
        let diff_summary = serde_json::from_str(&row.diff_summary_json)?;
        let target_files = serde_json::from_str(&row.target_files_json)?;
        let risks = serde_json::from_str(&row.risks_json)?;
        let validation_errors = serde_json::from_str(&row.validation_errors_json)?;

        // Derive intent_id from the embedded intent_json blob.
        let intent_id: String = serde_json::from_str(&row.intent_json)
            .ok()
            .and_then(|v: serde_json::Value| v.get("intentId").and_then(|i| i.as_str().map(|s| s.to_string())))
            .unwrap_or_default();

        let agent_kind: Option<AgentKind> = row
            .agent_kind
            .as_ref()
            .and_then(|s| serde_json::from_value(serde_json::Value::String(s.clone())).ok());

        Ok(ChangePlan {
            id: row.id,
            intent_id,
            status,
            agent_kind,
            target_files,
            operations,
            patches,
            diff_summary,
            risks,
            validation_errors,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    fn row_to_set(row: ChangePlanRow) -> ChangeResult<ChangeSet> {
        let plan = Self::row_to_plan(row.clone())?;
        Ok(ChangeSet::from_plan(&plan, row.backup_id))
    }
}

// ------------------------------------------------------------------
// Internal row representation
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ChangePlanRow {
    id: String,
    status: String,
    operations_json: String,
    patches_json: String,
    diff_summary_json: String,
    backup_id: Option<String>,
    #[allow(dead_code)]
    project_id: Option<String>,
    agent_kind: Option<String>,
    created_at: String,
    updated_at: String,
    intent_json: String,
    target_files_json: String,
    risks_json: String,
    validation_errors_json: String,
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ChangeOperation, DiffSummary, FilePatch};

    fn svc() -> (Arc<Database>, ChangeService) {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let svc = ChangeService::new(db.clone());
        (db, svc)
    }

    fn dummy_plan(id: &str, intent_id: &str, status: ChangeStatus) -> ChangePlan {
        ChangePlan {
            id: id.into(),
            intent_id: intent_id.into(),
            status,
            agent_kind: None,
            target_files: vec!["test.json".into()],
            operations: vec![ChangeOperation {
                kind: "update".into(),
                target: "test.json".into(),
                payload: serde_json::json!({}),
            }],
            patches: vec![FilePatch {
                path: "test.json".into(),
                before_hash: None,
                after_hash: None,
                diff: "+line".into(),
            }],
            diff_summary: DiffSummary {
                files_changed: 1,
                additions: 1,
                deletions: 0,
            },
            risks: vec![],
            validation_errors: vec![],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn save_and_get_plan() {
        let (_, svc) = svc();
        let plan = dummy_plan("p1", "i1", ChangeStatus::Draft);
        svc.save_plan(&plan).unwrap();
        let got = svc.get_plan("p1").unwrap();
        assert_eq!(got.id, "p1");
        assert_eq!(got.intent_id, "i1");
        assert_eq!(got.status, ChangeStatus::Draft);
        assert_eq!(got.target_files, vec!["test.json"]);
    }

    #[test]
    fn list_returns_saved_sets() {
        let (_, svc) = svc();
        svc.save_plan(&dummy_plan("p2", "i2", ChangeStatus::Draft)).unwrap();
        let list = svc.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "p2");
    }

    #[test]
    fn transition_persists_new_status() {
        let (_, svc) = svc();
        svc.save_plan(&dummy_plan("p3", "i3", ChangeStatus::Draft))
            .unwrap();
        svc.transition("p3", ChangeStatus::Previewed).unwrap();
        let got = svc.get_plan("p3").unwrap();
        assert_eq!(got.status, ChangeStatus::Previewed);
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let (_, svc) = svc();
        svc.save_plan(&dummy_plan("p4", "i4", ChangeStatus::Draft))
            .unwrap();
        let err = svc.transition("p4", ChangeStatus::Applied).unwrap_err();
        assert!(matches!(err, ChangeError::InvalidTransition { .. }));
    }

    #[test]
    fn confirm_blocked_when_validation_errors_exist() {
        let (_, svc) = svc();
        let mut plan = dummy_plan("p5", "i5", ChangeStatus::Previewed);
        plan.validation_errors = vec!["missing field".into()];
        svc.save_plan(&plan).unwrap();
        let err = svc.transition("p5", ChangeStatus::Confirmed).unwrap_err();
        assert!(matches!(err, ChangeError::ValidationFailed));
    }

    #[test]
    fn apply_rejects_unconfirmed_plan() {
        let (_, svc) = svc();
        let dir = tempfile::tempdir().unwrap();
        let app_data = crate::services::AppDataService::initialize(dir.path()).unwrap();
        let backup = crate::services::BackupService::new(svc.db.clone(), &app_data);
        let guard = app_data.guard().clone();
        let reg = crate::adapters::AdapterRegistry::new();

        svc.save_plan(&dummy_plan("p6", "i6", ChangeStatus::Draft))
            .unwrap();
        let err = svc.apply("p6", &backup, &guard, &reg).unwrap_err();
        assert!(matches!(err, ChangeError::InvalidTransition { .. }));
    }

    #[test]
    fn apply_writes_file_and_transitions_to_applied() {
        let (_, svc) = svc();
        let dir = tempfile::tempdir().unwrap();
        let app_data = crate::services::AppDataService::initialize(dir.path()).unwrap();
        let backup = crate::services::BackupService::new(svc.db.clone(), &app_data);
        let guard = app_data.guard().clone();
        let reg = crate::adapters::AdapterRegistry::new();

        // Allow writes into the temp dir.
        let target = dir.path().join("config.json");
        let mut plan = dummy_plan("p7", "i7", ChangeStatus::Confirmed);
        plan.target_files = vec![target.to_string_lossy().to_string()];
        plan.operations = vec![ChangeOperation {
            kind: "writeText".into(),
            target: target.to_string_lossy().to_string(),
            payload: serde_json::json!("hello world"),
        }];
        plan.patches = vec![FilePatch {
            path: target.to_string_lossy().to_string(),
            before_hash: None,
            after_hash: None,
            diff: "+hello world".into(),
        }];
        svc.save_plan(&plan).unwrap();

        let applied = svc.apply("p7", &backup, &guard, &reg).unwrap();
        assert_eq!(applied.status, ChangeStatus::Applied);
        assert!(target.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello world");
    }

    #[test]
    fn apply_creates_backup() {
        let (_, svc) = svc();
        let dir = tempfile::tempdir().unwrap();
        let app_data = crate::services::AppDataService::initialize(dir.path()).unwrap();
        let backup = crate::services::BackupService::new(svc.db.clone(), &app_data);
        let guard = app_data.guard().clone();
        let reg = crate::adapters::AdapterRegistry::new();

        let target = dir.path().join("existing.txt");
        std::fs::write(&target, "before").unwrap();
        let mut plan = dummy_plan("p8", "i8", ChangeStatus::Confirmed);
        plan.target_files = vec![target.to_string_lossy().to_string()];
        plan.operations = vec![ChangeOperation {
            kind: "writeText".into(),
            target: target.to_string_lossy().to_string(),
            payload: serde_json::json!("after"),
        }];
        plan.patches = vec![FilePatch {
            path: target.to_string_lossy().to_string(),
            before_hash: None,
            after_hash: None,
            diff: "-before\n+after".into(),
        }];
        svc.save_plan(&plan).unwrap();

        let applied = svc.apply("p8", &backup, &guard, &reg).unwrap();
        assert_eq!(applied.status, ChangeStatus::Applied);

        // Read backup_id directly from the DB row.
        let backup_id: Option<String> = svc
            .db
            .with_conn(|c| {
                c.query_row(
                    "SELECT backup_id FROM change_sets WHERE id = ?1",
                    params!["p8"],
                    |r| r.get(0),
                )
                .optional()
                .unwrap()
            });
        assert!(backup_id.is_some());

        // Verify backup manifest exists.
        let manifest = backup.load_manifest(backup_id.as_ref().unwrap()).unwrap();
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].size, 6); // "before"
    }

    #[test]
    fn apply_hash_mismatch_marks_failed() {
        let (_, svc) = svc();
        let dir = tempfile::tempdir().unwrap();
        let app_data = crate::services::AppDataService::initialize(dir.path()).unwrap();
        let backup = crate::services::BackupService::new(svc.db.clone(), &app_data);
        let guard = app_data.guard().clone();
        let reg = crate::adapters::AdapterRegistry::new();

        let target = dir.path().join("hash.txt");
        let mut plan = dummy_plan("p9", "i9", ChangeStatus::Confirmed);
        plan.target_files = vec![target.to_string_lossy().to_string()];
        plan.operations = vec![ChangeOperation {
            kind: "writeText".into(),
            target: target.to_string_lossy().to_string(),
            payload: serde_json::json!("content"),
        }];
        plan.patches = vec![FilePatch {
            path: target.to_string_lossy().to_string(),
            before_hash: None,
            after_hash: Some("badhash".into()),
            diff: "+content".into(),
        }];
        svc.save_plan(&plan).unwrap();

        let err = svc.apply("p9", &backup, &guard, &reg).unwrap_err();
        assert!(matches!(err, ChangeError::ApplyFailed(_)));

        // Plan should be marked as failed.
        let failed = svc.get_plan("p9").unwrap();
        assert_eq!(failed.status, ChangeStatus::Failed);
    }

    #[test]
    fn create_plan_from_intent_builds_and_persists_plan() {
        let (_, svc) = svc();
        let mut reg = crate::adapters::AdapterRegistry::new();

        struct TestAdapter;

        impl crate::adapters::AgentAdapter for TestAdapter {
            fn kind(&self) -> AgentKind {
                AgentKind::Codex
            }

            fn detect_installation(
                &self,
                _ctx: &crate::adapters::ScanContext,
            ) -> crate::adapters::AdapterResult<crate::adapters::DetectionResult> {
                Ok(crate::adapters::DetectionResult {
                    installed: true,
                    version: None,
                    notes: vec![],
                })
            }

            fn locate_global_config(
                &self,
                _ctx: &crate::adapters::ScanContext,
            ) -> crate::adapters::AdapterResult<Option<crate::adapters::ScopeLocation>> {
                Ok(None)
            }

            fn locate_project_config(
                &self,
                _ctx: &crate::adapters::ScanContext,
            ) -> crate::adapters::AdapterResult<Option<crate::adapters::ScopeLocation>> {
                Ok(None)
            }

            fn scan(
                &self,
                _ctx: &crate::adapters::ScanContext,
            ) -> crate::adapters::AdapterResult<crate::adapters::ScanOutcome> {
                Ok(crate::adapters::ScanOutcome::default())
            }

            fn build_change_plan(
                &self,
                _ctx: &crate::adapters::ScanContext,
                _intent: &crate::adapters::ChangeIntent,
            ) -> crate::adapters::AdapterResult<crate::adapters::ChangePlanDraft> {
                Ok(crate::adapters::ChangePlanDraft {
                    operations: vec![ChangeOperation {
                        kind: "writeJson".into(),
                        target: "mcp.json".into(),
                        payload: serde_json::json!({"name": "test"}),
                    }],
                    target_files: vec![std::path::PathBuf::from("mcp.json")],
                    warnings: vec!["demo warning".into()],
                    patches: vec![FilePatch {
                        path: "mcp.json".into(),
                        before_hash: None,
                        after_hash: None,
                        diff: "-old\n+new".into(),
                    }],
                })
            }
        }

        reg.register(std::sync::Arc::new(TestAdapter));

        let intent = DomainIntent {
            id: "intent-1".into(),
            change_type: "createMcp".into(),
            agent_kind: Some(AgentKind::Codex),
            project_id: None,
            scope_type: Some(ScopeType::Global),
            resource_id: None,
            payload: serde_json::json!({"name": "test"}),
            created_at: Utc::now().to_rfc3339(),
        };

        let ctx = crate::adapters::ScanContext::empty();
        let plan = svc.create_plan_from_intent(&intent, &reg, &ctx).unwrap();
        assert_eq!(plan.status, ChangeStatus::Draft);
        assert_eq!(plan.intent_id, "intent-1");
        assert_eq!(plan.agent_kind, Some(AgentKind::Codex));
        assert_eq!(plan.target_files, vec!["mcp.json"]);
        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.risks, vec!["demo warning"]);
        assert!(plan.validation_errors.is_empty());
        assert_eq!(plan.patches.len(), 1);
        assert_eq!(plan.diff_summary.files_changed, 1);
        assert_eq!(plan.diff_summary.additions, 1);
        assert_eq!(plan.diff_summary.deletions, 1);

        // Should be persisted.
        let loaded = svc.get_plan(&plan.id).unwrap();
        assert_eq!(loaded.id, plan.id);
    }

    #[test]
    fn create_plan_from_intent_computes_diff_summary_for_multiple_patches() {
        let (_, svc) = svc();
        let mut reg = crate::adapters::AdapterRegistry::new();

        struct MultiPatchAdapter;

        impl crate::adapters::AgentAdapter for MultiPatchAdapter {
            fn kind(&self) -> AgentKind {
                AgentKind::Codex
            }

            fn detect_installation(
                &self,
                _ctx: &crate::adapters::ScanContext,
            ) -> crate::adapters::AdapterResult<crate::adapters::DetectionResult> {
                Ok(crate::adapters::DetectionResult {
                    installed: true,
                    version: None,
                    notes: vec![],
                })
            }

            fn locate_global_config(
                &self,
                _ctx: &crate::adapters::ScanContext,
            ) -> crate::adapters::AdapterResult<Option<crate::adapters::ScopeLocation>> {
                Ok(None)
            }

            fn locate_project_config(
                &self,
                _ctx: &crate::adapters::ScanContext,
            ) -> crate::adapters::AdapterResult<Option<crate::adapters::ScopeLocation>> {
                Ok(None)
            }

            fn scan(
                &self,
                _ctx: &crate::adapters::ScanContext,
            ) -> crate::adapters::AdapterResult<crate::adapters::ScanOutcome> {
                Ok(crate::adapters::ScanOutcome::default())
            }

            fn build_change_plan(
                &self,
                _ctx: &crate::adapters::ScanContext,
                _intent: &crate::adapters::ChangeIntent,
            ) -> crate::adapters::AdapterResult<crate::adapters::ChangePlanDraft> {
                Ok(crate::adapters::ChangePlanDraft {
                    operations: vec![],
                    target_files: vec![],
                    warnings: vec![],
                    patches: vec![
                        FilePatch {
                            path: "a.txt".into(),
                            before_hash: None,
                            after_hash: None,
                            diff: "+line1\n+line2".into(),
                        },
                        FilePatch {
                            path: "b.txt".into(),
                            before_hash: None,
                            after_hash: None,
                            diff: "-removed".into(),
                        },
                    ],
                })
            }
        }

        reg.register(std::sync::Arc::new(MultiPatchAdapter));

        let intent = DomainIntent {
            id: "intent-multi".into(),
            change_type: "createMcp".into(),
            agent_kind: Some(AgentKind::Codex),
            project_id: None,
            scope_type: Some(ScopeType::Global),
            resource_id: None,
            payload: serde_json::json!({}),
            created_at: Utc::now().to_rfc3339(),
        };

        let ctx = crate::adapters::ScanContext::empty();
        let plan = svc.create_plan_from_intent(&intent, &reg, &ctx).unwrap();
        assert_eq!(plan.diff_summary.files_changed, 2);
        assert_eq!(plan.diff_summary.additions, 2);
        assert_eq!(plan.diff_summary.deletions, 1);
        assert_eq!(plan.patches.len(), 2);
    }

    #[test]
    fn create_plan_from_intent_rejects_missing_agent_kind() {
        let (_, svc) = svc();
        let reg = crate::adapters::AdapterRegistry::new();

        let intent = DomainIntent {
            id: "intent-2".into(),
            change_type: "createMcp".into(),
            agent_kind: None,
            project_id: None,
            scope_type: Some(ScopeType::Global),
            resource_id: None,
            payload: serde_json::json!({}),
            created_at: Utc::now().to_rfc3339(),
        };

        let ctx = crate::adapters::ScanContext::empty();
        let err = svc.create_plan_from_intent(&intent, &reg, &ctx).unwrap_err();
        assert!(matches!(err, ChangeError::Adapter(_)));
    }

    #[test]
    fn create_plan_from_intent_rejects_missing_adapter() {
        let (_, svc) = svc();
        let reg = crate::adapters::AdapterRegistry::new();

        let intent = DomainIntent {
            id: "intent-3".into(),
            change_type: "createMcp".into(),
            agent_kind: Some(AgentKind::ClaudeCode),
            project_id: None,
            scope_type: Some(ScopeType::Global),
            resource_id: None,
            payload: serde_json::json!({}),
            created_at: Utc::now().to_rfc3339(),
        };

        let ctx = crate::adapters::ScanContext::empty();
        let err = svc.create_plan_from_intent(&intent, &reg, &ctx).unwrap_err();
        assert!(matches!(err, ChangeError::Adapter(_)));
    }
}
