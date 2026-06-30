//! Token-grant-aware multi-hop bundle preparation and acknowledgement.

use core::fmt;

use mesh_bundle::Bundle;
use mesh_codec::control::{BundleMeta, ControlPayload};
use mesh_store::{
    CommitEvidence, GrantCommitOutcome, GrantReservationRequest, Store, StoreBundleOutcome,
};
use mesh_types::{CopyTokens, PacketId, PeerLinkHash, TokenGrantId, TransferId, WireBundleHash};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedTransfer {
    pub meta: BundleMeta,
    pub wire_bytes: Vec<u8>,
    pub wire_sha256: [u8; 32],
    pub payload_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelayTransferRequest {
    pub peer: PeerLinkHash,
    pub grant_id: TokenGrantId,
    pub transfer_id: TransferId,
    pub chunk_size: u16,
    pub now_ms: u64,
    pub retain_until_ms: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub enum MultihopError {
    Bundle(mesh_bundle::BundleError),
    Store(mesh_store::StoreError),
    WaitOnly,
    HopLimit,
    InvalidChunkSize,
    GrantMismatch,
}

impl fmt::Display for MultihopError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("multi-hop transfer failed")
    }
}

impl std::error::Error for MultihopError {}

impl From<mesh_bundle::BundleError> for MultihopError {
    fn from(value: mesh_bundle::BundleError) -> Self {
        Self::Bundle(value)
    }
}

impl From<mesh_store::StoreError> for MultihopError {
    fn from(value: mesh_store::StoreError) -> Self {
        Self::Store(value)
    }
}

pub fn prepare_relay_transfer(
    store: &mut Store,
    current_wire: &[u8],
    request: RelayTransferRequest,
) -> Result<PreparedTransfer, MultihopError> {
    if request.chunk_size == 0 || request.chunk_size > 4096 {
        return Err(MultihopError::InvalidChunkSize);
    }
    let decoded = Bundle::decode(current_wire)?;
    if decoded.bundle.hops.exhausted_for_relay() {
        return Err(MultihopError::HopLimit);
    }
    let (sender_tokens, receiver_tokens) =
        mesh_routing::split_tokens(decoded.bundle.routing.copy_tokens.get())
            .ok_or(MultihopError::WaitOnly)?;

    let mut sender_variant = decoded.bundle.clone();
    sender_variant.routing.copy_tokens =
        CopyTokens::new(sender_tokens).map_err(|_| MultihopError::WaitOnly)?;
    let sender_wire = sender_variant.encode()?;
    let sender_hash: [u8; 32] = Sha256::digest(&sender_wire).into();

    let mut receiver_variant = decoded.bundle;
    receiver_variant.routing.copy_tokens =
        CopyTokens::new(receiver_tokens).map_err(|_| MultihopError::WaitOnly)?;
    receiver_variant.hops = receiver_variant
        .hops
        .increment()
        .map_err(|_| MultihopError::HopLimit)?;
    let receiver_wire = receiver_variant.encode()?;
    let reservation = store.reserve_token_grant(&GrantReservationRequest {
        grant_id: request.grant_id,
        packet_id: receiver_variant.routing.packet_id,
        peer: request.peer,
        transfer_id: request.transfer_id,
        now_ms: request.now_ms,
        retain_until_ms: request.retain_until_ms,
        sender_wire_bytes: sender_wire,
        sender_wire_hash: sender_hash,
    })?;
    if reservation.sender_tokens != sender_tokens || reservation.receiver_tokens != receiver_tokens
    {
        return Err(MultihopError::GrantMismatch);
    }
    let meta = transfer_meta(
        request.transfer_id,
        Some(request.grant_id),
        receiver_variant.routing.packet_id,
        &receiver_wire,
        request.chunk_size,
        receiver_tokens,
        sender_tokens,
    )?;
    prepared(
        meta,
        receiver_wire,
        *receiver_variant.routing.payload_hash.as_bytes(),
    )
}

