//! Signed DME plaintext and RFC 9180 HPKE Base protection.

use hpke::{
    Deserializable, Kem as KemTrait, OpModeR, OpModeS, Serializable, aead::ChaCha20Poly1305,
    kdf::HkdfSha256, kem::X25519HkdfSha256, single_shot_open, single_shot_seal,
};
use mesh_codec::{CborValue, DecodeLimits, decode_deterministic, encode_deterministic};
use mesh_types::{
    BundleLifetime, ConversationId, CreationSequence, MessageClass, MessageId, PacketId, Priority,
    RandomSourceId, RoutingSlot,
};
use rand_core::{OsRng, TryRngCore};
use zeroize::Zeroizing;

use crate::CryptoError;
use crate::identity::{Identity, IdentityPublic, sha256, verify_signature};

const SIGN_DOMAIN: &[u8] = b"DisasterMesh/DME-SIGN/1";
const INFO_DOMAIN: &[u8] = b"DisasterMesh/DME/1";

type DmeKem = X25519HkdfSha256;
type DmeKdf = HkdfSha256;
type DmeAead = ChaCha20Poly1305;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MessageType {
    DirectText = 1,
    CheckIn = 2,
    PrivateSos = 3,
    LocationUpdate = 4,
    DeliveryReceipt = 5,
    Cancel = 6,
}

impl TryFrom<u8> for MessageType {
    type Error = CryptoError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::DirectText),
            2 => Ok(Self::CheckIn),
            3 => Ok(Self::PrivateSos),
            4 => Ok(Self::LocationUpdate),
            5 => Ok(Self::DeliveryReceipt),
            6 => Ok(Self::Cancel),
            _ => Err(CryptoError::UnsupportedVersion),
        }
    }
}

impl MessageType {
    #[must_use]
    pub const fn class(self) -> MessageClass {
        match self {
            Self::DirectText => MessageClass::Direct,
            Self::CheckIn | Self::LocationUpdate => MessageClass::CheckIn,
            Self::PrivateSos => MessageClass::Sos,
            Self::DeliveryReceipt => MessageClass::Receipt,
            Self::Cancel => MessageClass::Cancel,
        }
    }

