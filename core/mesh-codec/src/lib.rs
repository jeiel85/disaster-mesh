//! Deterministic encoding and wire-codec boundary.

#![forbid(unsafe_code)]

/// Identifies this bootstrap crate without defining codec behavior.
pub const CRATE_NAME: &str = "mesh-codec";

pub mod ble;

/// Confirms the dependency direction toward `mesh-types`.
#[must_use]
pub const fn type_boundary() -> &'static str {
    mesh_types::CRATE_NAME
}
