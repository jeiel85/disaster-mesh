//! Signed contact cards, QR framing, display IDs, and trust transitions.

use unicode_normalization::UnicodeNormalization;

use mesh_codec::{CborValue, DecodeLimits, decode_deterministic, encode_deterministic};
use mesh_types::RoutingSlot;

use crate::CryptoError;
use crate::identity::{Identity, sha256, verify_signature};

const CONTACT_DOMAIN: &[u8] = b"DisasterMesh/CONTACT/1";
const SAFETY_DOMAIN: &[u8] = b"DisasterMesh/SAFETY/1";
const BASE45: &[u8; 45] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactCard {
    pub signing_public_key: [u8; 32],
    pub hpke_public_key: [u8; 32],
    pub inbound_routing_slot: RoutingSlot,
    pub display_name: String,
    pub key_version: u32,
    pub capabilities: u32,
    pub signature: [u8; 64],
}

impl ContactCard {
    pub fn create(
        identity: &Identity,
        display_name: &str,
        capabilities: u32,
    ) -> Result<Self, CryptoError> {
        let display_name = validate_display_name(display_name)?;
        if u64::from(capabilities)
            & !mesh_types::generated_contracts::protocol::CAPABILITIES_KNOWN_MASK
            != 0
        {
            return Err(CryptoError::InvalidCapabilities);
        }
        let public = identity.public();
        let mut card = Self {
            signing_public_key: public.signing_public_key,
            hpke_public_key: public.hpke_public_key,
            inbound_routing_slot: public.inbound_routing_slot,
            display_name,
            key_version: public.key_version,
            capabilities,
            signature: [0; 64],
        };
        let signature_input = card.signature_input()?;
        card.signature = identity.sign(&signature_input);
        Ok(card)
    }

    pub fn encode(&self) -> Result<Vec<u8>, CryptoError> {
        self.validate_fields()?;
        encode_deterministic(&CborValue::Array(vec![
            CborValue::Unsigned(1),
            CborValue::Bytes(self.signing_public_key.to_vec()),
            CborValue::Bytes(self.hpke_public_key.to_vec()),
            CborValue::Bytes(self.inbound_routing_slot.as_bytes().to_vec()),
            CborValue::Text(self.display_name.clone()),
            CborValue::Unsigned(u64::from(self.key_version)),
            CborValue::Unsigned(u64::from(self.capabilities)),
            CborValue::Bytes(self.signature.to_vec()),
        ]))
        .map_err(|_| CryptoError::InvalidContactCard)
    }

    pub fn decode(input: &[u8]) -> Result<Self, CryptoError> {
        if input.len() > 512 {
            return Err(CryptoError::SizeLimit);
        }
        let value = decode_deterministic(input, DecodeLimits::default())
            .map_err(|_| CryptoError::InvalidContactCard)?;
        let values = expect_array(value, 8)?;
        if expect_u64(&values[0])? != 1 {
            return Err(CryptoError::UnsupportedVersion);
        }
        let card = Self {
            signing_public_key: fixed::<32>(&values[1])?,
            hpke_public_key: fixed::<32>(&values[2])?,
            inbound_routing_slot: RoutingSlot::from(fixed::<16>(&values[3])?),
            display_name: expect_text(&values[4])?.to_owned(),
            key_version: u32::try_from(expect_u64(&values[5])?)
                .map_err(|_| CryptoError::InvalidContactCard)?,
            capabilities: u32::try_from(expect_u64(&values[6])?)
                .map_err(|_| CryptoError::InvalidContactCard)?,
            signature: fixed::<64>(&values[7])?,
        };
        card.validate_fields()?;
        verify_signature(
            &card.signing_public_key,
            &card.signature_input()?,
            &card.signature,
        )?;
        Ok(card)
    }

