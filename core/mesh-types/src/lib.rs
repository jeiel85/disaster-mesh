//! Shared DisasterMesh domain type boundary.
//!
//! Concrete protocol types are introduced only after Goal 0.5 freezes the
//! normative contracts.

#![forbid(unsafe_code)]

/// Identifies this bootstrap crate without defining protocol behavior.
pub const CRATE_NAME: &str = "mesh-types";

#[cfg(test)]
mod tests {
    #[test]
    fn crate_boundary_is_named() {
        assert_eq!(super::CRATE_NAME, "mesh-types");
    }
}
