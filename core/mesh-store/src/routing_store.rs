//! Persistent bundle deduplication and token-grant escrow transactions.

use mesh_bundle::DecodedBundle;
use mesh_routing::split_tokens;
use mesh_types::{PacketId, PayloadHash, PeerLinkHash, TokenGrantId, TransferId};
use rusqlite::{OptionalExtension, params};

use crate::{Store, StoreError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreBundleOutcome {
    Inserted,
    Duplicate,
    ConflictQuarantined,
    Tombstoned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrantReservationRequest {
    pub grant_id: TokenGrantId,
    pub packet_id: PacketId,
    pub peer: PeerLinkHash,
    pub transfer_id: TransferId,
    pub now_ms: u64,
    pub retain_until_ms: u64,
    pub sender_wire_bytes: Vec<u8>,
    pub sender_wire_hash: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GrantReservation {
    pub sender_tokens: u8,
    pub receiver_tokens: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitEvidence {
    pub payload_hash: PayloadHash,
    pub wire_hash: [u8; 32],
    pub accepted_tokens: u8,
    pub committed_at_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrantCommitOutcome {
    Transferred,
    SameGrant,
    Conflict,
}

impl Store {
    pub fn put_bundle(
        &mut self,
        decoded: &DecodedBundle,
        wire_bytes: &[u8],
        received_boot_id: [u8; 16],
        age_anchor_elapsed_ms: u64,
        created_local_ms: u64,
        origin: u8,
    ) -> Result<StoreBundleOutcome, StoreError> {
        let bundle = &decoded.bundle;
        let transaction = self.connection.transaction()?;
        let tombstoned: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM tombstones WHERE packet_id = ?1)",
            [bundle.routing.packet_id.as_bytes().as_slice()],
            |row| row.get(0),
        )?;
        if tombstoned {
            return Ok(StoreBundleOutcome::Tombstoned);
        }
        let existing: Option<Vec<u8>> = transaction
            .query_row(
                "SELECT payload_sha256 FROM bundles WHERE packet_id = ?1",
                [bundle.routing.packet_id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            return Ok(if existing == bundle.routing.payload_hash.as_bytes() {
                StoreBundleOutcome::Duplicate
            } else {
                StoreBundleOutcome::ConflictQuarantined
            });
        }

        transaction.execute(
            "INSERT INTO bundles (
                packet_id, bp_identity_hash, destination_slot, random_source_id,
                creation_sequence, message_class_hint, priority, lifetime_ms,
                stored_age_ms, age_anchor_elapsed_ms, received_boot_id,
                hop_count, hop_limit, copy_tokens, payload_size, payload_sha256,
                wire_sha256, state, origin, created_local_ms
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, 0, ?18, ?19
             )",
            params![
                bundle.routing.packet_id.as_bytes().as_slice(),
                decoded.bp_identity_hash.as_bytes().as_slice(),
                bundle.destination.as_bytes().as_slice(),
                bundle.source.as_bytes().as_slice(),
                bundle.creation_sequence.as_bytes().as_slice(),
                bundle.routing.message_class as u8,
                bundle.routing.priority as u8,
                i64_value(bundle.lifetime.as_millis())?,
                i64_value(bundle.age_millis)?,
                i64_value(age_anchor_elapsed_ms)?,
                received_boot_id.as_slice(),
                bundle.hops.count(),
                bundle.hops.limit(),
                bundle.routing.copy_tokens.get(),
                bundle.routing.payload_size,
                bundle.routing.payload_hash.as_bytes().as_slice(),
                decoded.wire_hash.as_bytes().as_slice(),
                origin,
                i64_value(created_local_ms)?,
            ],
        )?;
        transaction.execute(
            "INSERT INTO bundle_payloads(packet_id, bp_bundle_bytes) VALUES (?1, ?2)",
            params![bundle.routing.packet_id.as_bytes().as_slice(), wire_bytes],
        )?;
        transaction.commit()?;
        Ok(StoreBundleOutcome::Inserted)
    }

    pub fn reserve_token_grant(
        &mut self,
        request: &GrantReservationRequest,
    ) -> Result<GrantReservation, StoreError> {
        let transaction = self.connection.transaction()?;
        let tokens: u8 = transaction.query_row(
            "SELECT copy_tokens FROM bundles WHERE packet_id = ?1 AND state = 0",
            [request.packet_id.as_bytes().as_slice()],
            |row| row.get(0),
        )?;
        let (sender, receiver) = split_tokens(tokens).ok_or(StoreError::WaitOnly)?;
        transaction.execute(
            "UPDATE bundles SET copy_tokens = ?1, wire_sha256 = ?2 WHERE packet_id = ?3",
            params![
                sender,
                request.sender_wire_hash.as_slice(),
                request.packet_id.as_bytes().as_slice()
            ],
        )?;
        transaction.execute(
            "UPDATE bundle_payloads SET bp_bundle_bytes = ?1 WHERE packet_id = ?2",
            params![
                request.sender_wire_bytes,
                request.packet_id.as_bytes().as_slice()
            ],
        )?;
        transaction.execute(
            "INSERT INTO token_grants (
                grant_id, packet_id, peer_link_hash, direction, state, token_count,
                transfer_id, created_at_ms, updated_at_ms, retain_until_ms
             ) VALUES (?1, ?2, ?3, 0, 0, ?4, ?5, ?6, ?6, ?7)",
            params![
                request.grant_id.as_bytes().as_slice(),
                request.packet_id.as_bytes().as_slice(),
                request.peer.as_bytes().as_slice(),
                receiver,
                request.transfer_id.as_bytes().as_slice(),
                i64_value(request.now_ms)?,
                i64_value(request.retain_until_ms)?,
            ],
        )?;
        transaction.commit()?;
        Ok(GrantReservation {
            sender_tokens: sender,
            receiver_tokens: receiver,
        })
    }

    pub fn mark_grant_uncertain(
        &mut self,
        grant_id: TokenGrantId,
        now_ms: u64,
    ) -> Result<(), StoreError> {
        let changed = self.connection.execute(
            "UPDATE token_grants SET state = 1, updated_at_ms = ?1
             WHERE grant_id = ?2 AND direction = 0 AND state = 0",
            params![i64_value(now_ms)?, grant_id.as_bytes().as_slice()],
        )?;
        if changed != 1 {
            return Err(StoreError::InvalidGrantTransition);
        }
        Ok(())
    }

    pub fn reconcile_outbound_grant(
        &mut self,
        grant_id: TokenGrantId,
        evidence: CommitEvidence,
    ) -> Result<GrantCommitOutcome, StoreError> {
        let existing = load_grant(&self.connection, grant_id)?;
        if existing.tokens != evidence.accepted_tokens {
            return Ok(GrantCommitOutcome::Conflict);
        }
        if existing.state == 2 {
            return Ok(if existing.evidence == Some(evidence) {
                GrantCommitOutcome::SameGrant
            } else {
                GrantCommitOutcome::Conflict
            });
        }
        if !matches!(existing.state, 0 | 1) {
            return Err(StoreError::InvalidGrantTransition);
        }
        self.connection.execute(
            "UPDATE token_grants SET
                state = 2, updated_at_ms = ?1, committed_payload_sha256 = ?2,
                committed_wire_sha256 = ?3, accepted_tokens = ?4, committed_at_ms = ?1
             WHERE grant_id = ?5 AND direction = 0 AND state IN (0,1)",
            params![
                i64_value(evidence.committed_at_ms)?,
                evidence.payload_hash.as_bytes().as_slice(),
                evidence.wire_hash.as_slice(),
                evidence.accepted_tokens,
                grant_id.as_bytes().as_slice(),
            ],
        )?;
        Ok(GrantCommitOutcome::Transferred)
    }

    pub fn release_grant_confirmed_not_committed(
        &mut self,
        grant_id: TokenGrantId,
        restored_wire_bytes: &[u8],
        restored_wire_hash: [u8; 32],
        now_ms: u64,
    ) -> Result<u8, StoreError> {
        let transaction = self.connection.transaction()?;
        let (packet_id, state, grant_tokens): (Vec<u8>, u8, u8) = transaction.query_row(
            "SELECT packet_id, state, token_count FROM token_grants
             WHERE grant_id = ?1 AND direction = 0",
            [grant_id.as_bytes().as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        if !matches!(state, 0 | 1) {
            return Err(StoreError::InvalidGrantTransition);
        }
        let available: u8 = transaction.query_row(
            "SELECT copy_tokens FROM bundles WHERE packet_id = ?1",
            [packet_id.as_slice()],
            |row| row.get(0),
        )?;
        let restored = available
            .checked_add(grant_tokens)
            .filter(|value| *value <= 16)
            .ok_or(StoreError::TokenOverflow)?;
        transaction.execute(
            "UPDATE bundles SET copy_tokens = ?1, wire_sha256 = ?2 WHERE packet_id = ?3",
            params![
                restored,
                restored_wire_hash.as_slice(),
                packet_id.as_slice()
            ],
        )?;
        transaction.execute(
            "UPDATE bundle_payloads SET bp_bundle_bytes = ?1 WHERE packet_id = ?2",
            params![restored_wire_bytes, packet_id.as_slice()],
        )?;
        transaction.execute(
            "UPDATE token_grants SET state = 3, updated_at_ms = ?1 WHERE grant_id = ?2",
            params![i64_value(now_ms)?, grant_id.as_bytes().as_slice()],
        )?;
        transaction.commit()?;
        Ok(restored)
    }

    pub fn record_inbound_grant(
        &mut self,
        grant_id: TokenGrantId,
        packet_id: PacketId,
        peer: PeerLinkHash,
        transfer_id: TransferId,
        retain_until_ms: u64,
        evidence: CommitEvidence,
    ) -> Result<GrantCommitOutcome, StoreError> {
        if let Some(existing) = load_grant_optional(&self.connection, grant_id)? {
            return Ok(
                if existing.tokens == evidence.accepted_tokens
                    && existing.evidence == Some(evidence)
                {
                    GrantCommitOutcome::SameGrant
                } else {
                    GrantCommitOutcome::Conflict
                },
            );
        }
        self.connection.execute(
            "INSERT INTO token_grants (
                grant_id, packet_id, peer_link_hash, direction, state, token_count,
                transfer_id, created_at_ms, updated_at_ms, retain_until_ms,
                committed_payload_sha256, committed_wire_sha256, accepted_tokens,
                committed_at_ms
             ) VALUES (?1, ?2, ?3, 1, 2, ?4, ?5, ?6, ?6, ?7, ?8, ?9, ?4, ?6)",
            params![
                grant_id.as_bytes().as_slice(),
                packet_id.as_bytes().as_slice(),
                peer.as_bytes().as_slice(),
                evidence.accepted_tokens,
                transfer_id.as_bytes().as_slice(),
                i64_value(evidence.committed_at_ms)?,
                i64_value(retain_until_ms)?,
                evidence.payload_hash.as_bytes().as_slice(),
                evidence.wire_hash.as_slice(),
            ],
        )?;
        Ok(GrantCommitOutcome::Transferred)
    }

    pub fn available_bundle_tokens(&self, packet_id: PacketId) -> Result<u8, StoreError> {
        self.connection
            .query_row(
                "SELECT copy_tokens FROM bundles WHERE packet_id = ?1",
                [packet_id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn grant_state(&self, grant_id: TokenGrantId) -> Result<u8, StoreError> {
        self.connection
            .query_row(
                "SELECT state FROM token_grants WHERE grant_id = ?1",
                [grant_id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }
}

#[derive(Clone, Copy)]
struct StoredGrant {
    state: u8,
    tokens: u8,
    evidence: Option<CommitEvidence>,
}

fn load_grant(
    connection: &rusqlite::Connection,
    grant_id: TokenGrantId,
) -> Result<StoredGrant, StoreError> {
    load_grant_optional(connection, grant_id)?.ok_or(StoreError::UnknownGrant)
}

fn load_grant_optional(
    connection: &rusqlite::Connection,
    grant_id: TokenGrantId,
) -> Result<Option<StoredGrant>, StoreError> {
    connection
        .query_row(
            "SELECT state, token_count, committed_payload_sha256,
                    committed_wire_sha256, accepted_tokens, committed_at_ms
             FROM token_grants WHERE grant_id = ?1",
            [grant_id.as_bytes().as_slice()],
            |row| {
                let state = row.get(0)?;
                let tokens = row.get(1)?;
                let payload: Option<Vec<u8>> = row.get(2)?;
                let wire: Option<Vec<u8>> = row.get(3)?;
                let accepted: Option<u8> = row.get(4)?;
                let committed: Option<i64> = row.get(5)?;
                let evidence = match (payload, wire, accepted, committed) {
                    (Some(payload), Some(wire), Some(accepted_tokens), Some(committed_at_ms)) => {
                        let payload_hash =
                            PayloadHash::try_from(payload.as_slice()).map_err(|_| {
                                rusqlite::Error::InvalidColumnType(
                                    2,
                                    "committed_payload_sha256".into(),
                                    rusqlite::types::Type::Blob,
                                )
                            })?;
                        let wire_hash = wire.as_slice().try_into().map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                3,
                                "committed_wire_sha256".into(),
                                rusqlite::types::Type::Blob,
                            )
                        })?;
                        let committed_at_ms = u64::try_from(committed_at_ms).map_err(|_| {
                            rusqlite::Error::IntegralValueOutOfRange(5, committed_at_ms)
                        })?;
                        Some(CommitEvidence {
                            payload_hash,
                            wire_hash,
                            accepted_tokens,
                            committed_at_ms,
                        })
                    }
                    (None, None, None, None) => None,
                    _ => {
                        return Err(rusqlite::Error::InvalidQuery);
                    }
                };
                Ok(StoredGrant {
                    state,
                    tokens,
                    evidence,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn i64_value(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::IntegerOutOfRange)
}

#[cfg(test)]
mod tests {
    use mesh_bundle::{Bundle, DmeCiphertext, RoutingBlock};
    use mesh_types::{
        BundleLifetime, CopyTokens, CreationSequence, HopState, MessageClass, PacketId,
        PayloadHash, Priority, RandomSourceId, RoutingSlot,
    };
    use sha2::{Digest, Sha256};

    use super::*;

    fn sample(tokens: u8) -> (Bundle, Vec<u8>, DecodedBundle) {
        let payload = DmeCiphertext {
            encapsulated_key: [1; 32],
            aad_hash: [2; 32],
            ciphertext: vec![3; 32],
        }
        .encode()
        .unwrap();
        let hash: [u8; 32] = Sha256::digest(&payload).into();
        let bundle = Bundle {
            destination: RoutingSlot::from([4; 16]),
            source: RandomSourceId::from([5; 16]),
            creation_sequence: CreationSequence::from_u64(6),
            lifetime: BundleLifetime::from_millis(60_000).unwrap(),
            age_millis: 0,
            hops: HopState::new(0, 12).unwrap(),
            routing: RoutingBlock {
                packet_id: PacketId::from([7; 16]),
                message_class: MessageClass::Direct,
                priority: Priority::P2,
                copy_tokens: CopyTokens::new(tokens).unwrap(),
                payload_size: payload.len() as u16,
                payload_hash: PayloadHash::from(hash),
            },
            payload,
        };
        let wire = bundle.encode().unwrap();
        let decoded = Bundle::decode(&wire).unwrap();
        (bundle, wire, decoded)
    }

    #[test]
    fn reservation_and_lost_ack_survive_reopen() {
        let unique = format!(
            "disaster-mesh-store-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        let grant_id = TokenGrantId::from([8; 16]);
        let packet_id = PacketId::from([7; 16]);
        {
            let (_, wire, decoded) = sample(6);
            let mut store = Store::open(&path).unwrap();
            assert_eq!(
                store.put_bundle(&decoded, &wire, [9; 16], 0, 0, 0).unwrap(),
                StoreBundleOutcome::Inserted
            );
            let (sender_bundle, sender_wire, _) = sample(3);
            let sender_hash: [u8; 32] = Sha256::digest(&sender_wire).into();
            let reservation = store
                .reserve_token_grant(&GrantReservationRequest {
                    grant_id,
                    packet_id,
                    peer: PeerLinkHash::from([10; 32]),
                    transfer_id: TransferId::from([11; 16]),
                    now_ms: 1,
                    retain_until_ms: 100,
                    sender_wire_bytes: sender_bundle.encode().unwrap(),
                    sender_wire_hash: sender_hash,
                })
                .unwrap();
            assert_eq!(
                reservation,
                GrantReservation {
                    sender_tokens: 3,
                    receiver_tokens: 3
                }
            );
            store.mark_grant_uncertain(grant_id, 2).unwrap();
        }
        {
            let mut store = Store::open(&path).unwrap();
            assert_eq!(store.available_bundle_tokens(packet_id).unwrap(), 3);
            assert_eq!(store.grant_state(grant_id).unwrap(), 1);
            let evidence = CommitEvidence {
                payload_hash: PayloadHash::from([12; 32]),
                wire_hash: [13; 32],
                accepted_tokens: 3,
                committed_at_ms: 3,
            };
            assert_eq!(
                store.reconcile_outbound_grant(grant_id, evidence).unwrap(),
                GrantCommitOutcome::Transferred
            );
            assert_eq!(
                store.reconcile_outbound_grant(grant_id, evidence).unwrap(),
                GrantCommitOutcome::SameGrant
            );
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite3-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite3-shm"));
    }

    #[test]
    fn packet_dedup_detects_hash_conflict() {
        let (_, wire, decoded) = sample(6);
        let mut store = Store::open_in_memory().unwrap();
        assert_eq!(
            store.put_bundle(&decoded, &wire, [1; 16], 0, 0, 0).unwrap(),
            StoreBundleOutcome::Inserted
        );
        assert_eq!(
            store.put_bundle(&decoded, &wire, [1; 16], 0, 0, 0).unwrap(),
            StoreBundleOutcome::Duplicate
        );
        let mut conflict = decoded;
        conflict.bundle.routing.payload_hash = PayloadHash::from([99; 32]);
        assert_eq!(
            store
                .put_bundle(&conflict, &wire, [1; 16], 0, 0, 0)
                .unwrap(),
            StoreBundleOutcome::ConflictQuarantined
        );
    }
}
