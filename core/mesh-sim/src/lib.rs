//! Deterministic contact-graph simulator using production routing decisions.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use mesh_routing::{
    BundleSummary, PeerContext, RequestReason, RouteDecision, route_decision, split_tokens,
};
use mesh_types::{CopyTokens, HopState, MessageClass, PacketId, Priority, RoutingSlot};

pub type NodeId = usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Injection {
    pub source: NodeId,
    pub destination: NodeId,
    pub packet_id: PacketId,
    pub tokens: CopyTokens,
    pub priority: Priority,
    pub message_class: MessageClass,
    pub lifetime_seconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DropReason {
    Offline,
    Malicious,
    RoutingRejected,
    Expired,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Metrics {
    pub contacts: u64,
    pub relay_commits: u64,
    pub direct_deliveries: u64,
    pub transferred_bytes: u64,
    pub drops: BTreeMap<String, u64>,
    pub storage_high_water_copies: usize,
}

#[derive(Clone, Debug)]
struct SimCopy {
    packet_id: PacketId,
    destination: NodeId,
    priority: Priority,
    message_class: MessageClass,
    created_at_seconds: u64,
    lifetime_seconds: u64,
    hop_count: u8,
    hop_limit: u8,
    tokens: u8,
    size_bytes: u32,
}

#[derive(Clone, Debug, Default)]
struct SimNode {
    copies: BTreeMap<PacketId, SimCopy>,
    delivered: BTreeSet<PacketId>,
    online: bool,
    malicious_drop: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LostAckState {
    Uncertain,
    Reconciled,
}

#[derive(Clone, Copy, Debug)]
struct LostAckGrant {
    packet_id: PacketId,
    receiver: NodeId,
    state: LostAckState,
}

#[derive(Clone, Debug)]
pub struct Simulation {
    nodes: Vec<SimNode>,
    now_seconds: u64,
    metrics: Metrics,
    lost_ack_grants: BTreeMap<u64, LostAckGrant>,
}

impl Simulation {
    #[must_use]
    pub fn new(node_count: usize) -> Self {
        let mut nodes = vec![SimNode::default(); node_count];
        for node in &mut nodes {
            node.online = true;
        }
        Self {
            nodes,
            now_seconds: 0,
            metrics: Metrics::default(),
            lost_ack_grants: BTreeMap::new(),
        }
    }

    pub fn set_time(&mut self, seconds: u64) {
        self.now_seconds = seconds;
    }

    pub fn set_online(&mut self, node: NodeId, online: bool) {
        self.nodes[node].online = online;
    }

    pub fn set_malicious_drop(&mut self, node: NodeId, malicious: bool) {
        self.nodes[node].malicious_drop = malicious;
    }

    pub fn inject(&mut self, injection: Injection) {
        self.nodes[injection.source].copies.insert(
            injection.packet_id,
            SimCopy {
                packet_id: injection.packet_id,
                destination: injection.destination,
                priority: injection.priority,
                message_class: injection.message_class,
                created_at_seconds: self.now_seconds,
                lifetime_seconds: injection.lifetime_seconds,
                hop_count: 0,
                hop_limit: 16,
                tokens: injection.tokens.get(),
                size_bytes: 1_024,
            },
        );
        self.update_high_water();
    }

    pub fn contact(&mut self, first: NodeId, second: NodeId) {
        self.metrics.contacts += 1;
        self.transfer_direction(first, second);
        self.transfer_direction(second, first);
        self.update_high_water();
    }

    pub fn transfer_with_lost_ack(
        &mut self,
        sender: NodeId,
        receiver: NodeId,
        packet_id: PacketId,
        grant_id: u64,
    ) -> bool {
        if self.lost_ack_grants.contains_key(&grant_id)
            || self.nodes[receiver].copies.contains_key(&packet_id)
        {
            return false;
        }
        let Some(mut copy) = self.nodes[sender].copies.get(&packet_id).cloned() else {
            return false;
        };
        let Some((sender_tokens, receiver_tokens)) = split_tokens(copy.tokens) else {
            return false;
        };
        copy.hop_count = copy.hop_count.saturating_add(1);
        let mut receiver_copy = copy.clone();
        receiver_copy.tokens = receiver_tokens;
        self.nodes[sender]
            .copies
            .get_mut(&packet_id)
            .expect("sender copy")
            .tokens = sender_tokens;
        self.nodes[receiver].copies.insert(packet_id, receiver_copy);
        self.lost_ack_grants.insert(
            grant_id,
            LostAckGrant {
                packet_id,
                receiver,
                state: LostAckState::Uncertain,
            },
        );
        true
    }

    pub fn reconcile_same_grant(&mut self, grant_id: u64) -> bool {
        let Some(grant) = self.lost_ack_grants.get_mut(&grant_id) else {
            return false;
        };
        if !self.nodes[grant.receiver]
            .copies
            .contains_key(&grant.packet_id)
            || grant.state == LostAckState::Reconciled
        {
            return false;
        }
        grant.state = LostAckState::Reconciled;
        true
    }

    #[must_use]
    pub fn grant_is_uncertain(&self, grant_id: u64) -> bool {
        self.lost_ack_grants
            .get(&grant_id)
            .is_some_and(|grant| grant.state == LostAckState::Uncertain)
    }

    #[must_use]
    pub fn token_total(&self, packet_id: PacketId) -> u16 {
        self.nodes
            .iter()
            .filter_map(|node| node.copies.get(&packet_id))
            .map(|copy| u16::from(copy.tokens))
            .sum()
    }

    #[must_use]
    pub fn delivered(&self, node: NodeId, packet_id: PacketId) -> bool {
        self.nodes[node].delivered.contains(&packet_id)
    }

    #[must_use]
    pub fn holders(&self, packet_id: PacketId) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(id, node)| node.copies.contains_key(&packet_id).then_some(id))
            .collect()
    }

    #[must_use]
    pub const fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    fn transfer_direction(&mut self, sender: NodeId, receiver: NodeId) {
        if !self.nodes[sender].online || !self.nodes[receiver].online {
            self.record_drop(DropReason::Offline);
            return;
        }
        let packet_ids: Vec<_> = self.nodes[sender].copies.keys().copied().collect();
        for packet_id in packet_ids {
            if self.nodes[receiver].delivered.contains(&packet_id)
                || self.nodes[receiver].copies.contains_key(&packet_id)
            {
                continue;
            }
            let copy = self.nodes[sender]
                .copies
                .get(&packet_id)
                .expect("snapshot packet")
                .clone();
            let elapsed = self.now_seconds.saturating_sub(copy.created_at_seconds);
            let remaining = copy.lifetime_seconds.saturating_sub(elapsed);
            if remaining == 0 {
                self.record_drop(DropReason::Expired);
                continue;
            }
            let summary = BundleSummary {
                packet_id: copy.packet_id,
                destination: slot(copy.destination),
                priority: copy.priority,
                message_class: copy.message_class,
                remaining_lifetime_seconds: remaining,
                hops: HopState::new(copy.hop_count, copy.hop_limit).expect("valid simulation hops"),
                copy_tokens: CopyTokens::new(copy.tokens).expect("valid simulation tokens"),
                total_bundle_bytes: copy.size_bytes,
            };
            let context = PeerContext {
                owns_destination: receiver == copy.destination,
                tombstoned: false,
                already_committed: false,
                session_capacity_available: true,
                ingress_allowed: true,
                unverified_direct_quota_available: true,
                relay_enabled: true,
                relay_quota_available: true,
                source_allowed: true,
            };
            match route_decision(summary, context) {
                RouteDecision::Request(RequestReason::DirectDestination) => {
                    if self.nodes[receiver].malicious_drop {
                        self.record_drop(DropReason::Malicious);
                    } else {
                        self.nodes[receiver].delivered.insert(packet_id);
                        self.metrics.direct_deliveries += 1;
                        self.metrics.transferred_bytes += u64::from(copy.size_bytes);
                    }
                }
                RouteDecision::Request(
                    RequestReason::RelayCopy | RequestReason::ReceiptOrCancel,
                ) => {
                    if self.nodes[receiver].malicious_drop {
                        self.record_drop(DropReason::Malicious);
                        continue;
                    }
                    let Some((sender_tokens, receiver_tokens)) = split_tokens(copy.tokens) else {
                        self.record_drop(DropReason::RoutingRejected);
                        continue;
                    };
                    self.nodes[sender]
                        .copies
                        .get_mut(&packet_id)
                        .expect("sender copy")
                        .tokens = sender_tokens;
                    let mut receiver_copy = copy;
                    receiver_copy.tokens = receiver_tokens;
                    receiver_copy.hop_count = receiver_copy.hop_count.saturating_add(1);
                    self.nodes[receiver].copies.insert(packet_id, receiver_copy);
                    self.metrics.relay_commits += 1;
                    self.metrics.transferred_bytes += u64::from(summary.total_bundle_bytes);
                }
                RouteDecision::Reject(_) => self.record_drop(DropReason::RoutingRejected),
            }
        }
    }

    fn record_drop(&mut self, reason: DropReason) {
        *self.metrics.drops.entry(format!("{reason:?}")).or_default() += 1;
    }

    fn update_high_water(&mut self) {
        let count = self.nodes.iter().map(|node| node.copies.len()).sum();
        self.metrics.storage_high_water_copies = self.metrics.storage_high_water_copies.max(count);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SeededEntropy(u64);

impl SeededEntropy {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        })
    }

    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    #[must_use]
    pub fn next_node(&mut self, node_count: usize) -> NodeId {
        self.next_u64() as usize % node_count
    }
}

