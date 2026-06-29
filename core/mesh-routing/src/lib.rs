//! Routing decision boundary.
//!
//! No forwarding or queue policy is implemented in Goal 0.

#![forbid(unsafe_code)]

/// Identifies this bootstrap crate without defining routing behavior.
pub const CRATE_NAME: &str = "mesh-routing";

/// Confirms the dependency direction toward the bundle layer.
#[must_use]
pub const fn bundle_boundary() -> &'static str {
    mesh_bundle::CRATE_NAME
}
