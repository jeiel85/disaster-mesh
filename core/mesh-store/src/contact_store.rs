//! Signed contact-card persistence and explicit trust transitions.

use mesh_crypto::{
    ColumnContext, ContactCard, ContactTrustState, CryptoError, DbMasterKey, decrypt_local_value,
    encrypt_local_value, identity_id_from_signing_public, safety_number,
};
use mesh_types::ContactId;
use rusqlite::{OptionalExtension, params};

use crate::{Store, StoreError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredContact {
    pub contact_id: ContactId,
    pub signing_public_key: [u8; 32],
    pub hpke_public_key: [u8; 32],
    pub destination_routing_slot: mesh_types::RoutingSlot,
    pub display_name: String,
    pub key_version: u32,
    pub capabilities: u32,
    pub trust: ContactTrustState,
    pub safety_verified: bool,
}

impl Store {
    pub fn import_contact(
        &mut self,
        master_key: &DbMasterKey,
        contact_id: ContactId,
        card: ContactCard,
        now_ms: u64,
    ) -> Result<StoredContact, StoreError> {
        let card = ContactCard::decode(&card.encode()?)?;
        let identity_id = identity_id_from_signing_public(&card.signing_public_key);
        let local_identity_hash = self.local_identity_hash()?;
        let existing: Option<ExistingContact> = self
            .connection
            .query_row(
                "SELECT contact_id, signing_public_key, hpke_public_key, key_version,
                        trust_state, safety_verified
                 FROM contacts WHERE identity_hash = ?1",
                [identity_id.as_bytes().as_slice()],
                |row| {
                    Ok(ExistingContact {
                        contact_id: row.get(0)?,
                        signing_public_key: row.get(1)?,
                        hpke_public_key: row.get(2)?,
                        key_version: row.get(3)?,
                        trust_state: row.get(4)?,
                        safety_verified: row.get(5)?,
                    })
                },
            )
            .optional()?;
        let (contact_id, trust, safety_verified) = if let Some(existing) = existing {
            let contact_id = ContactId::from(fixed(&existing.contact_id)?);
            let changed = existing.signing_public_key != card.signing_public_key
                || existing.hpke_public_key != card.hpke_public_key
                || existing.key_version != card.key_version;
            (
                contact_id,
                if changed {
                    ContactTrustState::KeyChanged
                } else {
                    trust_from_i64(existing.trust_state)?
                },
                !changed && existing.safety_verified,
            )
        } else {
            (contact_id, ContactTrustState::Unverified, false)
        };
        let encrypted_display_name = encrypt_local_value(
            master_key,
            contact_name_context(&contact_id, &local_identity_hash),
            card.display_name.as_bytes(),
        )?;
        let now_ms = i64::try_from(now_ms).map_err(|_| StoreError::IntegerOutOfRange)?;
        self.connection.execute(
            "INSERT INTO contacts (
                contact_id, identity_hash, signing_public_key, hpke_public_key,
                destination_routing_slot, encrypted_display_name, trust_state,
                safety_verified, capabilities, outbound_sender_sequence,
                key_version, created_at_ms, updated_at_ms, revoked_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?11, ?11, NULL)
             ON CONFLICT(identity_hash) DO UPDATE SET
                signing_public_key = excluded.signing_public_key,
                hpke_public_key = excluded.hpke_public_key,
                destination_routing_slot = excluded.destination_routing_slot,
                encrypted_display_name = excluded.encrypted_display_name,
                trust_state = excluded.trust_state,
                safety_verified = excluded.safety_verified,
                capabilities = excluded.capabilities,
                key_version = excluded.key_version,
                updated_at_ms = excluded.updated_at_ms",
            params![
                contact_id.as_bytes().as_slice(),
                identity_id.as_bytes().as_slice(),
                card.signing_public_key.as_slice(),
                card.hpke_public_key.as_slice(),
                card.inbound_routing_slot.as_bytes().as_slice(),
                encrypted_display_name,
                trust_to_i64(trust),
                safety_verified,
                card.capabilities,
                card.key_version,
                now_ms,
            ],
        )?;
        Ok(StoredContact {
            contact_id,
            signing_public_key: card.signing_public_key,
            hpke_public_key: card.hpke_public_key,
            destination_routing_slot: card.inbound_routing_slot,
            display_name: card.display_name,
            key_version: card.key_version,
            capabilities: card.capabilities,
            trust,
            safety_verified,
        })
    }

    pub fn load_contact(
        &self,
        master_key: &DbMasterKey,
        contact_id: ContactId,
    ) -> Result<StoredContact, StoreError> {
        let local_identity_hash = self.local_identity_hash()?;
        let row: ContactRow = self
            .connection
            .query_row(
                "SELECT signing_public_key, hpke_public_key, destination_routing_slot,
                        encrypted_display_name, trust_state, safety_verified,
                        capabilities, key_version
                 FROM contacts WHERE contact_id = ?1",
                [contact_id.as_bytes().as_slice()],
                |row| {
                    Ok(ContactRow {
                        signing_public_key: row.get(0)?,
                        hpke_public_key: row.get(1)?,
                        routing_slot: row.get(2)?,
                        encrypted_display_name: row.get(3)?,
                        trust_state: row.get(4)?,
                        safety_verified: row.get(5)?,
                        capabilities: row.get(6)?,
                        key_version: row.get(7)?,
                    })
                },
            )
            .optional()?
            .ok_or(StoreError::ContactNotFound)?;
        let display_name = decrypt_local_value(
            master_key,
            contact_name_context(&contact_id, &local_identity_hash),
            &row.encrypted_display_name,
        )?;
        Ok(StoredContact {
            contact_id,
            signing_public_key: fixed(&row.signing_public_key)?,
            hpke_public_key: fixed(&row.hpke_public_key)?,
            destination_routing_slot: mesh_types::RoutingSlot::from(fixed(&row.routing_slot)?),
            display_name: String::from_utf8(display_name.to_vec())
                .map_err(|_| StoreError::KeyMaterialMismatch)?,
            key_version: row.key_version,
            capabilities: row.capabilities,
            trust: trust_from_i64(row.trust_state)?,
            safety_verified: row.safety_verified,
        })
    }

    pub fn verify_contact_in_person(
        &mut self,
        contact_id: ContactId,
        local_signing_public_key: &[u8; 32],
        remote_signing_public_key: &[u8; 32],
        displayed_safety_number: &str,
        now_ms: u64,
    ) -> Result<(), StoreError> {
        if safety_number(local_signing_public_key, remote_signing_public_key)
            != displayed_safety_number
        {
            return Err(StoreError::Crypto(CryptoError::InvalidField.to_string()));
        }
        let changed = self.connection.execute(
            "UPDATE contacts SET trust_state = 1, safety_verified = 1, updated_at_ms = ?2
             WHERE contact_id = ?1 AND trust_state != 3 AND signing_public_key = ?3",
            params![
                contact_id.as_bytes().as_slice(),
                i64::try_from(now_ms).map_err(|_| StoreError::IntegerOutOfRange)?,
                remote_signing_public_key.as_slice(),
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::ContactNotFound);
        }
        Ok(())
    }

    pub fn next_sender_sequence(&mut self, contact_id: ContactId) -> Result<u64, StoreError> {
        let transaction = self.connection.transaction()?;
        let current: i64 = transaction
            .query_row(
                "SELECT outbound_sender_sequence FROM contacts WHERE contact_id = ?1",
                [contact_id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::ContactNotFound)?;
        let next = current
            .checked_add(1)
            .ok_or(StoreError::IntegerOutOfRange)?;
        transaction.execute(
            "UPDATE contacts SET outbound_sender_sequence = ?2 WHERE contact_id = ?1",
            params![contact_id.as_bytes().as_slice(), next],
        )?;
        transaction.commit()?;
        u64::try_from(next).map_err(|_| StoreError::IntegerOutOfRange)
    }

    fn local_identity_hash(&self) -> Result<[u8; 32], StoreError> {
        let bytes: Vec<u8> = self.connection.query_row(
            "SELECT identity_hash FROM identity WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        fixed(&bytes)
    }
}

struct ExistingContact {
    contact_id: Vec<u8>,
    signing_public_key: Vec<u8>,
    hpke_public_key: Vec<u8>,
    key_version: u32,
    trust_state: i64,
    safety_verified: bool,
}

struct ContactRow {
    signing_public_key: Vec<u8>,
    hpke_public_key: Vec<u8>,
    routing_slot: Vec<u8>,
    encrypted_display_name: Vec<u8>,
    trust_state: i64,
    safety_verified: bool,
    capabilities: u32,
    key_version: u32,
}

fn contact_name_context<'a>(
    contact_id: &'a ContactId,
    identity_hash: &'a [u8; 32],
) -> ColumnContext<'a> {
    ColumnContext {
        schema_version: 1,
        table: "contacts",
        column: "encrypted_display_name",
        primary_key: contact_id.as_bytes(),
        key_version: 1,
        identity_hash,
    }
}