pub fn prepare_direct_destination_transfer(
    current_wire: &[u8],
    transfer_id: TransferId,
    chunk_size: u16,
) -> Result<PreparedTransfer, MultihopError> {
    if chunk_size == 0 || chunk_size > 4096 {
        return Err(MultihopError::InvalidChunkSize);
    }
    let decoded = Bundle::decode(current_wire)?;
    let sender_tokens = decoded.bundle.routing.copy_tokens.get();
    let mut receiver_variant = decoded.bundle;
    receiver_variant.routing.copy_tokens =
        CopyTokens::new(1).map_err(|_| MultihopError::WaitOnly)?;
    receiver_variant.hops = receiver_variant
        .hops
        .increment()
        .map_err(|_| MultihopError::HopLimit)?;
    let packet_id = receiver_variant.routing.packet_id;
    let payload_hash = *receiver_variant.routing.payload_hash.as_bytes();
    let wire = receiver_variant.encode()?;
    let meta = transfer_meta(
        transfer_id,
        None,
        packet_id,
        &wire,
        chunk_size,
        1,
        sender_tokens,
    )?;
    prepared(meta, wire, payload_hash)
}

fn transfer_meta(
    transfer_id: TransferId,
    grant_id: Option<TokenGrantId>,
    packet_id: PacketId,
    wire_bytes: &[u8],
    chunk_size: u16,
    receiver_tokens: u8,
    sender_tokens: u8,
) -> Result<BundleMeta, MultihopError> {
    let total_size =
        u32::try_from(wire_bytes.len()).map_err(|_| MultihopError::InvalidChunkSize)?;
    let chunk_count = total_size.div_ceil(u32::from(chunk_size));
    let chunk_count = u16::try_from(chunk_count).map_err(|_| MultihopError::InvalidChunkSize)?;
    let wire_sha256 = Sha256::digest(wire_bytes).into();
    Ok(BundleMeta {
        transfer_id,
        token_grant_id: grant_id,
        packet_id,
        total_size,
        sha256: wire_sha256,
        chunk_size,
        chunk_count,
        proposed_receiver_tokens: receiver_tokens,
        sender_remaining_tokens_after_reservation: sender_tokens,
    })
}

fn prepared(
    meta: BundleMeta,
    wire_bytes: Vec<u8>,
    payload_sha256: [u8; 32],
) -> Result<PreparedTransfer, MultihopError> {
    let wire_sha256 = meta.sha256;
    // Exercise the normative encoder here so invalid state cannot leave this boundary.
    ControlPayload::BundleMeta(meta.clone())
        .encode()
        .map_err(|_| MultihopError::InvalidChunkSize)?;
    Ok(PreparedTransfer {
        meta,
        wire_bytes,
        wire_sha256,
        payload_sha256,
    })
}

pub fn record_relay_commit(
    receiver: &mut Store,
    transfer: &PreparedTransfer,
    peer: PeerLinkHash,
    committed_at_ms: u64,
    retain_until_ms: u64,
) -> Result<GrantCommitOutcome, MultihopError> {
    let grant_id = transfer
        .meta
        .token_grant_id
        .ok_or(MultihopError::GrantMismatch)?;
    receiver
        .record_inbound_grant(
            grant_id,
            transfer.meta.packet_id,
            peer,
            transfer.meta.transfer_id,
            retain_until_ms,
            CommitEvidence {
                payload_hash: transfer.payload_sha256.into(),
                wire_hash: transfer.wire_sha256,
                accepted_tokens: transfer.meta.proposed_receiver_tokens,
                committed_at_ms,
            },
        )
        .map_err(Into::into)
}

pub fn acknowledge_relay_commit(
    sender: &mut Store,
    transfer: &PreparedTransfer,
    committed_at_ms: u64,
) -> Result<GrantCommitOutcome, MultihopError> {
    let grant_id = transfer
        .meta
        .token_grant_id
        .ok_or(MultihopError::GrantMismatch)?;
    sender
        .reconcile_outbound_grant(
            grant_id,
            CommitEvidence {
                payload_hash: transfer.payload_sha256.into(),
                wire_hash: transfer.wire_sha256,
                accepted_tokens: transfer.meta.proposed_receiver_tokens,
                committed_at_ms,
            },
        )
        .map_err(Into::into)
}

pub fn persist_prepared_transfer(
    receiver: &mut Store,
    transfer: &PreparedTransfer,
    boot_id: [u8; 16],
    elapsed_ms: u64,
    now_ms: u64,
) -> Result<StoreBundleOutcome, MultihopError> {
    let decoded = Bundle::decode(&transfer.wire_bytes)?;
    receiver
        .put_bundle(
            &decoded,
            &transfer.wire_bytes,
            boot_id,
            elapsed_ms,
            now_ms,
            2,
        )
        .map_err(Into::into)
}

