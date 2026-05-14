// Agent adapter surface.
//
// Adapters are strictly responsible for: detection, configuration scan,
// building change plans and post-write validation. Adapters never write
// files directly — all writes flow through ChangeService.

pub mod mock;
pub mod registry;
pub mod traits;

pub use mock::MockAdapter;
pub use registry::AdapterRegistry;
pub use traits::{
    AdapterError, AdapterResult, AgentAdapter, ChangeIntent, ChangePlanDraft, DetectionResult,
    DoctorReport, ScanContext, ScanOutcome, ScopeLocation, ValidationOutcome,
};