fn trust_to_i64(trust: ContactTrustState) -> i64 {
    match trust {
        ContactTrustState::Unverified => 0,
        ContactTrustState::VerifiedInPerson => 1,
        ContactTrustState::KeyChanged => 2,
        ContactTrustState::Revoked => 3,
    }
}

fn trust_from_i64(value: i64) -> Result<ContactTrustState, StoreError> {
    match value {
        0 => Ok(ContactTrustState::Unverified),
        1 => Ok(ContactTrustState::VerifiedInPerson),
        2 => Ok(ContactTrustState::KeyChanged),
        3 => Ok(ContactTrustState::Revoked),
        _ => Err(StoreError::KeyMaterialMismatch),
    }
}

fn fixed<const N: usize>(bytes: &[u8]) -> Result<[u8; N], StoreError> {
    bytes
        .try_into()
        .map_err(|_| StoreError::KeyMaterialMismatch)
}

#[cfg(test)]
mod tests {
    use mesh_crypto::{ContactCard, Identity};

    use super::*;

    #[test]
    fn imported_contact_is_persistent_unverified_and_can_be_verified() {
        let mut store = Store::open_in_memory().unwrap();
        let master = DbMasterKey::from_bytes([3; 32]);
        let (local, _) = store.bootstrap_identity(&master, 1).unwrap();
        let remote = Identity::generate().unwrap();
        let card = ContactCard::create(&remote, "Remote", 0x1f).unwrap();
        let contact_id = ContactId::from([4; 16]);
        let imported = store.import_contact(&master, contact_id, card, 2).unwrap();
        assert_eq!(imported.trust, ContactTrustState::Unverified);
        assert_eq!(
            store
                .load_contact(&master, contact_id)
                .unwrap()
                .display_name,
            "Remote"
        );

        let number = safety_number(
            &local.public().signing_public_key,
            &remote.public().signing_public_key,
        );
        store
            .verify_contact_in_person(
                contact_id,
                &local.public().signing_public_key,
                &remote.public().signing_public_key,
                &number,
                3,
            )
            .unwrap();
        assert_eq!(
            store.load_contact(&master, contact_id).unwrap().trust,
            ContactTrustState::VerifiedInPerson
        );
    }

    #[test]
    fn changed_key_for_same_identity_hash_cannot_preserve_verification() {
        let mut store = Store::open_in_memory().unwrap();
        let master = DbMasterKey::from_bytes([3; 32]);
        store.bootstrap_identity(&master, 1).unwrap();
        let remote = Identity::generate().unwrap();
        let contact_id = ContactId::from([4; 16]);
        store
            .import_contact(
                &master,
                contact_id,
                ContactCard::create(&remote, "Remote", 0).unwrap(),
                2,
            )
            .unwrap();
        let mut replacement = ContactCard::create(&remote, "Remote", 0).unwrap();
        replacement.hpke_public_key[0] ^= 1;
        // The old signature no longer covers the changed key.
        assert!(
            store
                .import_contact(&master, contact_id, replacement, 3)
                .is_err()
        );
    }
}
