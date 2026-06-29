//! DisasterMesh identity, endpoint security, and local secret handling.

#![forbid(unsafe_code)]

use core::fmt;

pub const CRATE_NAME: &str = "mesh-crypto";

pub mod contact;
pub mod dme;
pub mod identity;
pub mod local_envelope;
pub mod replay;

pub use contact::*;
pub use dme::*;
pub use identity::*;
pub use local_envelope::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CryptoError {
    RandomFailure,
    InvalidKey,
    InvalidSignature,
    InvalidCiphertext,
    InvalidContactCard,
    InvalidQr,
    InvalidDisplayName,
    InvalidCapabilities,
    InvalidField,
    SizeLimit,
    RecipientMismatch,
    AadMismatch,
    UnsupportedVersion,
    CorruptEncryptedValue,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "security operation failed: {self:?}")
    }
}

impl std::error::Error for CryptoError {}

#[must_use]
pub const fn type_boundary() -> &'static str {
    mesh_types::CRATE_NAME
}
