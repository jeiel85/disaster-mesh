//! Bundle profile boundary.
//!
//! BPv7 and DME behavior is deferred until the normative contracts are frozen.

#![forbid(unsafe_code)]

/// Identifies this bootstrap crate without defining bundle behavior.
pub const CRATE_NAME: &str = "mesh-bundle";

/// Names the lower-level boundaries used by future bundle code.
#[must_use]
pub const fn lower_boundaries() -> [&'static str; 3] {
    [
        mesh_types::CRATE_NAME,
        mesh_codec::CRATE_NAME,
        mesh_crypto::CRATE_NAME,
    ]
}