    pub fn to_qr(&self) -> Result<String, CryptoError> {
        let bytes = self.encode()?;
        let qr = format!(
            "DM1:{}~{:08x}",
            base45_encode(&bytes),
            crc32c::crc32c(&bytes)
        );
        if qr.len() > 512 || !qr.is_ascii() {
            return Err(CryptoError::SizeLimit);
        }
        Ok(qr)
    }

    pub fn from_qr(qr: &str) -> Result<Self, CryptoError> {
        if qr.len() > 512 || !qr.is_ascii() {
            return Err(CryptoError::InvalidQr);
        }
        let payload = qr.strip_prefix("DM1:").ok_or(CryptoError::InvalidQr)?;
        let mut parts = payload.split('~');
        let encoded = parts.next().ok_or(CryptoError::InvalidQr)?;
        let checksum = parts.next().ok_or(CryptoError::InvalidQr)?;
        if parts.next().is_some()
            || checksum.len() != 8
            || checksum
                .bytes()
                .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
        {
            return Err(CryptoError::InvalidQr);
        }
        let expected = u32::from_str_radix(checksum, 16).map_err(|_| CryptoError::InvalidQr)?;
        let bytes = base45_decode(encoded)?;
        if crc32c::crc32c(&bytes) != expected {
            return Err(CryptoError::InvalidQr);
        }
        Self::decode(&bytes)
    }

    #[must_use]
    pub fn display_id(&self) -> String {
        display_id(&self.signing_public_key)
    }

    fn unsigned_value(&self) -> CborValue {
        CborValue::Array(vec![
            CborValue::Unsigned(1),
            CborValue::Bytes(self.signing_public_key.to_vec()),
            CborValue::Bytes(self.hpke_public_key.to_vec()),
            CborValue::Bytes(self.inbound_routing_slot.as_bytes().to_vec()),
            CborValue::Text(self.display_name.clone()),
            CborValue::Unsigned(u64::from(self.key_version)),
            CborValue::Unsigned(u64::from(self.capabilities)),
        ])
    }

    fn signature_input(&self) -> Result<Vec<u8>, CryptoError> {
        let unsigned = encode_deterministic(&self.unsigned_value())
            .map_err(|_| CryptoError::InvalidContactCard)?;
        let mut input = Vec::with_capacity(CONTACT_DOMAIN.len() + 32);
        input.extend_from_slice(CONTACT_DOMAIN);
        input.extend_from_slice(&sha256(&unsigned));
        Ok(input)
    }

    fn validate_fields(&self) -> Result<(), CryptoError> {
        if self.key_version == 0 {
            return Err(CryptoError::InvalidContactCard);
        }
        if u64::from(self.capabilities)
            & !mesh_types::generated_contracts::protocol::CAPABILITIES_KNOWN_MASK
            != 0
        {
            return Err(CryptoError::InvalidCapabilities);
        }
        if validate_display_name(&self.display_name)? != self.display_name {
            return Err(CryptoError::InvalidDisplayName);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactTrustState {
    Unverified,
    VerifiedInPerson,
    KeyChanged,
    Revoked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactRecord {
    pub card: ContactCard,
    pub trust: ContactTrustState,
}

impl ContactRecord {
    #[must_use]
    pub fn import(card: ContactCard) -> Self {
        Self {
            card,
            trust: ContactTrustState::Unverified,
        }
    }

    pub fn verify_in_person(
        &mut self,
        local_signing_public: &[u8; 32],
        displayed: &str,
    ) -> Result<(), CryptoError> {
        if self.trust == ContactTrustState::Revoked
            || safety_number(local_signing_public, &self.card.signing_public_key) != displayed
        {
            return Err(CryptoError::InvalidField);
        }
        self.trust = ContactTrustState::VerifiedInPerson;
        Ok(())
    }

    pub fn apply_card(&mut self, replacement: ContactCard) -> Result<(), CryptoError> {
        let replacement = ContactCard::decode(&replacement.encode()?)?;
        if replacement.signing_public_key != self.card.signing_public_key
            || replacement.hpke_public_key != self.card.hpke_public_key
            || replacement.key_version != self.card.key_version
        {
            self.trust = ContactTrustState::KeyChanged;
        }
        self.card = replacement;
        Ok(())
    }

    pub fn revoke(&mut self) {
        self.trust = ContactTrustState::Revoked;
    }
}

pub fn validate_display_name(input: &str) -> Result<String, CryptoError> {
    let normalized: String = input.nfc().collect();
    if normalized.len() > 64
        || normalized.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '\u{200e}'
                        | '\u{200f}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{2069}'
                )
        })
    {
        return Err(CryptoError::InvalidDisplayName);
    }
    Ok(normalized)
}

#[must_use]
pub fn safety_number(first: &[u8; 32], second: &[u8; 32]) -> String {
    let (lower, higher) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };
    let mut input = Vec::with_capacity(SAFETY_DOMAIN.len() + 64);
    input.extend_from_slice(SAFETY_DOMAIN);
    input.extend_from_slice(lower);
    input.extend_from_slice(higher);
    let hash = sha256(&input);
    let encoded = encode_top_bits(&hash, 60);
    format!("{}-{}-{}", &encoded[..4], &encoded[4..8], &encoded[8..12])
}

