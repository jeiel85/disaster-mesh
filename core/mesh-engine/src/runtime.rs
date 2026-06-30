//! Persistent single-owner engine facade used by the Android coordinator.

use core::fmt;

use mesh_crypto::{
    ContactCard, ContactTrustState, DbMasterKey, Identity, IdentityPublic,
    identity_id_from_signing_public,
};
use mesh_store::{IdentityBootstrapOutcome, Store, StoreBundleOutcome, StoredContact};
use mesh_types::{
    BundleLifetime, ContactId, ConversationId, CopyTokens, CreationSequence, MessageId, PacketId,
    RandomSourceId,
};
use rand_core::{OsRng, TryRngCore};

use crate::{SecureBundleError, SecureMessageDraft, create_secure_bundle};

pub struct MeshRuntimeEngine {
    store: Store,
    master_key: DbMasterKey,
    identity: Identity,
    local_display_name: String,
}

impl fmt::Debug for MeshRuntimeEngine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MeshRuntimeEngine")
            .field("identity", &"[REDACTED]")
            .field("master_key", &"[REDACTED]")
            .field("local_display_name", &self.local_display_name)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeContact {
    pub contact_id: ContactId,
    pub display_name: String,
    pub display_id: String,
    pub safety_number: String,
    pub trust: ContactTrustState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectSendResult {
    pub packet_id: PacketId,
    pub message_id: MessageId,
    pub conversation_id: ConversationId,
    pub wire_bytes: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RuntimeError {
    Crypto(mesh_crypto::CryptoError),
    Store(mesh_store::StoreError),
    SecureBundle(SecureBundleError),
    RandomFailure,
    RevokedContact,
    InvalidInput,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("runtime operation failed")
    }
}

impl std::error::Error for RuntimeError {}

impl From<mesh_crypto::CryptoError> for RuntimeError {
    fn from(value: mesh_crypto::CryptoError) -> Self {
        Self::Crypto(value)
    }
}

impl From<mesh_store::StoreError> for RuntimeError {
    fn from(value: mesh_store::StoreError) -> Self {
        Self::Store(value)
    }
}

impl From<SecureBundleError> for RuntimeError {
    fn from(value: SecureBundleError) -> Self {
        Self::SecureBundle(value)
    }
}

impl MeshRuntimeEngine {
    pub fn open(
        path: impl AsRef<std::path::Path>,
        master_key: [u8; 32],
        local_display_name: String,
        now_ms: u64,
    ) -> Result<(Self, IdentityBootstrapOutcome), RuntimeError> {
        let store = Store::open(path)?;
        Self::from_store(store, master_key, local_display_name, now_ms)
    }

    pub fn open_in_memory(
        master_key: [u8; 32],
        local_display_name: String,
        now_ms: u64,
    ) -> Result<(Self, IdentityBootstrapOutcome), RuntimeError> {
        let store = Store::open_in_memory()?;
        Self::from_store(store, master_key, local_display_name, now_ms)
    }

    fn from_store(
        mut store: Store,
        master_key: [u8; 32],
        local_display_name: String,
        now_ms: u64,
    ) -> Result<(Self, IdentityBootstrapOutcome), RuntimeError> {
        let master_key = DbMasterKey::from_bytes(master_key);
        let (identity, outcome) = store.bootstrap_identity(&master_key, now_ms)?;
        Ok((
            Self {
                store,
                master_key,
                identity,
                local_display_name,
            },
            outcome,
        ))
    }

    pub fn own_contact_qr(&self, capabilities: u32) -> Result<String, RuntimeError> {
        Ok(ContactCard::create(&self.identity, &self.local_display_name, capabilities)?.to_qr()?)
    }

    pub fn import_contact_qr(
        &mut self,
        qr: &str,
        now_ms: u64,
    ) -> Result<RuntimeContact, RuntimeError> {
        let card = ContactCard::from_qr(qr)?;
        let contact_id = ContactId::from(random_array()?);
        let stored = self
            .store
            .import_contact(&self.master_key, contact_id, card, now_ms)?;
        Ok(self.runtime_contact(stored))
    }

    pub fn load_contact(&self, contact_id: ContactId) -> Result<RuntimeContact, RuntimeError> {
        Ok(self.runtime_contact(self.store.load_contact(&self.master_key, contact_id)?))
    }

    pub fn verify_contact(
        &mut self,
        contact_id: ContactId,
        displayed_safety_number: &str,
        now_ms: u64,
    ) -> Result<(), RuntimeError> {
        let contact = self.store.load_contact(&self.master_key, contact_id)?;
        self.store.verify_contact_in_person(
            contact_id,
            &self.identity.public().signing_public_key,
            &contact.signing_public_key,
            displayed_safety_number,
            now_ms,
        )?;
        Ok(())
    }

    pub fn send_direct_text(
        &mut self,
        contact_id: ContactId,
        text: String,
        now_ms: u64,
        boot_id: [u8; 16],
        elapsed_ms: u64,
    ) -> Result<DirectSendResult, RuntimeError> {
        if text.is_empty() {
            return Err(RuntimeError::InvalidInput);
        }
        self.send_message(
            contact_id,
            mesh_crypto::MessageBody::DirectText {
                text,
                reply_to: None,
            },
            259_200_000,
            12,
            6,
            now_ms,
            boot_id,
            elapsed_ms,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_check_in(
        &mut self,
        contact_id: ContactId,
        status: u8,
        people_count: u8,
        note: String,
        manual_location: Option<String>,
        battery_percent: Option<u8>,
        now_ms: u64,
        boot_id: [u8; 16],
        elapsed_ms: u64,
    ) -> Result<DirectSendResult, RuntimeError> {
        self.send_message(
            contact_id,
            mesh_crypto::MessageBody::CheckIn {
                status,
                people_count,
                note,
                location: manual_location
                    .map(|description| mesh_crypto::Location::Manual { description }),
                battery_percent,
            },
            172_800_000,
            12,
            8,
            now_ms,
            boot_id,
            elapsed_ms,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_private_sos(
        &mut self,
        contact_id: ContactId,
        category: u8,
        description: String,
        people_count: u8,
        severe_injury_count: u8,
        manual_location: Option<String>,
        movement_direction: String,
        battery_percent: Option<u8>,
        now_ms: u64,
        boot_id: [u8; 16],
        elapsed_ms: u64,
    ) -> Result<DirectSendResult, RuntimeError> {
        self.send_message(
            contact_id,
            mesh_crypto::MessageBody::PrivateSos {
                category,
                description,
                people_count,
                severe_injury_count,
                location: manual_location
                    .map(|description| mesh_crypto::Location::Manual { description }),
                movement_direction,
                battery_percent,
            },
            86_400_000,
            16,
            12,
            now_ms,
            boot_id,
            elapsed_ms,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_cancel(
        &mut self,
        contact_id: ContactId,
        target_packet_id: PacketId,
        target_message_id: MessageId,
        reason: u8,
        now_ms: u64,
        boot_id: [u8; 16],
        elapsed_ms: u64,
    ) -> Result<DirectSendResult, RuntimeError> {
        let cancel = self.send_message(
            contact_id,
            mesh_crypto::MessageBody::Cancel {
                target_packet_id,
                target_message_id,
                reason,
            },
            604_800_000,
            16,
            12,
            now_ms,
            boot_id,
            elapsed_ms,
        )?;
        self.store.suppress_outbound_offer(
            target_packet_id,
            now_ms,
            now_ms.saturating_add(691_200_000),
        )?;
        Ok(cancel)
    }

    #[allow(clippy::too_many_arguments)]
    fn send_message(
        &mut self,
        contact_id: ContactId,
        body: mesh_crypto::MessageBody,
        lifetime_ms: u64,
        hop_limit: u8,
        copy_tokens: u8,
        now_ms: u64,
        boot_id: [u8; 16],
        elapsed_ms: u64,
    ) -> Result<DirectSendResult, RuntimeError> {
        let contact = self.store.load_contact(&self.master_key, contact_id)?;
        if contact.trust == ContactTrustState::Revoked {
            return Err(RuntimeError::RevokedContact);
        }
        let sender_sequence = self.store.next_sender_sequence(contact_id)?;
        let packet_id = PacketId::from(random_array()?);
        let message_id = MessageId::from(random_array()?);
        let conversation_id = ConversationId::from(random_array()?);
        let recipient = recipient_public(&contact);
        let secured = create_secure_bundle(
            &self.identity,
            &recipient,
            SecureMessageDraft {
                packet_id,
                message_id,
                conversation_id,
                destination: contact.destination_routing_slot,
                source: RandomSourceId::from(random_array()?),
                creation_sequence: CreationSequence::from_u64(u64::from_be_bytes(random_array()?)),
                lifetime: BundleLifetime::from_millis(lifetime_ms)
                    .map_err(|_| RuntimeError::InvalidInput)?,
                hop_limit,
                copy_tokens: CopyTokens::new(copy_tokens)
                    .map_err(|_| RuntimeError::InvalidInput)?,
                sender_sequence,
                created_time_ms: Some(now_ms),
                body,
            },
        )?;
        let outcome = self.store.put_bundle(
            &secured.decoded,
            &secured.wire_bytes,
            boot_id,
            elapsed_ms,
            now_ms,
            0,
        )?;
        if outcome != StoreBundleOutcome::Inserted {
            return Err(RuntimeError::InvalidInput);
        }
        Ok(DirectSendResult {
            packet_id,
            message_id,
            conversation_id,
            wire_bytes: secured.wire_bytes,
        })
    }

    #[must_use]
    pub const fn identity(&self) -> &Identity {
        &self.identity
    }

    fn runtime_contact(&self, contact: StoredContact) -> RuntimeContact {
        RuntimeContact {
            contact_id: contact.contact_id,
            display_name: contact.display_name,
            display_id: mesh_crypto::display_id(&contact.signing_public_key),
            safety_number: mesh_crypto::safety_number(
                &self.identity.public().signing_public_key,
                &contact.signing_public_key,
            ),
            trust: contact.trust,
        }
    }
}

fn recipient_public(contact: &StoredContact) -> IdentityPublic {
    IdentityPublic {
        identity_id: identity_id_from_signing_public(&contact.signing_public_key),
        signing_public_key: contact.signing_public_key,
        hpke_public_key: contact.hpke_public_key,
        noise_public_key: [0; 32],
        inbound_routing_slot: contact.destination_routing_slot,
        key_version: contact.key_version,
    }
}

fn random_array<const N: usize>() -> Result<[u8; N], RuntimeError> {
    let mut output = [0; N];
    OsRng
        .try_fill_bytes(&mut output)
        .map_err(|_| RuntimeError::RandomFailure)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_secure_bundle;

    #[test]
    fn persistent_runtime_imports_contact_and_stores_direct_ciphertext() {
        let (mut alice, outcome) =
            MeshRuntimeEngine::open_in_memory([1; 32], "Alice".into(), 1).unwrap();
        assert_eq!(outcome, IdentityBootstrapOutcome::Created);
        let (bob, _) = MeshRuntimeEngine::open_in_memory([2; 32], "Bob".into(), 1).unwrap();
        let contact = alice
            .import_contact_qr(&bob.own_contact_qr(0x1f).unwrap(), 2)
            .unwrap();
        assert_eq!(contact.trust, ContactTrustState::Unverified);
        let sent = alice
            .send_direct_text(contact.contact_id, "hello".into(), 3, [4; 16], 5)
            .unwrap();
        let (_, plaintext) = open_secure_bundle(bob.identity(), &sent.wire_bytes).unwrap();
        assert!(matches!(
            plaintext.body,
            mesh_crypto::MessageBody::DirectText { ref text, .. } if text == "hello"
        ));
    }

    #[test]
    fn check_in_sos_without_location_and_cancel_use_fixed_product_policy() {
        let (mut alice, _) = MeshRuntimeEngine::open_in_memory([1; 32], "Alice".into(), 1).unwrap();
        let (bob, _) = MeshRuntimeEngine::open_in_memory([2; 32], "Bob".into(), 1).unwrap();
        let contact = alice
            .import_contact_qr(&bob.own_contact_qr(0x1f).unwrap(), 2)
            .unwrap();
        let check_in = alice
            .send_check_in(
                contact.contact_id,
                1,
                2,
                "safe".into(),
                None,
                Some(50),
                3,
                [4; 16],
                5,
            )
            .unwrap();
        let (_, plaintext) = open_secure_bundle(bob.identity(), &check_in.wire_bytes).unwrap();
        assert!(matches!(
            plaintext.body,
            mesh_crypto::MessageBody::CheckIn { location: None, .. }
        ));
        let sos = alice
            .send_private_sos(
                contact.contact_id,
                1,
                "need help".into(),
                2,
                1,
                Some("north shelter".into()),
                "north".into(),
                Some(9),
                4,
                [4; 16],
                6,
            )
            .unwrap();
        let (_, plaintext) = open_secure_bundle(bob.identity(), &sos.wire_bytes).unwrap();
        assert!(matches!(
            plaintext.body,
            mesh_crypto::MessageBody::PrivateSos {
                location: Some(mesh_crypto::Location::Manual { .. }),
                ..
            }
        ));
        let cancel = alice
            .send_cancel(
                contact.contact_id,
                check_in.packet_id,
                check_in.message_id,
                1,
                5,
                [4; 16],
                7,
            )
            .unwrap();
        let (_, plaintext) = open_secure_bundle(bob.identity(), &cancel.wire_bytes).unwrap();
        assert!(
            matches!(plaintext.body, mesh_crypto::MessageBody::Cancel { target_packet_id, .. } if target_packet_id == check_in.packet_id)
        );
        assert_eq!(
            alice
                .store
                .connection()
                .query_row(
                    "SELECT state FROM bundles WHERE packet_id = ?1",
                    [check_in.packet_id.as_bytes().as_slice()],
                    |row| row.get::<_, u8>(0)
                )
                .unwrap(),
            1
        );
    }
}
