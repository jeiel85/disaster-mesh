//! Separate Ed25519 identity, X25519 HPKE, and X25519 Noise static keys.

use core::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use mesh_types::{IdentityId, RoutingSlot};
use rand_core::{OsRng, TryRngCore};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::CryptoError;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct IdentityPublic {
    pub identity_id: IdentityId,
    pub signing_public_key: [u8; 32],
    pub hpke_public_key: [u8; 32],
    pub noise_public_key: [u8; 32],
    pub inbound_routing_slot: RoutingSlot,
    pub key_version: u32,
}

impl fmt::Debug for IdentityPublic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IdentityPublic")
            .field("key_version", &self.key_version)
            .field("key_material", &"[REDACTED]")
            .finish()
    }
}

#[derive(ZeroizeOnDrop)]
pub struct IdentitySecrets {
    signing: SigningKey,
    hpke: StaticSecret,
    noise: StaticSecret,
}

impl fmt::Debug for IdentitySecrets {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IdentitySecrets([REDACTED])")
    }
}

pub struct Identity {
    public: IdentityPublic,
    secrets: IdentitySecrets,
}

impl fmt::Debug for Identity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Identity")
            .field("public", &self.public)
            .field("secrets", &"[REDACTED]")
            .finish()
    }
}

impl Identity {
    pub fn generate() -> Result<Self, CryptoError> {
        let mut signing_seed = random_array()?;
        let mut hpke_seed = random_array()?;
        let mut noise_seed = random_array()?;
        let routing_slot = RoutingSlot::from(random_array()?);
        let result =
            Self::from_private_material(signing_seed, hpke_seed, noise_seed, routing_slot, 1);
        signing_seed.zeroize();
        hpke_seed.zeroize();
        noise_seed.zeroize();
        result
    }

    pub fn from_private_material(
        signing_seed: [u8; 32],
        hpke_seed: [u8; 32],
        noise_seed: [u8; 32],
        inbound_routing_slot: RoutingSlot,
        key_version: u32,
    ) -> Result<Self, CryptoError> {
        let signing_seed = Zeroizing::new(signing_seed);
        let hpke_seed = Zeroizing::new(hpke_seed);
        let noise_seed = Zeroizing::new(noise_seed);
        if key_version == 0 {
            return Err(CryptoError::InvalidKey);
        }
        let signing = SigningKey::from_bytes(&signing_seed);
        let hpke = StaticSecret::from(*hpke_seed);
        let noise = StaticSecret::from(*noise_seed);
        let signing_public_key = signing.verifying_key().to_bytes();
        let hpke_public_key = X25519PublicKey::from(&hpke).to_bytes();
        let noise_public_key = X25519PublicKey::from(&noise).to_bytes();
        let identity_id = identity_id_from_signing_public(&signing_public_key);
        Ok(Self {
            public: IdentityPublic {
                identity_id,
                signing_public_key,
                hpke_public_key,
                noise_public_key,
                inbound_routing_slot,
                key_version,
            },
            secrets: IdentitySecrets {
                signing,
                hpke,
                noise,
            },
        })
    }

    #[must_use]
    pub const fn public(&self) -> &IdentityPublic {
        &self.public
    }

    #[must_use]
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.secrets.signing.sign(message).to_bytes()
    }

    #[must_use]
    pub fn private_material(&self) -> Zeroizing<Vec<u8>> {
        let mut material = Zeroizing::new(Vec::with_capacity(96));
        material.extend_from_slice(&self.secrets.signing.to_bytes());
        material.extend_from_slice(&self.secrets.hpke.to_bytes());
        material.extend_from_slice(&self.secrets.noise.to_bytes());
        material
    }

    #[must_use]
    pub(crate) const fn hpke_secret(&self) -> &StaticSecret {
        &self.secrets.hpke
    }

    #[must_use]
    pub(crate) fn noise_secret_bytes(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.secrets.noise.to_bytes())
    }
}

#[must_use]
pub fn identity_id_from_signing_public(signing_public_key: &[u8; 32]) -> IdentityId {
    IdentityId::from(sha256(signing_public_key))
}

pub fn verify_signature(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> Result<(), CryptoError> {
    let key = VerifyingKey::from_bytes(public_key).map_err(|_| CryptoError::InvalidKey)?;
    let signature = Signature::from_bytes(signature);
    key.verify_strict(message, &signature)
        .map_err(|_| CryptoError::InvalidSignature)
}

pub(crate) fn random_array<const N: usize>() -> Result<[u8; N], CryptoError> {
    let mut output = [0; N];
    OsRng
        .try_fill_bytes(&mut output)
        .map_err(|_| CryptoError::RandomFailure)?;
    Ok(output)
}

pub(crate) fn sha256(input: &[u8]) -> [u8; 32] {
    Sha256::digest(input).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separate_keys_are_generated_and_signatures_verify() {
        let identity = Identity::generate().unwrap();
        assert_ne!(
            identity.public().hpke_public_key,
            identity.public().noise_public_key
        );
        let signature = identity.sign(b"message");
        verify_signature(
            &identity.public().signing_public_key,
            b"message",
            &signature,
        )
        .unwrap();
        assert_eq!(
            verify_signature(
                &identity.public().signing_public_key,
                b"tampered",
                &signature,
            ),
            Err(CryptoError::InvalidSignature)
        );
    }

    #[test]
    fn secret_debug_is_redacted() {
        let identity = Identity::from_private_material(
            [42; 32],
            [43; 32],
            [44; 32],
            RoutingSlot::from([45; 16]),
            1,
        )
        .unwrap();
        let debug = format!("{identity:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("42, 42"));
    }
}