#[must_use]
pub fn display_id(signing_public: &[u8; 32]) -> String {
    let hash = sha256(signing_public);
    let prefix = encode_top_bits(&hash, 80);
    let checksum = crc32c::crc32c(&hash);
    format!("{prefix}-{checksum:08x}")
}

fn encode_top_bits(bytes: &[u8], bit_count: usize) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut output = String::with_capacity(bit_count.div_ceil(5));
    for group in 0..bit_count.div_ceil(5) {
        let mut value = 0u8;
        for bit in 0..5 {
            let index = group * 5 + bit;
            value <<= 1;
            if index < bit_count {
                value |= (bytes[index / 8] >> (7 - index % 8)) & 1;
            }
        }
        output.push(ALPHABET[value as usize] as char);
    }
    output
}

fn base45_encode(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len().div_ceil(2) * 3);
    let mut chunks = input.chunks_exact(2);
    for pair in &mut chunks {
        let value = u16::from_be_bytes([pair[0], pair[1]]) as usize;
        output.push(BASE45[value % 45] as char);
        output.push(BASE45[(value / 45) % 45] as char);
        output.push(BASE45[value / (45 * 45)] as char);
    }
    if let Some(byte) = chunks.remainder().first() {
        let value = *byte as usize;
        output.push(BASE45[value % 45] as char);
        output.push(BASE45[value / 45] as char);
    }
    output
}

