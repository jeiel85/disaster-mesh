//! Deterministic encoding and wire-codec boundary.

#![forbid(unsafe_code)]

/// Identifies this bootstrap crate without defining codec behavior.
pub const CRATE_NAME: &str = "mesh-codec";

pub mod base32;
pub mod ble;
pub mod cbor;
pub mod control;

pub use cbor::{
    CborError, CborValue, DecodeLimits, decode_deterministic, decode_deterministic_prefix,
    encode_deterministic,
};

/// Confirms the dependency direction toward `mesh-types`.
#[must_use]
pub const fn type_boundary() -> &'static str {
    mesh_types::CRATE_NAME
}
