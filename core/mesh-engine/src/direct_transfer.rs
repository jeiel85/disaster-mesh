//! Goal 3 safe full-retry direct transfer assembly and durable commit.

use core::fmt;

use mesh_bundle::{Bundle, DecodedBundle};
use mesh_codec::ble::BundleChunk;
use mesh_store::{Store, StoreBundleOutcome};
use mesh_types::{PacketId, TransferId};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectTransferMeta {
    pub transfer_id: TransferId,
    pub packet_id: PacketId,
    pub total_size: u32,
    pub sha256: [u8; 32],
    pub chunk_size: u16,
    pub chunk_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectTransferProgress {
    Accepted,
    Duplicate,
    Complete,
}

#[derive(Debug, Eq, PartialEq)]
pub enum DirectTransferError {
    InvalidMeta,
    WrongTransfer,
    InvalidChunk,
    ChunkConflict,
    Incomplete,
    HashMismatch,
    PacketMismatch,
    Bundle(mesh_bundle::BundleError),
    Store(mesh_store::StoreError),
}

impl fmt::Display for DirectTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("direct transfer failed")
    }
}

impl std::error::Error for DirectTransferError {}

impl From<mesh_bundle::BundleError> for DirectTransferError {
    fn from(value: mesh_bundle::BundleError) -> Self {
        Self::Bundle(value)
    }
}

impl From<mesh_store::StoreError> for DirectTransferError {
    fn from(value: mesh_store::StoreError) -> Self {
        Self::Store(value)
    }
}

pub struct DirectInboundTransfer {
    meta: DirectTransferMeta,
    chunks: Vec<Option<Vec<u8>>>,
    received_bytes: usize,
}

impl DirectInboundTransfer {
    pub fn new(meta: DirectTransferMeta) -> Result<Self, DirectTransferError> {
        let expected_count = usize::try_from(meta.total_size)
            .map_err(|_| DirectTransferError::InvalidMeta)?
            .div_ceil(usize::from(meta.chunk_size.max(1)));
        if meta.total_size == 0
            || meta.total_size > 12_288
            || meta.chunk_size == 0
            || meta.chunk_size > 4096
            || meta.chunk_count == 0
            || meta.chunk_count > 1024
            || expected_count != usize::from(meta.chunk_count)
        {
            return Err(DirectTransferError::InvalidMeta);
        }
        Ok(Self {
            meta,
            chunks: vec![None; usize::from(meta.chunk_count)],
            received_bytes: 0,
        })
    }

    pub fn accept_chunk(
        &mut self,
        chunk: BundleChunk,
    ) -> Result<DirectTransferProgress, DirectTransferError> {
        if chunk.transfer_id != *self.meta.transfer_id.as_bytes() {
            return Err(DirectTransferError::WrongTransfer);
        }
        let index =
            usize::try_from(chunk.chunk_index).map_err(|_| DirectTransferError::InvalidChunk)?;
        if index >= self.chunks.len() {
            return Err(DirectTransferError::InvalidChunk);
        }
        let expected = if index + 1 == self.chunks.len() {
            usize::try_from(self.meta.total_size).map_err(|_| DirectTransferError::InvalidMeta)?
                - usize::from(self.meta.chunk_size) * (self.chunks.len() - 1)
        } else {
            usize::from(self.meta.chunk_size)
        };
        if chunk.bytes.len() != expected {
            return Err(DirectTransferError::InvalidChunk);
        }
        if let Some(existing) = &self.chunks[index] {
            return if existing == &chunk.bytes {
                Ok(DirectTransferProgress::Duplicate)
            } else {
                Err(DirectTransferError::ChunkConflict)
            };
        }
        self.received_bytes = self
            .received_bytes
            .checked_add(chunk.bytes.len())
            .ok_or(DirectTransferError::InvalidChunk)?;
        self.chunks[index] = Some(chunk.bytes);
        if self.received_bytes == self.meta.total_size as usize
            && self.chunks.iter().all(Option::is_some)
        {
            Ok(DirectTransferProgress::Complete)
        } else {
            Ok(DirectTransferProgress::Accepted)
        }
    }

    pub fn commit(
        self,
        store: &mut Store,
        received_boot_id: [u8; 16],
        elapsed_ms: u64,
        now_ms: u64,
    ) -> Result<(DecodedBundle, StoreBundleOutcome), DirectTransferError> {
        if self.received_bytes != self.meta.total_size as usize
            || self.chunks.iter().any(Option::is_none)
        {
            return Err(DirectTransferError::Incomplete);
        }
        let wire_bytes = self
            .chunks
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(DirectTransferError::Incomplete)?
            .concat();
        if <[u8; 32]>::from(Sha256::digest(&wire_bytes)) != self.meta.sha256 {
            return Err(DirectTransferError::HashMismatch);
        }
        let decoded = Bundle::decode(&wire_bytes)?;
        if decoded.bundle.routing.packet_id != self.meta.packet_id {
            return Err(DirectTransferError::PacketMismatch);
        }
        let outcome = store.put_bundle(
            &decoded,
            &wire_bytes,
            received_boot_id,
            elapsed_ms,
            now_ms,
            2,
        )?;
        Ok((decoded, outcome))
    }
}

