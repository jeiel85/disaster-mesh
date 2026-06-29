//! Direct Delivery and Binary Spray-and-Wait decisions.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use mesh_types::{
    BootId, CopyTokens, HopState, MessageClass, PacketId, PayloadHash, PeerLinkHash, Priority,
    RoutingSlot, TokenGrantId,
};

pub const CRATE_NAME: &str = "mesh-routing";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestReason {
    DirectDestination,
    RelayCopy,
    ReceiptOrCancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectReason {
    Tombstoned,
    Duplicate,
    Expired,
    SizeOrSessionLimit,
    IngressRateLimited,
    RelayDisabled,
    HopLimit,
    WaitOnly,
    RelayQuota,
    SourceRateLimited,
    UnverifiedDirectQuota,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteDecision {
    Request(RequestReason),
    Reject(RejectReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BundleSummary {
    pub packet_id: PacketId,
    pub destination: RoutingSlot,
    pub priority: Priority,
    pub message_class: MessageClass,
    pub remaining_lifetime_seconds: u64,
    pub hops: HopState,
    pub copy_tokens: CopyTokens,
    pub total_bundle_bytes: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeerContext {
    pub owns_destination: bool,
    pub tombstoned: bool,
    pub already_committed: bool,
    pub session_capacity_available: bool,
    pub ingress_allowed: bool,
    pub unverified_direct_quota_available: bool,
    pub relay_enabled: bool,
    pub relay_quota_available: bool,
    pub source_allowed: bool,
}

#[must_use]
pub fn route_decision(summary: BundleSummary, peer: PeerContext) -> RouteDecision {
    if peer.tombstoned {
        return RouteDecision::Reject(RejectReason::Tombstoned);
    }
    if peer.already_committed {
        return RouteDecision::Reject(RejectReason::Duplicate);
    }
    if summary.remaining_lifetime_seconds == 0 {
        return RouteDecision::Reject(RejectReason::Expired);
    }
    if summary.total_bundle_bytes == 0
        || summary.total_bundle_bytes > 12_288
        || !peer.session_capacity_available
    {
        return RouteDecision::Reject(RejectReason::SizeOrSessionLimit);
    }
    if !peer.ingress_allowed {
        return RouteDecision::Reject(RejectReason::IngressRateLimited);
    }
    if peer.owns_destination {
        return if peer.unverified_direct_quota_available {
            RouteDecision::Request(RequestReason::DirectDestination)
        } else {
            RouteDecision::Reject(RejectReason::UnverifiedDirectQuota)
        };
    }
    if !peer.relay_enabled {
        return RouteDecision::Reject(RejectReason::RelayDisabled);
    }
    if summary.hops.exhausted_for_relay() {
        return RouteDecision::Reject(RejectReason::HopLimit);
    }
    if summary.copy_tokens.get() < 2 {
        return RouteDecision::Reject(RejectReason::WaitOnly);
    }
    if !peer.relay_quota_available {
        return RouteDecision::Reject(RejectReason::RelayQuota);
    }
    if !peer.source_allowed {
        return RouteDecision::Reject(RejectReason::SourceRateLimited);
    }
    if matches!(
        summary.message_class,
        MessageClass::Receipt | MessageClass::Cancel
    ) {
        RouteDecision::Request(RequestReason::ReceiptOrCancel)
    } else {
        RouteDecision::Request(RequestReason::RelayCopy)
    }
}

#[must_use]
pub const fn split_tokens(tokens: u8) -> Option<(u8, u8)> {
    if tokens < 2 {
        return None;
    }
    let receiver = tokens / 2;
    Some((tokens - receiver, receiver))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScoreInput {
    pub priority: Priority,
    pub direct_destination: bool,
    pub message_class: MessageClass,
    pub remaining_seconds: u64,
    pub age_minutes: u64,
    pub size_bytes: u32,
    pub offered_to_same_peer_within_ten_minutes: bool,
    pub peer_failures_24h: u32,
}

#[must_use]
pub fn queue_score(input: ScoreInput) -> i64 {
    let priority_weight = match input.priority {
        Priority::P0 => 1_000_000,
        Priority::P1 => 500_000,
        Priority::P2 => 100_000,
        Priority::P3 => 0,
    };
    let direct = if input.direct_destination {
        2_000_000
    } else {
        0
    };
    let control = if matches!(
        input.message_class,
        MessageClass::Receipt | MessageClass::Cancel
    ) {
        750_000
    } else {
        0
    };
    let urgency = 100_000u64.saturating_sub(input.remaining_seconds) as i64;
    let age = input.age_minutes.min(10_000) as i64;
    let efficiency = 8_192u32.saturating_sub(input.size_bytes) as i64;
    let recent = if input.offered_to_same_peer_within_ten_minutes {
        100_000
    } else {
        0
    };
    let failures = i64::from(input.peer_failures_24h.saturating_mul(10_000).min(100_000));
    priority_weight + direct + control + urgency + age + efficiency - recent - failures
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OfferCandidate {
    pub packet_id: PacketId,
    pub score: i64,
}

pub fn sort_offers(candidates: &mut [OfferCandidate]) {
    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.packet_id.cmp(&right.packet_id))
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrantState {
    Reserved,
    Uncertain,
    Transferred,
    Released,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GrantEvidence {
    pub payload_hash: PayloadHash,
    pub wire_hash: [u8; 32],
    pub accepted_tokens: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenGrant {
    pub id: TokenGrantId,
    pub peer: PeerLinkHash,
    pub tokens: u8,
    pub state: GrantState,
    pub evidence: Option<GrantEvidence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconcileOutcome {
    Transferred,
    SameGrant,
    Conflict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GrantError {
    WaitOnly,
    DuplicateGrant,
    UnknownGrant,
    InvalidTransition,
    TokenOverflow,
}

#[derive(Clone, Debug)]
pub struct TokenLedger {
    initial_tokens: u8,
    available_tokens: u8,
    grants: BTreeMap<TokenGrantId, TokenGrant>,
}

impl TokenLedger {
    #[must_use]
    pub fn new(tokens: CopyTokens) -> Self {
        Self {
            initial_tokens: tokens.get(),
            available_tokens: tokens.get(),
            grants: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn available_tokens(&self) -> u8 {
        self.available_tokens
    }

    pub fn reserve(
        &mut self,
        id: TokenGrantId,
        peer: PeerLinkHash,
    ) -> Result<TokenGrant, GrantError> {
        if self.grants.contains_key(&id) {
            return Err(GrantError::DuplicateGrant);
        }
        let (sender, receiver) = split_tokens(self.available_tokens).ok_or(GrantError::WaitOnly)?;
        self.available_tokens = sender;
        let grant = TokenGrant {
            id,
            peer,
            tokens: receiver,
            state: GrantState::Reserved,
            evidence: None,
        };
        self.grants.insert(id, grant);
        Ok(grant)
    }

    pub fn mark_uncertain(&mut self, id: TokenGrantId) -> Result<(), GrantError> {
        let grant = self.grants.get_mut(&id).ok_or(GrantError::UnknownGrant)?;
        if grant.state != GrantState::Reserved {
            return Err(GrantError::InvalidTransition);
        }
        grant.state = GrantState::Uncertain;
        Ok(())
    }

    pub fn reconcile_committed(
        &mut self,
        id: TokenGrantId,
        evidence: GrantEvidence,
    ) -> Result<ReconcileOutcome, GrantError> {
        let grant = self.grants.get_mut(&id).ok_or(GrantError::UnknownGrant)?;
        if evidence.accepted_tokens != grant.tokens {
            return Ok(ReconcileOutcome::Conflict);
        }
        match grant.state {
            GrantState::Reserved | GrantState::Uncertain => {
                grant.state = GrantState::Transferred;
                grant.evidence = Some(evidence);
                Ok(ReconcileOutcome::Transferred)
            }
            GrantState::Transferred if grant.evidence == Some(evidence) => {
                Ok(ReconcileOutcome::SameGrant)
            }
            GrantState::Transferred => Ok(ReconcileOutcome::Conflict),
            GrantState::Released => Err(GrantError::InvalidTransition),
        }
    }

    pub fn release_confirmed_not_committed(&mut self, id: TokenGrantId) -> Result<(), GrantError> {
        let grant = self.grants.get_mut(&id).ok_or(GrantError::UnknownGrant)?;
        if !matches!(grant.state, GrantState::Reserved | GrantState::Uncertain) {
            return Err(GrantError::InvalidTransition);
        }
        self.available_tokens = self
            .available_tokens
            .checked_add(grant.tokens)
            .filter(|tokens| *tokens <= 16)
            .ok_or(GrantError::TokenOverflow)?;
        grant.state = GrantState::Released;
        Ok(())
    }

    #[must_use]
    pub fn grant(&self, id: TokenGrantId) -> Option<&TokenGrant> {
        self.grants.get(&id)
    }

    #[must_use]
    pub fn allocated_tokens(&self) -> u8 {
        self.available_tokens
            + self
                .grants
                .values()
                .filter(|grant| grant.state != GrantState::Released)
                .map(|grant| grant.tokens)
                .sum::<u8>()
    }

    #[must_use]
    pub const fn initial_tokens(&self) -> u8 {
        self.initial_tokens
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DedupDecision {
    Inserted,
    Duplicate,
    ConflictQuarantined,
}

#[derive(Default)]
pub struct DedupIndex(BTreeMap<PacketId, PayloadHash>);

impl DedupIndex {
    pub fn observe(&mut self, packet_id: PacketId, payload_hash: PayloadHash) -> DedupDecision {
        match self.0.get(&packet_id) {
            None => {
                self.0.insert(packet_id, payload_hash);
                DedupDecision::Inserted
            }
            Some(existing) if existing == &payload_hash => DedupDecision::Duplicate,
            Some(_) => DedupDecision::ConflictQuarantined,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgeResult {
    Offerable(u64),
    Expired,
    AgeUncertain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgeContext {
    pub stored_age_ms: u64,
    pub age_anchor_elapsed_ms: u64,
    pub received_boot_id: BootId,
    pub current_boot_id: BootId,
    pub current_elapsed_ms: u64,
    pub previous_wall_ms: Option<u64>,
    pub current_wall_ms: Option<u64>,
    pub lifetime_ms: u64,
}

#[must_use]
pub fn current_age(context: AgeContext) -> AgeResult {
    let delta = if context.received_boot_id == context.current_boot_id {
        let Some(delta) = context
            .current_elapsed_ms
            .checked_sub(context.age_anchor_elapsed_ms)
        else {
            return AgeResult::AgeUncertain;
        };
        delta
    } else {
        let (Some(previous), Some(current)) = (context.previous_wall_ms, context.current_wall_ms)
        else {
            return AgeResult::AgeUncertain;
        };
        let Some(delta) = current.checked_sub(previous) else {
            return AgeResult::AgeUncertain;
        };
        delta
    };
    let age = context.stored_age_ms.saturating_add(delta);
    if age >= context.lifetime_ms {
        AgeResult::Expired
    } else {
        AgeResult::Offerable(age)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuotaSnapshot {
    pub peer_daily_bytes: u64,
    pub peer_unverified_direct_daily_bytes: u64,
    pub source_bytes: u64,
    pub relay_total_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuotaDecision {
    Accept,
    PeerDailyExceeded,
    UnverifiedDirectExceeded,
    SourceExceeded,
    RelayTotalExceeded,
}

#[must_use]
pub fn quota_decision(
    snapshot: QuotaSnapshot,
    incoming: u64,
    direct_unverified: bool,
) -> QuotaDecision {
    if snapshot.peer_daily_bytes.saturating_add(incoming) > 8_388_608 {
        return QuotaDecision::PeerDailyExceeded;
    }
    if direct_unverified
        && snapshot
            .peer_unverified_direct_daily_bytes
            .saturating_add(incoming)
            > 2_097_152
    {
        return QuotaDecision::UnverifiedDirectExceeded;
    }
    if snapshot.source_bytes.saturating_add(incoming) > 4_194_304 {
        return QuotaDecision::SourceExceeded;
    }
    if snapshot.relay_total_bytes.saturating_add(incoming) > 33_554_432 {
        return QuotaDecision::RelayTotalExceeded;
    }
    QuotaDecision::Accept
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvictionKind {
    InvalidOrQuarantined,
    Expired,
    ReceiptConfirmedOriginal,
    CanceledOriginal,
    StalePartial,
    Relay,
    VerifiedLocal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvictionCandidate {
    pub packet_id: PacketId,
    pub kind: EvictionKind,
    pub priority: Priority,
    pub created_at_ms: u64,
    pub score: i64,
    pub size_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvictionPolicy {
    pub protected_bytes: u64,
    pub protected_floor_bytes: u64,
    pub disk_full: bool,
}

#[must_use]
pub fn eviction_plan(
    candidates: &[EvictionCandidate],
    required_bytes: u64,
    policy: EvictionPolicy,
) -> Vec<PacketId> {
    let mut ordered = candidates.to_vec();
    ordered.sort_by(|left, right| {
        eviction_rank(*left, policy.disk_full)
            .cmp(&eviction_rank(*right, policy.disk_full))
            .then_with(|| left.score.cmp(&right.score))
            .then_with(|| left.created_at_ms.cmp(&right.created_at_ms))
            .then_with(|| left.packet_id.cmp(&right.packet_id))
    });
    let mut freed = 0u64;
    let mut protected_remaining = policy.protected_bytes;
    let mut result = Vec::new();
    for candidate in ordered {
        if eviction_rank(candidate, policy.disk_full) == u8::MAX {
            continue;
        }
        if candidate.kind == EvictionKind::VerifiedLocal
            && protected_remaining.saturating_sub(candidate.size_bytes)
                < policy.protected_floor_bytes
        {
            continue;
        }
        if candidate.kind == EvictionKind::VerifiedLocal {
            protected_remaining = protected_remaining.saturating_sub(candidate.size_bytes);
        }
        result.push(candidate.packet_id);
        freed = freed.saturating_add(candidate.size_bytes);
        if freed >= required_bytes {
            break;
        }
    }
    result
}

fn eviction_rank(candidate: EvictionCandidate, disk_full: bool) -> u8 {
    match candidate.kind {
        EvictionKind::InvalidOrQuarantined => 0,
        EvictionKind::Expired => 1,
        EvictionKind::ReceiptConfirmedOriginal => 2,
        EvictionKind::CanceledOriginal => 3,
        EvictionKind::StalePartial => 4,
        EvictionKind::Relay => match candidate.priority {
            Priority::P3 => 5,
            Priority::P2 => 6,
            Priority::P1 => 7,
            Priority::P0 => 8,
        },
        EvictionKind::VerifiedLocal => match candidate.priority {
            Priority::P1 | Priority::P2 | Priority::P3 => 9,
            Priority::P0 if disk_full => 10,
            Priority::P0 => u8::MAX,
        },
    }
}

#[must_use]
pub fn tombstone_expiry(created_at_ms: u64, original_lifetime_ms: u64, priority: Priority) -> u64 {
    let retention = original_lifetime_ms
        .saturating_add(86_400_000)
        .min(691_200_000);
    let retention = if priority == Priority::P0 {
        retention.max(172_800_000)
    } else {
        retention
    };
    created_at_ms.saturating_add(retention)
}

#[must_use]
pub const fn bundle_boundary() -> &'static str {
    mesh_bundle::CRATE_NAME
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn summary(tokens: u8) -> BundleSummary {
        BundleSummary {
            packet_id: PacketId::from([1; 16]),
            destination: RoutingSlot::from([2; 16]),
            priority: Priority::P2,
            message_class: MessageClass::Direct,
            remaining_lifetime_seconds: 60,
            hops: HopState::new(0, 12).unwrap(),
            copy_tokens: CopyTokens::new(tokens).unwrap(),
            total_bundle_bytes: 100,
        }
    }

    fn peer() -> PeerContext {
        PeerContext {
            owns_destination: false,
            tombstoned: false,
            already_committed: false,
            session_capacity_available: true,
            ingress_allowed: true,
            unverified_direct_quota_available: true,
            relay_enabled: true,
            relay_quota_available: true,
            source_allowed: true,
        }
    }

    #[test]
    fn direct_bypasses_tokens_but_not_expiry_or_quota() {
        let mut context = peer();
        context.owns_destination = true;
        assert_eq!(
            route_decision(summary(1), context),
            RouteDecision::Request(RequestReason::DirectDestination)
        );
        let mut expired = summary(1);
        expired.remaining_lifetime_seconds = 0;
        assert_eq!(
            route_decision(expired, context),
            RouteDecision::Reject(RejectReason::Expired)
        );
    }

    #[test]
    fn hop_boundary_and_wait_only_are_enforced() {
        assert_eq!(
            route_decision(summary(1), peer()),
            RouteDecision::Reject(RejectReason::WaitOnly)
        );
        let mut at_boundary = summary(2);
        at_boundary.hops = HopState::new(11, 12).unwrap();
        assert_eq!(
            route_decision(at_boundary, peer()),
            RouteDecision::Reject(RejectReason::HopLimit)
        );
    }

    #[test]
    fn uncertain_grant_is_not_reused_and_same_grant_is_idempotent() {
        let grant_id = TokenGrantId::from([3; 16]);
        let mut ledger = TokenLedger::new(CopyTokens::new(6).unwrap());
        let grant = ledger
            .reserve(grant_id, PeerLinkHash::from([4; 32]))
            .unwrap();
        assert_eq!((ledger.available_tokens(), grant.tokens), (3, 3));
        ledger.mark_uncertain(grant_id).unwrap();
        let second = ledger.reserve(TokenGrantId::from([5; 16]), PeerLinkHash::from([6; 32]));
        assert_eq!(second.unwrap().tokens, 1);
        let evidence = GrantEvidence {
            payload_hash: PayloadHash::from([7; 32]),
            wire_hash: [8; 32],
            accepted_tokens: 3,
        };
        assert_eq!(
            ledger.reconcile_committed(grant_id, evidence).unwrap(),
            ReconcileOutcome::Transferred
        );
        assert_eq!(
            ledger.reconcile_committed(grant_id, evidence).unwrap(),
            ReconcileOutcome::SameGrant
        );
        assert_eq!(ledger.allocated_tokens(), ledger.initial_tokens());
    }

    #[test]
    fn score_and_tie_break_are_deterministic() {
        let mut candidates = [
            OfferCandidate {
                packet_id: PacketId::from([2; 16]),
                score: 10,
            },
            OfferCandidate {
                packet_id: PacketId::from([1; 16]),
                score: 10,
            },
            OfferCandidate {
                packet_id: PacketId::from([3; 16]),
                score: 11,
            },
        ];
        sort_offers(&mut candidates);
        assert_eq!(candidates[0].packet_id, PacketId::from([3; 16]));
        assert_eq!(candidates[1].packet_id, PacketId::from([1; 16]));
    }

    #[test]
    fn invalid_reboot_checkpoint_fails_closed() {
        let result = current_age(AgeContext {
            stored_age_ms: 1,
            age_anchor_elapsed_ms: 10,
            received_boot_id: BootId::from([1; 16]),
            current_boot_id: BootId::from([2; 16]),
            current_elapsed_ms: 5,
            previous_wall_ms: Some(100),
            current_wall_ms: Some(99),
            lifetime_ms: 1_000,
        });
        assert_eq!(result, AgeResult::AgeUncertain);
    }

    proptest! {
        #[test]
        fn token_split_conserves_every_valid_count(tokens in 2u8..=16) {
            let (sender, receiver) = split_tokens(tokens).unwrap();
            prop_assert_eq!(sender + receiver, tokens);
            prop_assert!(sender >= 1);
            prop_assert!(receiver >= 1);
        }

        #[test]
        fn age_never_decreases_within_boot(
            stored in 0u64..1_000_000,
            first in 0u64..1_000_000,
            delta in 0u64..1_000_000,
        ) {
            let boot = BootId::from([9; 16]);
            let lifetime = u64::MAX;
            let first_age = current_age(AgeContext {
                stored_age_ms: stored,
                age_anchor_elapsed_ms: 0,
                received_boot_id: boot,
                current_boot_id: boot,
                current_elapsed_ms: first,
                previous_wall_ms: None,
                current_wall_ms: None,
                lifetime_ms: lifetime,
            });
            let second_age = current_age(AgeContext {
                stored_age_ms: stored,
                age_anchor_elapsed_ms: 0,
                received_boot_id: boot,
                current_boot_id: boot,
                current_elapsed_ms: first.saturating_add(delta),
                previous_wall_ms: None,
                current_wall_ms: None,
                lifetime_ms: lifetime,
            });
            let (AgeResult::Offerable(first_age), AgeResult::Offerable(second_age)) = (first_age, second_age) else {
                prop_assert!(false, "ages should be offerable");
                return Ok(());
            };
            prop_assert!(second_age >= first_age);
        }
    }
}
