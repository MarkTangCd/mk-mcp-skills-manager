// Cross-cutting services. Each submodule owns a focused responsibility and
// is consumed by Tauri commands or other services.

pub mod agents;
pub mod app_data;
pub mod projects;
pub mod scan;

pub use agents::AgentService;
pub use app_data::{AppDataLayout, AppDataService};
pub use projects::{ProjectError, ProjectService};
pub use scan::{ScanError, ScanRunReport, ScanService};
