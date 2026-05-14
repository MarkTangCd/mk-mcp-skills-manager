use std::sync::Arc;

use crate::adapters::{AdapterRegistry, MockAdapter};
use crate::db::Database;
use crate::domain::AgentKind;
use crate::services::{AppDataService, ProjectService, ScanService};

/// Application state shared across Tauri commands.
#[derive(Clone)]
pub struct AppState {
    pub app_data: AppDataService,
    pub db: Arc<Database>,
    pub registry: Arc<AdapterRegistry>,
    pub projects: ProjectService,
    pub scans: ScanService,
}

impl AppState {
    pub fn new(app_data: AppDataService, db: Database) -> Self {
        let db = Arc::new(db);
        // Phase 2 wires only MockAdapters. Real per-agent adapters land in
        // Phase 3 and replace these entries by re-registering on the same key.
        let mut reg = AdapterRegistry::new();
        for kind in [
            AgentKind::ClaudeCode,
            AgentKind::Codex,
            AgentKind::Opencode,
            AgentKind::Pi,
        ] {
            reg.register(Arc::new(MockAdapter::new(kind)));
        }
        let registry = Arc::new(reg);
        let projects = ProjectService::new(db.clone());
        let scans = ScanService::new(db.clone(), registry.clone());
        Self {
            app_data,
            db,
            registry,
            projects,
            scans,
        }
    }
}
