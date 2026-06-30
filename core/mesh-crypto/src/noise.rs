//! BLE-CLA v1 VERSION_HELLO and Noise XX secure-frame state.

use core::fmt;

use mesh_codec::ble::EncryptedFrame;
use mesh_codec::{CborValue, DecodeLimits, decode_deterministic, encode_deterministic};
use mesh_types::generated_contracts::protocol;
use snow::{Builder, HandshakeState, TransportState, params::NoiseParams};

use crate::identity::sha256;
use crate::{CryptoError, Identity};

const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
const PROLOGUE_DOMAIN: &[u8] = b"DisasterMesh/BLE-CLA/1";
const NOISE_MAX_MESSAGE: usize = 65_535;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VersionHello {
    pub protocol_minor: u32,
    pub beacon_id: [u8; 8],
    pub max_control_frame: u16,
    pub max_data_chunk: u16,
    pub capabilities: u32,
    pub session_nonce: [u8; 16],
}

impl VersionHello {
    pub fn encode(&self) -> Result<Vec<u8>, CryptoError> {
        self.validate()?;
        encode_deterministic(&CborValue::Array(vec![
            CborValue::Unsigned(protocol::PROTOCOL_MAJOR),
            CborValue::Unsigned(u64::from(self.protocol_minor)),
            CborValue::Bytes(self.beacon_id.to_vec()),
            CborValue::Unsigned(u64::from(self.max_control_frame)),
            CborValue::Unsigned(u64::from(self.max_data_chunk)),
            CborValue::Unsigned(u64::from(self.capabilities)),
            CborValue::Bytes(self.session_nonce.to_vec()),
        ]))
        .map_err(|_| CryptoError::InvalidHandshake)
    }

    pub fn decode(input: &[u8]) -> Result<Self, CryptoError> {
        if input.len() > 512 {
            return Err(CryptoError::SizeLimit);
        }
        let CborValue::Array(values) = decode_deterministic(input, DecodeLimits::default())
            .map_err(|_| CryptoError::InvalidHandshake)?
        else {
            return Err(CryptoError::InvalidHandshake);
        };
        if values.len() != 7 || unsigned(&values[0])? != protocol::PROTOCOL_MAJOR {
            return Err(CryptoError::UnsupportedVersion);
        }
        let hello = Self {
            protocol_minor: u32::try_from(unsigned(&values[1])?)
                .map_err(|_| CryptoError::InvalidHandshake)?,
            beacon_id: fixed(&values[2])?,
            max_control_frame: u16::try_from(unsigned(&values[3])?)
                .map_err(|_| CryptoError::InvalidHandshake)?,
            max_data_chunk: u16::try_from(unsigned(&values[4])?)
                .map_err(|_| CryptoError::InvalidHandshake)?,
            capabilities: u32::try_from(unsigned(&values[5])?)
                .map_err(|_| CryptoError::InvalidHandshake)?,
            session_nonce: fixed(&values[6])?,
        };
        hello.validate()?;
        Ok(hello)
    }

