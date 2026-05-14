// Cross-cutting services. Each submodule owns a focused responsibility and
// is consumed by Tauri commands or other services.

pub mod agents;
pub mod app_data;
pub mod projects;
pub mod resources;
pub mod scan;

pub use agents::AgentService;
pub use app_data::{AppDataLayout, AppDataService};
pub use projects::{ProjectError, ProjectService};
pub use resources::{
    MatrixCell, MatrixRow, MatrixSource, PiResourceKindSummary, PiResourceSummary, ProjectMatrix,
    ResourceBindingRecord, ResourceError, ResourceIndexer, ResourceRecord, ResourceService,
};
pub use scan::{ScanError, ScanRunReport, ScanService};
