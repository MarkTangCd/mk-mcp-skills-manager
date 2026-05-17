use std::sync::Arc;

use crate::adapters::{
    AdapterRegistry, ClaudeCodeAdapter, CodexAdapter, OpencodeAdapter, PiAdapter,
};
use crate::db::Database;
use crate::services::{AgentService, AppDataService, DoctorService, ProjectService, ResourceService, ScanService};

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
    pub doctor: DoctorService,
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
        let doctor = DoctorService::new(db.clone(), resources.clone()).with_rules(vec![
            // MCP rules
            Arc::new(crate::services::DuplicateMcpRule),
            Arc::new(crate::services::MissingEnvRule),
            Arc::new(crate::services::PlaintextSecretRule),
            Arc::new(crate::services::DangerousCommandRule),
            Arc::new(crate::services::DisabledButReferencedRule),
            // Skill rules
            Arc::new(crate::services::SkillMissingDescriptionRule),
            Arc::new(crate::services::SkillMissingEntryRule),
            Arc::new(crate::services::SkillBrokenPathRule),
            Arc::new(crate::services::SkillUnusedRule),
            // Sub-agent rules
            Arc::new(crate::services::SubAgentNameConflictRule),
            Arc::new(crate::services::SubAgentMissingMcpRule),
            Arc::new(crate::services::SubAgentMissingSkillRule),
            Arc::new(crate::services::SubAgentOverPermissionRule),
            // Pi rules
            Arc::new(crate::services::PiMissingPathRule),
            Arc::new(crate::services::PiDuplicatePackageRule),
            Arc::new(crate::services::PiUntrustedExtensionRule),
            Arc::new(crate::services::PiProjectOverrideRule),
        ]);
        Self {
            app_data,
            db,
            registry,
            agents,
            projects,
            resources,
            scans,
            doctor,
        }
    }
}
