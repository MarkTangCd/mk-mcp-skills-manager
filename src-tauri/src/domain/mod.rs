// Core domain model for AgentHub Local.
//
// All structs use camelCase via serde so they serialize cleanly into the
// TypeScript surface that the frontend consumes through Tauri commands.

pub mod enums;
pub mod models;

pub use enums::*;
pub use models::*;