#[must_use]
pub fn wire_hash(bytes: &[u8]) -> WireBundleHash {
    WireBundleHash::from(<[u8; 32]>::from(Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use mesh_crypto::{Identity, MessageBody};
    use mesh_store::GrantCommitOutcome;
    use mesh_types::{
        BundleLifetime, ConversationId, CopyTokens, CreationSequence, MessageId, RandomSourceId,
    };

    use super::*;
    use crate::{SecureMessageDraft, create_secure_bundle, open_secure_bundle};

    fn id16(tag: u8, run: u8) -> [u8; 16] {
        let mut value = [tag; 16];
        value[15] = run;
        value
    }

    fn store_local(store: &mut Store, wire: &[u8], now: u64) {
        let decoded = Bundle::decode(wire).unwrap();
        assert_eq!(
            store
                .put_bundle(&decoded, wire, [1; 16], now, now, 0)
                .unwrap(),
            StoreBundleOutcome::Inserted
        );
    }

    #[test]
    fn fifty_three_node_store_carry_forward_runs_and_receipt_returns() {
        let alice = Identity::generate().unwrap();
        let relay = Identity::generate().unwrap();
        let carol = Identity::generate().unwrap();

        for run in 0u8..50 {
            let mut a = Store::open_in_memory().unwrap();
            let mut b = Store::open_in_memory().unwrap();
            let mut c = Store::open_in_memory().unwrap();
            let packet = PacketId::from(id16(1, run));
            let message = MessageId::from(id16(2, run));
            let conversation = ConversationId::from(id16(3, run));
            let secured = create_secure_bundle(
                &alice,
                carol.public(),
                SecureMessageDraft {
                    packet_id: packet,
                    message_id: message,
                    conversation_id: conversation,
                    destination: carol.public().inbound_routing_slot,
                    source: RandomSourceId::from(id16(4, run)),
                    creation_sequence: CreationSequence::from_u64(u64::from(run) + 1),
                    lifetime: BundleLifetime::from_millis(259_200_000).unwrap(),
                    hop_limit: 12,
                    copy_tokens: CopyTokens::new(6).unwrap(),
                    sender_sequence: u64::from(run) + 1,
                    created_time_ms: None,
                    body: MessageBody::DirectText {
                        text: format!("run {run}"),
                        reply_to: None,
                    },
                },
            )
            .unwrap();
            store_local(&mut a, &secured.wire_bytes, 1);

            let ab = prepare_relay_transfer(
                &mut a,
                &secured.wire_bytes,
                RelayTransferRequest {
                    peer: PeerLinkHash::from([10; 32]),
                    grant_id: TokenGrantId::from(id16(11, run)),
                    transfer_id: TransferId::from(id16(12, run)),
                    chunk_size: 128,
                    now_ms: 10,
                    retain_until_ms: 700_000,
                },
            )
            .unwrap();
            assert_eq!(
                persist_prepared_transfer(&mut b, &ab, [2; 16], 20, 20).unwrap(),
                StoreBundleOutcome::Inserted
            );
            assert!(open_secure_bundle(&relay, &ab.wire_bytes).is_err());
            assert_eq!(
                record_relay_commit(&mut b, &ab, PeerLinkHash::from([20; 32]), 21, 700_000)
                    .unwrap(),
                GrantCommitOutcome::Transferred
            );
            assert_eq!(
                acknowledge_relay_commit(&mut a, &ab, 21).unwrap(),
                GrantCommitOutcome::Transferred
            );
            assert_eq!(a.available_bundle_tokens(packet).unwrap(), 3);
            assert_eq!(b.available_bundle_tokens(packet).unwrap(), 3);

            let bc = prepare_direct_destination_transfer(
                &ab.wire_bytes,
                TransferId::from(id16(13, run)),
                128,
            )
            .unwrap();
            assert_eq!(
                persist_prepared_transfer(&mut c, &bc, [3; 16], 30, 30).unwrap(),
                StoreBundleOutcome::Inserted
            );
            let (_, opened) = open_secure_bundle(&carol, &bc.wire_bytes).unwrap();
            assert!(matches!(opened.body, MessageBody::DirectText { .. }));

            let receipt_packet = PacketId::from(id16(21, run));
            let receipt = create_secure_bundle(
                &carol,
                alice.public(),
                SecureMessageDraft {
                    packet_id: receipt_packet,
                    message_id: MessageId::from(id16(22, run)),
                    conversation_id: conversation,
                    destination: alice.public().inbound_routing_slot,
                    source: RandomSourceId::from(id16(23, run)),
                    creation_sequence: CreationSequence::from_u64(100 + u64::from(run)),
                    lifetime: BundleLifetime::from_millis(604_800_000).unwrap(),
                    hop_limit: 16,
                    copy_tokens: CopyTokens::new(12).unwrap(),
                    sender_sequence: u64::from(run) + 1,
                    created_time_ms: None,
                    body: MessageBody::DeliveryReceipt {
                        original_packet_id: packet,
                        original_message_id: message,
                        receiver_note: None,
                    },
                },
            )
            .unwrap();
            store_local(&mut c, &receipt.wire_bytes, 40);
            let cb = prepare_relay_transfer(
                &mut c,
                &receipt.wire_bytes,
                RelayTransferRequest {
                    peer: PeerLinkHash::from([30; 32]),
                    grant_id: TokenGrantId::from(id16(31, run)),
                    transfer_id: TransferId::from(id16(32, run)),
                    chunk_size: 128,
                    now_ms: 41,
                    retain_until_ms: 700_000,
                },
            )
            .unwrap();
            assert_eq!(
                persist_prepared_transfer(&mut b, &cb, [2; 16], 42, 42).unwrap(),
                StoreBundleOutcome::Inserted
            );
            record_relay_commit(&mut b, &cb, PeerLinkHash::from([40; 32]), 43, 700_000).unwrap();
            acknowledge_relay_commit(&mut c, &cb, 43).unwrap();
            let ba = prepare_direct_destination_transfer(
                &cb.wire_bytes,
                TransferId::from(id16(33, run)),
                128,
            )
            .unwrap();
            assert_eq!(
                persist_prepared_transfer(&mut a, &ba, [1; 16], 44, 44).unwrap(),
                StoreBundleOutcome::Inserted
            );
            assert!(
                matches!(open_secure_bundle(&alice, &ba.wire_bytes).unwrap().1.body, MessageBody::DeliveryReceipt { original_packet_id, .. } if original_packet_id == packet)
            );
        }
    }

    #[test]
    fn lost_ack_remains_uncertain_and_same_grant_is_idempotent() {
        let sender = Identity::generate().unwrap();
        let recipient = Identity::generate().unwrap();
        let packet = PacketId::from([1; 16]);
        let secured = create_secure_bundle(
            &sender,
            recipient.public(),
            SecureMessageDraft {
                packet_id: packet,
                message_id: MessageId::from([2; 16]),
                conversation_id: ConversationId::from([3; 16]),
                destination: recipient.public().inbound_routing_slot,
                source: RandomSourceId::from([4; 16]),
                creation_sequence: CreationSequence::from_u64(1),
                lifetime: BundleLifetime::from_millis(60_000).unwrap(),
                hop_limit: 12,
                copy_tokens: CopyTokens::new(6).unwrap(),
                sender_sequence: 1,
                created_time_ms: None,
                body: MessageBody::DirectText {
                    text: "lost ACK".into(),
                    reply_to: None,
                },
            },
        )
        .unwrap();
        let mut a = Store::open_in_memory().unwrap();
        let mut b = Store::open_in_memory().unwrap();
        store_local(&mut a, &secured.wire_bytes, 1);
        let transfer = prepare_relay_transfer(
            &mut a,
            &secured.wire_bytes,
            RelayTransferRequest {
                peer: PeerLinkHash::from([5; 32]),
                grant_id: TokenGrantId::from([6; 16]),
                transfer_id: TransferId::from([7; 16]),
                chunk_size: 128,
                now_ms: 2,
                retain_until_ms: 1000,
            },
        )
        .unwrap();
        persist_prepared_transfer(&mut b, &transfer, [0; 16], 3, 3).unwrap();
        assert_eq!(
            record_relay_commit(&mut b, &transfer, PeerLinkHash::from([8; 32]), 4, 1000).unwrap(),
            GrantCommitOutcome::Transferred
        );
        a.mark_grant_uncertain(TokenGrantId::from([6; 16]), 5)
            .unwrap();
        assert_eq!(a.grant_state(TokenGrantId::from([6; 16])).unwrap(), 1);
        assert_eq!(
            record_relay_commit(&mut b, &transfer, PeerLinkHash::from([8; 32]), 4, 1000).unwrap(),
            GrantCommitOutcome::SameGrant
        );
        assert_eq!(
            acknowledge_relay_commit(&mut a, &transfer, 4).unwrap(),
            GrantCommitOutcome::Transferred
        );
        assert_eq!(
            acknowledge_relay_commit(&mut a, &transfer, 4).unwrap(),
            GrantCommitOutcome::SameGrant
        );
        assert_eq!(a.available_bundle_tokens(packet).unwrap(), 3);
    }
}
