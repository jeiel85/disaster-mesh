//! The only foreign-function facade exported by DisasterMesh.
//!
//! Goal 0 deliberately exposes only [`version`].

#![forbid(unsafe_code)]

/// Returns the Rust core package version.
#[uniffi::export]
#[must_use]
pub fn version() -> String {
    mesh_engine::version()
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    #[test]
    fn facade_version_matches_package() {
        assert_eq!(super::version(), env!("CARGO_PKG_VERSION"));
    }
}