fn slot(node: NodeId) -> RoutingSlot {
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&(node as u64).to_be_bytes());
    RoutingSlot::from(bytes)
}

#[must_use]
pub fn core_version() -> String {
    mesh_engine::version()
}

#[cfg(test)]
mod tests {
    use mesh_routing::{
        AgeContext, AgeResult, EvictionCandidate, EvictionKind, EvictionPolicy, QuotaDecision,
        QuotaSnapshot, current_age, eviction_plan, quota_decision,
    };
    use mesh_types::BootId;

    use super::*;

    fn packet(value: u8) -> PacketId {
        PacketId::from([value; 16])
    }

    #[test]
    fn sim_001_linear_delayed_path() {
        let id = packet(1);
        let mut sim = Simulation::new(3);
        sim.inject(Injection {
            source: 0,
            destination: 2,
            packet_id: id,
            tokens: CopyTokens::new(6).unwrap(),
            priority: Priority::P2,
            message_class: MessageClass::Direct,
            lifetime_seconds: 3_600,
        });
        sim.contact(0, 1);
        sim.set_time(20);
        sim.contact(1, 2);
        assert!(sim.delivered(2, id));
        assert_eq!(sim.metrics().direct_deliveries, 1);
    }

    #[test]
    fn lost_ack_conserves_tokens_and_same_grant_never_duplicates() {
        let id = packet(2);
        let mut sim = Simulation::new(4);
        sim.inject(Injection {
            source: 0,
            destination: 3,
            packet_id: id,
            tokens: CopyTokens::new(6).unwrap(),
            priority: Priority::P1,
            message_class: MessageClass::CheckIn,
            lifetime_seconds: 3_600,
        });
        assert!(sim.transfer_with_lost_ack(0, 1, id, 44));
        assert!(sim.grant_is_uncertain(44));
        assert_eq!(sim.token_total(id), 6);
        assert!(!sim.transfer_with_lost_ack(0, 2, id, 44));
        assert!(sim.reconcile_same_grant(44));
        assert!(!sim.reconcile_same_grant(44));
        assert_eq!(sim.holders(id), vec![0, 1]);
        assert_eq!(sim.token_total(id), 6);
    }

