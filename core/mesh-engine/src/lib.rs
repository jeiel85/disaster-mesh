//! Deterministic engine boundary.
//!
//! Goal 0 exposes only build metadata. Engine behavior begins in later goals.

#![forbid(unsafe_code)]

/// Returns the Rust core package version embedded at compile time.
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

pub mod control;

/// Names the lower-level boundaries used by the future engine.
#[must_use]
pub const fn lower_boundaries() -> [&'static str; 3] {
    [
        mesh_types::CRATE_NAME,
        mesh_routing::CRATE_NAME,
        mesh_store::CRATE_NAME,
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_matches_workspace_package() {
        assert_eq!(super::version(), env!("CARGO_PKG_VERSION"));
    }
}
