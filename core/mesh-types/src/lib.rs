//! Shared DisasterMesh domain types and generated normative constants.

#![forbid(unsafe_code)]

/// Identifies this bootstrap crate without defining protocol behavior.
pub const CRATE_NAME: &str = "mesh-types";

pub mod domain;
pub mod generated_contracts;

pub use domain::*;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_boundary_is_named() {
        assert_eq!(super::CRATE_NAME, "mesh-types");
    }

    #[test]
    fn generated_constants_match_contract_sentinels() {
        use super::generated_contracts::{protocol, state_codes};

        assert_eq!(protocol::PROTOCOL_MAJOR, 1);
        assert_eq!(protocol::BLE_WIRE_OUTER_MAGIC, 216);
        assert_eq!(protocol::REPLAY_WINDOW_BITS, 4096);
        assert_eq!(state_codes::TOKEN_GRANT_STATE_UNCERTAIN, 1);
        assert_eq!(state_codes::VERSION, 1);
    }
}
