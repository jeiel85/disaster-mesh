//! DM-BP7-1 constrained bundle profile.

#![forbid(unsafe_code)]

use core::fmt;

use mesh_codec::{
    CborError, CborValue, DecodeLimits, decode_deterministic, decode_deterministic_prefix,
    encode_deterministic,
};
use mesh_types::{
    BpIdentityHash, BundleLifetime, CopyTokens, CreationSequence, HopState, MessageClass, PacketId,
    PayloadHash, Priority, RandomSourceId, RoutingSlot, WireBundleHash,
};
use sha2::{Digest, Sha256};

pub const CRATE_NAME: &str = "mesh-bundle";
pub const BUNDLE_FLAGS: u64 = 0x0004;
pub const CRC_TYPE_CRC32C: u64 = 2;
pub const ROUTING_BLOCK_TYPE: u64 = 192;
pub const ROUTING_BLOCK_FLAGS: u64 = 0x10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BundleError {
    Cbor(CborError),
    TooLarge,
    InvalidOuterArray,
    InvalidBlock,
    InvalidProfile,
    InvalidEndpoint,
    InvalidCrc,
    InvalidPayload,
    HashMismatch,
    Value,
}

impl fmt::Display for BundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid DM-BP7-1 bundle: {self:?}")
    }
}

impl std::error::Error for BundleError {}