    #[test]
    fn sim_002_partition_rejoin() {
        let id = packet(3);
        let mut sim = Simulation::new(50);
        sim.inject(Injection {
            source: 0,
            destination: 49,
            packet_id: id,
            tokens: CopyTokens::new(16).unwrap(),
            priority: Priority::P0,
            message_class: MessageClass::Sos,
            lifetime_seconds: 10_800,
        });
        sim.contact(0, 24);
        sim.set_time(7_200);
        sim.contact(24, 25);
        sim.contact(25, 49);
        assert!(sim.delivered(49, id));
    }

    #[test]
    fn sim_003_one_hundred_nodes_is_seeded_and_deterministic() {
        fn run() -> (Metrics, bool) {
            let id = packet(4);
            let mut sim = Simulation::new(100);
            let mut entropy = SeededEntropy::new(0x5eed);
            sim.inject(Injection {
                source: 0,
                destination: 99,
                packet_id: id,
                tokens: CopyTokens::new(12).unwrap(),
                priority: Priority::P1,
                message_class: MessageClass::CheckIn,
                lifetime_seconds: 86_400,
            });
            for node in 0..100 {
                if node != 0 && node != 99 && entropy.next_u64() % 10 < 3 {
                    sim.set_online(node, false);
                }
                if node != 99 && entropy.next_u64().is_multiple_of(10) {
                    sim.set_malicious_drop(node, true);
                }
            }
            for _ in 0..5_000 {
                let first = entropy.next_node(100);
                let mut second = entropy.next_node(100);
                if first == second {
                    second = (second + 1) % 100;
                }
                sim.contact(first, second);
            }
            for holder in sim.holders(id) {
                sim.contact(holder, 99);
            }
            (sim.metrics().clone(), sim.delivered(99, id))
        }

        let first = run();
        let second = run();
        assert_eq!(first, second);
        assert!(first.1);
    }

