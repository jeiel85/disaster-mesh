//! Exact Core Deterministic CBOR payloads for BLE-CLA v1 encrypted frames.

use core::fmt;

use mesh_types::{PacketId, RoutingSlot, TokenGrantId, TransferId};
use sha2::{Digest, Sha256};

use crate::{CborError, CborValue, DecodeLimits, decode_deterministic, encode_deterministic};

pub const SESSION_HELLO: u8 = 0x10;
pub const ROUTING_SLOTS: u8 = 0x11;
pub const INVENTORY_PAGE: u8 = 0x12;
pub const BUNDLE_REQUEST: u8 = 0x13;
pub const BUNDLE_META: u8 = 0x14;
pub const BUNDLE_COMMIT: u8 = 0x16;
pub const TRANSFER_ACK: u8 = 0x17;
pub const CREDIT_UPDATE: u8 = 0x18;
pub const PING: u8 = 0x19;
pub const PONG: u8 = 0x1a;
pub const ERROR: u8 = 0x1b;
pub const GOODBYE: u8 = 0x1c;
pub const RESUME_QUERY: u8 = 0x1d;
pub const RESUME_STATE: u8 = 0x1e;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlError {
    Cbor(CborError),
    UnknownFrameType,
    WrongShape,
    InvalidValue,
}

impl fmt::Display for ControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid BLE control payload")
    }
}

impl std::error::Error for ControlError {}

