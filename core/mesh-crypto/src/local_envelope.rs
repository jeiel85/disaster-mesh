//! DMEV v1 local encrypted-value envelope.

use core::fmt;

use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use hkdf::Hkdf;
use mesh_codec::{CborValue, encode_deterministic};
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::CryptoError;
use crate::identity::random_array;

const MAGIC: &[u8; 4] = b"DMEV";
const VERSION: u8 = 1;
const HEADER_BYTES: usize = 4 + 1 + 2 + 24;

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DbMasterKey([u8; 32]);

impl fmt::Debug for DbMasterKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DbMasterKey([REDACTED])")
    }
}

impl DbMasterKey {
    pub fn generate() -> Result<Self, CryptoError> {
        Ok(Self(random_array()?))
    }

    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColumnContext<'a> {
    pub schema_version: u64,
    pub table: &'a str,
    pub column: &'a str,
    pub primary_key: &'a [u8],
    pub key_version: u16,
    pub identity_hash: &'a [u8; 32],
}

pub fn encrypt_local_value(
    master_key: &DbMasterKey,
    context: ColumnContext<'_>,
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if context.table.is_empty() || context.column.is_empty() || context.key_version == 0 {
        return Err(CryptoError::InvalidField);
    }
    let key = derive_column_key(master_key, context)?;
    let cipher =
        XChaCha20Poly1305::new_from_slice(&key[..]).map_err(|_| CryptoError::InvalidKey)?;
    let nonce = random_array::<24>()?;
    let aad = local_aad(context)?;
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::CorruptEncryptedValue)?;
    let mut envelope = Vec::with_capacity(HEADER_BYTES + ciphertext.len());
    envelope.extend_from_slice(MAGIC);
    envelope.push(VERSION);
    envelope.extend_from_slice(&context.key_version.to_be_bytes());
    envelope.extend_from_slice(&nonce);
    envelope.extend(ciphertext);
    Ok(envelope)
}

pub fn decrypt_local_value(
    master_key: &DbMasterKey,
    context: ColumnContext<'_>,
    envelope: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    if envelope.len() < HEADER_BYTES + 16 || &envelope[..4] != MAGIC || envelope[4] != VERSION {
        return Err(CryptoError::CorruptEncryptedValue);
    }
    let key_version = u16::from_be_bytes([envelope[5], envelope[6]]);
    if key_version != context.key_version {
        return Err(CryptoError::CorruptEncryptedValue);
    }
    let nonce: &[u8; 24] = envelope[7..31]
        .try_into()
        .map_err(|_| CryptoError::CorruptEncryptedValue)?;
    let key = derive_column_key(master_key, context)?;
    let cipher =
        XChaCha20Poly1305::new_from_slice(&key[..]).map_err(|_| CryptoError::InvalidKey)?;
    let aad = local_aad(context)?;
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: &envelope[HEADER_BYTES..],
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::CorruptEncryptedValue)?;
    Ok(Zeroizing::new(plaintext))
}

fn derive_column_key(
    master_key: &DbMasterKey,
    context: ColumnContext<'_>,
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    let mut info = Vec::with_capacity(32 + context.table.len() + context.column.len());
    info.extend_from_slice(b"DisasterMesh/DB/1");
    info.extend_from_slice(context.table.as_bytes());
    info.push(0);
    info.extend_from_slice(context.column.as_bytes());
    info.push(0);
    info.extend_from_slice(&context.key_version.to_be_bytes());
    let hkdf = Hkdf::<Sha256>::new(Some(context.identity_hash), master_key.as_bytes());
    let mut key = Zeroizing::new([0; 32]);
    hkdf.expand(&info, key.as_mut())
        .map_err(|_| CryptoError::InvalidKey)?;
    Ok(key)
}

fn local_aad(context: ColumnContext<'_>) -> Result<Vec<u8>, CryptoError> {
    encode_deterministic(&CborValue::Array(vec![
        CborValue::Unsigned(context.schema_version),
        CborValue::Text(context.table.to_owned()),
        CborValue::Text(context.column.to_owned()),
        CborValue::Bytes(context.primary_key.to_vec()),
        CborValue::Unsigned(u64::from(context.key_version)),
    ]))
    .map_err(|_| CryptoError::InvalidField)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context<'a>(primary_key: &'a [u8], column: &'a str) -> ColumnContext<'a> {
        ColumnContext {
            schema_version: 1,
            table: "local_messages",
            column,
            primary_key,
            key_version: 1,
            identity_hash: &[7; 32],
        }
    }

    #[test]
    fn envelope_round_trip_and_header_are_exact() {
        let key = DbMasterKey::from_bytes([1; 32]);
        let encrypted =
            encrypt_local_value(&key, context(&[2; 16], "encrypted_body"), b"secret").unwrap();
        assert_eq!(&encrypted[..4], b"DMEV");
        assert_eq!(encrypted[4], 1);
        assert_eq!(&encrypted[5..7], &[0, 1]);
        assert_eq!(
            decrypt_local_value(&key, context(&[2; 16], "encrypted_body"), &encrypted)
                .unwrap()
                .as_slice(),
            b"secret"
        );
    }

    #[test]
    fn row_and_column_swaps_fail_closed() {
        let key = DbMasterKey::from_bytes([1; 32]);
        let encrypted =
            encrypt_local_value(&key, context(&[2; 16], "encrypted_body"), b"secret").unwrap();
        assert_eq!(
            decrypt_local_value(&key, context(&[3; 16], "encrypted_body"), &encrypted),
            Err(CryptoError::CorruptEncryptedValue)
        );
        assert_eq!(
            decrypt_local_value(&key, context(&[2; 16], "other_column"), &encrypted),
            Err(CryptoError::CorruptEncryptedValue)
        );
    }

    #[test]
    fn master_key_debug_is_redacted() {
        let key = DbMasterKey::from_bytes([42; 32]);
        assert_eq!(format!("{key:?}"), "DbMasterKey([REDACTED])");
    }
}
