//! Persistent identity bootstrap with DMEV-encrypted private material.

use mesh_crypto::{ColumnContext, DbMasterKey, Identity, decrypt_local_value, encrypt_local_value};
use mesh_types::RoutingSlot;
use rusqlite::{OptionalExtension, params};

use crate::{Store, StoreError};

const IDENTITY_PRIMARY_KEY: [u8; 8] = 1u64.to_be_bytes();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityBootstrapOutcome {
    Created,
    Restored,
}

impl Store {
    pub fn bootstrap_identity(
        &mut self,
        master_key: &DbMasterKey,
        now_ms: u64,
    ) -> Result<(Identity, IdentityBootstrapOutcome), StoreError> {
        let stored: Option<StoredIdentity> = self
            .connection
            .query_row(
                "SELECT i.identity_hash, i.signing_public_key, i.hpke_public_key,
                        i.noise_public_key, i.encrypted_private_keys, i.key_version,
                        s.routing_slot
                 FROM identity i
                 JOIN inbound_routing_slots s ON s.state = 0
                 WHERE i.id = 1",
                [],
                |row| {
                    Ok(StoredIdentity {
                        identity_hash: row.get(0)?,
                        signing_public_key: row.get(1)?,
                        hpke_public_key: row.get(2)?,
                        noise_public_key: row.get(3)?,
                        encrypted_private_keys: row.get(4)?,
                        key_version: row.get(5)?,
                        routing_slot: row.get(6)?,
                    })
                },
            )
            .optional()?;
        if let Some(stored) = stored {
            return Ok((
                restore_identity(master_key, &stored)?,
                IdentityBootstrapOutcome::Restored,
            ));
        }

        let identity = Identity::generate()?;
        let public = *identity.public();
        let key_version =
            u16::try_from(public.key_version).map_err(|_| StoreError::IntegerOutOfRange)?;
        let context = identity_context(key_version, public.identity_id.as_bytes());
        let private_material = identity.private_material();
        let encrypted_private_keys =
            encrypt_local_value(master_key, context, private_material.as_slice())?;
        let now_ms = i64::try_from(now_ms).map_err(|_| StoreError::IntegerOutOfRange)?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO identity (
                id, identity_hash, signing_public_key, hpke_public_key,
                noise_public_key, encrypted_private_keys, key_version, created_at_ms
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                public.identity_id.as_bytes().as_slice(),
                public.signing_public_key.as_slice(),
                public.hpke_public_key.as_slice(),
                public.noise_public_key.as_slice(),
                encrypted_private_keys,
                public.key_version,
                now_ms,
            ],
        )?;
        transaction.execute(
            "INSERT INTO inbound_routing_slots (
                routing_slot, state, key_version, created_at_ms, retire_after_ms
             ) VALUES (?1, 0, ?2, ?3, NULL)",
            params![
                public.inbound_routing_slot.as_bytes().as_slice(),
                public.key_version,
                now_ms,
            ],
        )?;
        transaction.commit()?;
        Ok((identity, IdentityBootstrapOutcome::Created))
    }
}

struct StoredIdentity {
    identity_hash: Vec<u8>,
    signing_public_key: Vec<u8>,
    hpke_public_key: Vec<u8>,
    noise_public_key: Vec<u8>,
    encrypted_private_keys: Vec<u8>,
    key_version: u32,
    routing_slot: Vec<u8>,
}

fn restore_identity(
    master_key: &DbMasterKey,
    stored: &StoredIdentity,
) -> Result<Identity, StoreError> {
    let identity_hash: [u8; 32] = stored
        .identity_hash
        .as_slice()
        .try_into()
        .map_err(|_| StoreError::KeyMaterialMismatch)?;
    let key_version =
        u16::try_from(stored.key_version).map_err(|_| StoreError::IntegerOutOfRange)?;
    let material = decrypt_local_value(
        master_key,
        identity_context(key_version, &identity_hash),
        &stored.encrypted_private_keys,
    )?;
    if material.len() != 96 {
        return Err(StoreError::KeyMaterialMismatch);
    }
    let routing_slot: [u8; 16] = stored
        .routing_slot
        .as_slice()
        .try_into()
        .map_err(|_| StoreError::KeyMaterialMismatch)?;
    let identity = Identity::from_private_material(
        material[0..32]
            .try_into()
            .map_err(|_| StoreError::KeyMaterialMismatch)?,
        material[32..64]
            .try_into()
            .map_err(|_| StoreError::KeyMaterialMismatch)?,
        material[64..96]
            .try_into()
            .map_err(|_| StoreError::KeyMaterialMismatch)?,
        RoutingSlot::from(routing_slot),
        stored.key_version,
    )?;
    let public = identity.public();
    if public.identity_id.as_bytes() != &identity_hash
        || public.signing_public_key.as_slice() != stored.signing_public_key
        || public.hpke_public_key.as_slice() != stored.hpke_public_key
        || public.noise_public_key.as_slice() != stored.noise_public_key
    {
        return Err(StoreError::KeyMaterialMismatch);
    }
    Ok(identity)
}

fn identity_context<'a>(key_version: u16, identity_hash: &'a [u8; 32]) -> ColumnContext<'a> {
    ColumnContext {
        schema_version: 1,
        table: "identity",
        column: "encrypted_private_keys",
        primary_key: &IDENTITY_PRIMARY_KEY,
        key_version,
        identity_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_created_once_and_restored_from_encrypted_material() {
        let mut store = Store::open_in_memory().unwrap();
        let master_key = DbMasterKey::from_bytes([9; 32]);
        let (created, outcome) = store.bootstrap_identity(&master_key, 100).unwrap();
        assert_eq!(outcome, IdentityBootstrapOutcome::Created);
        let expected = *created.public();
        drop(created);

        let (restored, outcome) = store.bootstrap_identity(&master_key, 200).unwrap();
        assert_eq!(outcome, IdentityBootstrapOutcome::Restored);
        assert_eq!(*restored.public(), expected);
        assert_eq!(
            store
                .connection()
                .query_row("SELECT COUNT(*) FROM identity", [], |row| row
                    .get::<_, u32>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn wrong_master_key_fails_closed() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .bootstrap_identity(&DbMasterKey::from_bytes([1; 32]), 100)
            .unwrap();
        assert!(
            store
                .bootstrap_identity(&DbMasterKey::from_bytes([2; 32]), 200)
                .is_err()
        );
    }
}
