//! Byte-exact BLE-CLA v1 frame headers.

use core::fmt;

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
}
