// Agent adapter surface.
//
// Adapters are strictly responsible for: detection, configuration scan,
// building change plans and post-write validation. Adapters never write
// files directly — all writes flow through ChangeService.

pub mod claude_code;
pub mod codex;
pub mod common;
pub mod mock;
pub mod opencode;
pub mod pi;
pub mod registry;
pub mod traits;

pub use claude_code::ClaudeCodeAdapter;
pub use codex::CodexAdapter;
pub use mock::MockAdapter;
pub use opencode::OpencodeAdapter;
pub use pi::PiAdapter;
pub use registry::AdapterRegistry;
pub use traits::{
    AdapterError, AdapterResult, AgentAdapter, ChangeIntent, ChangePlanDraft, DetectionResult,
    DoctorReport, ScanContext, ScanOutcome, ScopeLocation, ValidationOutcome,
};
