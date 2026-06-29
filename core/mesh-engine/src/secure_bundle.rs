//! End-to-end secured DME payload integrated with DM-BP7-1.

use core::fmt;

use mesh_bundle::{Bundle, DecodedBundle, DmeCiphertext, RoutingBlock};
use mesh_crypto::{
    DmeAad, DmeDraft, DmePlaintext, EncryptedDme, Identity, IdentityPublic, MessageBody, open_dme,
    seal_dme,
};
use mesh_types::{
    BundleLifetime, ConversationId, CopyTokens, CreationSequence, HopState, MessageId, PacketId,
    PayloadHash, RandomSourceId, RoutingSlot,
};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecureMessageDraft {
    pub packet_id: PacketId,
    pub message_id: MessageId,
    pub conversation_id: ConversationId,
    pub destination: RoutingSlot,
    pub source: RandomSourceId,
    pub creation_sequence: CreationSequence,
    pub lifetime: BundleLifetime,
    pub hop_limit: u8,
    pub copy_tokens: CopyTokens,
    pub sender_sequence: u64,
    pub created_time_ms: Option<u64>,
    pub body: MessageBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecuredBundle {
    pub wire_bytes: Vec<u8>,
    pub plaintext: DmePlaintext,
    pub decoded: DecodedBundle,
}

pub fn create_secure_bundle(
    sender: &Identity,
    recipient: &IdentityPublic,
    draft: SecureMessageDraft,
) -> Result<SecuredBundle, SecureBundleError> {
    let aad = DmeAad {
        packet_id: draft.packet_id,
        destination: draft.destination,
        message_class: draft.body.message_type().class(),
        priority: draft.body.message_type().priority(),
        lifetime: draft.lifetime,
        hop_limit: draft.hop_limit,
        source: draft.source,
        creation_sequence: draft.creation_sequence,
    };
    let (encrypted, plaintext) = seal_dme(
        sender,
        recipient,
        aad,
        DmeDraft {
            packet_id: draft.packet_id,
            message_id: draft.message_id,
            conversation_id: draft.conversation_id,
            sender_sequence: draft.sender_sequence,
            created_time_ms: draft.created_time_ms,
            body: draft.body,
        },
    )?;
    let payload = encrypted.encode()?;
    let payload_hash: [u8; 32] = Sha256::digest(&payload).into();
    let bundle = Bundle {
        destination: draft.destination,
        source: draft.source,
        creation_sequence: draft.creation_sequence,
        lifetime: draft.lifetime,
        age_millis: 0,
        hops: HopState::new(0, draft.hop_limit).map_err(|_| SecureBundleError::InvalidDraft)?,
        routing: RoutingBlock {
            packet_id: draft.packet_id,
            message_class: plaintext.body.message_type().class(),
            priority: plaintext.body.message_type().priority(),
            copy_tokens: draft.copy_tokens,
            payload_size: u16::try_from(payload.len())
                .map_err(|_| SecureBundleError::InvalidDraft)?,
            payload_hash: PayloadHash::from(payload_hash),
        },
        payload,
    };
    let wire_bytes = bundle.encode()?;
    let decoded = Bundle::decode(&wire_bytes)?;
    Ok(SecuredBundle {
        wire_bytes,
        plaintext,
        decoded,
    })
}

pub fn open_secure_bundle(
    recipient: &Identity,
    wire_bytes: &[u8],
) -> Result<(DecodedBundle, DmePlaintext), SecureBundleError> {
    let decoded = Bundle::decode(wire_bytes)?;
    let envelope = DmeCiphertext::decode(&decoded.bundle.payload)?;
    let encrypted = EncryptedDme {
        encapsulated_key: envelope.encapsulated_key,
        aad_hash: envelope.aad_hash,
        ciphertext: envelope.ciphertext,
    };
    let aad = DmeAad {
        packet_id: decoded.bundle.routing.packet_id,
        destination: decoded.bundle.destination,
        message_class: decoded.bundle.routing.message_class,
        priority: decoded.bundle.routing.priority,
        lifetime: decoded.bundle.lifetime,
        hop_limit: decoded.bundle.hops.limit(),
        source: decoded.bundle.source,
        creation_sequence: decoded.bundle.creation_sequence,
    };
    let plaintext = open_dme(recipient, aad, &encrypted)?;
    Ok((decoded, plaintext))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SecureBundleError {
    Crypto(mesh_crypto::CryptoError),
    Bundle(mesh_bundle::BundleError),
    InvalidDraft,
}

impl fmt::Display for SecureBundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "secure bundle operation failed: {self:?}")
    }
}

impl std::error::Error for SecureBundleError {}

impl From<mesh_crypto::CryptoError> for SecureBundleError {
    fn from(value: mesh_crypto::CryptoError) -> Self {
        Self::Crypto(value)
    }
}

impl From<mesh_bundle::BundleError> for SecureBundleError {
    fn from(value: mesh_bundle::BundleError) -> Self {
        Self::Bundle(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> SecureMessageDraft {
        SecureMessageDraft {
            packet_id: PacketId::from([1; 16]),
            message_id: MessageId::from([2; 16]),
            conversation_id: ConversationId::from([3; 16]),
            destination: RoutingSlot::from([4; 16]),
            source: RandomSourceId::from([5; 16]),
            creation_sequence: CreationSequence::from_u64(6),
            lifetime: BundleLifetime::from_millis(259_200_000).unwrap(),
            hop_limit: 12,
            copy_tokens: CopyTokens::new(6).unwrap(),
            sender_sequence: 1,
            created_time_ms: None,
            body: MessageBody::DirectText {
                text: "A to C through B".into(),
                reply_to: None,
            },
        }
    }

    #[test]
    fn secured_bundle_round_trip() {
        let sender = Identity::generate().unwrap();
        let recipient = Identity::generate().unwrap();
        let secured = create_secure_bundle(&sender, recipient.public(), draft()).unwrap();
        let (_, opened) = open_secure_bundle(&recipient, &secured.wire_bytes).unwrap();
        assert_eq!(opened, secured.plaintext);
    }

    #[test]
    fn immutable_hop_limit_tamper_fails_aad() {
        let sender = Identity::generate().unwrap();
        let recipient = Identity::generate().unwrap();
        let secured = create_secure_bundle(&sender, recipient.public(), draft()).unwrap();
        let mut tampered = secured.decoded.bundle;
        tampered.hops = HopState::new(0, 13).unwrap();
        let wire = tampered.encode().unwrap();
        assert_eq!(
            open_secure_bundle(&recipient, &wire),
            Err(SecureBundleError::Crypto(
                mesh_crypto::CryptoError::AadMismatch
            ))
        );
    }
}