    #[test]
    fn sim_004_flood_obeys_quota_and_protected_floor() {
        assert_eq!(
            quota_decision(
                QuotaSnapshot {
                    peer_daily_bytes: 8_388_608,
                    peer_unverified_direct_daily_bytes: 0,
                    source_bytes: 0,
                    relay_total_bytes: 0,
                },
                1,
                false,
            ),
            QuotaDecision::PeerDailyExceeded
        );
        let local = packet(5);
        let relay = packet(6);
        let plan = eviction_plan(
            &[
                EvictionCandidate {
                    packet_id: local,
                    kind: EvictionKind::VerifiedLocal,
                    priority: Priority::P0,
                    created_at_ms: 0,
                    score: 1,
                    size_bytes: 1_024,
                },
                EvictionCandidate {
                    packet_id: relay,
                    kind: EvictionKind::Relay,
                    priority: Priority::P2,
                    created_at_ms: 1,
                    score: 2,
                    size_bytes: 1_024,
                },
            ],
            1_024,
            EvictionPolicy {
                protected_bytes: 8_388_608,
                protected_floor_bytes: 8_388_608,
                disk_full: false,
            },
        );
        assert_eq!(plan, vec![relay]);
    }

    #[test]
    fn sim_005_clock_disorder_is_monotonic_or_fail_closed() {
        let boot = BootId::from([1; 16]);
        assert_eq!(
            current_age(AgeContext {
                stored_age_ms: 1_000,
                age_anchor_elapsed_ms: 100,
                received_boot_id: boot,
                current_boot_id: boot,
                current_elapsed_ms: 200,
                previous_wall_ms: None,
                current_wall_ms: None,
                lifetime_ms: 10_000,
            }),
            AgeResult::Offerable(1_100)
        );
        assert_eq!(
            current_age(AgeContext {
                stored_age_ms: 1_000,
                age_anchor_elapsed_ms: 100,
                received_boot_id: boot,
                current_boot_id: BootId::from([2; 16]),
                current_elapsed_ms: 0,
                previous_wall_ms: Some(10_000),
                current_wall_ms: Some(9_000),
                lifetime_ms: 10_000,
            }),
            AgeResult::AgeUncertain
        );
    }

    #[test]
    fn expired_bundle_is_never_offered() {
        let id = packet(7);
        let mut sim = Simulation::new(2);
        sim.inject(Injection {
            source: 0,
            destination: 1,
            packet_id: id,
            tokens: CopyTokens::new(6).unwrap(),
            priority: Priority::P2,
            message_class: MessageClass::Direct,
            lifetime_seconds: 1,
        });
        sim.set_time(1);
        sim.contact(0, 1);
        assert!(!sim.delivered(1, id));
    }
}
