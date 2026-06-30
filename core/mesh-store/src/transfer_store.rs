//! Crash-safe partial bundle transfer retention and bitmap resume.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use mesh_bundle::{Bundle, DecodedBundle};
use mesh_types::{PacketId, PeerLinkHash, TokenGrantId, TransferId};
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};

use crate::{Store, StoreBundleOutcome, StoreError};

pub const PARTIAL_TOTAL_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialTransferMeta {
    pub transfer_id: TransferId,
    pub packet_id: PacketId,
    pub token_grant_id: Option<TokenGrantId>,
    pub peer: PeerLinkHash,
    pub expected_wire_sha256: [u8; 32],
    pub meta_fingerprint: [u8; 32],
    pub total_size: u32,
    pub chunk_size: u16,
    pub chunk_count: u16,
    pub proposed_receiver_tokens: u8,
    pub sender_tokens_after_reservation: u8,
    pub protocol_minor: u64,
    pub now_elapsed_ms: u64,
    pub resume_expires_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialResumeState {
    pub transfer_id: TransferId,
    pub packet_id: PacketId,
    pub total_sha256: [u8; 32],
    pub chunk_size: u16,
    pub chunk_count: u16,
    pub received_bitmap: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkWriteOutcome {
    Accepted,
    Duplicate,
    Complete,
}

struct StoredPartial {
    packet_id: PacketId,
    hash: [u8; 32],
    total_size: u32,
    chunk_size: u16,
    chunk_count: u16,
    bitmap: Vec<u8>,
    temp_path: PathBuf,
}

impl Store {
    pub fn begin_partial_transfer(
        &mut self,
        meta: &PartialTransferMeta,
        partial_directory: &Path,
    ) -> Result<PartialResumeState, StoreError> {
        validate_meta(meta)?;
        fs::create_dir_all(partial_directory).map_err(io_error)?;
        if let Some(existing) = self.partial_by_transfer(meta.transfer_id)? {
            if existing.packet_id != meta.packet_id
                || existing.hash != meta.expected_wire_sha256
                || existing.total_size != meta.total_size
                || existing.chunk_size != meta.chunk_size
                || existing.chunk_count != meta.chunk_count
            {
                self.discard_partial_transfer(meta.transfer_id)?;
                return Err(StoreError::TransferConflict);
            }
            return Ok(resume_state(meta.transfer_id, existing));
        }

        let retained: i64 = self.connection.query_row(
            "SELECT COALESCE(SUM(total_size), 0) FROM transfers WHERE state = 0",
            [],
            |row| row.get(0),
        )?;
        let retained = u64::try_from(retained).map_err(|_| StoreError::IntegerOutOfRange)?;
        if retained.saturating_add(u64::from(meta.total_size)) > PARTIAL_TOTAL_BYTES {
            return Err(StoreError::PartialQuotaExceeded);
        }
        let path = partial_directory.join(format!("{}.part", hex_id(meta.transfer_id.as_bytes())));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(io_error)?;
        file.set_len(u64::from(meta.total_size)).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;

        let bitmap = vec![0u8; usize::from(meta.chunk_count).div_ceil(8)];
        let insert = self.connection.execute(
            "INSERT INTO transfers (
                transfer_id, packet_id, token_grant_id, peer_link_hash, direction,
                state, protocol_minor, expected_wire_sha256, meta_fingerprint,
                total_size, chunk_size, chunk_count, received_bitmap,
                proposed_receiver_tokens, sender_tokens_after_reservation,
                started_elapsed_ms, updated_elapsed_ms, resume_expires_at_ms, temp_path
             ) VALUES (?1, ?2, ?3, ?4, 1, 0, ?5, ?6, ?7, ?8, ?9, ?10,
                       ?11, ?12, ?13, ?14, ?14, ?15, ?16)",
            params![
                meta.transfer_id.as_bytes().as_slice(),
                meta.packet_id.as_bytes().as_slice(),
                meta.token_grant_id.map(|x| x.into_bytes().to_vec()),
                meta.peer.as_bytes().as_slice(),
                i64_value(meta.protocol_minor)?,
                meta.expected_wire_sha256.as_slice(),
                meta.meta_fingerprint.as_slice(),
                meta.total_size,
                meta.chunk_size,
                meta.chunk_count,
                bitmap,
                meta.proposed_receiver_tokens,
                meta.sender_tokens_after_reservation,
                i64_value(meta.now_elapsed_ms)?,
                i64_value(meta.resume_expires_at_ms)?,
                path.to_string_lossy(),
            ],
        );
        if let Err(error) = insert {
            let _ = fs::remove_file(&path);
            return Err(error.into());
        }
        Ok(PartialResumeState {
            transfer_id: meta.transfer_id,
            packet_id: meta.packet_id,
            total_sha256: meta.expected_wire_sha256,
            chunk_size: meta.chunk_size,
            chunk_count: meta.chunk_count,
            received_bitmap: vec![0u8; usize::from(meta.chunk_count).div_ceil(8)],
        })
    }

    pub fn write_partial_chunk(
        &mut self,
        transfer_id: TransferId,
        chunk_index: u32,
        bytes: &[u8],
        now_elapsed_ms: u64,
    ) -> Result<ChunkWriteOutcome, StoreError> {
        let mut partial = self
            .partial_by_transfer(transfer_id)?
            .ok_or(StoreError::TransferNotFound)?;
        let index = usize::try_from(chunk_index).map_err(|_| StoreError::TransferConflict)?;
        if index >= usize::from(partial.chunk_count) {
            return Err(StoreError::TransferConflict);
        }
        let expected = if index + 1 == usize::from(partial.chunk_count) {
            usize::try_from(partial.total_size).map_err(|_| StoreError::IntegerOutOfRange)?
                - usize::from(partial.chunk_size) * index
        } else {
            usize::from(partial.chunk_size)
        };
        if bytes.len() != expected {
            return Err(StoreError::TransferConflict);
        }
        let byte = index / 8;
        let bit = 1u8 << (index % 8);
        let offset = u64::try_from(index)
            .ok()
            .and_then(|x| x.checked_mul(u64::from(partial.chunk_size)))
            .ok_or(StoreError::IntegerOutOfRange)?;
        if partial.bitmap[byte] & bit != 0 {
            let mut existing = vec![0; expected];
            let mut file = File::open(&partial.temp_path).map_err(io_error)?;
            file.seek(SeekFrom::Start(offset)).map_err(io_error)?;
            file.read_exact(&mut existing).map_err(io_error)?;
            return if existing == bytes {
                Ok(ChunkWriteOutcome::Duplicate)
            } else {
                Err(StoreError::TransferConflict)
            };
        }

        let mut file = OpenOptions::new()
            .write(true)
            .open(&partial.temp_path)
            .map_err(io_error)?;
        file.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        file.write_all(bytes).map_err(io_error)?;
        file.sync_data().map_err(io_error)?;
        partial.bitmap[byte] |= bit;
        self.connection.execute(
            "UPDATE transfers SET received_bitmap = ?1, updated_elapsed_ms = ?2
             WHERE transfer_id = ?3 AND state = 0",
            params![
                partial.bitmap,
                i64_value(now_elapsed_ms)?,
                transfer_id.as_bytes().as_slice()
            ],
        )?;
        if bitmap_complete(&partial.bitmap, partial.chunk_count) {
            Ok(ChunkWriteOutcome::Complete)
        } else {
            Ok(ChunkWriteOutcome::Accepted)
        }
    }

    pub fn resume_partial_transfer(
        &self,
        packet_id: PacketId,
        expected_total_sha256: [u8; 32],
        now_elapsed_ms: u64,
    ) -> Result<Option<PartialResumeState>, StoreError> {
        let transfer_id: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT transfer_id FROM transfers
             WHERE packet_id = ?1 AND expected_wire_sha256 = ?2 AND state = 0
               AND resume_expires_at_ms >= ?3
             ORDER BY updated_elapsed_ms DESC LIMIT 1",
                params![
                    packet_id.as_bytes().as_slice(),
                    expected_total_sha256.as_slice(),
                    i64_value(now_elapsed_ms)?
                ],
                |row| row.get(0),
            )
            .optional()?;
        transfer_id
            .map(|id| TransferId::try_from(id.as_slice()).map_err(|_| StoreError::TransferConflict))
            .transpose()?
            .map(|id| {
                self.partial_by_transfer(id)?
                    .ok_or(StoreError::TransferNotFound)
                    .map(|p| resume_state(id, p))
            })
            .transpose()
    }

    pub fn commit_partial_transfer(
        &mut self,
        transfer_id: TransferId,
        received_boot_id: [u8; 16],
        elapsed_ms: u64,
        now_ms: u64,
    ) -> Result<(DecodedBundle, StoreBundleOutcome), StoreError> {
        let partial = self
            .partial_by_transfer(transfer_id)?
            .ok_or(StoreError::TransferNotFound)?;
        if !bitmap_complete(&partial.bitmap, partial.chunk_count) {
            return Err(StoreError::IncompleteTransfer);
        }
        let wire = fs::read(&partial.temp_path).map_err(io_error)?;
        if wire.len() != partial.total_size as usize
            || <[u8; 32]>::from(Sha256::digest(&wire)) != partial.hash
        {
            return Err(StoreError::TransferHashMismatch);
        }
        let decoded = Bundle::decode(&wire).map_err(|_| StoreError::TransferHashMismatch)?;
        if decoded.bundle.routing.packet_id != partial.packet_id {
            return Err(StoreError::TransferConflict);
        }
        let outcome = self.put_bundle(&decoded, &wire, received_boot_id, elapsed_ms, now_ms, 2)?;
        self.discard_partial_transfer(transfer_id)?;
        Ok((decoded, outcome))
    }

    pub fn cleanup_expired_partial_transfers(
        &mut self,
        now_elapsed_ms: u64,
    ) -> Result<usize, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT transfer_id FROM transfers WHERE state = 0 AND resume_expires_at_ms < ?1",
        )?;
        let rows =
            statement.query_map([i64_value(now_elapsed_ms)?], |row| row.get::<_, Vec<u8>>(0))?;
        let ids = rows.collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        for id in &ids {
            let transfer_id =
                TransferId::try_from(id.as_slice()).map_err(|_| StoreError::TransferConflict)?;
            self.discard_partial_transfer(transfer_id)?;
        }
        Ok(ids.len())
    }

    pub fn discard_partial_transfer(&mut self, transfer_id: TransferId) -> Result<(), StoreError> {
        let path: Option<String> = self
            .connection
            .query_row(
                "SELECT temp_path FROM transfers WHERE transfer_id = ?1",
                [transfer_id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()?;
        self.connection.execute(
            "DELETE FROM transfers WHERE transfer_id = ?1",
            [transfer_id.as_bytes().as_slice()],
        )?;
        if let Some(path) = path {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(io_error(error)),
            }
        }
        Ok(())
    }

    fn partial_by_transfer(
        &self,
        transfer_id: TransferId,
    ) -> Result<Option<StoredPartial>, StoreError> {
        self.connection
            .query_row(
                "SELECT packet_id, expected_wire_sha256, total_size, chunk_size,
                    chunk_count, received_bitmap, temp_path
             FROM transfers WHERE transfer_id = ?1 AND state = 0",
                [transfer_id.as_bytes().as_slice()],
                |row| {
                    let packet: Vec<u8> = row.get(0)?;
                    let hash: Vec<u8> = row.get(1)?;
                    Ok(StoredPartial {
                        packet_id: PacketId::try_from(packet.as_slice())
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        hash: hash.try_into().map_err(|_| rusqlite::Error::InvalidQuery)?,
                        total_size: row.get(2)?,
                        chunk_size: row.get(3)?,
                        chunk_count: row.get(4)?,
                        bitmap: row.get(5)?,
                        temp_path: PathBuf::from(row.get::<_, String>(6)?),
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }
}

fn validate_meta(meta: &PartialTransferMeta) -> Result<(), StoreError> {
    let expected = meta.total_size.div_ceil(u32::from(meta.chunk_size.max(1)));
    if !(1..=12_288).contains(&meta.total_size)
        || !(1..=4096).contains(&meta.chunk_size)
        || !(1..=1024).contains(&meta.chunk_count)
        || expected != u32::from(meta.chunk_count)
        || !(1..=16).contains(&meta.proposed_receiver_tokens)
        || !(1..=16).contains(&meta.sender_tokens_after_reservation)
        || meta.resume_expires_at_ms < meta.now_elapsed_ms
    {
        return Err(StoreError::TransferConflict);
    }
    Ok(())
}

fn resume_state(transfer_id: TransferId, partial: StoredPartial) -> PartialResumeState {
    PartialResumeState {
        transfer_id,
        packet_id: partial.packet_id,
        total_sha256: partial.hash,
        chunk_size: partial.chunk_size,
        chunk_count: partial.chunk_count,
        received_bitmap: partial.bitmap,
    }
}

fn bitmap_complete(bitmap: &[u8], count: u16) -> bool {
    (0..usize::from(count)).all(|index| bitmap[index / 8] & (1 << (index % 8)) != 0)
}

fn hex_id(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn io_error(error: std::io::Error) -> StoreError {
    StoreError::Io(error.to_string())
}

fn i64_value(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::IntegerOutOfRange)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use mesh_bundle::{Bundle, DmeCiphertext, RoutingBlock};
    use mesh_types::{
        BundleLifetime, CopyTokens, CreationSequence, HopState, MessageClass, PayloadHash,
        Priority, RandomSourceId, RoutingSlot,
    };

    use super::*;

    fn unique_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("dm-{label}-{}-{nonce}", std::process::id()))
    }

    fn wire(packet_id: PacketId) -> Vec<u8> {
        let payload = DmeCiphertext {
            encapsulated_key: [10; 32],
            aad_hash: [11; 32],
            ciphertext: vec![9],
        }
        .encode()
        .unwrap();
        Bundle {
            destination: RoutingSlot::from([2; 16]),
            source: RandomSourceId::from([3; 16]),
            creation_sequence: CreationSequence::from_u64(4),
            lifetime: BundleLifetime::from_millis(60_000).unwrap(),
            age_millis: 0,
            hops: HopState::new(0, 4).unwrap(),
            routing: RoutingBlock {
                packet_id,
                message_class: MessageClass::Direct,
                priority: Priority::P2,
                copy_tokens: CopyTokens::new(3).unwrap(),
                payload_size: payload.len() as u16,
                payload_hash: PayloadHash::from(<[u8; 32]>::from(Sha256::digest(&payload))),
            },
            payload,
        }
        .encode()
        .unwrap()
    }

    fn meta(wire: &[u8], packet_id: PacketId) -> PartialTransferMeta {
        PartialTransferMeta {
            transfer_id: TransferId::from([1; 16]),
            packet_id,
            token_grant_id: Some(TokenGrantId::from([4; 16])),
            peer: PeerLinkHash::from([5; 32]),
            expected_wire_sha256: Sha256::digest(wire).into(),
            meta_fingerprint: [6; 32],
            total_size: wire.len() as u32,
            chunk_size: 31,
            chunk_count: wire.len().div_ceil(31) as u16,
            proposed_receiver_tokens: 3,
            sender_tokens_after_reservation: 3,
            protocol_minor: 0,
            now_elapsed_ms: 10,
            resume_expires_at_ms: 610_000,
        }
    }

    #[test]
    fn partial_transfer_survives_reopen_resumes_and_commits() {
        let root = unique_dir("resume");
        fs::create_dir_all(&root).unwrap();
        let database = root.join("mesh.db");
        let partials = root.join("partial");
        let packet = PacketId::from([7; 16]);
        let bytes = wire(packet);
        let meta = meta(&bytes, packet);
        {
            let mut store = Store::open(&database).unwrap();
            store.begin_partial_transfer(&meta, &partials).unwrap();
            store
                .write_partial_chunk(meta.transfer_id, 0, &bytes[..31], 20)
                .unwrap();
        }
        {
            let mut store = Store::open(&database).unwrap();
            let state = store
                .resume_partial_transfer(packet, meta.expected_wire_sha256, 30)
                .unwrap()
                .unwrap();
            assert_eq!(state.received_bitmap[0] & 1, 1);
            for (index, chunk) in bytes.chunks(31).enumerate().skip(1) {
                store
                    .write_partial_chunk(meta.transfer_id, index as u32, chunk, 40)
                    .unwrap();
            }
            let (_, outcome) = store
                .commit_partial_transfer(meta.transfer_id, [8; 16], 50, 60)
                .unwrap();
            assert_eq!(outcome, StoreBundleOutcome::Inserted);
            assert!(
                store
                    .resume_partial_transfer(packet, meta.expected_wire_sha256, 70)
                    .unwrap()
                    .is_none()
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn chunk_conflict_and_expiry_fail_closed() {
        let root = unique_dir("conflict");
        let packet = PacketId::from([7; 16]);
        let bytes = wire(packet);
        let meta = meta(&bytes, packet);
        let mut store = Store::open_in_memory().unwrap();
        store.begin_partial_transfer(&meta, &root).unwrap();
        store
            .write_partial_chunk(meta.transfer_id, 0, &bytes[..31], 20)
            .unwrap();
        assert_eq!(
            store.write_partial_chunk(meta.transfer_id, 0, &[0; 31], 30),
            Err(StoreError::TransferConflict)
        );
        assert_eq!(store.cleanup_expired_partial_transfers(700_000).unwrap(), 1);
        assert!(
            !root
                .join(format!("{}.part", hex_id(meta.transfer_id.as_bytes())))
                .exists()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