    #[must_use]
    pub const fn priority(self) -> Priority {
        match self {
            Self::DirectText => Priority::P2,
            Self::CheckIn | Self::LocationUpdate => Priority::P1,
            Self::PrivateSos | Self::DeliveryReceipt | Self::Cancel => Priority::P0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Location {
    Geo {
        latitude_e7: i64,
        longitude_e7: i64,
        accuracy_meters: u32,
        altitude_meters: Option<i64>,
        captured_before_send_ms: u64,
        note: String,
    },
    Manual {
        description: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageBody {
    DirectText {
        text: String,
        reply_to: Option<MessageId>,
    },
    CheckIn {
        status: u8,
        people_count: u8,
        note: String,
        location: Option<Location>,
        battery_percent: Option<u8>,
    },
    PrivateSos {
        category: u8,
        description: String,
        people_count: u8,
        severe_injury_count: u8,
        location: Option<Location>,
        movement_direction: String,
        battery_percent: Option<u8>,
    },
    LocationUpdate(Location),
    DeliveryReceipt {
        original_packet_id: PacketId,
        original_message_id: MessageId,
        receiver_note: Option<String>,
    },
    Cancel {
        target_packet_id: PacketId,
        target_message_id: MessageId,
        reason: u8,
    },
}

impl MessageBody {
    #[must_use]
    pub const fn message_type(&self) -> MessageType {
        match self {
            Self::DirectText { .. } => MessageType::DirectText,
            Self::CheckIn { .. } => MessageType::CheckIn,
            Self::PrivateSos { .. } => MessageType::PrivateSos,
            Self::LocationUpdate(_) => MessageType::LocationUpdate,
            Self::DeliveryReceipt { .. } => MessageType::DeliveryReceipt,
            Self::Cancel { .. } => MessageType::Cancel,
        }
    }

    fn validate(&self) -> Result<(), CryptoError> {
        match self {
            Self::DirectText { text, .. } => bounded_text(text, 1, 2_000, 7_800),
            Self::CheckIn {
                status,
                people_count,
                note,
                location,
                battery_percent,
            } => {
                range(*status, 1, 5)?;
                range(*people_count, 1, 99)?;
                bounded_text(note, 0, 500, 2_000)?;
                optional_battery(*battery_percent)?;
                if let Some(location) = location {
                    location.validate()?;
                }
                Ok(())
            }
            Self::PrivateSos {
                category,
                description,
                people_count,
                severe_injury_count,
                location,
                movement_direction,
                battery_percent,
            } => {
                range(*category, 1, 6)?;
                range(*people_count, 1, 99)?;
                if *severe_injury_count > *people_count {
                    return Err(CryptoError::InvalidField);
                }
                bounded_text(description, 1, 800, 3_200)?;
                bounded_text(movement_direction, 0, 100, 400)?;
                optional_battery(*battery_percent)?;
                if let Some(location) = location {
                    location.validate()?;
                }
                Ok(())
            }
            Self::LocationUpdate(location) => location.validate(),
            Self::DeliveryReceipt { receiver_note, .. } => {
                if let Some(note) = receiver_note {
                    bounded_text(note, 0, 100, 400)?;
                }
                Ok(())
            }
            Self::Cancel { reason, .. } => range(*reason, 1, 4),
        }
    }

    fn to_cbor(&self) -> CborValue {
        match self {
            Self::DirectText { text, reply_to } => CborValue::Array(vec![
                CborValue::Text(text.clone()),
                optional_id(reply_to.as_ref().map(MessageId::as_bytes)),
            ]),
            Self::CheckIn {
                status,
                people_count,
                note,
                location,
                battery_percent,
            } => CborValue::Array(vec![
                CborValue::Unsigned(u64::from(*status)),
                CborValue::Unsigned(u64::from(*people_count)),
                CborValue::Text(note.clone()),
                location.as_ref().map_or(CborValue::Null, Location::to_cbor),
                battery_percent.map_or(CborValue::Null, |value| {
                    CborValue::Unsigned(u64::from(value))
                }),
            ]),
            Self::PrivateSos {
                category,
                description,
                people_count,
                severe_injury_count,
                location,
                movement_direction,
                battery_percent,
            } => CborValue::Array(vec![
                CborValue::Unsigned(u64::from(*category)),
                CborValue::Text(description.clone()),
                CborValue::Unsigned(u64::from(*people_count)),
                CborValue::Unsigned(u64::from(*severe_injury_count)),
                location.as_ref().map_or(CborValue::Null, Location::to_cbor),
                CborValue::Text(movement_direction.clone()),
                battery_percent.map_or(CborValue::Null, |value| {
                    CborValue::Unsigned(u64::from(value))
                }),
            ]),
            Self::LocationUpdate(location) => CborValue::Array(vec![location.to_cbor()]),
            Self::DeliveryReceipt {
                original_packet_id,
                original_message_id,
                receiver_note,
            } => CborValue::Array(vec![
                CborValue::Bytes(original_packet_id.as_bytes().to_vec()),
                CborValue::Bytes(original_message_id.as_bytes().to_vec()),
                CborValue::Unsigned(1),
                receiver_note
                    .as_ref()
                    .map_or(CborValue::Null, |value| CborValue::Text(value.clone())),
            ]),
            Self::Cancel {
                target_packet_id,
                target_message_id,
                reason,
            } => CborValue::Array(vec![
                CborValue::Bytes(target_packet_id.as_bytes().to_vec()),
                CborValue::Bytes(target_message_id.as_bytes().to_vec()),
                CborValue::Unsigned(u64::from(*reason)),
            ]),
        }
    }

    fn from_cbor(message_type: MessageType, value: &CborValue) -> Result<Self, CryptoError> {
        let body = match message_type {
            MessageType::DirectText => {
                let values = array_ref(value, 2)?;
                Self::DirectText {
                    text: text(&values[0])?.to_owned(),
                    reply_to: optional_fixed::<16>(&values[1])?.map(MessageId::from),
                }
            }
            MessageType::CheckIn => {
                let values = array_ref(value, 5)?;
                Self::CheckIn {
                    status: u8_value(&values[0])?,
                    people_count: u8_value(&values[1])?,
                    note: text(&values[2])?.to_owned(),
                    location: optional_location(&values[3])?,
                    battery_percent: optional_u8(&values[4])?,
                }
            }
            MessageType::PrivateSos => {
                let values = array_ref(value, 7)?;
                Self::PrivateSos {
                    category: u8_value(&values[0])?,
                    description: text(&values[1])?.to_owned(),
                    people_count: u8_value(&values[2])?,
                    severe_injury_count: u8_value(&values[3])?,
                    location: optional_location(&values[4])?,
                    movement_direction: text(&values[5])?.to_owned(),
                    battery_percent: optional_u8(&values[6])?,
                }
            }
            MessageType::LocationUpdate => {
                let values = array_ref(value, 1)?;
                Self::LocationUpdate(Location::from_cbor(&values[0])?)
            }
            MessageType::DeliveryReceipt => {
                let values = array_ref(value, 4)?;
                if unsigned(&values[2])? != 1 {
                    return Err(CryptoError::InvalidField);
                }
                Self::DeliveryReceipt {
                    original_packet_id: PacketId::from(fixed::<16>(&values[0])?),
                    original_message_id: MessageId::from(fixed::<16>(&values[1])?),
                    receiver_note: optional_text(&values[3])?,
                }
            }
            MessageType::Cancel => {
                let values = array_ref(value, 3)?;
                Self::Cancel {
                    target_packet_id: PacketId::from(fixed::<16>(&values[0])?),
                    target_message_id: MessageId::from(fixed::<16>(&values[1])?),
                    reason: u8_value(&values[2])?,
                }
            }
        };
        body.validate()?;
        Ok(body)
    }
}

impl Location {
    fn validate(&self) -> Result<(), CryptoError> {
        match self {
            Self::Geo {
                latitude_e7,
                longitude_e7,
                accuracy_meters,
                captured_before_send_ms,
                note,
                ..
            } => {
                if !(-900_000_000..=900_000_000).contains(latitude_e7)
                    || !(-1_800_000_000..=1_800_000_000).contains(longitude_e7)
                    || *accuracy_meters > 50_000
                    || *captured_before_send_ms > 86_400_000
                {
                    return Err(CryptoError::InvalidField);
                }
                bounded_text(note, 0, 200, 800)
            }
            Self::Manual { description } => bounded_text(description, 1, 200, 800),
        }
    }

    fn to_cbor(&self) -> CborValue {
        match self {
            Self::Geo {
                latitude_e7,
                longitude_e7,
                accuracy_meters,
                altitude_meters,
                captured_before_send_ms,
                note,
            } => CborValue::Array(vec![
                CborValue::Unsigned(1),
                signed(*latitude_e7),
                signed(*longitude_e7),
                CborValue::Unsigned(u64::from(*accuracy_meters)),
                altitude_meters.map_or(CborValue::Null, signed),
                CborValue::Unsigned(*captured_before_send_ms),
                CborValue::Text(note.clone()),
            ]),
            Self::Manual { description } => CborValue::Array(vec![
                CborValue::Unsigned(2),
                CborValue::Text(description.clone()),
            ]),
        }
    }

    fn from_cbor(value: &CborValue) -> Result<Self, CryptoError> {
        let CborValue::Array(values) = value else {
            return Err(CryptoError::InvalidField);
        };
        let location = match values.first().map(unsigned).transpose()? {
            Some(1) if values.len() == 7 => Self::Geo {
                latitude_e7: integer(&values[1])?,
                longitude_e7: integer(&values[2])?,
                accuracy_meters: u32::try_from(unsigned(&values[3])?)
                    .map_err(|_| CryptoError::InvalidField)?,
                altitude_meters: optional_integer(&values[4])?,
                captured_before_send_ms: unsigned(&values[5])?,
                note: text(&values[6])?.to_owned(),
            },
            Some(2) if values.len() == 2 => Self::Manual {
                description: text(&values[1])?.to_owned(),
            },
            _ => return Err(CryptoError::InvalidField),
        };
        location.validate()?;
        Ok(location)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmeAad {
    pub packet_id: PacketId,
    pub destination: RoutingSlot,
    pub message_class: MessageClass,
    pub priority: Priority,
    pub lifetime: BundleLifetime,
    pub hop_limit: u8,
    pub source: RandomSourceId,
    pub creation_sequence: CreationSequence,
}

impl DmeAad {
    pub fn encode(&self) -> Result<Vec<u8>, CryptoError> {
        if self.hop_limit == 0 || self.hop_limit > 32 {
            return Err(CryptoError::InvalidField);
        }
        encode_deterministic(&CborValue::Array(vec![
            CborValue::Unsigned(1),
            CborValue::Bytes(self.packet_id.as_bytes().to_vec()),
            CborValue::Bytes(self.destination.as_bytes().to_vec()),
            CborValue::Unsigned(self.message_class as u64),
            CborValue::Unsigned(self.priority as u64),
            CborValue::Unsigned(self.lifetime.as_millis()),
            CborValue::Unsigned(u64::from(self.hop_limit)),
            CborValue::Bytes(self.source.as_bytes().to_vec()),
            CborValue::Bytes(self.creation_sequence.as_bytes().to_vec()),
        ]))
        .map_err(|_| CryptoError::InvalidField)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmePlaintext {
    pub packet_id: PacketId,
    pub message_id: MessageId,
    pub conversation_id: ConversationId,
    pub sender_signing_public_key: [u8; 32],
    pub sender_hpke_public_key: [u8; 32],
    pub recipient_identity_hash: [u8; 32],
    pub sender_sequence: u64,
    pub reply_routing_slot: RoutingSlot,
    pub created_time_ms: Option<u64>,
    pub body: MessageBody,
    pub signature: [u8; 64],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmeDraft {
    pub packet_id: PacketId,
    pub message_id: MessageId,
    pub conversation_id: ConversationId,
    pub sender_sequence: u64,
    pub created_time_ms: Option<u64>,
    pub body: MessageBody,
}

impl DmePlaintext {
    pub fn create(
        sender: &Identity,
        recipient: &IdentityPublic,
        draft: DmeDraft,
        aad_hash: &[u8; 32],
    ) -> Result<Self, CryptoError> {
        let mut plaintext = Self {
            packet_id: draft.packet_id,
            message_id: draft.message_id,
            conversation_id: draft.conversation_id,
            sender_signing_public_key: sender.public().signing_public_key,
            sender_hpke_public_key: sender.public().hpke_public_key,
            recipient_identity_hash: recipient.identity_id.into_bytes(),
            sender_sequence: draft.sender_sequence,
            reply_routing_slot: sender.public().inbound_routing_slot,
            created_time_ms: draft.created_time_ms,
            body: draft.body,
            signature: [0; 64],
        };
        plaintext.validate()?;
        plaintext.signature = sender.sign(&plaintext.signature_input(aad_hash)?);
        Ok(plaintext)
    }

    pub fn encode(&self) -> Result<Vec<u8>, CryptoError> {
        self.validate()?;
        let mut values = self.unsigned_values();
        values.push(CborValue::Bytes(self.signature.to_vec()));
        encode_deterministic(&CborValue::Array(values)).map_err(|_| CryptoError::InvalidField)
    }

    pub fn decode(input: &[u8]) -> Result<Self, CryptoError> {
        let value = decode_deterministic(input, DecodeLimits::default())
            .map_err(|_| CryptoError::InvalidCiphertext)?;
        let values = expect_array(value, 13)?;
        if unsigned(&values[0])? != 1 {
            return Err(CryptoError::UnsupportedVersion);
        }
        let message_type = MessageType::try_from(u8_value(&values[1])?)?;
        let plaintext = Self {
            packet_id: PacketId::from(fixed::<16>(&values[2])?),
            message_id: MessageId::from(fixed::<16>(&values[3])?),
            conversation_id: ConversationId::from(fixed::<16>(&values[4])?),
            sender_signing_public_key: fixed::<32>(&values[5])?,
            sender_hpke_public_key: fixed::<32>(&values[6])?,
            recipient_identity_hash: fixed::<32>(&values[7])?,
            sender_sequence: unsigned(&values[8])?,
            reply_routing_slot: RoutingSlot::from(fixed::<16>(&values[9])?),
            created_time_ms: optional_u64(&values[10])?,
            body: MessageBody::from_cbor(message_type, &values[11])?,
            signature: fixed::<64>(&values[12])?,
        };
        plaintext.validate()?;
        Ok(plaintext)
    }

    pub fn verify_signature(&self, aad_hash: &[u8; 32]) -> Result<(), CryptoError> {
        verify_signature(
            &self.sender_signing_public_key,
            &self.signature_input(aad_hash)?,
            &self.signature,
        )
    }

    fn unsigned_values(&self) -> Vec<CborValue> {
        vec![
            CborValue::Unsigned(1),
            CborValue::Unsigned(self.body.message_type() as u64),
            CborValue::Bytes(self.packet_id.as_bytes().to_vec()),
            CborValue::Bytes(self.message_id.as_bytes().to_vec()),
            CborValue::Bytes(self.conversation_id.as_bytes().to_vec()),
            CborValue::Bytes(self.sender_signing_public_key.to_vec()),
            CborValue::Bytes(self.sender_hpke_public_key.to_vec()),
            CborValue::Bytes(self.recipient_identity_hash.to_vec()),
            CborValue::Unsigned(self.sender_sequence),
            CborValue::Bytes(self.reply_routing_slot.as_bytes().to_vec()),
            self.created_time_ms
                .map_or(CborValue::Null, CborValue::Unsigned),
            self.body.to_cbor(),
        ]
    }

    fn signature_input(&self, aad_hash: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
        let unsigned = encode_deterministic(&CborValue::Array(self.unsigned_values()))
            .map_err(|_| CryptoError::InvalidField)?;
        let mut input = Vec::with_capacity(SIGN_DOMAIN.len() + 64);
        input.extend_from_slice(SIGN_DOMAIN);
        input.extend_from_slice(&sha256(&unsigned));
        input.extend_from_slice(aad_hash);
        Ok(input)
    }

    fn validate(&self) -> Result<(), CryptoError> {
        if self.sender_sequence > i64::MAX as u64 {
            return Err(CryptoError::InvalidField);
        }
        self.body.validate()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedDme {
    pub encapsulated_key: [u8; 32],
    pub aad_hash: [u8; 32],
    pub ciphertext: Vec<u8>,
}

impl EncryptedDme {
    pub fn encode(&self) -> Result<Vec<u8>, CryptoError> {
        if self.ciphertext.is_empty() || self.ciphertext.len() > 8_118 {
            return Err(CryptoError::SizeLimit);
        }
        let encoded = encode_deterministic(&CborValue::Array(vec![
            CborValue::Unsigned(1),
            CborValue::Unsigned(1),
            CborValue::Bytes(self.encapsulated_key.to_vec()),
            CborValue::Bytes(self.aad_hash.to_vec()),
            CborValue::Bytes(self.ciphertext.clone()),
        ]))
        .map_err(|_| CryptoError::InvalidCiphertext)?;
        if encoded.len() > 8_192 {
            return Err(CryptoError::SizeLimit);
        }
        Ok(encoded)
    }

    pub fn decode(input: &[u8]) -> Result<Self, CryptoError> {
        if input.len() > 8_192 {
            return Err(CryptoError::SizeLimit);
        }
        let values = expect_array(
            decode_deterministic(input, DecodeLimits::default())
                .map_err(|_| CryptoError::InvalidCiphertext)?,
            5,
        )?;
        if unsigned(&values[0])? != 1 || unsigned(&values[1])? != 1 {
            return Err(CryptoError::UnsupportedVersion);
        }
        let result = Self {
            encapsulated_key: fixed::<32>(&values[2])?,
            aad_hash: fixed::<32>(&values[3])?,
            ciphertext: bytes(&values[4])?.to_vec(),
        };
        if result.ciphertext.is_empty() || result.ciphertext.len() > 8_118 {
            return Err(CryptoError::SizeLimit);
        }
        Ok(result)
    }
}

pub fn seal_dme(
    sender: &Identity,
    recipient: &IdentityPublic,
    aad: DmeAad,
    draft: DmeDraft,
) -> Result<(EncryptedDme, DmePlaintext), CryptoError> {
    if aad.packet_id != draft.packet_id
        || aad.message_class != draft.body.message_type().class()
        || aad.priority != draft.body.message_type().priority()
    {
        return Err(CryptoError::InvalidField);
    }
    let aad_bytes = aad.encode()?;
    let aad_hash = sha256(&aad_bytes);
    let plaintext = DmePlaintext::create(sender, recipient, draft, &aad_hash)?;
    let plaintext_bytes = Zeroizing::new(plaintext.encode()?);
    let public_key = <DmeKem as KemTrait>::PublicKey::from_bytes(&recipient.hpke_public_key)
        .map_err(|_| CryptoError::InvalidKey)?;
    let mut info = Vec::with_capacity(INFO_DOMAIN.len() + 16);
    info.extend_from_slice(INFO_DOMAIN);
    info.extend_from_slice(aad.packet_id.as_bytes());
    let mut rng = OsRng.unwrap_err();
    let (encapsulated, ciphertext) = single_shot_seal::<DmeAead, DmeKdf, DmeKem, _>(
        &OpModeS::Base,
        &public_key,
        &info,
        &plaintext_bytes,
        &aad_bytes,
        &mut rng,
    )
    .map_err(|_| CryptoError::InvalidCiphertext)?;
    let encapsulated_key: [u8; 32] = encapsulated
        .to_bytes()
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidKey)?;
    let encrypted = EncryptedDme {
        encapsulated_key,
        aad_hash,
        ciphertext,
    };
    encrypted.encode()?;
    Ok((encrypted, plaintext))
}

pub fn open_dme(
    recipient: &Identity,
    aad: DmeAad,
    encrypted: &EncryptedDme,
) -> Result<DmePlaintext, CryptoError> {
    let aad_bytes = aad.encode()?;
    let aad_hash = sha256(&aad_bytes);
    if encrypted.aad_hash != aad_hash {
        return Err(CryptoError::AadMismatch);
    }
    let secret_key =
        <DmeKem as KemTrait>::PrivateKey::from_bytes(&recipient.hpke_secret().to_bytes())
            .map_err(|_| CryptoError::InvalidKey)?;
    let encapsulated = <DmeKem as KemTrait>::EncappedKey::from_bytes(&encrypted.encapsulated_key)
        .map_err(|_| CryptoError::InvalidCiphertext)?;
    let mut info = Vec::with_capacity(INFO_DOMAIN.len() + 16);
    info.extend_from_slice(INFO_DOMAIN);
    info.extend_from_slice(aad.packet_id.as_bytes());
    let plaintext_bytes = Zeroizing::new(
        single_shot_open::<DmeAead, DmeKdf, DmeKem>(
            &OpModeR::Base,
            &secret_key,
            &encapsulated,
            &info,
            &encrypted.ciphertext,
            &aad_bytes,
        )
        .map_err(|_| CryptoError::InvalidCiphertext)?,
    );
    let plaintext = DmePlaintext::decode(&plaintext_bytes)?;
    if plaintext.packet_id != aad.packet_id
        || plaintext.recipient_identity_hash != recipient.public().identity_id.into_bytes()
        || plaintext.body.message_type().class() != aad.message_class
        || plaintext.body.message_type().priority() != aad.priority
    {
        return Err(CryptoError::RecipientMismatch);
    }
    plaintext.verify_signature(&aad_hash)?;
    Ok(plaintext)
}

pub fn validate_control_signer(
    body: &MessageBody,
    signer_public_key: &[u8; 32],
    original_sender_public_key: &[u8; 32],
    original_recipient_public_key: &[u8; 32],
) -> Result<(), CryptoError> {
    let expected = match body {
        MessageBody::DeliveryReceipt { .. } => original_recipient_public_key,
        MessageBody::Cancel { .. } => original_sender_public_key,
        _ => return Err(CryptoError::InvalidField),
    };
    if signer_public_key != expected {
        return Err(CryptoError::InvalidSignature);
    }
    Ok(())
}

fn bounded_text(
    value: &str,
    min_scalars: usize,
    max_scalars: usize,
    max_bytes: usize,
) -> Result<(), CryptoError> {
    let scalars = value.chars().count();
    if !(min_scalars..=max_scalars).contains(&scalars) || value.len() > max_bytes {
        return Err(CryptoError::SizeLimit);
    }
    Ok(())
}

fn range(value: u8, minimum: u8, maximum: u8) -> Result<(), CryptoError> {
    if !(minimum..=maximum).contains(&value) {
        return Err(CryptoError::InvalidField);
    }
    Ok(())
}

fn optional_battery(value: Option<u8>) -> Result<(), CryptoError> {
    if value.is_some_and(|value| value > 100) {
        return Err(CryptoError::InvalidField);
    }
    Ok(())
}

fn signed(value: i64) -> CborValue {
    if value >= 0 {
        CborValue::Unsigned(value as u64)
    } else {
        CborValue::Negative(value)
    }
}

fn optional_id(value: Option<&[u8; 16]>) -> CborValue {
    value.map_or(CborValue::Null, |value| CborValue::Bytes(value.to_vec()))
}

fn optional_location(value: &CborValue) -> Result<Option<Location>, CryptoError> {
    if matches!(value, CborValue::Null) {
        Ok(None)
    } else {
        Location::from_cbor(value).map(Some)
    }
}

fn optional_text(value: &CborValue) -> Result<Option<String>, CryptoError> {
    if matches!(value, CborValue::Null) {
        Ok(None)
    } else {
        text(value).map(str::to_owned).map(Some)
    }
}

fn optional_u8(value: &CborValue) -> Result<Option<u8>, CryptoError> {
    if matches!(value, CborValue::Null) {
        Ok(None)
    } else {
        u8_value(value).map(Some)
    }
}

fn optional_u64(value: &CborValue) -> Result<Option<u64>, CryptoError> {
    if matches!(value, CborValue::Null) {
        Ok(None)
    } else {
        unsigned(value).map(Some)
    }
}

fn optional_integer(value: &CborValue) -> Result<Option<i64>, CryptoError> {
    if matches!(value, CborValue::Null) {
        Ok(None)
    } else {
        integer(value).map(Some)
    }
}

fn optional_fixed<const N: usize>(value: &CborValue) -> Result<Option<[u8; N]>, CryptoError> {
    if matches!(value, CborValue::Null) {
        Ok(None)
    } else {
        fixed(value).map(Some)
    }
}

fn expect_array(value: CborValue, length: usize) -> Result<Vec<CborValue>, CryptoError> {
    let CborValue::Array(values) = value else {
        return Err(CryptoError::InvalidField);
    };
    if values.len() != length {
        return Err(CryptoError::InvalidField);
    }
    Ok(values)
}

fn array_ref(value: &CborValue, length: usize) -> Result<&[CborValue], CryptoError> {
    let CborValue::Array(values) = value else {
        return Err(CryptoError::InvalidField);
    };
    if values.len() != length {
        return Err(CryptoError::InvalidField);
    }
    Ok(values)
}

fn unsigned(value: &CborValue) -> Result<u64, CryptoError> {
    let CborValue::Unsigned(value) = value else {
        return Err(CryptoError::InvalidField);
    };
    Ok(*value)
}

fn integer(value: &CborValue) -> Result<i64, CryptoError> {
    match value {
        CborValue::Unsigned(value) => i64::try_from(*value).map_err(|_| CryptoError::InvalidField),
        CborValue::Negative(value) => Ok(*value),
        _ => Err(CryptoError::InvalidField),
    }
}

fn u8_value(value: &CborValue) -> Result<u8, CryptoError> {
    u8::try_from(unsigned(value)?).map_err(|_| CryptoError::InvalidField)
}

fn text(value: &CborValue) -> Result<&str, CryptoError> {
    let CborValue::Text(value) = value else {
        return Err(CryptoError::InvalidField);
    };
    Ok(value)
}

fn bytes(value: &CborValue) -> Result<&[u8], CryptoError> {
    let CborValue::Bytes(value) = value else {
        return Err(CryptoError::InvalidCiphertext);
    };
    Ok(value)
}

fn fixed<const N: usize>(value: &CborValue) -> Result<[u8; N], CryptoError> {
    bytes(value)?
        .try_into()
        .map_err(|_| CryptoError::InvalidField)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aad(packet_id: PacketId) -> DmeAad {
        DmeAad {
            packet_id,
            destination: RoutingSlot::from([7; 16]),
            message_class: MessageClass::Direct,
            priority: Priority::P2,
            lifetime: BundleLifetime::from_millis(259_200_000).unwrap(),
            hop_limit: 12,
            source: RandomSourceId::from([8; 16]),
            creation_sequence: CreationSequence::from_u64(9),
        }
    }

    #[test]
    fn hpke_round_trip_and_wrong_inputs_fail() {
        let sender = Identity::generate().unwrap();
        let recipient = Identity::generate().unwrap();
        let wrong = Identity::generate().unwrap();
        let packet_id = PacketId::from([1; 16]);
        let context = aad(packet_id);
        let (encrypted, expected) = seal_dme(
            &sender,
            recipient.public(),
            context,
            DmeDraft {
                packet_id,
                message_id: MessageId::from([2; 16]),
                conversation_id: ConversationId::from([3; 16]),
                sender_sequence: 1,
                created_time_ms: None,
                body: MessageBody::DirectText {
                    text: "offline hello".into(),
                    reply_to: None,
                },
            },
        )
        .unwrap();
        assert_eq!(open_dme(&recipient, context, &encrypted).unwrap(), expected);
        assert_eq!(
            open_dme(&wrong, context, &encrypted),
            Err(CryptoError::InvalidCiphertext)
        );
        let mutated_aad = [
            DmeAad {
                packet_id: PacketId::from([2; 16]),
                ..context
            },
            DmeAad {
                destination: RoutingSlot::from([6; 16]),
                ..context
            },
            DmeAad {
                message_class: MessageClass::CheckIn,
                ..context
            },
            DmeAad {
                priority: Priority::P1,
                ..context
            },
            DmeAad {
                lifetime: BundleLifetime::from_millis(60_000).unwrap(),
                ..context
            },
            DmeAad {
                hop_limit: 13,
                ..context
            },
            DmeAad {
                source: RandomSourceId::from([9; 16]),
                ..context
            },
            DmeAad {
                creation_sequence: CreationSequence::from_u64(10),
                ..context
            },
        ];
        for changed in mutated_aad {
            assert_eq!(
                open_dme(&recipient, changed, &encrypted),
                Err(CryptoError::AadMismatch)
            );
        }
    }

    #[test]
    fn signature_and_ciphertext_mutation_fail() {
        let sender = Identity::generate().unwrap();
        let recipient = Identity::generate().unwrap();
        let context = aad(PacketId::from([4; 16]));
        let (mut encrypted, _) = seal_dme(
            &sender,
            recipient.public(),
            context,
            DmeDraft {
                packet_id: context.packet_id,
                message_id: MessageId::from([5; 16]),
                conversation_id: ConversationId::from([6; 16]),
                sender_sequence: 1,
                created_time_ms: None,
                body: MessageBody::DirectText {
                    text: "authenticated".into(),
                    reply_to: None,
                },
            },
        )
        .unwrap();
        encrypted.ciphertext[0] ^= 1;
        assert_eq!(
            open_dme(&recipient, context, &encrypted),
            Err(CryptoError::InvalidCiphertext)
        );
    }

    #[test]
    fn signature_substitution_and_control_role_fail() {
        let sender = Identity::generate().unwrap();
        let recipient = Identity::generate().unwrap();
        let context = aad(PacketId::from([10; 16]));
        let (_, mut plaintext) = seal_dme(
            &sender,
            recipient.public(),
            context,
            DmeDraft {
                packet_id: context.packet_id,
                message_id: MessageId::from([11; 16]),
                conversation_id: ConversationId::from([12; 16]),
                sender_sequence: 1,
                created_time_ms: None,
                body: MessageBody::DirectText {
                    text: "signed".into(),
                    reply_to: None,
                },
            },
        )
        .unwrap();
        plaintext.signature[0] ^= 1;
        assert_eq!(
            plaintext.verify_signature(&sha256(&context.encode().unwrap())),
            Err(CryptoError::InvalidSignature)
        );

        let receipt = MessageBody::DeliveryReceipt {
            original_packet_id: PacketId::from([1; 16]),
            original_message_id: MessageId::from([2; 16]),
            receiver_note: None,
        };
        assert_eq!(
            validate_control_signer(
                &receipt,
                &sender.public().signing_public_key,
                &sender.public().signing_public_key,
                &recipient.public().signing_public_key,
            ),
            Err(CryptoError::InvalidSignature)
        );
    }

    #[test]
    fn dual_text_and_sos_semantic_limits_are_enforced() {
        assert!(bounded_text(&"é".repeat(2_000), 1, 2_000, 7_800).is_ok());
        assert!(bounded_text(&"😀".repeat(2_000), 1, 2_000, 7_800).is_err());
        assert_eq!(
            MessageBody::PrivateSos {
                category: 1,
                description: "help".into(),
                people_count: 1,
                severe_injury_count: 2,
                location: None,
                movement_direction: String::new(),
                battery_percent: None,
            }
            .validate(),
            Err(CryptoError::InvalidField)
        );
    }

    #[test]
    fn receipt_is_never_mistyped() {
        let body = MessageBody::DeliveryReceipt {
            original_packet_id: PacketId::from([1; 16]),
            original_message_id: MessageId::from([2; 16]),
            receiver_note: None,
        };
        assert_eq!(body.message_type(), MessageType::DeliveryReceipt);
        assert_eq!(body.message_type().class(), MessageClass::Receipt);
    }
}
