// Tauri command surface. Commands are grouped by domain area; each module
// exports `register` so `lib.rs` only wires up the handler list.

pub mod agents;
pub mod app;
pub mod backups;
pub mod changes;
pub mod doctor;
pub mod projects;
pub mod prompts;

pub use agents::*;
pub use app::*;
pub use backups::*;
pub use changes::*;
pub use doctor::*;
pub use projects::*;
pub use prompts::*;