fn base45_decode(input: &str) -> Result<Vec<u8>, CryptoError> {
    if input.len() % 3 == 1 {
        return Err(CryptoError::InvalidQr);
    }
    let values = input
        .bytes()
        .map(|byte| {
            BASE45
                .iter()
                .position(|candidate| *candidate == byte)
                .ok_or(CryptoError::InvalidQr)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = Vec::with_capacity(values.len() * 2 / 3 + 1);
    let mut index = 0;
    while values.len() - index >= 3 {
        let value = values[index] + values[index + 1] * 45 + values[index + 2] * 45 * 45;
        if value > u16::MAX as usize {
            return Err(CryptoError::InvalidQr);
        }
        output.extend_from_slice(&(value as u16).to_be_bytes());
        index += 3;
    }
    if values.len() - index == 2 {
        let value = values[index] + values[index + 1] * 45;
        if value > u8::MAX as usize {
            return Err(CryptoError::InvalidQr);
        }
        output.push(value as u8);
    }
    Ok(output)
}

fn expect_array(value: CborValue, length: usize) -> Result<Vec<CborValue>, CryptoError> {
    let CborValue::Array(values) = value else {
        return Err(CryptoError::InvalidContactCard);
    };
    if values.len() != length {
        return Err(CryptoError::InvalidContactCard);
    }
    Ok(values)
}

fn expect_u64(value: &CborValue) -> Result<u64, CryptoError> {
    let CborValue::Unsigned(value) = value else {
        return Err(CryptoError::InvalidContactCard);
    };
    Ok(*value)
}

fn expect_text(value: &CborValue) -> Result<&str, CryptoError> {
    let CborValue::Text(value) = value else {
        return Err(CryptoError::InvalidContactCard);
    };
    Ok(value)
}

fn fixed<const N: usize>(value: &CborValue) -> Result<[u8; N], CryptoError> {
    let CborValue::Bytes(value) = value else {
        return Err(CryptoError::InvalidContactCard);
    };
    value
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidContactCard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base45_matches_rfc_9285_example() {
        assert_eq!(base45_encode(b"AB"), "BB8");
        assert_eq!(base45_decode("BB8").unwrap(), b"AB");
        assert_eq!(base45_encode(b"Hello!!"), "%69 VD92EX0");
        assert_eq!(base45_decode("%69 VD92EX0").unwrap(), b"Hello!!");
    }

    #[test]
    fn contact_qr_round_trip_and_crc_failure() {
        let identity = Identity::generate().unwrap();
        let card = ContactCard::create(&identity, "구조대 A", 0x1f).unwrap();
        let qr = card.to_qr().unwrap();
        assert_eq!(ContactCard::from_qr(&qr).unwrap(), card);
        let mut corrupted = qr.into_bytes();
        let index = corrupted.len() - 1;
        corrupted[index] = if corrupted[index] == b'0' { b'1' } else { b'0' };
        assert_eq!(
            ContactCard::from_qr(core::str::from_utf8(&corrupted).unwrap()),
            Err(CryptoError::InvalidQr)
        );
    }

    #[test]
    fn safety_number_is_symmetric_and_import_is_unverified() {
        let first = Identity::generate().unwrap();
        let second = Identity::generate().unwrap();
        let a = safety_number(
            &first.public().signing_public_key,
            &second.public().signing_public_key,
        );
        let b = safety_number(
            &second.public().signing_public_key,
            &first.public().signing_public_key,
        );
        assert_eq!(a, b);
        assert_eq!(a.len(), 14);
        let mut record = ContactRecord::import(ContactCard::create(&second, "Second", 0).unwrap());
        assert_eq!(record.trust, ContactTrustState::Unverified);
        record
            .verify_in_person(&first.public().signing_public_key, &a)
            .unwrap();
        assert_eq!(record.trust, ContactTrustState::VerifiedInPerson);
    }

    #[test]
    fn bidi_and_unknown_capability_are_rejected() {
        let identity = Identity::generate().unwrap();
        assert_eq!(
            ContactCard::create(&identity, "safe\u{202e}name", 0),
            Err(CryptoError::InvalidDisplayName)
        );
        assert_eq!(
            ContactCard::create(&identity, "safe", 0x20),
            Err(CryptoError::InvalidCapabilities)
        );
    }

    #[test]
    fn changed_or_unsigned_keys_cannot_preserve_trust() {
        let local = Identity::generate().unwrap();
        let first = Identity::generate().unwrap();
        let replacement = Identity::generate().unwrap();
        let mut record = ContactRecord::import(ContactCard::create(&first, "First", 0).unwrap());
        let safety = safety_number(
            &local.public().signing_public_key,
            &first.public().signing_public_key,
        );
        record
            .verify_in_person(&local.public().signing_public_key, &safety)
            .unwrap();

        record
            .apply_card(ContactCard::create(&replacement, "First", 0).unwrap())
            .unwrap();
        assert_eq!(record.trust, ContactTrustState::KeyChanged);

        let mut forged = ContactCard::create(&replacement, "First", 0).unwrap();
        forged.signature[0] ^= 1;
        assert_eq!(
            record.apply_card(forged),
            Err(CryptoError::InvalidSignature)
        );
    }
}