impl From<CborError> for ControlError {
    fn from(value: CborError) -> Self {
        Self::Cbor(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionHello {
    pub session_id: [u8; 16],
    pub negotiated_minor: u64,
    pub node_capabilities: u64,
    pub mode: u8,
    pub max_concurrent_streams: u8,
    pub max_session_bytes: u32,
    pub max_session_seconds: u8,
    pub current_age_resolution_ms: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingSlotsPage {
    pub page: u64,
    pub is_last: bool,
    pub slots: Vec<RoutingSlot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleSummary {
    pub packet_id: PacketId,
    pub destination_slot: RoutingSlot,
    pub priority: u8,
    pub remaining_lifetime_seconds: u64,
    pub hop_count: u8,
    pub hop_limit: u8,
    pub copy_tokens: u8,
    pub total_bundle_bytes: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventoryPage {
    pub page_token: u64,
    pub is_last: bool,
    pub entries: Vec<BundleSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleRequestEntry {
    pub packet_id: PacketId,
    pub reason: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleMeta {
    pub transfer_id: TransferId,
    pub token_grant_id: Option<TokenGrantId>,
    pub packet_id: PacketId,
    pub total_size: u32,
    pub sha256: [u8; 32],
    pub chunk_size: u16,
    pub chunk_count: u16,
    pub proposed_receiver_tokens: u8,
    pub sender_remaining_tokens_after_reservation: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleCommit {
    pub transfer_id: TransferId,
    pub packet_id: PacketId,
    pub total_sha256: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferAck {
    pub transfer_id: TransferId,
    pub packet_id: PacketId,
    pub status: u8,
    pub accepted_tokens: u8,
    pub committed_payload_sha256: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreditUpdate {
    pub stream_id: u64,
    pub granted_bytes: u32,
    pub credit_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorFrame {
    pub error_code: u8,
    pub related_frame_type: Option<u64>,
    pub retry_after_ms: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Goodbye {
    pub reason: u8,
    pub retry_after_ms: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeQuery {
    pub packet_id: PacketId,
    pub expected_total_sha256: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeState {
    pub packet_id: PacketId,
    pub total_sha256: [u8; 32],
    pub chunk_size: u16,
    pub chunk_count: u16,
    pub received_bitmap: Vec<u8>,
    pub bitmap_sha256: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlPayload {
    SessionHello(SessionHello),
    RoutingSlots(RoutingSlotsPage),
    InventoryPage(InventoryPage),
    BundleRequest(Vec<BundleRequestEntry>),
    BundleMeta(BundleMeta),
    BundleCommit(BundleCommit),
    TransferAck(TransferAck),
    CreditUpdate(CreditUpdate),
    Ping([u8; 8]),
    Pong([u8; 8]),
    Error(ErrorFrame),
    Goodbye(Goodbye),
    ResumeQuery(ResumeQuery),
    ResumeState(ResumeState),
}

impl ControlPayload {
    #[must_use]
    pub const fn frame_type(&self) -> u8 {
        match self {
            Self::SessionHello(_) => SESSION_HELLO,
            Self::RoutingSlots(_) => ROUTING_SLOTS,
            Self::InventoryPage(_) => INVENTORY_PAGE,
            Self::BundleRequest(_) => BUNDLE_REQUEST,
            Self::BundleMeta(_) => BUNDLE_META,
            Self::BundleCommit(_) => BUNDLE_COMMIT,
            Self::TransferAck(_) => TRANSFER_ACK,
            Self::CreditUpdate(_) => CREDIT_UPDATE,
            Self::Ping(_) => PING,
            Self::Pong(_) => PONG,
            Self::Error(_) => ERROR,
            Self::Goodbye(_) => GOODBYE,
            Self::ResumeQuery(_) => RESUME_QUERY,
            Self::ResumeState(_) => RESUME_STATE,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ControlError> {
        validate(self)?;
        encode_deterministic(&to_value(self)).map_err(Into::into)
    }

    pub fn decode(frame_type: u8, input: &[u8]) -> Result<Self, ControlError> {
        let limits = DecodeLimits {
            max_input_bytes: 65_536,
            max_byte_string_bytes: 12_288,
            max_text_bytes: 0,
            max_collection_items: 32,
            max_depth: 5,
        };
        let value = decode_deterministic(input, limits)?;
        let payload = from_value(frame_type, &value)?;
        validate(&payload)?;
        Ok(payload)
    }
}

fn to_value(payload: &ControlPayload) -> CborValue {
    use ControlPayload as P;
    let array = match payload {
        P::SessionHello(v) => vec![
            bytes(&v.session_id),
            uint(v.negotiated_minor),
            uint(v.node_capabilities),
            uint(v.mode),
            uint(v.max_concurrent_streams),
            uint(v.max_session_bytes),
            uint(v.max_session_seconds),
            uint(v.current_age_resolution_ms),
        ],
        P::RoutingSlots(v) => vec![
            uint(v.page),
            CborValue::Bool(v.is_last),
            CborValue::Array(v.slots.iter().map(|x| bytes(x.as_bytes())).collect()),
        ],
        P::InventoryPage(v) => vec![
            uint(v.page_token),
            CborValue::Bool(v.is_last),
            CborValue::Array(v.entries.iter().map(summary_value).collect()),
        ],
        P::BundleRequest(v) => v
            .iter()
            .map(|x| CborValue::Array(vec![bytes(x.packet_id.as_bytes()), uint(x.reason)]))
            .collect(),
        P::BundleMeta(v) => vec![
            bytes(v.transfer_id.as_bytes()),
            v.token_grant_id
                .map_or(CborValue::Null, |x| bytes(x.as_bytes())),
            bytes(v.packet_id.as_bytes()),
            uint(v.total_size),
            bytes(&v.sha256),
            uint(v.chunk_size),
            uint(v.chunk_count),
            uint(v.proposed_receiver_tokens),
            uint(v.sender_remaining_tokens_after_reservation),
        ],
        P::BundleCommit(v) => vec![
            bytes(v.transfer_id.as_bytes()),
            bytes(v.packet_id.as_bytes()),
            bytes(&v.total_sha256),
        ],
        P::TransferAck(v) => vec![
            bytes(v.transfer_id.as_bytes()),
            bytes(v.packet_id.as_bytes()),
            uint(v.status),
            uint(v.accepted_tokens),
            v.committed_payload_sha256
                .map_or(CborValue::Null, |x| bytes(&x)),
        ],
        P::CreditUpdate(v) => vec![
            uint(v.stream_id),
            uint(v.granted_bytes),
            uint(v.credit_sequence),
        ],
        P::Ping(v) | P::Pong(v) => vec![bytes(v)],
        P::Error(v) => vec![
            uint(v.error_code),
            option_uint(v.related_frame_type),
            option_uint(v.retry_after_ms),
        ],
        P::Goodbye(v) => vec![uint(v.reason), option_uint(v.retry_after_ms)],
        P::ResumeQuery(v) => vec![
            bytes(v.packet_id.as_bytes()),
            bytes(&v.expected_total_sha256),
        ],
        P::ResumeState(v) => vec![
            bytes(v.packet_id.as_bytes()),
            bytes(&v.total_sha256),
            uint(v.chunk_size),
            uint(v.chunk_count),
            CborValue::Bytes(v.received_bitmap.clone()),
            bytes(&v.bitmap_sha256),
        ],
    };
    CborValue::Array(array)
}

fn from_value(frame_type: u8, value: &CborValue) -> Result<ControlPayload, ControlError> {
    let a = array(value)?;
    Ok(match frame_type {
        SESSION_HELLO if a.len() == 8 => ControlPayload::SessionHello(SessionHello {
            session_id: fixed(&a[0])?,
            negotiated_minor: unsigned(&a[1])?,
            node_capabilities: unsigned(&a[2])?,
            mode: u8v(&a[3])?,
            max_concurrent_streams: u8v(&a[4])?,
            max_session_bytes: u32v(&a[5])?,
            max_session_seconds: u8v(&a[6])?,
            current_age_resolution_ms: u32v(&a[7])?,
        }),
        ROUTING_SLOTS if a.len() == 3 => ControlPayload::RoutingSlots(RoutingSlotsPage {
            page: unsigned(&a[0])?,
            is_last: boolean(&a[1])?,
            slots: array(&a[2])?
                .iter()
                .map(|x| {
                    RoutingSlot::try_from(byte_slice(x)?).map_err(|_| ControlError::InvalidValue)
                })
                .collect::<Result<_, _>>()?,
        }),
        INVENTORY_PAGE if a.len() == 3 => ControlPayload::InventoryPage(InventoryPage {
            page_token: unsigned(&a[0])?,
            is_last: boolean(&a[1])?,
            entries: array(&a[2])?
                .iter()
                .map(parse_summary)
                .collect::<Result<_, _>>()?,
        }),
        BUNDLE_REQUEST => ControlPayload::BundleRequest(
            a.iter()
                .map(|x| {
                    let x = array_len(x, 2)?;
                    Ok(BundleRequestEntry {
                        packet_id: PacketId::try_from(byte_slice(&x[0])?)
                            .map_err(|_| ControlError::InvalidValue)?,
                        reason: u8v(&x[1])?,
                    })
                })
                .collect::<Result<_, ControlError>>()?,
        ),
        BUNDLE_META if a.len() == 9 => ControlPayload::BundleMeta(BundleMeta {
            transfer_id: TransferId::try_from(byte_slice(&a[0])?)
                .map_err(|_| ControlError::InvalidValue)?,
            token_grant_id: optional_fixed_id(&a[1])?,
            packet_id: PacketId::try_from(byte_slice(&a[2])?)
                .map_err(|_| ControlError::InvalidValue)?,
            total_size: u32v(&a[3])?,
            sha256: fixed(&a[4])?,
            chunk_size: u16v(&a[5])?,
            chunk_count: u16v(&a[6])?,
            proposed_receiver_tokens: u8v(&a[7])?,
            sender_remaining_tokens_after_reservation: u8v(&a[8])?,
        }),
        BUNDLE_COMMIT if a.len() == 3 => ControlPayload::BundleCommit(BundleCommit {
            transfer_id: TransferId::try_from(byte_slice(&a[0])?)
                .map_err(|_| ControlError::InvalidValue)?,
            packet_id: PacketId::try_from(byte_slice(&a[1])?)
                .map_err(|_| ControlError::InvalidValue)?,
            total_sha256: fixed(&a[2])?,
        }),
        TRANSFER_ACK if a.len() == 5 => ControlPayload::TransferAck(TransferAck {
            transfer_id: TransferId::try_from(byte_slice(&a[0])?)
                .map_err(|_| ControlError::InvalidValue)?,
            packet_id: PacketId::try_from(byte_slice(&a[1])?)
                .map_err(|_| ControlError::InvalidValue)?,
            status: u8v(&a[2])?,
            accepted_tokens: u8v(&a[3])?,
            committed_payload_sha256: optional_fixed(&a[4])?,
        }),
        CREDIT_UPDATE if a.len() == 3 => ControlPayload::CreditUpdate(CreditUpdate {
            stream_id: unsigned(&a[0])?,
            granted_bytes: u32v(&a[1])?,
            credit_sequence: unsigned(&a[2])?,
        }),
        PING if a.len() == 1 => ControlPayload::Ping(fixed(&a[0])?),
        PONG if a.len() == 1 => ControlPayload::Pong(fixed(&a[0])?),
        ERROR if a.len() == 3 => ControlPayload::Error(ErrorFrame {
            error_code: u8v(&a[0])?,
            related_frame_type: optional_uint(&a[1])?,
            retry_after_ms: optional_uint(&a[2])?
                .map(u32::try_from)
                .transpose()
                .map_err(|_| ControlError::InvalidValue)?,
        }),
        GOODBYE if a.len() == 2 => ControlPayload::Goodbye(Goodbye {
            reason: u8v(&a[0])?,
            retry_after_ms: optional_uint(&a[1])?
                .map(u32::try_from)
                .transpose()
                .map_err(|_| ControlError::InvalidValue)?,
        }),
        RESUME_QUERY if a.len() == 2 => ControlPayload::ResumeQuery(ResumeQuery {
            packet_id: PacketId::try_from(byte_slice(&a[0])?)
                .map_err(|_| ControlError::InvalidValue)?,
            expected_total_sha256: fixed(&a[1])?,
        }),
        RESUME_STATE if a.len() == 6 => ControlPayload::ResumeState(ResumeState {
            packet_id: PacketId::try_from(byte_slice(&a[0])?)
                .map_err(|_| ControlError::InvalidValue)?,
            total_sha256: fixed(&a[1])?,
            chunk_size: u16v(&a[2])?,
            chunk_count: u16v(&a[3])?,
            received_bitmap: byte_slice(&a[4])?.to_vec(),
            bitmap_sha256: fixed(&a[5])?,
        }),
        0x10..=0x1e => return Err(ControlError::WrongShape),
        _ => return Err(ControlError::UnknownFrameType),
    })
}

fn validate(payload: &ControlPayload) -> Result<(), ControlError> {
    use ControlPayload as P;
    let valid = match payload {
        P::SessionHello(v) => {
            v.node_capabilities & !31 == 0
                && v.mode <= 2
                && (1..=4).contains(&v.max_concurrent_streams)
                && (1024..=1_048_576).contains(&v.max_session_bytes)
                && (1..=120).contains(&v.max_session_seconds)
                && (1..=60_000).contains(&v.current_age_resolution_ms)
        }
        P::RoutingSlots(v) => v.slots.len() <= 32,
        P::InventoryPage(v) => v.entries.len() <= 32 && v.entries.iter().all(valid_summary),
        P::BundleRequest(v) => v.len() <= 16 && v.iter().all(|x| (1..=3).contains(&x.reason)),
        P::BundleMeta(v) => {
            let count = v.total_size.div_ceil(u32::from(v.chunk_size.max(1)));
            (1..=12_288).contains(&v.total_size)
                && (1..=4096).contains(&v.chunk_size)
                && (1..=1024).contains(&v.chunk_count)
                && count == u32::from(v.chunk_count)
                && (1..=16).contains(&v.proposed_receiver_tokens)
                && (1..=16).contains(&v.sender_remaining_tokens_after_reservation)
                && (v.token_grant_id.is_some() || v.proposed_receiver_tokens == 1)
        }
        P::BundleCommit(_) | P::ResumeQuery(_) | P::Ping(_) | P::Pong(_) => true,
        P::TransferAck(v) => {
            (1..=7).contains(&v.status)
                && if v.status <= 2 {
                    (1..=16).contains(&v.accepted_tokens) && v.committed_payload_sha256.is_some()
                } else {
                    v.accepted_tokens == 0 && v.committed_payload_sha256.is_none()
                }
        }
        P::CreditUpdate(v) => (1..=262_144).contains(&v.granted_bytes),
        P::Error(v) => {
            (1..=12).contains(&v.error_code) && v.retry_after_ms.is_none_or(|x| x <= 600_000)
        }
        P::Goodbye(v) => {
            (1..=9).contains(&v.reason) && v.retry_after_ms.is_none_or(|x| x <= 600_000)
        }
        P::ResumeState(v) => {
            let expected = usize::from(v.chunk_count).div_ceil(8);
            let unused = expected * 8 - usize::from(v.chunk_count);
            (1..=4096).contains(&v.chunk_size)
                && (1..=1024).contains(&v.chunk_count)
                && v.received_bitmap.len() == expected
                && (unused == 0
                    || v.received_bitmap
                        .last()
                        .is_some_and(|x| x & (!0u8 << (8 - unused)) == 0))
                && <[u8; 32]>::from(Sha256::digest(&v.received_bitmap)) == v.bitmap_sha256
        }
    };
    if valid {
        Ok(())
    } else {
        Err(ControlError::InvalidValue)
    }
}

fn valid_summary(v: &BundleSummary) -> bool {
    v.priority <= 3
        && v.hop_limit > 0
        && v.hop_limit <= 32
        && v.hop_count < v.hop_limit
        && (1..=16).contains(&v.copy_tokens)
        && (1..=12_288).contains(&v.total_bundle_bytes)
}

fn summary_value(v: &BundleSummary) -> CborValue {
    CborValue::Array(vec![
        bytes(v.packet_id.as_bytes()),
        bytes(v.destination_slot.as_bytes()),
        uint(v.priority),
        uint(v.remaining_lifetime_seconds),
        uint(v.hop_count),
        uint(v.hop_limit),
        uint(v.copy_tokens),
        uint(v.total_bundle_bytes),
    ])
}

fn parse_summary(v: &CborValue) -> Result<BundleSummary, ControlError> {
    let a = array_len(v, 8)?;
    Ok(BundleSummary {
        packet_id: PacketId::try_from(byte_slice(&a[0])?)
            .map_err(|_| ControlError::InvalidValue)?,
        destination_slot: RoutingSlot::try_from(byte_slice(&a[1])?)
            .map_err(|_| ControlError::InvalidValue)?,
        priority: u8v(&a[2])?,
        remaining_lifetime_seconds: unsigned(&a[3])?,
        hop_count: u8v(&a[4])?,
        hop_limit: u8v(&a[5])?,
        copy_tokens: u8v(&a[6])?,
        total_bundle_bytes: u32v(&a[7])?,
    })
}

fn bytes(value: &[u8]) -> CborValue {
    CborValue::Bytes(value.to_vec())
}
fn uint(value: impl Into<u64>) -> CborValue {
    CborValue::Unsigned(value.into())
}
fn option_uint(value: Option<impl Into<u64>>) -> CborValue {
    value.map_or(CborValue::Null, uint)
}
fn array(value: &CborValue) -> Result<&[CborValue], ControlError> {
    if let CborValue::Array(v) = value {
        Ok(v)
    } else {
        Err(ControlError::WrongShape)
    }
}
fn array_len(value: &CborValue, len: usize) -> Result<&[CborValue], ControlError> {
    let v = array(value)?;
    if v.len() == len {
        Ok(v)
    } else {
        Err(ControlError::WrongShape)
    }
}
fn byte_slice(value: &CborValue) -> Result<&[u8], ControlError> {
    if let CborValue::Bytes(v) = value {
        Ok(v)
    } else {
        Err(ControlError::WrongShape)
    }
}
fn unsigned(value: &CborValue) -> Result<u64, ControlError> {
    if let CborValue::Unsigned(v) = value {
        Ok(*v)
    } else {
        Err(ControlError::WrongShape)
    }
}
fn u8v(value: &CborValue) -> Result<u8, ControlError> {
    u8::try_from(unsigned(value)?).map_err(|_| ControlError::InvalidValue)
}
fn u16v(value: &CborValue) -> Result<u16, ControlError> {
    u16::try_from(unsigned(value)?).map_err(|_| ControlError::InvalidValue)
}
fn u32v(value: &CborValue) -> Result<u32, ControlError> {
    u32::try_from(unsigned(value)?).map_err(|_| ControlError::InvalidValue)
}
fn boolean(value: &CborValue) -> Result<bool, ControlError> {
    if let CborValue::Bool(v) = value {
        Ok(*v)
    } else {
        Err(ControlError::WrongShape)
    }
}
fn fixed<const N: usize>(value: &CborValue) -> Result<[u8; N], ControlError> {
    byte_slice(value)?
        .try_into()
        .map_err(|_| ControlError::InvalidValue)
}
fn optional_fixed<const N: usize>(value: &CborValue) -> Result<Option<[u8; N]>, ControlError> {
    if value == &CborValue::Null {
        Ok(None)
    } else {
        fixed(value).map(Some)
    }
}
fn optional_fixed_id(value: &CborValue) -> Result<Option<TokenGrantId>, ControlError> {
    optional_fixed(value).map(|x| x.map(TokenGrantId::from))
}
fn optional_uint(value: &CborValue) -> Result<Option<u64>, ControlError> {
    if value == &CborValue::Null {
        Ok(None)
    } else {
        unsigned(value).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(payload: ControlPayload) {
        let encoded = payload.encode().unwrap();
        assert_eq!(
            ControlPayload::decode(payload.frame_type(), &encoded).unwrap(),
            payload
        );
    }

    #[test]
    fn all_control_shapes_round_trip() {
        round_trip(ControlPayload::SessionHello(SessionHello {
            session_id: [1; 16],
            negotiated_minor: 0,
            node_capabilities: 31,
            mode: 1,
            max_concurrent_streams: 1,
            max_session_bytes: 65_536,
            max_session_seconds: 30,
            current_age_resolution_ms: 1000,
        }));
        round_trip(ControlPayload::RoutingSlots(RoutingSlotsPage {
            page: 0,
            is_last: true,
            slots: vec![RoutingSlot::from([2; 16])],
        }));
        round_trip(ControlPayload::InventoryPage(InventoryPage {
            page_token: 7,
            is_last: true,
            entries: vec![BundleSummary {
                packet_id: PacketId::from([3; 16]),
                destination_slot: RoutingSlot::from([4; 16]),
                priority: 0,
                remaining_lifetime_seconds: 60,
                hop_count: 0,
                hop_limit: 12,
                copy_tokens: 6,
                total_bundle_bytes: 100,
            }],
        }));
        round_trip(ControlPayload::BundleRequest(vec![BundleRequestEntry {
            packet_id: PacketId::from([3; 16]),
            reason: 2,
        }]));
        round_trip(ControlPayload::BundleMeta(BundleMeta {
            transfer_id: TransferId::from([5; 16]),
            token_grant_id: Some(TokenGrantId::from([6; 16])),
            packet_id: PacketId::from([3; 16]),
            total_size: 100,
            sha256: [7; 32],
            chunk_size: 50,
            chunk_count: 2,
            proposed_receiver_tokens: 3,
            sender_remaining_tokens_after_reservation: 3,
        }));
        round_trip(ControlPayload::BundleCommit(BundleCommit {
            transfer_id: TransferId::from([5; 16]),
            packet_id: PacketId::from([3; 16]),
            total_sha256: [7; 32],
        }));
        round_trip(ControlPayload::TransferAck(TransferAck {
            transfer_id: TransferId::from([5; 16]),
            packet_id: PacketId::from([3; 16]),
            status: 1,
            accepted_tokens: 3,
            committed_payload_sha256: Some([8; 32]),
        }));
        round_trip(ControlPayload::CreditUpdate(CreditUpdate {
            stream_id: 1,
            granted_bytes: 4096,
            credit_sequence: 0,
        }));
        round_trip(ControlPayload::Ping([9; 8]));
        round_trip(ControlPayload::Pong([9; 8]));
        round_trip(ControlPayload::Error(ErrorFrame {
            error_code: 1,
            related_frame_type: Some(BUNDLE_META.into()),
            retry_after_ms: Some(10),
        }));
        round_trip(ControlPayload::Goodbye(Goodbye {
            reason: 1,
            retry_after_ms: None,
        }));
        round_trip(ControlPayload::ResumeQuery(ResumeQuery {
            packet_id: PacketId::from([3; 16]),
            expected_total_sha256: [7; 32],
        }));
        let bitmap = vec![0b0000_0111];
        round_trip(ControlPayload::ResumeState(ResumeState {
            packet_id: PacketId::from([3; 16]),
            total_sha256: [7; 32],
            chunk_size: 50,
            chunk_count: 3,
            bitmap_sha256: Sha256::digest(&bitmap).into(),
            received_bitmap: bitmap,
        }));
    }

    #[test]
    fn semantic_bounds_and_resume_bitmap_fail_closed() {
        let invalid = ControlPayload::BundleRequest(
            (0..17)
                .map(|x| BundleRequestEntry {
                    packet_id: PacketId::from([x; 16]),
                    reason: 2,
                })
                .collect(),
        );
        assert_eq!(invalid.encode(), Err(ControlError::InvalidValue));
        let bad = ControlPayload::ResumeState(ResumeState {
            packet_id: PacketId::from([1; 16]),
            total_sha256: [2; 32],
            chunk_size: 4,
            chunk_count: 3,
            received_bitmap: vec![0xff],
            bitmap_sha256: Sha256::digest([0xff]).into(),
        });
        assert_eq!(bad.encode(), Err(ControlError::InvalidValue));
    }
}