#[cfg(test)]
mod tests {
    use mesh_crypto::{Identity, MessageBody};
    use mesh_types::{
        BundleLifetime, ConversationId, CopyTokens, CreationSequence, MessageId, RandomSourceId,
    };

    use super::*;
    use crate::{SecureMessageDraft, create_secure_bundle, open_secure_bundle};

    fn chunks(wire: &[u8], transfer_id: TransferId, chunk_size: usize) -> Vec<BundleChunk> {
        wire.chunks(chunk_size)
            .enumerate()
            .map(|(index, bytes)| BundleChunk {
                transfer_id: *transfer_id.as_bytes(),
                chunk_index: u32::try_from(index).unwrap(),
                bytes: bytes.to_vec(),
            })
            .collect()
    }

    #[test]
    fn direct_e2ee_transfer_commits_only_after_full_hash_and_receipt_returns() {
        let sender = Identity::generate().unwrap();
        let recipient = Identity::generate().unwrap();
        let packet_id = PacketId::from([1; 16]);
        let message_id = MessageId::from([2; 16]);
        let draft = SecureMessageDraft {
            packet_id,
            message_id,
            conversation_id: ConversationId::from([3; 16]),
            destination: recipient.public().inbound_routing_slot,
            source: RandomSourceId::from([4; 16]),
            creation_sequence: CreationSequence::from_u64(5),
            lifetime: BundleLifetime::from_millis(259_200_000).unwrap(),
            hop_limit: 12,
            copy_tokens: CopyTokens::new(6).unwrap(),
            sender_sequence: 1,
            created_time_ms: None,
            body: MessageBody::DirectText {
                text: "direct BLE E2EE".into(),
                reply_to: None,
            },
        };
        let secured = create_secure_bundle(&sender, recipient.public(), draft).unwrap();
        let transfer_id = TransferId::from([6; 16]);
        let chunk_size = 37usize;
        let parts = chunks(&secured.wire_bytes, transfer_id, chunk_size);
        let meta = DirectTransferMeta {
            transfer_id,
            packet_id,
            total_size: u32::try_from(secured.wire_bytes.len()).unwrap(),
            sha256: Sha256::digest(&secured.wire_bytes).into(),
            chunk_size: u16::try_from(chunk_size).unwrap(),
            chunk_count: u16::try_from(parts.len()).unwrap(),
        };
        let mut incoming = DirectInboundTransfer::new(meta).unwrap();
        for part in parts.into_iter().rev() {
            incoming.accept_chunk(part).unwrap();
        }
        let mut store = Store::open_in_memory().unwrap();
        let (decoded, outcome) = incoming.commit(&mut store, [7; 16], 10, 20).unwrap();
        assert_eq!(outcome, StoreBundleOutcome::Inserted);
        let (_, plaintext) = open_secure_bundle(&recipient, &secured.wire_bytes).unwrap();
        assert!(matches!(plaintext.body, MessageBody::DirectText { .. }));
        assert_eq!(decoded.bundle.routing.packet_id, packet_id);

        let receipt = create_secure_bundle(
            &recipient,
            sender.public(),
            SecureMessageDraft {
                packet_id: PacketId::from([8; 16]),
                message_id: MessageId::from([9; 16]),
                conversation_id: ConversationId::from([3; 16]),
                destination: sender.public().inbound_routing_slot,
                source: RandomSourceId::from([10; 16]),
                creation_sequence: CreationSequence::from_u64(11),
                lifetime: BundleLifetime::from_millis(604_800_000).unwrap(),
                hop_limit: 16,
                copy_tokens: CopyTokens::new(12).unwrap(),
                sender_sequence: 1,
                created_time_ms: None,
                body: MessageBody::DeliveryReceipt {
                    original_packet_id: packet_id,
                    original_message_id: message_id,
                    receiver_note: None,
                },
            },
        )
        .unwrap();
        assert!(matches!(
            open_secure_bundle(&sender, &receipt.wire_bytes)
                .unwrap()
                .1
                .body,
            MessageBody::DeliveryReceipt { .. }
        ));
    }

    #[test]
    fn interrupted_or_corrupt_transfer_never_creates_bundle_row() {
        let mut store = Store::open_in_memory().unwrap();
        let meta = DirectTransferMeta {
            transfer_id: TransferId::from([1; 16]),
            packet_id: PacketId::from([2; 16]),
            total_size: 8,
            sha256: [3; 32],
            chunk_size: 4,
            chunk_count: 2,
        };
        let mut incoming = DirectInboundTransfer::new(meta).unwrap();
        incoming
            .accept_chunk(BundleChunk {
                transfer_id: [1; 16],
                chunk_index: 0,
                bytes: vec![0; 4],
            })
            .unwrap();
        assert_eq!(
            incoming.commit(&mut store, [0; 16], 0, 0),
            Err(DirectTransferError::Incomplete)
        );
        assert_eq!(
            store
                .connection()
                .query_row("SELECT COUNT(*) FROM bundles", [], |row| row
                    .get::<_, u32>(0))
                .unwrap(),
            0
        );
    }
}
