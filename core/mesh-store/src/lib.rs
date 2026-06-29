//! Persistence boundary.
//!
//! SQLite integration and migrations begin only after contract freeze.

#![forbid(unsafe_code)]

/// Identifies this bootstrap crate without defining persistence behavior.
pub const CRATE_NAME: &str = "mesh-store";

/// Confirms the dependency direction toward the bundle layer.
#[must_use]
pub const fn bundle_boundary() -> &'static str {
    mesh_bundle::CRATE_NAME
}
