// Cross-cutting services. Each submodule owns a focused responsibility and
// is consumed by Tauri commands or other services.

pub mod app_data;

pub use app_data::{AppDataLayout, AppDataService};
