//! Cryptography adapter boundary and security-state pure models.

#![forbid(unsafe_code)]

/// Identifies this bootstrap crate without defining cryptographic behavior.
pub const CRATE_NAME: &str = "mesh-crypto";

pub mod replay;

/// Confirms the dependency direction toward `mesh-types`.
#[must_use]
pub const fn type_boundary() -> &'static str {
    mesh_types::CRATE_NAME
}
