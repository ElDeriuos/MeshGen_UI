// State module — re-exports all state types.

pub mod easymesh;
pub mod project;
pub mod structured;

// These two are consumed via `crate::state::{EasyMeshState, StructuredState}`
// in ui/validation.rs.  All other types are imported directly from their
// sub-modules by the rest of the codebase.
pub use easymesh::EasyMeshState;
pub use structured::StructuredState;