    fn validate(&self) -> Result<(), CryptoError> {
        if !(64..=4096).contains(&self.max_control_frame)
            || !(1..=4096).contains(&self.max_data_chunk)
            || u64::from(self.capabilities) & !protocol::CAPABILITIES_KNOWN_MASK != 0
            || u64::from(self.capabilities) & protocol::CAPABILITIES_NOISE_XX == 0
        {
            return Err(CryptoError::InvalidHandshake);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NegotiatedHello {
    pub protocol_minor: u32,
    pub max_control_frame: u16,
    pub max_data_chunk: u16,
    pub capabilities: u32,
}

pub fn negotiate_hello(
    local: VersionHello,
    remote: VersionHello,
) -> Result<NegotiatedHello, CryptoError> {
    local.validate()?;
    remote.validate()?;
    let capabilities = local.capabilities & remote.capabilities;
    if u64::from(capabilities) & protocol::CAPABILITIES_NOISE_XX == 0 {
        return Err(CryptoError::InvalidHandshake);
    }
    Ok(NegotiatedHello {
        protocol_minor: local.protocol_minor.min(remote.protocol_minor),
        max_control_frame: local.max_control_frame.min(remote.max_control_frame),
        max_data_chunk: local.max_data_chunk.min(remote.max_data_chunk),
        capabilities,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoiseRole {
    Initiator,
    Responder,
}

pub struct NoiseHandshake {
    role: NoiseRole,
    state: HandshakeState,
}

impl fmt::Debug for NoiseHandshake {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NoiseHandshake")
            .field("role", &self.role)
            .field("state", &"[REDACTED]")
            .finish()
    }
}

impl NoiseHandshake {
    pub fn new(
        identity: &Identity,
        role: NoiseRole,
        initiator_hello: &[u8],
        responder_hello: &[u8],
    ) -> Result<Self, CryptoError> {
        VersionHello::decode(initiator_hello)?;
        VersionHello::decode(responder_hello)?;
        let prologue = handshake_prologue(initiator_hello, responder_hello);
        let params: NoiseParams = NOISE_PATTERN
            .parse()
            .map_err(|_| CryptoError::InvalidHandshake)?;
        let private_key = identity.noise_secret_bytes();
        let builder = Builder::new(params)
            .prologue(&prologue)
            .map_err(|_| CryptoError::InvalidHandshake)?
            .local_private_key(private_key.as_ref())
            .map_err(|_| CryptoError::InvalidHandshake)?;
        let state = match role {
            NoiseRole::Initiator => builder.build_initiator(),
            NoiseRole::Responder => builder.build_responder(),
        }
        .map_err(|_| CryptoError::InvalidHandshake)?;
        Ok(Self { role, state })
    }

    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if payload.len() > 512 {
            return Err(CryptoError::SizeLimit);
        }
        let mut output = vec![0; NOISE_MAX_MESSAGE];
        let written = self
            .state
            .write_message(payload, &mut output)
            .map_err(|_| CryptoError::InvalidHandshake)?;
        output.truncate(written);
        Ok(output)
    }

    pub fn read_message(&mut self, message: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if message.len() > NOISE_MAX_MESSAGE {
            return Err(CryptoError::SizeLimit);
        }
        let mut output = vec![0; NOISE_MAX_MESSAGE];
        let read = self
            .state
            .read_message(message, &mut output)
            .map_err(|_| CryptoError::InvalidHandshake)?;
        output.truncate(read);
        Ok(output)
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.state.is_handshake_finished()
    }

    pub fn into_transport(self) -> Result<NoiseTransport, CryptoError> {
        if !self.is_finished() {
            return Err(CryptoError::InvalidHandshake);
        }
        let state = self
            .state
            .into_transport_mode()
            .map_err(|_| CryptoError::InvalidHandshake)?;
        Ok(NoiseTransport {
            state,
            send_sequence: [0; 2],
            receive_sequence: [0; 2],
        })
    }
}

pub struct NoiseTransport {
    state: TransportState,
    send_sequence: [u32; 2],
    receive_sequence: [u32; 2],
}

impl fmt::Debug for NoiseTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NoiseTransport")
            .field("state", &"[REDACTED]")
            .field("send_sequence", &self.send_sequence)
            .field("receive_sequence", &self.receive_sequence)
            .finish()
    }
}

impl NoiseTransport {
    pub fn seal_frame(
        &mut self,
        frame_type: u8,
        stream_id: u32,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, CryptoError> {
        let index = stream_index(stream_id)?;
        let sequence = self.send_sequence[index];
        let frame = EncryptedFrame {
            frame_type,
            stream_id,
            sequence,
            payload,
        }
        .encode()
        .map_err(|_| CryptoError::InvalidField)?;
        if frame.len() + 16 > NOISE_MAX_MESSAGE {
            return Err(CryptoError::SizeLimit);
        }
        let mut output = vec![0; frame.len() + 16];
        let written = self
            .state
            .write_message(&frame, &mut output)
            .map_err(|_| CryptoError::InvalidCiphertext)?;
        self.send_sequence[index] = sequence
            .checked_add(1)
            .ok_or(CryptoError::InvalidFrameSequence)?;
        output.truncate(written);
        Ok(output)
    }

    pub fn open_frame(&mut self, ciphertext: &[u8]) -> Result<EncryptedFrame, CryptoError> {
        if ciphertext.len() > NOISE_MAX_MESSAGE || ciphertext.len() < 16 {
            return Err(CryptoError::SizeLimit);
        }
        let mut plaintext = vec![0; ciphertext.len()];
        let read = self
            .state
            .read_message(ciphertext, &mut plaintext)
            .map_err(|_| CryptoError::InvalidCiphertext)?;
        plaintext.truncate(read);
        let frame = EncryptedFrame::decode(&plaintext).map_err(|_| CryptoError::InvalidField)?;
        let index = stream_index(frame.stream_id)?;
        if frame.sequence != self.receive_sequence[index] {
            return Err(CryptoError::InvalidFrameSequence);
        }
        self.receive_sequence[index] = frame
            .sequence
            .checked_add(1)
            .ok_or(CryptoError::InvalidFrameSequence)?;
        Ok(frame)
    }
}

#[must_use]
pub fn handshake_prologue(initiator_hello: &[u8], responder_hello: &[u8]) -> [u8; 32] {
    let mut input =
        Vec::with_capacity(PROLOGUE_DOMAIN.len() + initiator_hello.len() + responder_hello.len());
    input.extend_from_slice(PROLOGUE_DOMAIN);
    input.extend_from_slice(initiator_hello);
    input.extend_from_slice(responder_hello);
    sha256(&input)
}

fn stream_index(stream_id: u32) -> Result<usize, CryptoError> {
    match stream_id {
        0 => Ok(0),
        1 => Ok(1),
        _ => Err(CryptoError::InvalidField),
    }
}

fn unsigned(value: &CborValue) -> Result<u64, CryptoError> {
    let CborValue::Unsigned(value) = value else {
        return Err(CryptoError::InvalidHandshake);
    };
    Ok(*value)
}

fn fixed<const N: usize>(value: &CborValue) -> Result<[u8; N], CryptoError> {
    let CborValue::Bytes(value) = value else {
        return Err(CryptoError::InvalidHandshake);
    };
    value
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidHandshake)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(beacon: u8, nonce: u8) -> VersionHello {
        VersionHello {
            protocol_minor: 0,
            beacon_id: [beacon; 8],
            max_control_frame: 4096,
            max_data_chunk: 1024,
            capabilities: protocol::CAPABILITIES_KNOWN_MASK as u32,
            session_nonce: [nonce; 16],
        }
    }

    #[test]
    fn version_hello_is_canonical_and_bounded() {
        let encoded = hello(1, 2).encode().unwrap();
        assert_eq!(VersionHello::decode(&encoded).unwrap(), hello(1, 2));

        let mut invalid = hello(1, 2);
        invalid.max_control_frame = 63;
        assert_eq!(invalid.encode(), Err(CryptoError::InvalidHandshake));
        invalid = hello(1, 2);
        invalid.capabilities |= 1 << 31;
        assert_eq!(invalid.encode(), Err(CryptoError::InvalidHandshake));
    }

    #[test]
    fn noise_xx_round_trip_and_frame_sequences() {
        let initiator_identity = Identity::generate().unwrap();
        let responder_identity = Identity::generate().unwrap();
        let initiator_hello = hello(1, 2).encode().unwrap();
        let responder_hello = hello(3, 4).encode().unwrap();
        let mut initiator = NoiseHandshake::new(
            &initiator_identity,
            NoiseRole::Initiator,
            &initiator_hello,
            &responder_hello,
        )
        .unwrap();
        let mut responder = NoiseHandshake::new(
            &responder_identity,
            NoiseRole::Responder,
            &initiator_hello,
            &responder_hello,
        )
        .unwrap();

        let first = initiator.write_message(b"").unwrap();
        assert_eq!(responder.read_message(&first).unwrap(), b"");
        let second = responder.write_message(b"").unwrap();
        assert_eq!(initiator.read_message(&second).unwrap(), b"");
        let third = initiator.write_message(b"").unwrap();
        assert_eq!(responder.read_message(&third).unwrap(), b"");
        assert!(initiator.is_finished());
        assert!(responder.is_finished());

        let mut initiator = initiator.into_transport().unwrap();
        let mut responder = responder.into_transport().unwrap();
        let ciphertext = initiator.seal_frame(0x10, 0, vec![1, 2, 3]).unwrap();
        let frame = responder.open_frame(&ciphertext).unwrap();
        assert_eq!(frame.sequence, 0);
        assert_eq!(frame.payload, vec![1, 2, 3]);

        let response = responder.seal_frame(0x19, 0, vec![9; 8]).unwrap();
        assert_eq!(initiator.open_frame(&response).unwrap().sequence, 0);
    }

    #[test]
    fn hello_order_changes_the_prologue() {
        let first = hello(1, 2).encode().unwrap();
        let second = hello(3, 4).encode().unwrap();
        assert_ne!(
            handshake_prologue(&first, &second),
            handshake_prologue(&second, &first)
        );
    }
}