impl From<CborError> for BundleError {
    fn from(value: CborError) -> Self {
        Self::Cbor(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmeCiphertext {
    pub encapsulated_key: [u8; 32],
    pub aad_hash: [u8; 32],
    pub ciphertext: Vec<u8>,
}

impl DmeCiphertext {
    pub fn encode(&self) -> Result<Vec<u8>, BundleError> {
        if self.ciphertext.is_empty() || self.ciphertext.len() > 8_118 {
            return Err(BundleError::TooLarge);
        }
        let encoded = encode_deterministic(&CborValue::Array(vec![
            CborValue::Unsigned(1),
            CborValue::Unsigned(1),
            CborValue::Bytes(self.encapsulated_key.to_vec()),
            CborValue::Bytes(self.aad_hash.to_vec()),
            CborValue::Bytes(self.ciphertext.clone()),
        ]))?;
        if encoded.len() > 8_192 {
            return Err(BundleError::TooLarge);
        }
        Ok(encoded)
    }

    pub fn decode(input: &[u8]) -> Result<Self, BundleError> {
        if input.len() > 8_192 {
            return Err(BundleError::TooLarge);
        }
        let values = expect_array(decode_deterministic(input, DecodeLimits::default())?, 5)?;
        if expect_u64(&values[0])? != 1 || expect_u64(&values[1])? != 1 {
            return Err(BundleError::InvalidPayload);
        }
        let encapsulated_key = expect_fixed_bytes::<32>(&values[2])?;
        let aad_hash = expect_fixed_bytes::<32>(&values[3])?;
        let ciphertext = expect_bytes(&values[4])?.to_vec();
        if ciphertext.is_empty() || ciphertext.len() > 8_118 {
            return Err(BundleError::TooLarge);
        }
        Ok(Self {
            encapsulated_key,
            aad_hash,
            ciphertext,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingBlock {
    pub packet_id: PacketId,
    pub message_class: MessageClass,
    pub priority: Priority,
    pub copy_tokens: CopyTokens,
    pub payload_size: u16,
    pub payload_hash: PayloadHash,
}

impl RoutingBlock {
    pub fn encode(&self) -> Result<Vec<u8>, BundleError> {
        encode_deterministic(&CborValue::Array(vec![
            CborValue::Unsigned(1),
            CborValue::Bytes(self.packet_id.as_bytes().to_vec()),
            CborValue::Unsigned(self.message_class as u64),
            CborValue::Unsigned(self.priority as u64),
            CborValue::Unsigned(u64::from(self.copy_tokens.get())),
            CborValue::Unsigned(u64::from(self.payload_size)),
            CborValue::Bytes(self.payload_hash.as_bytes().to_vec()),
        ]))
        .map_err(Into::into)
    }

    pub fn decode(input: &[u8]) -> Result<Self, BundleError> {
        let values = expect_array(decode_deterministic(input, DecodeLimits::default())?, 7)?;
        if expect_u64(&values[0])? != 1 {
            return Err(BundleError::InvalidProfile);
        }
        let packet_id = PacketId::from(expect_fixed_bytes::<16>(&values[1])?);
        let message_class =
            MessageClass::try_from(u8_value(&values[2])?).map_err(|_| BundleError::Value)?;
        let priority = Priority::try_from(u8_value(&values[3])?).map_err(|_| BundleError::Value)?;
        let copy_tokens = CopyTokens::new(u8_value(&values[4])?).map_err(|_| BundleError::Value)?;
        let payload_size =
            u16::try_from(expect_u64(&values[5])?).map_err(|_| BundleError::Value)?;
        if payload_size == 0 || payload_size > 8_192 {
            return Err(BundleError::TooLarge);
        }
        Ok(Self {
            packet_id,
            message_class,
            priority,
            copy_tokens,
            payload_size,
            payload_hash: PayloadHash::from(expect_fixed_bytes::<32>(&values[6])?),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bundle {
    pub destination: RoutingSlot,
    pub source: RandomSourceId,
    pub creation_sequence: CreationSequence,
    pub lifetime: BundleLifetime,
    pub age_millis: u64,
    pub hops: HopState,
    pub routing: RoutingBlock,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedBundle {
    pub bundle: Bundle,
    pub wire_hash: WireBundleHash,
    pub bp_identity_hash: BpIdentityHash,
}

impl Bundle {
    pub fn encode(&self) -> Result<Vec<u8>, BundleError> {
        self.validate()?;
        let destination = endpoint("r", self.destination.as_bytes());
        let source = endpoint("s", self.source.as_bytes());
        let creation = CborValue::Array(vec![
            CborValue::Unsigned(0),
            CborValue::Unsigned(self.creation_sequence.as_u64()),
        ]);
        let primary = encode_crc_block(vec![
            CborValue::Unsigned(7),
            CborValue::Unsigned(BUNDLE_FLAGS),
            CborValue::Unsigned(CRC_TYPE_CRC32C),
            destination,
            source,
            CborValue::Array(vec![CborValue::Unsigned(1), CborValue::Unsigned(0)]),
            creation,
            CborValue::Unsigned(self.lifetime.as_millis()),
        ])?;

        let age = canonical_block(
            7,
            2,
            0,
            encode_deterministic(&CborValue::Unsigned(self.age_millis))?,
        )?;
        let hop_data = encode_deterministic(&CborValue::Array(vec![
            CborValue::Unsigned(u64::from(self.hops.limit())),
            CborValue::Unsigned(u64::from(self.hops.count())),
        ]))?;
        let hops = canonical_block(10, 3, 0, hop_data)?;
        let routing = canonical_block(
            ROUTING_BLOCK_TYPE,
            4,
            ROUTING_BLOCK_FLAGS,
            self.routing.encode()?,
        )?;
        let payload = canonical_block(1, 1, 0, self.payload.clone())?;

        let total = 2 + primary.len() + age.len() + hops.len() + routing.len() + payload.len();
        if total > 12_288 {
            return Err(BundleError::TooLarge);
        }
        let mut output = Vec::with_capacity(total);
        output.push(0x9f);
        output.extend(primary);
        output.extend(age);
        output.extend(hops);
        output.extend(routing);
        output.extend(payload);
        output.push(0xff);
        Ok(output)
    }

    pub fn decode(input: &[u8]) -> Result<DecodedBundle, BundleError> {
        if input.len() > 12_288 {
            return Err(BundleError::TooLarge);
        }
        if input.first() != Some(&0x9f) || input.last() != Some(&0xff) {
            return Err(BundleError::InvalidOuterArray);
        }
        let mut position = 1usize;
        let mut blocks = Vec::with_capacity(5);
        for _ in 0..5 {
            let start = position;
            let limits = DecodeLimits {
                max_input_bytes: input.len() - position,
                max_byte_string_bytes: 12_288,
                ..DecodeLimits::default()
            };
            let (value, consumed) = decode_deterministic_prefix(&input[position..], limits)?;
            position += consumed;
            blocks.push((value, &input[start..position]));
        }
        if position + 1 != input.len() || input[position] != 0xff {
            return Err(BundleError::InvalidOuterArray);
        }

        validate_crc_block(&blocks[0].0, blocks[0].1)?;
        let primary = expect_array_ref(&blocks[0].0, 9)?;
        if expect_u64(&primary[0])? != 7
            || expect_u64(&primary[1])? != BUNDLE_FLAGS
            || expect_u64(&primary[2])? != CRC_TYPE_CRC32C
        {
            return Err(BundleError::InvalidProfile);
        }
        let destination = parse_endpoint(&primary[3], "r")?;
        let source = parse_endpoint(&primary[4], "s")?;
        let report_to = expect_array_ref(&primary[5], 2)?;
        if expect_u64(&report_to[0])? != 1 || expect_u64(&report_to[1])? != 0 {
            return Err(BundleError::InvalidProfile);
        }
        let creation = expect_array_ref(&primary[6], 2)?;
        if expect_u64(&creation[0])? != 0 {
            return Err(BundleError::InvalidProfile);
        }
        let creation_sequence = CreationSequence::from_u64(expect_u64(&creation[1])?);
        let lifetime = BundleLifetime::from_millis(expect_u64(&primary[7])?)
            .map_err(|_| BundleError::Value)?;

        let (age_type, age_number, age_flags, age_data) = parse_canonical(&blocks[1])?;
        let (hop_type, hop_number, hop_flags, hop_data) = parse_canonical(&blocks[2])?;
        let (route_type, route_number, route_flags, route_data) = parse_canonical(&blocks[3])?;
        let (payload_type, payload_number, payload_flags, payload) = parse_canonical(&blocks[4])?;
        if (age_type, age_number, age_flags) != (7, 2, 0)
            || (hop_type, hop_number, hop_flags) != (10, 3, 0)
            || (route_type, route_number, route_flags)
                != (ROUTING_BLOCK_TYPE, 4, ROUTING_BLOCK_FLAGS)
            || (payload_type, payload_number, payload_flags) != (1, 1, 0)
        {
            return Err(BundleError::InvalidProfile);
        }

        let age_millis = expect_u64(&decode_deterministic(&age_data, DecodeLimits::default())?)?;
        let hop_values =
            expect_array(decode_deterministic(&hop_data, DecodeLimits::default())?, 2)?;
        let hops = HopState::new(u8_value(&hop_values[1])?, u8_value(&hop_values[0])?)
            .map_err(|_| BundleError::Value)?;
        let routing = RoutingBlock::decode(&route_data)?;
        DmeCiphertext::decode(&payload)?;
        if routing.payload_size as usize != payload.len() {
            return Err(BundleError::InvalidPayload);
        }
        let payload_hash = sha256(&payload);
        if routing.payload_hash.as_bytes() != &payload_hash {
            return Err(BundleError::HashMismatch);
        }

        let bundle = Self {
            destination: RoutingSlot::from(destination),
            source: RandomSourceId::from(source),
            creation_sequence,
            lifetime,
            age_millis,
            hops,
            routing,
            payload,
        };
        bundle.validate()?;

        let mut identity_input = encode_deterministic(&primary[4])?;
        identity_input.extend(encode_deterministic(&primary[6])?);
        Ok(DecodedBundle {
            bundle,
            wire_hash: WireBundleHash::from(sha256(input)),
            bp_identity_hash: BpIdentityHash::from(sha256(&identity_input)),
        })
    }

    fn validate(&self) -> Result<(), BundleError> {
        if self.payload.is_empty() || self.payload.len() > 8_192 {
            return Err(BundleError::TooLarge);
        }
        DmeCiphertext::decode(&self.payload)?;
        if self.routing.payload_size as usize != self.payload.len()
            || self.routing.payload_hash.as_bytes() != &sha256(&self.payload)
        {
            return Err(BundleError::HashMismatch);
        }
        Ok(())
    }
}

fn endpoint(kind: &str, bytes: &[u8; 16]) -> CborValue {
    CborValue::Array(vec![
        CborValue::Unsigned(1),
        CborValue::Text(format!(
            "dtn://dm/{kind}/{}",
            mesh_codec::base32::encode_16(bytes)
        )),
    ])
}

fn parse_endpoint(value: &CborValue, kind: &str) -> Result<[u8; 16], BundleError> {
    let values = expect_array_ref(value, 2)?;
    if expect_u64(&values[0])? != 1 {
        return Err(BundleError::InvalidEndpoint);
    }
    let CborValue::Text(text) = &values[1] else {
        return Err(BundleError::InvalidEndpoint);
    };
    let prefix = format!("dtn://dm/{kind}/");
    let encoded = text
        .strip_prefix(&prefix)
        .ok_or(BundleError::InvalidEndpoint)?;
    mesh_codec::base32::decode_16(encoded).map_err(|_| BundleError::InvalidEndpoint)
}

fn canonical_block(
    block_type: u64,
    block_number: u64,
    flags: u64,
    data: Vec<u8>,
) -> Result<Vec<u8>, BundleError> {
    encode_crc_block(vec![
        CborValue::Unsigned(block_type),
        CborValue::Unsigned(block_number),
        CborValue::Unsigned(flags),
        CborValue::Unsigned(CRC_TYPE_CRC32C),
        CborValue::Bytes(data),
    ])
}

fn encode_crc_block(mut fields: Vec<CborValue>) -> Result<Vec<u8>, BundleError> {
    fields.push(CborValue::Bytes(vec![0; 4]));
    let zeroed = encode_deterministic(&CborValue::Array(fields.clone()))?;
    let checksum = crc32c::crc32c(&zeroed).to_be_bytes();
    *fields.last_mut().expect("CRC field") = CborValue::Bytes(checksum.to_vec());
    encode_deterministic(&CborValue::Array(fields)).map_err(Into::into)
}

fn validate_crc_block(value: &CborValue, encoded: &[u8]) -> Result<(), BundleError> {
    let CborValue::Array(fields) = value else {
        return Err(BundleError::InvalidBlock);
    };
    let CborValue::Bytes(checksum) = fields.last().ok_or(BundleError::InvalidBlock)? else {
        return Err(BundleError::InvalidBlock);
    };
    if checksum.len() != 4 {
        return Err(BundleError::InvalidCrc);
    }
    let mut zeroed = fields.clone();
    *zeroed.last_mut().expect("non-empty block") = CborValue::Bytes(vec![0; 4]);
    let zeroed = encode_deterministic(&CborValue::Array(zeroed))?;
    if encoded != encode_deterministic(value)?
        || checksum.as_slice() != crc32c::crc32c(&zeroed).to_be_bytes()
    {
        return Err(BundleError::InvalidCrc);
    }
    Ok(())
}

fn parse_canonical(block: &(CborValue, &[u8])) -> Result<(u64, u64, u64, Vec<u8>), BundleError> {
    validate_crc_block(&block.0, block.1)?;
    let values = expect_array_ref(&block.0, 6)?;
    if expect_u64(&values[3])? != CRC_TYPE_CRC32C {
        return Err(BundleError::InvalidProfile);
    }
    Ok((
        expect_u64(&values[0])?,
        expect_u64(&values[1])?,
        expect_u64(&values[2])?,
        expect_bytes(&values[4])?.to_vec(),
    ))
}

fn expect_array(value: CborValue, length: usize) -> Result<Vec<CborValue>, BundleError> {
    let CborValue::Array(values) = value else {
        return Err(BundleError::InvalidBlock);
    };
    if values.len() != length {
        return Err(BundleError::InvalidBlock);
    }
    Ok(values)
}

fn expect_array_ref(value: &CborValue, length: usize) -> Result<&[CborValue], BundleError> {
    let CborValue::Array(values) = value else {
        return Err(BundleError::InvalidBlock);
    };
    if values.len() != length {
        return Err(BundleError::InvalidBlock);
    }
    Ok(values)
}

fn expect_u64(value: &CborValue) -> Result<u64, BundleError> {
    let CborValue::Unsigned(value) = value else {
        return Err(BundleError::InvalidBlock);
    };
    Ok(*value)
}

fn u8_value(value: &CborValue) -> Result<u8, BundleError> {
    u8::try_from(expect_u64(value)?).map_err(|_| BundleError::Value)
}

fn expect_bytes(value: &CborValue) -> Result<&[u8], BundleError> {
    let CborValue::Bytes(value) = value else {
        return Err(BundleError::InvalidBlock);
    };
    Ok(value)
}

fn expect_fixed_bytes<const N: usize>(value: &CborValue) -> Result<[u8; N], BundleError> {
    expect_bytes(value)?
        .try_into()
        .map_err(|_| BundleError::InvalidBlock)
}

fn sha256(input: &[u8]) -> [u8; 32] {
    Sha256::digest(input).into()
}

#[must_use]
pub const fn lower_boundaries() -> [&'static str; 3] {
    [
        mesh_types::CRATE_NAME,
        mesh_codec::CRATE_NAME,
        mesh_crypto::CRATE_NAME,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bundle() -> Bundle {
        let payload = DmeCiphertext {
            encapsulated_key: [1; 32],
            aad_hash: [2; 32],
            ciphertext: vec![3; 64],
        }
        .encode()
        .unwrap();
        Bundle {
            destination: RoutingSlot::from([4; 16]),
            source: RandomSourceId::from([5; 16]),
            creation_sequence: CreationSequence::from_u64(6),
            lifetime: BundleLifetime::from_millis(60_000).unwrap(),
            age_millis: 7,
            hops: HopState::new(0, 12).unwrap(),
            routing: RoutingBlock {
                packet_id: PacketId::from([8; 16]),
                message_class: MessageClass::Direct,
                priority: Priority::P2,
                copy_tokens: CopyTokens::new(6).unwrap(),
                payload_size: payload.len() as u16,
                payload_hash: PayloadHash::from(sha256(&payload)),
            },
            payload,
        }
    }

    #[test]
    fn dm_bp7_outer_array_and_profile_round_trip() {
        let bundle = sample_bundle();
        let encoded = bundle.encode().unwrap();
        assert_eq!(encoded.first(), Some(&0x9f));
        assert_eq!(encoded.last(), Some(&0xff));
        let decoded = Bundle::decode(&encoded).unwrap();
        assert_eq!(decoded.bundle, bundle);
    }

    #[test]
    fn crc_and_payload_mutation_are_rejected() {
        let mut encoded = sample_bundle().encode().unwrap();
        encoded[20] ^= 1;
        assert!(matches!(
            Bundle::decode(&encoded),
            Err(BundleError::InvalidCrc | BundleError::InvalidEndpoint)
        ));
    }

    #[test]
    fn dme_limits_are_enforced() {
        let oversized = DmeCiphertext {
            encapsulated_key: [0; 32],
            aad_hash: [0; 32],
            ciphertext: vec![0; 8_119],
        };
        assert_eq!(oversized.encode(), Err(BundleError::TooLarge));
    }
}
