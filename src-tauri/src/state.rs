use std::sync::Arc;

use crate::adapters::{
    AdapterRegistry, ClaudeCodeAdapter, CodexAdapter, OpencodeAdapter, PiAdapter,
};
use crate::db::Database;
use crate::services::{AgentService, AppDataService, ProjectService, ResourceService, ScanService};

/// Application state shared across Tauri commands.
#[derive(Clone)]
pub struct AppState {
    pub app_data: AppDataService,
    pub db: Arc<Database>,
    pub registry: Arc<AdapterRegistry>,
    pub agents: AgentService,
    pub projects: ProjectService,
    pub resources: ResourceService,
    pub scans: ScanService,
}

impl AppState {
    pub fn new(app_data: AppDataService, db: Database) -> Self {
        let db = Arc::new(db);
        let mut reg = AdapterRegistry::new();
        reg.register(Arc::new(ClaudeCodeAdapter::new()));
        reg.register(Arc::new(CodexAdapter::new()));
        reg.register(Arc::new(OpencodeAdapter::new()));
        reg.register(Arc::new(PiAdapter::new()));
        let registry = Arc::new(reg);
        let agents = AgentService::new(registry.clone());
        let projects = ProjectService::new(db.clone());
        let resources = ResourceService::new(db.clone());
        let scans = ScanService::new(db.clone(), registry.clone());
        Self {
            app_data,
            db,
            registry,
            agents,
            projects,
            resources,
            scans,
        }
    }
}
