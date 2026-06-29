//! Deterministic simulation boundary.
//!
//! Encounter graph and delivery simulation behavior begins in Goal 1.

#![forbid(unsafe_code)]

/// Returns the linked engine version for bootstrap verification.
#[must_use]
pub fn core_version() -> String {
    mesh_engine::version()
}
