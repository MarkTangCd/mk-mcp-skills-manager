// Cross-cutting services. Each submodule owns a focused responsibility and
// is consumed by Tauri commands or other services.

pub mod agents;
pub mod app_data;
pub mod doctor;
pub mod doctor_rules;
pub mod projects;
pub mod resources;
pub mod scan;

pub use agents::AgentService;
pub use app_data::{AppDataLayout, AppDataService};
pub use doctor::{DoctorError, DoctorResult, DoctorRule, DoctorService, IssueSummary, RawIssue, RuleContext};
pub use doctor_rules::{
    DangerousCommandRule, DisabledButReferencedRule, DuplicateMcpRule, MissingEnvRule,
    PlaintextSecretRule, PiDuplicatePackageRule, PiMissingPathRule, PiProjectOverrideRule,
    PiUntrustedExtensionRule, SkillBrokenPathRule, SkillMissingDescriptionRule,
    SkillMissingEntryRule, SkillUnusedRule, SubAgentMissingMcpRule, SubAgentMissingSkillRule,
    SubAgentNameConflictRule, SubAgentOverPermissionRule,
};
pub use projects::{ProjectError, ProjectService};
pub use resources::{
    MatrixCell, MatrixRow, MatrixSource, PiResourceKindSummary, PiResourceSummary, ProjectMatrix,
    ResourceBindingRecord, ResourceError, ResourceIndexer, ResourceRecord, ResourceService,
};
pub use scan::{ScanError, ScanRunReport, ScanService};
