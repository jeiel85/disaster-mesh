//! Byte-exact BLE-CLA v1 frame headers.

use core::fmt;
use std::collections::{BTreeMap, VecDeque};

use mesh_types::generated_contracts::protocol;

const OUTER_HEADER_BYTES: usize = 16;
const PLAIN_HEADER_BYTES: usize = 8;
const ENCRYPTED_HEADER_BYTES: usize = 16;
const BUNDLE_CHUNK_HEADER_BYTES: usize = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Channel {
    Control = 0,
    Data = 1,
}

impl TryFrom<u8> for Channel {
    type Error = FrameError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Control),
            1 => Ok(Self::Data),
            _ => Err(FrameError::InvalidChannel),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameError {
    TooShort,
    InvalidMagic,
    UnsupportedVersion,
    InvalidFlags,
    InvalidChannel,
    InvalidFrameId,
    InvalidSegmentLayout,
    InvalidLength,
    InvalidFrameType,
    InvalidStream,
    CrcMismatch,
    SegmentConflict,
    ReassemblyCapacity,
    FrameIdExhausted,
}

pub fn segment_logical_frame(
    channel: Channel,
    logical_frame_id: u32,
    logical: &[u8],
    max_att_payload: usize,
) -> Result<Vec<OuterSegment>, FrameError> {
    if logical_frame_id == 0
        || logical.is_empty()
        || logical.len() > protocol::BLE_WIRE_MAX_LOGICAL_FRAME_BYTES as usize
        || max_att_payload <= OUTER_HEADER_BYTES
    {
        return Err(FrameError::InvalidLength);
    }
    let segment_payload = max_att_payload - OUTER_HEADER_BYTES;
    let count = logical.len().div_ceil(segment_payload);
    if count == 0 || count > protocol::BLE_WIRE_MAX_SEGMENT_COUNT as usize {
        return Err(FrameError::InvalidSegmentLayout);
    }
    let segment_count = u16::try_from(count).map_err(|_| FrameError::InvalidSegmentLayout)?;
    let logical_length = u32::try_from(logical.len()).map_err(|_| FrameError::InvalidLength)?;
    logical
        .chunks(segment_payload)
        .enumerate()
        .map(|(index, bytes)| {
            let segment = OuterSegment {
                channel,
                logical_frame_id,
                segment_index: u16::try_from(index)
                    .map_err(|_| FrameError::InvalidSegmentLayout)?,
                segment_count,
                logical_length,
                bytes: bytes.to_vec(),
            };
            segment.validate()?;
            Ok(segment)
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReassemblyOutcome {
    Pending,
    Duplicate,
    Complete { channel: Channel, bytes: Vec<u8> },
}

#[derive(Clone, Debug)]
struct PartialFrame {
    channel: Channel,
    segment_count: u16,
    logical_length: u32,
    started_ms: u64,
    received_bytes: usize,
    segments: Vec<Option<Vec<u8>>>,
}

#[derive(Clone, Debug)]
struct CompletedFrame {
    frame_id: u32,
    channel: Channel,
    logical_length: u32,
    segments: Vec<Vec<u8>>,
}

#[derive(Debug)]
pub struct OuterReassembler {
    active: BTreeMap<u32, PartialFrame>,
    completed: VecDeque<CompletedFrame>,
    max_active: usize,
    completed_cache: usize,
    timeout_ms: u64,
}

impl OuterReassembler {
    #[must_use]
    pub fn new(max_active: usize, completed_cache: usize, timeout_ms: u64) -> Self {
        Self {
            active: BTreeMap::new(),
            completed: VecDeque::new(),
            max_active,
            completed_cache,
            timeout_ms,
        }
    }

    pub fn accept(
        &mut self,
        segment: OuterSegment,
        now_ms: u64,
    ) -> Result<ReassemblyOutcome, FrameError> {
        segment.validate()?;
        if let Some(completed) = self
            .completed
            .iter()
            .find(|completed| completed.frame_id == segment.logical_frame_id)
        {
            let same = completed.channel == segment.channel
                && completed.logical_length == segment.logical_length
                && completed.segments.len() == usize::from(segment.segment_count)
                && completed.segments[usize::from(segment.segment_index)] == segment.bytes;
            return if same {
                Ok(ReassemblyOutcome::Duplicate)
            } else {
                Err(FrameError::SegmentConflict)
            };
        }

        if !self.active.contains_key(&segment.logical_frame_id) {
            if self.active.len() >= self.max_active || self.max_active == 0 {
                return Err(FrameError::ReassemblyCapacity);
            }
            self.active.insert(
                segment.logical_frame_id,
                PartialFrame {
                    channel: segment.channel,
                    segment_count: segment.segment_count,
                    logical_length: segment.logical_length,
                    started_ms: now_ms,
                    received_bytes: 0,
                    segments: vec![None; usize::from(segment.segment_count)],
                },
            );
        }

        let partial = self
            .active
            .get_mut(&segment.logical_frame_id)
            .expect("inserted above");
        if partial.channel != segment.channel
            || partial.segment_count != segment.segment_count
            || partial.logical_length != segment.logical_length
        {
            self.active.remove(&segment.logical_frame_id);
            return Err(FrameError::SegmentConflict);
        }
        let index = usize::from(segment.segment_index);
        if let Some(existing) = &partial.segments[index] {
            return if existing == &segment.bytes {
                Ok(ReassemblyOutcome::Duplicate)
            } else {
                self.active.remove(&segment.logical_frame_id);
                Err(FrameError::SegmentConflict)
            };
        }
        partial.received_bytes = partial
            .received_bytes
            .checked_add(segment.bytes.len())
            .ok_or(FrameError::InvalidLength)?;
        if partial.received_bytes > partial.logical_length as usize {
            self.active.remove(&segment.logical_frame_id);
            return Err(FrameError::InvalidLength);
        }
        partial.segments[index] = Some(segment.bytes);
        if partial.segments.iter().any(Option::is_none) {
            return Ok(ReassemblyOutcome::Pending);
        }

        let partial = self
            .active
            .remove(&segment.logical_frame_id)
            .expect("active frame exists");
        if partial.received_bytes != partial.logical_length as usize {
            return Err(FrameError::InvalidLength);
        }
        let segments = partial
            .segments
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(FrameError::InvalidSegmentLayout)?;
        let bytes = segments.concat();
        if self.completed_cache > 0 {
            self.completed.push_back(CompletedFrame {
                frame_id: segment.logical_frame_id,
                channel: partial.channel,
                logical_length: partial.logical_length,
                segments,
            });
            while self.completed.len() > self.completed_cache {
                self.completed.pop_front();
            }
        }
        Ok(ReassemblyOutcome::Complete {
            channel: partial.channel,
            bytes,
        })
    }

    pub fn expire(&mut self, now_ms: u64) -> Vec<u32> {
        let expired = self
            .active
            .iter()
            .filter_map(|(frame_id, partial)| {
                (now_ms.saturating_sub(partial.started_ms) >= self.timeout_ms).then_some(*frame_id)
            })
            .collect::<Vec<_>>();
        for frame_id in &expired {
            self.active.remove(frame_id);
        }
        expired
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameIdSequence(Option<u32>);

impl FrameIdSequence {
    pub fn new(initial: u32) -> Result<Self, FrameError> {
        if initial == 0 {
            return Err(FrameError::InvalidFrameId);
        }
        Ok(Self(Some(initial)))
    }

    pub fn take(&mut self) -> Result<u32, FrameError> {
        let current = self.0.ok_or(FrameError::FrameIdExhausted)?;
        self.0 = current.checked_add(1);
        Ok(current)
    }
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BLE frame: {self:?}")
    }
}

impl std::error::Error for FrameError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterSegment {
    pub channel: Channel,
    pub logical_frame_id: u32,
    pub segment_index: u16,
    pub segment_count: u16,
    pub logical_length: u32,
    pub bytes: Vec<u8>,
}

impl OuterSegment {
    pub fn encode(&self) -> Result<Vec<u8>, FrameError> {
        self.validate()?;
        let mut output = Vec::with_capacity(OUTER_HEADER_BYTES + self.bytes.len());
        output.push(protocol::BLE_WIRE_OUTER_MAGIC as u8);
        output.push(protocol::BLE_WIRE_OUTER_VERSION as u8);
        output.push(self.expected_flags());
        output.push(self.channel as u8);
        output.extend_from_slice(&self.logical_frame_id.to_be_bytes());
        output.extend_from_slice(&self.segment_index.to_be_bytes());
        output.extend_from_slice(&self.segment_count.to_be_bytes());
        output.extend_from_slice(&self.logical_length.to_be_bytes());
        output.extend_from_slice(&self.bytes);
        Ok(output)
    }

    pub fn decode(input: &[u8]) -> Result<Self, FrameError> {
        if input.len() <= OUTER_HEADER_BYTES {
            return Err(FrameError::TooShort);
        }
        if input[0] != protocol::BLE_WIRE_OUTER_MAGIC as u8 {
            return Err(FrameError::InvalidMagic);
        }
        if input[1] != protocol::BLE_WIRE_OUTER_VERSION as u8 {
            return Err(FrameError::UnsupportedVersion);
        }
        let flags = input[2];
        if flags & !0x03 != 0 {
            return Err(FrameError::InvalidFlags);
        }
        let value = Self {
            channel: input[3].try_into()?,
            logical_frame_id: u32::from_be_bytes(input[4..8].try_into().expect("fixed slice")),
            segment_index: u16::from_be_bytes(input[8..10].try_into().expect("fixed slice")),
            segment_count: u16::from_be_bytes(input[10..12].try_into().expect("fixed slice")),
            logical_length: u32::from_be_bytes(input[12..16].try_into().expect("fixed slice")),
            bytes: input[16..].to_vec(),
        };
        value.validate()?;
        if flags != value.expected_flags() {
            return Err(FrameError::InvalidFlags);
        }
        Ok(value)
    }

    fn expected_flags(&self) -> u8 {
        u8::from(self.segment_index == 0)
            | (u8::from(self.segment_index + 1 == self.segment_count) << 1)
    }

    fn validate(&self) -> Result<(), FrameError> {
        if self.logical_frame_id == 0 {
            return Err(FrameError::InvalidFrameId);
        }
        if self.segment_count == 0
            || u64::from(self.segment_count) > protocol::BLE_WIRE_MAX_SEGMENT_COUNT
            || self.segment_index >= self.segment_count
        {
            return Err(FrameError::InvalidSegmentLayout);
        }
        if self.logical_length == 0
            || u64::from(self.logical_length) > protocol::BLE_WIRE_MAX_LOGICAL_FRAME_BYTES
            || self.bytes.is_empty()
            || self.bytes.len() > self.logical_length as usize
        {
            return Err(FrameError::InvalidLength);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlainFrame {
    pub frame_type: u8,
    pub sequence: u16,
    pub payload: Vec<u8>,
}

impl PlainFrame {
    pub fn encode(&self) -> Result<Vec<u8>, FrameError> {
        self.validate()?;
        let mut output = Vec::with_capacity(PLAIN_HEADER_BYTES + self.payload.len());
        output.extend_from_slice(&[0xd7, self.frame_type, 0, 1]);
        output.extend_from_slice(&(self.payload.len() as u16).to_be_bytes());
        output.extend_from_slice(&self.sequence.to_be_bytes());
        output.extend_from_slice(&self.payload);
        Ok(output)
    }

    pub fn decode(input: &[u8]) -> Result<Self, FrameError> {
        if input.len() < PLAIN_HEADER_BYTES {
            return Err(FrameError::TooShort);
        }
        if input[0] != 0xd7 {
            return Err(FrameError::InvalidMagic);
        }
        if input[2] != 0 {
            return Err(FrameError::InvalidFlags);
        }
        if input[3] != 1 {
            return Err(FrameError::UnsupportedVersion);
        }
        let length = u16::from_be_bytes(input[4..6].try_into().expect("fixed slice")) as usize;
        if input.len() != PLAIN_HEADER_BYTES + length {
            return Err(FrameError::InvalidLength);
        }
        let value = Self {
            frame_type: input[1],
            sequence: u16::from_be_bytes(input[6..8].try_into().expect("fixed slice")),
            payload: input[8..].to_vec(),
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), FrameError> {
        if !(1..=3).contains(&self.frame_type) {
            return Err(FrameError::InvalidFrameType);
        }
        if self.payload.len() > 512 || self.payload.len() > u16::MAX as usize {
            return Err(FrameError::InvalidLength);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedFrame {
    pub frame_type: u8,
    pub stream_id: u32,
    pub sequence: u32,
    pub payload: Vec<u8>,
}

impl EncryptedFrame {
    pub fn encode(&self) -> Result<Vec<u8>, FrameError> {
        self.validate()?;
        let mut output = Vec::with_capacity(ENCRYPTED_HEADER_BYTES + self.payload.len());
        output.extend_from_slice(&[self.frame_type, 0, 0, 0]);
        output.extend_from_slice(&self.stream_id.to_be_bytes());
        output.extend_from_slice(&self.sequence.to_be_bytes());
        output.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        output.extend_from_slice(&self.payload);
        Ok(output)
    }

    pub fn decode(input: &[u8]) -> Result<Self, FrameError> {
        if input.len() < ENCRYPTED_HEADER_BYTES {
            return Err(FrameError::TooShort);
        }
        if input[1..4] != [0, 0, 0] {
            return Err(FrameError::InvalidFlags);
        }
        let length = u32::from_be_bytes(input[12..16].try_into().expect("fixed slice")) as usize;
        if input.len() != ENCRYPTED_HEADER_BYTES + length {
            return Err(FrameError::InvalidLength);
        }
        let value = Self {
            frame_type: input[0],
            stream_id: u32::from_be_bytes(input[4..8].try_into().expect("fixed slice")),
            sequence: u32::from_be_bytes(input[8..12].try_into().expect("fixed slice")),
            payload: input[16..].to_vec(),
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), FrameError> {
        if !(0x10..=0x1e).contains(&self.frame_type) {
            return Err(FrameError::InvalidFrameType);
        }
        let transfer_frame = matches!(self.frame_type, 0x14..=0x18 | 0x1d | 0x1e);
        let expected_stream = u32::from(transfer_frame);
        if self.stream_id != expected_stream {
            return Err(FrameError::InvalidStream);
        }
        if self.payload.len() > protocol::BLE_WIRE_MAX_LOGICAL_FRAME_BYTES as usize {
            return Err(FrameError::InvalidLength);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleChunk {
    pub transfer_id: [u8; 16],
    pub chunk_index: u32,
    pub bytes: Vec<u8>,
}

impl BundleChunk {
    pub fn encode(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(BUNDLE_CHUNK_HEADER_BYTES + self.bytes.len());
        output.extend_from_slice(&self.transfer_id);
        output.extend_from_slice(&self.chunk_index.to_be_bytes());
        output.extend_from_slice(&crc32c::crc32c(&self.bytes).to_be_bytes());
        output.extend_from_slice(&self.bytes);
        output
    }

    pub fn decode(input: &[u8]) -> Result<Self, FrameError> {
        if input.len() < BUNDLE_CHUNK_HEADER_BYTES {
            return Err(FrameError::TooShort);
        }
        let expected_crc = u32::from_be_bytes(input[20..24].try_into().expect("fixed slice"));
        let bytes = input[24..].to_vec();
        if crc32c::crc32c(&bytes) != expected_crc {
            return Err(FrameError::CrcMismatch);
        }
        Ok(Self {
            transfer_id: input[..16].try_into().expect("fixed slice"),
            chunk_index: u32::from_be_bytes(input[16..20].try_into().expect("fixed slice")),
            bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outer_segment_matches_golden_header() {
        let segment = OuterSegment {
            channel: Channel::Control,
            logical_frame_id: 1,
            segment_index: 0,
            segment_count: 1,
            logical_length: 3,
            bytes: vec![0xaa, 0xbb, 0xcc],
        };
        let expected = vec![
            0xd8, 0x01, 0x03, 0x00, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 3, 0xaa, 0xbb, 0xcc,
        ];
        assert_eq!(segment.encode().unwrap(), expected);
        assert_eq!(OuterSegment::decode(&expected).unwrap(), segment);
    }

    #[test]
    fn plain_and_encrypted_headers_round_trip() {
        let plain = PlainFrame {
            frame_type: 1,
            sequence: 2,
            payload: vec![3, 4],
        };
        assert_eq!(PlainFrame::decode(&plain.encode().unwrap()).unwrap(), plain);

        let encrypted = EncryptedFrame {
            frame_type: 0x14,
            stream_id: 1,
            sequence: 7,
            payload: vec![8, 9],
        };
        assert_eq!(
            EncryptedFrame::decode(&encrypted.encode().unwrap()).unwrap(),
            encrypted
        );
    }

    #[test]
    fn invalid_headers_are_rejected() {
        let mut bytes = OuterSegment {
            channel: Channel::Data,
            logical_frame_id: 9,
            segment_index: 0,
            segment_count: 1,
            logical_length: 1,
            bytes: vec![1],
        }
        .encode()
        .unwrap();
        bytes[2] = 0x04;
        assert_eq!(OuterSegment::decode(&bytes), Err(FrameError::InvalidFlags));

        let frame = EncryptedFrame {
            frame_type: 0x10,
            stream_id: 1,
            sequence: 0,
            payload: vec![],
        };
        assert_eq!(frame.encode(), Err(FrameError::InvalidStream));
    }

    #[test]
    fn chunk_crc_is_checked() {
        let chunk = BundleChunk {
            transfer_id: [7; 16],
            chunk_index: 4,
            bytes: vec![1, 2, 3],
        };
        let mut encoded = chunk.encode();
        assert_eq!(BundleChunk::decode(&encoded).unwrap(), chunk);
        *encoded.last_mut().unwrap() ^= 1;
        assert_eq!(BundleChunk::decode(&encoded), Err(FrameError::CrcMismatch));
    }

    #[test]
    fn segmentation_reassembles_out_of_order_and_rejects_conflicts() {
        let logical = b"twenty bytes exactly!";
        let segments = segment_logical_frame(Channel::Data, 7, logical, 20).unwrap();
        assert_eq!(segments.len(), logical.len().div_ceil(4));
        assert!(
            segments
                .iter()
                .all(|segment| segment.encode().unwrap().len() <= 20)
        );

        let mut reassembler = OuterReassembler::new(2, 2, 10_000);
        for segment in segments.iter().skip(1).rev() {
            assert_eq!(
                reassembler.accept(segment.clone(), 0).unwrap(),
                ReassemblyOutcome::Pending
            );
        }
        assert_eq!(
            reassembler.accept(segments[0].clone(), 1).unwrap(),
            ReassemblyOutcome::Complete {
                channel: Channel::Data,
                bytes: logical.to_vec(),
            }
        );
        assert_eq!(
            reassembler.accept(segments[0].clone(), 2).unwrap(),
            ReassemblyOutcome::Duplicate
        );
        let mut conflict = segments[0].clone();
        conflict.bytes[0] ^= 1;
        assert_eq!(
            reassembler.accept(conflict, 3),
            Err(FrameError::SegmentConflict)
        );
    }

    #[test]
    fn incomplete_reassembly_expires_and_frame_ids_never_wrap() {
        let segment = segment_logical_frame(Channel::Control, 9, b"hello", 20)
            .unwrap()
            .remove(0);
        let mut reassembler = OuterReassembler::new(1, 0, 10_000);
        assert_eq!(
            reassembler.accept(segment, 100).unwrap(),
            ReassemblyOutcome::Pending
        );
        assert_eq!(reassembler.expire(10_099), Vec::<u32>::new());
        assert_eq!(reassembler.expire(10_100), vec![9]);

        let mut ids = FrameIdSequence::new(u32::MAX).unwrap();
        assert_eq!(ids.take().unwrap(), u32::MAX);
        assert_eq!(ids.take(), Err(FrameError::FrameIdExhausted));
    }
}
