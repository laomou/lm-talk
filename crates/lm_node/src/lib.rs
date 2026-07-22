//! Native node scaffold for LM Talk.
//!
//! This crate intentionally starts as a deterministic, testable scaffold rather
//! than a real Kademlia implementation. It owns the public-peer runtime model
//! that future UDP/TCP/WebSocket transports can plug into: signed public peer
//! announcements, an in-memory routing table, and an optional mailbox queue.

pub mod kademlia;
pub use kademlia::*;

pub mod mailbox;
pub use mailbox::*;

pub mod prekey_store;
pub use prekey_store::*;

pub mod config;
pub use config::*;

pub mod control;
pub use control::*;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
pub const MAX_ROUTING_PEER_ADDRESSES: usize = 16;
pub const MAX_ROUTING_PEER_ADDRESS_BYTES: usize = 512;

use lm_core::{
    Identity, IdentityBackupPackage, LmError, MailboxMessage, PreKeyBundle, PublicPeerAnnounce,
    PublicPeerCapability, Result, SignedOneTimePreKeyRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct RoutingTable {
    peers: HashMap<String, PublicPeerAnnounce>,
    identity_public_keys: HashMap<String, String>,
}

impl RoutingTable {
    pub fn insert_verified(
        &mut self,
        announce: PublicPeerAnnounce,
        identity_public_key: &[u8; 32],
    ) -> Result<()> {
        announce.verify(identity_public_key)?;
        self.identity_public_keys
            .insert(announce.peer_id.clone(), BASE64.encode(identity_public_key));
        self.peers.insert(announce.peer_id.clone(), announce);
        Ok(())
    }

    fn insert_trusted_announce(&mut self, announce: PublicPeerAnnounce) {
        self.peers.insert(announce.peer_id.clone(), announce);
    }

    fn insert_routing_peer(&mut self, peer: &RoutingPeer) {
        self.peers
            .insert(peer.announce.peer_id.clone(), peer.announce.clone());
        if let Some(identity_public_key) = &peer.identity_public_key {
            self.identity_public_keys
                .insert(peer.announce.peer_id.clone(), identity_public_key.clone());
        }
    }

    pub fn get(&self, peer_id: &str) -> Option<&PublicPeerAnnounce> {
        self.peers.get(peer_id)
    }

    pub fn identity_public_key_for(&self, peer_id: &str) -> Option<&str> {
        self.identity_public_keys.get(peer_id).map(String::as_str)
    }

    pub fn peers(&self) -> impl Iterator<Item = &PublicPeerAnnounce> {
        self.peers.values()
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct NativeNode {
    pub config: NodeConfig,
    pub routing_table: RoutingTable,
    pub kademlia: KademliaRoutingTable,
    pub mailbox: MailboxStore,
    pub mailbox_global_rate_limiter: MailboxGlobalRateLimiter,
    pub mailbox_sender_rate_limiter: MailboxSenderRateLimiter,
    pub prekeys: PreKeyStore,
    pub dht_records: DhtRecordStore,
    pub sync_status: NodeSyncStatus,
    pub maintenance: NodeMaintenanceStats,
}

fn dht_record_reject_reason(record: &DhtRecord, now: u64) -> Option<DhtRecordRejectReason> {
    if record.is_oversized() {
        Some(DhtRecordRejectReason::PayloadTooLarge)
    } else if record.is_expired_at(now) {
        Some(DhtRecordRejectReason::Expired)
    } else if record.ttl_too_long_at(now) {
        Some(DhtRecordRejectReason::TtlTooLong)
    } else if record.validate_for_store_at(now).is_err() {
        Some(DhtRecordRejectReason::InvalidRecord)
    } else {
        None
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSyncStatus {
    pub peers: BTreeMap<String, NodeSyncPeerStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSyncPeerStatus {
    pub url: String,
    pub attempts: u64,
    pub successes: u64,
    pub failures: u64,
    pub last_attempt_at: Option<u64>,
    pub last_success_at: Option<u64>,
    pub last_error_at: Option<u64>,
    pub last_error: Option<String>,
    #[serde(default)]
    pub next_attempt_at: Option<u64>,
    #[serde(default)]
    pub consecutive_failures: u32,
    pub last_imported_peers: usize,
    pub last_imported_mailbox_deliveries: usize,
    pub last_imported_prekey_bundles: usize,
    #[serde(default)]
    pub last_imported_signed_one_time_prekey_records: usize,
}

impl NodeSyncStatus {
    pub fn record_success(&mut self, url: &str, stats: NodeMergeStats) {
        let now = current_unix_timestamp();
        let entry = self.peer_entry(url);
        entry.attempts = entry.attempts.saturating_add(1);
        entry.successes = entry.successes.saturating_add(1);
        entry.last_attempt_at = Some(now);
        entry.last_success_at = Some(now);
        entry.last_error = None;
        entry.consecutive_failures = 0;
        entry.last_imported_peers = stats.peers;
        entry.last_imported_mailbox_deliveries = stats.mailbox_deliveries;
        entry.last_imported_prekey_bundles = stats.prekey_bundles;
        entry.last_imported_signed_one_time_prekey_records = stats.signed_one_time_prekey_records;
    }

    pub fn record_failure(&mut self, url: &str, error: impl Into<String>) {
        let now = current_unix_timestamp();
        let entry = self.peer_entry(url);
        entry.attempts = entry.attempts.saturating_add(1);
        entry.failures = entry.failures.saturating_add(1);
        entry.last_attempt_at = Some(now);
        entry.last_error_at = Some(now);
        entry.last_error = Some(error.into());
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
    }

    pub fn record_next_attempt(&mut self, url: &str, next_attempt_at: u64) {
        self.peer_entry(url).next_attempt_at = Some(next_attempt_at);
    }

    pub fn reset_peer_health(&mut self, url: &str) -> bool {
        let Some(entry) = self.peers.get_mut(url) else {
            return false;
        };
        entry.consecutive_failures = 0;
        entry.last_error = None;
        entry.last_error_at = None;
        entry.next_attempt_at = None;
        true
    }

    fn peer_entry(&mut self, url: &str) -> &mut NodeSyncPeerStatus {
        self.peers
            .entry(url.to_string())
            .or_insert_with(|| NodeSyncPeerStatus {
                url: url.to_string(),
                attempts: 0,
                successes: 0,
                failures: 0,
                last_attempt_at: None,
                last_success_at: None,
                last_error_at: None,
                last_error: None,
                next_attempt_at: None,
                consecutive_failures: 0,
                last_imported_peers: 0,
                last_imported_mailbox_deliveries: 0,
                last_imported_prekey_bundles: 0,
                last_imported_signed_one_time_prekey_records: 0,
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStateSnapshot {
    pub version: u16,
    pub config: NodeConfig,
    pub public_peers: Vec<PublicPeerAnnounce>,
    #[serde(default)]
    pub routing_peers: Vec<RoutingPeer>,
    #[serde(default)]
    pub mailbox_deliveries: Vec<MailboxDelivery>,
    #[serde(default)]
    pub mailbox_ack_receipts: Vec<MailboxAckReceipt>,
    #[serde(default)]
    pub mailbox_messages: Vec<MailboxMessage>,
    #[serde(default)]
    pub prekey_bundles: Vec<PreKeyBundle>,
    #[serde(default)]
    pub signed_one_time_prekey_records: Vec<SignedOneTimePreKeyRecord>,
    #[serde(default)]
    pub consumed_one_time_prekeys: Vec<ConsumedOneTimePreKey>,
    #[serde(default)]
    pub dht_records: Vec<DhtRecord>,
    #[serde(default)]
    pub sync_status: NodeSyncStatus,
    #[serde(default)]
    pub maintenance: NodeMaintenanceStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMergeStats {
    pub peers: usize,
    pub mailbox_deliveries: usize,
    pub prekey_bundles: usize,
    pub signed_one_time_prekey_records: usize,
    pub dht_records: usize,
}

impl NativeNode {
    pub fn new(config: NodeConfig) -> Self {
        let local_id = KademliaNodeId::from_peer_id(&config.peer_id);
        Self {
            config,
            routing_table: RoutingTable::default(),
            kademlia: KademliaRoutingTable::new(local_id, DEFAULT_K_BUCKET_SIZE),
            mailbox: MailboxStore::default(),
            mailbox_global_rate_limiter: MailboxGlobalRateLimiter::default(),
            mailbox_sender_rate_limiter: MailboxSenderRateLimiter::default(),
            prekeys: PreKeyStore::default(),
            dht_records: DhtRecordStore::default(),
            sync_status: NodeSyncStatus::default(),
            maintenance: NodeMaintenanceStats::default(),
        }
    }

    pub fn local_announce(&self, identity: &Identity) -> Result<PublicPeerAnnounce> {
        self.config.create_announce(identity)
    }

    pub fn maintenance_stats(&self) -> &NodeMaintenanceStats {
        &self.maintenance
    }

    pub fn prune_expired_records(&mut self) -> NodeMaintenanceStats {
        let now = current_unix_timestamp();
        let mailbox_removed = self.mailbox.prune_expired(now) as u64;
        let prekey_removed = self.prekeys.prune_expired(now) as u64;
        self.dht_records.prune_expired(now);
        self.mailbox_global_rate_limiter
            .prune(now, self.config.mailbox_global_rate_limit());
        self.mailbox_sender_rate_limiter
            .prune(now, self.config.mailbox_sender_rate_limit());
        self.maintenance.prune_runs = self.maintenance.prune_runs.saturating_add(1);
        self.maintenance.mailbox_expired_deliveries = self
            .maintenance
            .mailbox_expired_deliveries
            .saturating_add(mailbox_removed);
        self.maintenance.prekey_expired_bundles = self
            .maintenance
            .prekey_expired_bundles
            .saturating_add(prekey_removed);
        self.maintenance.last_pruned_at = Some(now);
        NodeMaintenanceStats {
            prune_runs: 1,
            mailbox_expired_deliveries: mailbox_removed,
            prekey_expired_bundles: prekey_removed,
            mailbox_push_rejects: MailboxPushRejectStats::default(),
            dht_record_rejects: DhtRecordRejectStats::default(),
            mailbox_ack_rejects: MailboxAckRejectStats::default(),
            routing_peer_rejects: RoutingPeerRejectStats::default(),
            last_pruned_at: Some(now),
        }
    }

    fn record_mailbox_push_reject(&mut self, reason: MailboxPushRejectReason) {
        let stats = &mut self.maintenance.mailbox_push_rejects;
        match reason {
            MailboxPushRejectReason::InvalidJson => {
                stats.invalid_json = stats.invalid_json.saturating_add(1)
            }
            MailboxPushRejectReason::InvalidMessageFormat => {
                stats.invalid_message_format = stats.invalid_message_format.saturating_add(1)
            }
            MailboxPushRejectReason::InvalidIdentityPublicKey => {
                stats.invalid_identity_public_key =
                    stats.invalid_identity_public_key.saturating_add(1)
            }
            MailboxPushRejectReason::InvalidSignature => {
                stats.invalid_signature = stats.invalid_signature.saturating_add(1)
            }
            MailboxPushRejectReason::ExpiredObject => {
                stats.expired_object = stats.expired_object.saturating_add(1)
            }
            MailboxPushRejectReason::DuplicateMessage => {
                stats.duplicate_message = stats.duplicate_message.saturating_add(1)
            }
            MailboxPushRejectReason::PayloadTooLarge => {
                stats.payload_too_large = stats.payload_too_large.saturating_add(1)
            }
            MailboxPushRejectReason::GlobalRateLimited => {
                stats.global_rate_limited = stats.global_rate_limited.saturating_add(1)
            }
            MailboxPushRejectReason::SenderRateLimited => {
                stats.sender_rate_limited = stats.sender_rate_limited.saturating_add(1)
            }
            MailboxPushRejectReason::Other => stats.other = stats.other.saturating_add(1),
        }
    }

    fn record_dht_record_reject(&mut self, reason: DhtRecordRejectReason) {
        let stats = &mut self.maintenance.dht_record_rejects;
        match reason {
            DhtRecordRejectReason::InvalidJson => {
                stats.invalid_json = stats.invalid_json.saturating_add(1)
            }
            DhtRecordRejectReason::Expired => stats.expired = stats.expired.saturating_add(1),
            DhtRecordRejectReason::TtlTooLong => {
                stats.ttl_too_long = stats.ttl_too_long.saturating_add(1)
            }
            DhtRecordRejectReason::PayloadTooLarge => {
                stats.payload_too_large = stats.payload_too_large.saturating_add(1)
            }
            DhtRecordRejectReason::InvalidRecord => {
                stats.invalid_record = stats.invalid_record.saturating_add(1)
            }
        }
    }

    pub fn accept_dht_record_from_peer(&mut self, record: DhtRecord) -> bool {
        let now = current_unix_timestamp();
        if let Some(reason) = dht_record_reject_reason(&record, now) {
            self.record_dht_record_reject(reason);
            return false;
        }
        self.dht_records.store(record)
    }

    pub fn merge_dht_records_from_peer(&mut self, records: Vec<DhtRecord>) -> usize {
        let mut inserted = 0usize;
        for record in records {
            if self.accept_dht_record_from_peer(record) {
                inserted = inserted.saturating_add(1);
            }
        }
        inserted
    }

    fn record_mailbox_ack_reject(&mut self, reason: MailboxAckRejectReason) {
        let stats = &mut self.maintenance.mailbox_ack_rejects;
        match reason {
            MailboxAckRejectReason::InvalidJson => {
                stats.invalid_json = stats.invalid_json.saturating_add(1)
            }
            MailboxAckRejectReason::InvalidUserId => {
                stats.invalid_user_id = stats.invalid_user_id.saturating_add(1)
            }
            MailboxAckRejectReason::TooManyIds => {
                stats.too_many_ids = stats.too_many_ids.saturating_add(1)
            }
            MailboxAckRejectReason::EmptyId => stats.empty_id = stats.empty_id.saturating_add(1),
            MailboxAckRejectReason::IdTooLarge => {
                stats.id_too_large = stats.id_too_large.saturating_add(1)
            }
        }
    }

    fn record_routing_peer_reject(&mut self, reason: RoutingPeerRejectReason) {
        let stats = &mut self.maintenance.routing_peer_rejects;
        match reason {
            RoutingPeerRejectReason::Expired => stats.expired = stats.expired.saturating_add(1),
            RoutingPeerRejectReason::MismatchedNodeId => {
                stats.mismatched_node_id = stats.mismatched_node_id.saturating_add(1)
            }
            RoutingPeerRejectReason::LocalNode => {
                stats.local_node = stats.local_node.saturating_add(1)
            }
            RoutingPeerRejectReason::MissingIdentityPublicKey => {
                stats.missing_identity_public_key =
                    stats.missing_identity_public_key.saturating_add(1)
            }
            RoutingPeerRejectReason::InvalidIdentityPublicKey => {
                stats.invalid_identity_public_key =
                    stats.invalid_identity_public_key.saturating_add(1)
            }
            RoutingPeerRejectReason::InvalidSignature => {
                stats.invalid_signature = stats.invalid_signature.saturating_add(1)
            }
            RoutingPeerRejectReason::TooManyAddresses => {
                stats.too_many_addresses = stats.too_many_addresses.saturating_add(1)
            }
            RoutingPeerRejectReason::AddressTooLarge => {
                stats.address_too_large = stats.address_too_large.saturating_add(1)
            }
        }
    }

    fn routing_peer_basic_reject_reason(
        &self,
        peer: &RoutingPeer,
        now: u64,
    ) -> Option<RoutingPeerRejectReason> {
        if peer.announce.expires_at <= now {
            return Some(RoutingPeerRejectReason::Expired);
        }
        if peer.announce.addresses.len() > MAX_ROUTING_PEER_ADDRESSES {
            return Some(RoutingPeerRejectReason::TooManyAddresses);
        }
        if peer
            .announce
            .addresses
            .iter()
            .any(|address| address.len() > MAX_ROUTING_PEER_ADDRESS_BYTES)
        {
            return Some(RoutingPeerRejectReason::AddressTooLarge);
        }
        let expected_node_id = KademliaNodeId::from_peer_id(&peer.announce.peer_id);
        if peer.node_id != expected_node_id {
            return Some(RoutingPeerRejectReason::MismatchedNodeId);
        }
        if expected_node_id == self.kademlia.local_id() {
            return Some(RoutingPeerRejectReason::LocalNode);
        }
        None
    }

    pub fn plan_dht_replication(&mut self, replication_factor: usize) -> DhtReplicationPlan {
        let now = current_unix_timestamp();
        let replication_factor = replication_factor.clamp(1, 64);
        let records = self
            .dht_records
            .due_for_republish(now)
            .into_iter()
            .map(|record| DhtRecordReplicationPlan {
                target_nodes: self
                    .kademlia
                    .closest(record.key.to_node_id(), replication_factor),
                record,
            })
            .collect();
        DhtReplicationPlan {
            generated_at: now,
            replication_factor,
            records,
        }
    }

    pub fn plan_dht_routing_refresh(&self) -> DhtRoutingRefreshPlan {
        DhtRoutingRefreshPlan {
            generated_at: current_unix_timestamp(),
            targets: self.kademlia.refresh_targets(),
        }
    }

    pub fn merge_trusted_routing_peers(&mut self, peers: Vec<RoutingPeer>) -> usize {
        let now = current_unix_timestamp();
        let mut inserted = 0usize;
        for peer in peers {
            if let Some(reason) = self.routing_peer_basic_reject_reason(&peer, now) {
                self.record_routing_peer_reject(reason);
                continue;
            }
            if let Some(identity_public_key) = &peer.identity_public_key {
                let Ok(public_key) = decode_identity_public_key_base64(identity_public_key) else {
                    self.record_routing_peer_reject(
                        RoutingPeerRejectReason::InvalidIdentityPublicKey,
                    );
                    continue;
                };
                if peer.announce.verify(&public_key).is_err() {
                    self.record_routing_peer_reject(RoutingPeerRejectReason::InvalidSignature);
                    continue;
                }
            }
            self.routing_table
                .insert_trusted_announce(peer.announce.clone());
            let before = self.kademlia.len();
            self.kademlia.insert_local_snapshot(peer.announce);
            if self.kademlia.len() > before {
                inserted = inserted.saturating_add(1);
            }
        }
        inserted
    }

    pub fn merge_verified_routing_peers(&mut self, peers: Vec<RoutingPeer>) -> usize {
        let now = current_unix_timestamp();
        let mut inserted = 0usize;
        for peer in peers {
            if let Some(reason) = self.routing_peer_basic_reject_reason(&peer, now) {
                self.record_routing_peer_reject(reason);
                continue;
            }
            let Some(identity_public_key) = &peer.identity_public_key else {
                self.record_routing_peer_reject(RoutingPeerRejectReason::MissingIdentityPublicKey);
                continue;
            };
            let Ok(public_key) = decode_identity_public_key_base64(identity_public_key) else {
                self.record_routing_peer_reject(RoutingPeerRejectReason::InvalidIdentityPublicKey);
                continue;
            };
            if self
                .routing_table
                .insert_verified(peer.announce.clone(), &public_key)
                .is_err()
            {
                self.record_routing_peer_reject(RoutingPeerRejectReason::InvalidSignature);
                continue;
            }
            let before = self.kademlia.len();
            if self
                .kademlia
                .insert_verified(peer.announce, &public_key)
                .is_ok()
                && self.kademlia.len() > before
            {
                inserted = inserted.saturating_add(1);
            }
        }
        inserted
    }

    pub fn handle_dht_rpc(&mut self, request: DhtRpcRequest) -> DhtRpcResponse {
        self.prune_expired_records();
        match request {
            DhtRpcRequest::FindNode {
                request_id,
                target,
                limit,
            } => DhtRpcResponse::Nodes {
                request_id,
                nodes: self.kademlia.closest(target, limit.clamp(1, 64)),
            },
            DhtRpcRequest::FindValue {
                request_id,
                key,
                limit,
            } => {
                let record = self.dht_records.find_value(&key);
                let closer_records = if record.is_none() {
                    self.dht_records.closest_records(key, limit.clamp(1, 64))
                } else {
                    Vec::new()
                };
                let closer_nodes = if record.is_none() {
                    self.kademlia.closest(key.to_node_id(), limit.clamp(1, 64))
                } else {
                    Vec::new()
                };
                DhtRpcResponse::Value {
                    request_id,
                    record,
                    closer_records,
                    closer_nodes,
                }
            }
            DhtRpcRequest::StoreRecord { request_id, record } => {
                let now = current_unix_timestamp();
                let reject_reason = dht_record_reject_reason(&record, now);
                if let Some(reason) = reject_reason {
                    self.record_dht_record_reject(reason);
                }
                let inserted = if reject_reason.is_some() {
                    false
                } else {
                    self.dht_records.store(record)
                };
                DhtRpcResponse::StoreResult {
                    request_id,
                    stored: reject_reason.is_none(),
                    inserted,
                }
            }
        }
    }

    pub fn to_state_snapshot(&self) -> NodeStateSnapshot {
        NodeStateSnapshot {
            version: 1,
            config: self.config.clone(),
            public_peers: self.routing_table.peers().cloned().collect(),
            routing_peers: self.kademlia.all_peers(),
            mailbox_deliveries: self.mailbox.all_deliveries(),
            mailbox_ack_receipts: self.mailbox.all_ack_receipts(),
            mailbox_messages: Vec::new(),
            prekey_bundles: self.prekeys.all_bundles(),
            signed_one_time_prekey_records: self.prekeys.all_signed_one_time_prekey_records(),
            consumed_one_time_prekeys: self
                .prekeys
                .consumed_one_time_prekeys
                .iter()
                .flat_map(|(user_id, ids)| {
                    ids.iter().map(|key_id| ConsumedOneTimePreKey {
                        user_id: user_id.clone(),
                        key_id: *key_id,
                    })
                })
                .collect(),
            dht_records: self.dht_records.all_records(),
            sync_status: self.sync_status.clone(),
            maintenance: self.maintenance.clone(),
        }
    }

    /// Restore a local trusted snapshot from disk.
    ///
    /// New snapshots include routing peers with identity public keys so DHT
    /// routing records can remain verifiable across restarts. Older snapshots
    /// are still accepted through the legacy `public_peers` field.
    pub fn from_state_snapshot(snapshot: NodeStateSnapshot) -> Self {
        let mut node = Self::new(snapshot.config);
        if snapshot.routing_peers.is_empty() {
            for announce in snapshot.public_peers {
                node.routing_table
                    .peers
                    .insert(announce.peer_id.clone(), announce.clone());
                node.kademlia.insert_local_snapshot(announce);
            }
        } else {
            for peer in snapshot.routing_peers {
                node.routing_table.insert_routing_peer(&peer);
                if let Some(identity_public_key) = &peer.identity_public_key
                    && let Ok(public_key) = decode_identity_public_key_base64(identity_public_key)
                    && node
                        .kademlia
                        .insert_verified(peer.announce.clone(), &public_key)
                        .is_ok()
                {
                    continue;
                }
                node.kademlia.insert_local_snapshot(peer.announce);
            }
        }
        if snapshot.mailbox_deliveries.is_empty() {
            node.mailbox.restore_messages(snapshot.mailbox_messages);
        } else {
            node.mailbox.restore_deliveries(snapshot.mailbox_deliveries);
        }
        node.mailbox
            .restore_ack_receipts(snapshot.mailbox_ack_receipts);
        node.prekeys.restore_bundles(snapshot.prekey_bundles);
        node.prekeys
            .restore_signed_one_time_prekey_records(snapshot.signed_one_time_prekey_records);
        node.prekeys
            .restore_consumed(snapshot.consumed_one_time_prekeys);
        node.dht_records.restore_records(snapshot.dht_records);
        node.sync_status = snapshot.sync_status;
        node.maintenance = snapshot.maintenance;
        node
    }

    pub fn merge_snapshot(&mut self, snapshot: NodeStateSnapshot) -> NodeMergeStats {
        let mut peers = 0;
        if snapshot.routing_peers.is_empty() {
            for announce in snapshot.public_peers {
                if self.routing_table.get(&announce.peer_id).is_none() {
                    peers += 1;
                }
                self.routing_table
                    .peers
                    .insert(announce.peer_id.clone(), announce.clone());
                self.kademlia.insert_local_snapshot(announce);
            }
        } else {
            for peer in snapshot.routing_peers {
                if self.routing_table.get(&peer.announce.peer_id).is_none() {
                    peers += 1;
                }
                self.routing_table.insert_routing_peer(&peer);
                if let Some(identity_public_key) = &peer.identity_public_key
                    && let Ok(public_key) = decode_identity_public_key_base64(identity_public_key)
                    && self
                        .kademlia
                        .insert_verified(peer.announce.clone(), &public_key)
                        .is_ok()
                {
                    continue;
                }
                self.kademlia.insert_local_snapshot(peer.announce);
            }
        }
        let mailbox_deliveries = if snapshot.mailbox_deliveries.is_empty() {
            let deliveries = snapshot
                .mailbox_messages
                .into_iter()
                .map(|message| MailboxDelivery {
                    delivery_id: Uuid::new_v4().to_string(),
                    message,
                    created_at: current_unix_timestamp(),
                    delivered_at: None,
                })
                .collect();
            self.mailbox.merge_deliveries(deliveries)
        } else {
            self.mailbox.merge_deliveries(snapshot.mailbox_deliveries)
        };
        self.mailbox
            .merge_ack_receipts(snapshot.mailbox_ack_receipts);
        let prekey_bundles = self.prekeys.merge_bundles(snapshot.prekey_bundles);
        let signed_one_time_prekey_records = self
            .prekeys
            .merge_signed_one_time_prekey_records(snapshot.signed_one_time_prekey_records);
        self.prekeys
            .merge_consumed(snapshot.consumed_one_time_prekeys);
        let dht_records = self.merge_dht_records_from_peer(snapshot.dht_records);
        for (url, status) in snapshot.sync_status.peers {
            self.sync_status.peers.entry(url).or_insert(status);
        }
        NodeMergeStats {
            peers,
            mailbox_deliveries,
            prekey_bundles,
            signed_one_time_prekey_records,
            dht_records,
        }
    }
}

pub fn parse_capability(value: &str) -> Result<PublicPeerCapability> {
    match value.trim().to_ascii_lowercase().as_str() {
        "bootstrap" => Ok(PublicPeerCapability::Bootstrap),
        "dht" => Ok(PublicPeerCapability::Dht),
        "signaling" => Ok(PublicPeerCapability::Signaling),
        "relay" => Ok(PublicPeerCapability::Relay),
        "mailbox" => Ok(PublicPeerCapability::Mailbox),
        _ => Err(LmError::InvalidFormat),
    }
}

pub fn parse_capabilities_csv(value: &str) -> Result<Vec<PublicPeerCapability>> {
    value
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .map(parse_capability)
        .collect()
}

pub fn restore_identity_from_backup_text(backup_text: &str, passphrase: &str) -> Result<Identity> {
    let backup = IdentityBackupPackage::from_export_text(backup_text)?;
    Identity::restore_from_backup(&backup, passphrase)
}

pub fn decode_identity_public_key_base64(value: &str) -> Result<[u8; 32]> {
    let bytes = BASE64
        .decode(value.as_bytes())
        .map_err(|_| LmError::InvalidSignature)?;
    bytes.try_into().map_err(|_| LmError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lm_core::MailboxMessageKind;
    use lm_core::UserId;

    fn fixture(name: &str) -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test-vectors")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn node_config_creates_verified_announce() {
        let (identity, _) = Identity::create_with_passphrase("node pass").unwrap();
        let config = NodeConfig {
            peer_id: "peer-a".into(),
            addresses: vec!["/ip4/127.0.0.1/tcp/4001".into()],
            capabilities: vec![
                PublicPeerCapability::Bootstrap,
                PublicPeerCapability::Mailbox,
            ],
            ..Default::default()
        };
        let announce = config.create_announce(&identity).unwrap();
        announce.verify(&identity.identity_public_key()).unwrap();
        assert_eq!(announce.peer_id, "peer-a");
    }

    #[test]
    fn public_peer_dht_vector_validates() {
        let v = fixture("public_peer_v1.json");
        let key = DhtRecordKey::for_public_peer(v["peer_id"].as_str().unwrap());
        assert_eq!(key.to_hex(), v["dht_key_hex"]);
        let record: DhtRecord =
            serde_json::from_str(v["dht_record_json"].as_str().unwrap()).unwrap();
        assert_eq!(record.kind, DhtRecordKind::PublicPeer);
        assert_eq!(record.key, key);
        assert_eq!(record.value, v["public_peer_text"]);
        record.validate_for_store_at(1_700_000_001).unwrap();
        let announce =
            PublicPeerAnnounce::from_export_text(v["public_peer_text"].as_str().unwrap()).unwrap();
        let pk =
            decode_identity_public_key_base64(v["identity_public_key_base64"].as_str().unwrap())
                .unwrap();
        if announce.expires_at > lm_core::unix_now() {
            announce.verify(&pk).unwrap();
        } else {
            assert!(announce.verify(&pk).is_err());
        }
        assert_eq!(announce.peer_id, v["peer_id"]);
        let mut tampered = record.clone();
        tampered.key = DhtRecordKey::for_public_peer("wrong-peer");
        assert!(tampered.validate_for_store_at(1_700_000_001).is_err());
    }

    #[test]
    fn contact_card_dht_vector_validates() {
        let v = fixture("contact_card_dht_v1.json");
        let user_id = UserId::from_raw(v["user_id"].as_str().unwrap().to_string()).unwrap();
        assert_eq!(
            DhtRecordKey::for_contact_card(&user_id).to_hex(),
            v["dht_key_hex"]
        );
        let record: DhtRecord =
            serde_json::from_str(v["dht_record_json"].as_str().unwrap()).unwrap();
        assert_eq!(record.kind, DhtRecordKind::ContactCard);
        assert_eq!(record.key, DhtRecordKey::for_contact_card(&user_id));
        assert_eq!(record.value, v["contact_card_text"]);
        record.validate_for_store_at(1_700_000_001).unwrap();
        let mut tampered = record.clone();
        tampered.key = DhtRecordKey::for_public_peer("wrong-key");
        assert!(tampered.validate_for_store_at(1_700_000_001).is_err());
    }

    #[test]
    fn routing_table_stores_verified_peers() {
        let (identity, _) = Identity::create_with_passphrase("node pass").unwrap();
        let announce = NodeConfig::default().create_announce(&identity).unwrap();
        let mut table = RoutingTable::default();
        table
            .insert_verified(announce.clone(), &identity.identity_public_key())
            .unwrap();
        assert_eq!(table.len(), 1);
        assert_eq!(
            table.get(&announce.peer_id).unwrap().user_id,
            *identity.user_id()
        );
    }

    #[test]
    fn mailbox_store_push_and_take() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let message = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext".into(),
            3600,
        )
        .unwrap();
        let mut mailbox = MailboxStore::default();
        mailbox
            .push_verified(message, &alice.identity_public_key())
            .unwrap();
        assert_eq!(mailbox.pending_for(bob.user_id()), 1);
        let deliveries = mailbox.take_for(bob.user_id());
        assert_eq!(deliveries.len(), 1);
        assert_eq!(mailbox.pending_for(bob.user_id()), 1);
        assert_eq!(
            mailbox.ack(bob.user_id(), &[deliveries[0].delivery_id.clone()]),
            1
        );
        assert_eq!(mailbox.pending_for(bob.user_id()), 0);
    }

    #[test]
    fn mailbox_store_rejects_duplicate_message_ids() {
        let (alice, _) = Identity::create_with_passphrase("alice dup").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob dup").unwrap();
        let message = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext".into(),
            3600,
        )
        .unwrap();
        let mut mailbox = MailboxStore::default();
        mailbox
            .push_verified(message.clone(), &alice.identity_public_key())
            .unwrap();
        assert_eq!(
            mailbox
                .push_verified(message, &alice.identity_public_key())
                .unwrap_err(),
            LmError::DuplicateMessage
        );
    }

    #[test]
    fn mailbox_take_for_limited_returns_page_without_deleting() {
        let (alice, _) = Identity::create_with_passphrase("alice take page").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob take page").unwrap();
        let mut mailbox = MailboxStore::default();
        for i in 0..3 {
            let message = MailboxMessage::new(
                &alice,
                bob.user_id().clone(),
                MailboxMessageKind::DirectEnvelope,
                format!("ciphertext-{i}"),
                3600,
            )
            .unwrap();
            mailbox
                .push_verified(message, &alice.identity_public_key())
                .unwrap();
        }
        let page = mailbox.take_for_limited(bob.user_id(), 2);
        assert_eq!(page.len(), 2);
        assert_eq!(mailbox.pending_for(bob.user_id()), 3);
        assert!(page.iter().all(|delivery| delivery.delivered_at.is_some()));
    }

    #[test]
    fn mailbox_store_prunes_expired_deliveries() {
        let (alice, _) = Identity::create_with_passphrase("alice prune").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob prune").unwrap();
        let message = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext".into(),
            3600,
        )
        .unwrap();
        let mut mailbox = MailboxStore::default();
        mailbox
            .push_verified(message, &alice.identity_public_key())
            .unwrap();
        assert_eq!(mailbox.pending_for(bob.user_id()), 1);
        assert_eq!(mailbox.prune_expired(u64::MAX), 1);
        assert_eq!(mailbox.pending_for(bob.user_id()), 0);
    }

    #[test]
    fn native_node_tracks_expired_record_prune_stats() {
        let (alice, _) = Identity::create_with_passphrase("alice maintenance").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob maintenance").unwrap();
        let mut message = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "expired".into(),
            3600,
        )
        .unwrap();
        message.expires_at = 1;
        let mut node = NativeNode::new(NodeConfig::default());
        node.mailbox
            .deliveries
            .entry(bob.user_id().clone())
            .or_default()
            .push(MailboxDelivery {
                delivery_id: Uuid::new_v4().to_string(),
                message,
                created_at: 1,
                delivered_at: None,
            });
        node.mailbox.rebuild_message_ids();
        assert_eq!(node.mailbox.total_pending(), 1);

        let delta = node.prune_expired_records();

        assert_eq!(delta.prune_runs, 1);
        assert_eq!(delta.mailbox_expired_deliveries, 1);
        assert_eq!(delta.prekey_expired_bundles, 0);
        assert_eq!(node.mailbox.total_pending(), 0);
        assert_eq!(node.maintenance.prune_runs, 1);
        assert_eq!(node.maintenance.mailbox_expired_deliveries, 1);
        assert!(node.maintenance.last_pruned_at.is_some());
        let snapshot = node.to_state_snapshot();
        assert_eq!(snapshot.maintenance.mailbox_expired_deliveries, 1);
        let restored = NativeNode::from_state_snapshot(snapshot);
        assert_eq!(restored.maintenance.mailbox_expired_deliveries, 1);
    }

    #[test]
    fn mailbox_store_enforces_limits() {
        let (alice, _) = Identity::create_with_passphrase("alice limits").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob limits").unwrap();
        let message = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext".into(),
            3600,
        )
        .unwrap();
        let mut mailbox = MailboxStore::default();
        assert_eq!(
            mailbox
                .push_verified_with_limits(
                    message.clone(),
                    &alice.identity_public_key(),
                    Some(1),
                    None,
                    None,
                    None,
                )
                .unwrap_err(),
            LmError::PayloadTooLarge
        );
        assert_eq!(
            mailbox
                .push_verified_with_limits(
                    message.clone(),
                    &alice.identity_public_key(),
                    None,
                    None,
                    None,
                    Some(1),
                )
                .unwrap_err(),
            LmError::PayloadTooLarge
        );
        assert_eq!(
            mailbox
                .push_verified_with_limits(
                    message.clone(),
                    &alice.identity_public_key(),
                    None,
                    Some(1),
                    None,
                    None,
                )
                .unwrap_err(),
            LmError::PayloadTooLarge
        );
        mailbox
            .push_verified_with_limits(
                message.clone(),
                &alice.identity_public_key(),
                None,
                None,
                Some(1),
                None,
            )
            .unwrap();
        let second = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext2".into(),
            3600,
        )
        .unwrap();
        assert_eq!(
            mailbox
                .push_verified_with_limits(
                    second,
                    &alice.identity_public_key(),
                    None,
                    None,
                    Some(1),
                    None,
                )
                .unwrap_err(),
            LmError::PayloadTooLarge
        );
    }

    #[test]
    fn mailbox_sender_rate_limiter_enforces_window_when_enabled() {
        let (alice, _) = Identity::create_with_passphrase("alice sender limit").unwrap();
        let mut limiter = MailboxSenderRateLimiter::default();
        let config = Some(MailboxRateLimitConfig {
            window_seconds: 10,
            max_messages: 2,
        });
        assert!(limiter.check(alice.user_id(), 100, config));
        assert!(limiter.check(alice.user_id(), 100, config));
        assert!(!limiter.check(alice.user_id(), 100, config));
        assert!(limiter.check(alice.user_id(), 110, config));
        limiter.prune(131, config);
        assert!(limiter.is_empty());
        assert!(limiter.check(alice.user_id(), 200, None));
    }

    #[test]
    fn mailbox_global_rate_limiter_enforces_window_when_enabled() {
        let mut limiter = MailboxGlobalRateLimiter::default();
        let config = Some(MailboxRateLimitConfig {
            window_seconds: 10,
            max_messages: 2,
        });
        assert!(limiter.check(100, config));
        assert!(limiter.check(100, config));
        assert!(!limiter.check(100, config));
        assert!(limiter.check(110, config));
        limiter.prune(131, config);
        assert_eq!(limiter.window_started_at, None);
        assert_eq!(limiter.count, 0);
        assert!(limiter.check(200, None));
    }

    #[test]
    fn dht_record_keys_are_namespaced_and_deterministic() {
        let (identity, _) = Identity::create_with_passphrase("dht key user").unwrap();
        let peer_key_a = DhtRecordKey::for_public_peer("peer-a");
        let peer_key_a_again = DhtRecordKey::for_public_peer("peer-a");
        let prekey_key = DhtRecordKey::for_prekey(identity.user_id());
        let mailbox_key = DhtRecordKey::for_mailbox_hint(identity.user_id());

        assert_eq!(peer_key_a, peer_key_a_again);
        assert_ne!(peer_key_a, prekey_key);
        assert_ne!(prekey_key, mailbox_key);
        assert_eq!(peer_key_a.to_hex().len(), KADEMLIA_ID_BYTES * 2);
        assert_eq!(peer_key_a.to_node_id().as_bytes(), peer_key_a.as_bytes());
    }

    #[test]
    fn dht_record_store_finds_closest_republishes_and_prunes() {
        let now = current_unix_timestamp();
        let key_a = DhtRecordKey::for_public_peer("peer-a");
        let key_b = DhtRecordKey::for_public_peer("peer-b");
        let key_c = DhtRecordKey::for_public_peer("peer-c");
        let record_a = DhtRecord {
            key: key_a,
            kind: DhtRecordKind::PublicPeer,
            value: "peer-a-ann".into(),
            created_at: now,
            expires_at: now + 100,
            republish_at: now + 10,
        };
        let record_b = DhtRecord {
            key: key_b,
            kind: DhtRecordKind::PublicPeer,
            value: "peer-b-ann".into(),
            created_at: now,
            expires_at: now + 100,
            republish_at: now + 50,
        };
        let expired = DhtRecord {
            key: key_c,
            kind: DhtRecordKind::PublicPeer,
            value: "expired".into(),
            created_at: now.saturating_sub(20),
            expires_at: now.saturating_sub(1),
            republish_at: now.saturating_sub(10),
        };
        let mut store = DhtRecordStore::default();

        assert!(store.store(record_a.clone()));
        assert!(store.store(record_b.clone()));
        assert!(!store.store(record_a.clone()));
        assert!(!store.store(expired));
        assert_eq!(store.len(), 2);
        assert_eq!(store.find_value(&key_a).unwrap().value, "peer-a-ann");

        let due = store.due_for_republish(now + 10);
        assert_eq!(due, vec![record_a.clone()]);
        let closest = store.closest_records(key_a, 1);
        assert_eq!(closest, vec![record_a.clone()]);
        assert_eq!(store.prune_expired(now + 100), 2);
        assert!(store.is_empty());
    }

    #[test]
    fn dht_record_store_restore_skips_invalid_records() {
        let (identity, _) = Identity::create_with_passphrase("restore dht records").unwrap();
        let valid = DhtRecord::mailbox_hint(identity.user_id(), "http://node/mailbox".into(), 3600);
        let invalid = DhtRecord {
            key: DhtRecordKey::for_public_peer("invalid-restore"),
            kind: DhtRecordKind::PublicPeer,
            value: "not-a-public-peer-announce".into(),
            created_at: current_unix_timestamp(),
            expires_at: current_unix_timestamp() + 3600,
            republish_at: current_unix_timestamp() + 1800,
        };
        let mut store = DhtRecordStore::default();
        store.restore_records(vec![valid.clone(), invalid]);
        assert_eq!(store.len(), 1);
        assert!(store.find_value(&valid.key).is_some());
        assert!(
            store
                .find_value(&DhtRecordKey::for_public_peer("invalid-restore"))
                .is_none()
        );
    }

    #[test]
    fn dht_record_store_rejects_ttl_above_max() {
        let now = current_unix_timestamp();
        let mut store = DhtRecordStore::default();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("ttl-too-long"),
            kind: DhtRecordKind::PublicPeer,
            value: "record".into(),
            created_at: now,
            expires_at: now + DEFAULT_MAX_DHT_RECORD_TTL_SECONDS + 1,
            republish_at: now + 50,
        };
        assert!(record.ttl_too_long_at(now));
        assert!(!store.store(record));
        assert!(store.is_empty());
    }

    #[test]
    fn dht_record_store_rejects_oversized_values() {
        let now = current_unix_timestamp();
        let mut store = DhtRecordStore::default();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("oversized-record"),
            kind: DhtRecordKind::PublicPeer,
            value: "A".repeat(DEFAULT_MAX_DHT_RECORD_VALUE_BYTES + 1),
            created_at: now,
            expires_at: now + 100,
            republish_at: now + 50,
        };
        assert!(record.is_oversized());
        assert!(!store.store(record));
        assert!(store.is_empty());
    }

    #[test]
    fn dht_record_store_evicts_earliest_expiring_records_when_full() {
        let now = current_unix_timestamp();
        let keep_key = DhtRecordKey::for_public_peer("capacity-keep");
        let drop_key = DhtRecordKey::for_public_peer("capacity-drop");
        let mut store = DhtRecordStore::default();
        store.enforce_capacity(0);

        assert!(store.store(DhtRecord {
            key: keep_key,
            kind: DhtRecordKind::PublicPeer,
            value: "keep".into(),
            created_at: now,
            expires_at: now + 200,
            republish_at: now + 100,
        }));
        assert!(store.store(DhtRecord {
            key: drop_key,
            kind: DhtRecordKind::PublicPeer,
            value: "drop".into(),
            created_at: now,
            expires_at: now + 10,
            republish_at: now + 5,
        }));

        store.enforce_capacity(1);

        assert_eq!(store.len(), 1);
        assert!(store.find_value(&keep_key).is_some());
        assert!(store.find_value(&drop_key).is_none());
    }

    #[test]
    fn dht_replication_and_routing_refresh_plans_are_deterministic_scaffolds() {
        let (alice, _) = Identity::create_with_passphrase("dht repl alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("dht repl bob").unwrap();
        let announce_a = NodeConfig {
            peer_id: "repl-peer-a".into(),
            ..Default::default()
        }
        .create_announce(&alice)
        .unwrap();
        let announce_b = NodeConfig {
            peer_id: "repl-peer-b".into(),
            ..Default::default()
        }
        .create_announce(&bob)
        .unwrap();
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "repl-local".into(),
            ..Default::default()
        });
        node.kademlia
            .insert_verified(announce_a, &alice.identity_public_key())
            .unwrap();
        node.kademlia
            .insert_verified(announce_b, &bob.identity_public_key())
            .unwrap();
        let now = current_unix_timestamp();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("repl-record"),
            kind: DhtRecordKind::PublicPeer,
            value: "replicate-me".into(),
            created_at: now,
            expires_at: now + 120,
            republish_at: now,
        };
        assert!(node.dht_records.store(record.clone()));

        let plan = node.plan_dht_replication(1);
        assert_eq!(plan.replication_factor, 1);
        assert_eq!(plan.records.len(), 1);
        assert_eq!(plan.records[0].record, record);
        assert_eq!(plan.records[0].target_nodes.len(), 1);

        let refresh = node.plan_dht_routing_refresh();
        assert_eq!(refresh.targets.len(), KADEMLIA_ID_BYTES * 8);
        assert_eq!(
            node.kademlia.local_id().bucket_index(&refresh.targets[0]),
            Some(0)
        );
        assert_eq!(
            node.kademlia.local_id().bucket_index(&refresh.targets[255]),
            Some(255)
        );
    }

    #[test]
    fn merge_trusted_routing_peers_accepts_valid_nonlocal_nodes() {
        let (identity, _) = Identity::create_with_passphrase("trusted routing peer").unwrap();
        let announce = NodeConfig {
            peer_id: "trusted-routing-peer".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let node_id = KademliaNodeId::from_peer_id(&announce.peer_id);
        let peer = RoutingPeer {
            node_id,
            announce: announce.clone(),
            identity_public_key: None,
            last_seen_at: current_unix_timestamp(),
        };
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "trusted-routing-local".into(),
            ..Default::default()
        });

        assert_eq!(node.merge_trusted_routing_peers(vec![peer]), 1);
        assert_eq!(node.kademlia.len(), 1);
        assert_eq!(
            node.routing_table
                .get("trusted-routing-peer")
                .unwrap()
                .peer_id,
            "trusted-routing-peer"
        );
        assert_eq!(node.merge_trusted_routing_peers(Vec::new()), 0);
    }

    #[test]
    fn merge_verified_routing_peers_requires_identity_public_key_and_signature() {
        let (identity, _) = Identity::create_with_passphrase("verified routing peer").unwrap();
        let (other, _) = Identity::create_with_passphrase("wrong verified routing key").unwrap();
        let announce = NodeConfig {
            peer_id: "verified-routing-peer".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let node_id = KademliaNodeId::from_peer_id(&announce.peer_id);
        let base_peer = RoutingPeer {
            node_id,
            announce: announce.clone(),
            identity_public_key: Some(BASE64.encode(identity.identity_public_key())),
            last_seen_at: current_unix_timestamp(),
        };
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "verified-routing-local".into(),
            ..Default::default()
        });

        let mut missing_key = base_peer.clone();
        missing_key.identity_public_key = None;
        let mut wrong_key = base_peer.clone();
        wrong_key.identity_public_key = Some(BASE64.encode(other.identity_public_key()));
        assert_eq!(
            node.merge_verified_routing_peers(vec![missing_key, wrong_key]),
            0
        );
        assert!(node.kademlia.is_empty());
        assert_eq!(
            node.maintenance
                .routing_peer_rejects
                .missing_identity_public_key,
            1
        );
        assert_eq!(node.maintenance.routing_peer_rejects.invalid_signature, 1);

        assert_eq!(node.merge_verified_routing_peers(vec![base_peer]), 1);
        assert_eq!(node.kademlia.len(), 1);
        let returned = node
            .kademlia
            .closest(KademliaNodeId::from_peer_id("anything"), 1);
        assert_eq!(
            returned[0].identity_public_key.as_deref(),
            Some(BASE64.encode(identity.identity_public_key()).as_str())
        );
    }

    #[test]
    fn merge_trusted_routing_peers_rejects_mismatched_expired_and_local_nodes() {
        let (identity, _) = Identity::create_with_passphrase("bad trusted routing peer").unwrap();
        let mut announce = NodeConfig {
            peer_id: "bad-trusted-routing-peer".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let valid_node_id = KademliaNodeId::from_peer_id(&announce.peer_id);
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "bad-trusted-routing-local".into(),
            ..Default::default()
        });
        let local_id = node.kademlia.local_id();

        let mismatched = RoutingPeer {
            node_id: KademliaNodeId::from_peer_id("different-peer"),
            announce: announce.clone(),
            identity_public_key: None,
            last_seen_at: current_unix_timestamp(),
        };
        announce.expires_at = current_unix_timestamp().saturating_sub(1);
        let expired = RoutingPeer {
            node_id: valid_node_id,
            announce: announce.clone(),
            identity_public_key: None,
            last_seen_at: current_unix_timestamp(),
        };
        let mut local_announce = announce.clone();
        local_announce.peer_id = "bad-trusted-routing-local".into();
        local_announce.expires_at = current_unix_timestamp().saturating_add(3600);
        let local = RoutingPeer {
            node_id: local_id,
            announce: local_announce,
            identity_public_key: None,
            last_seen_at: current_unix_timestamp(),
        };

        assert_eq!(
            node.merge_trusted_routing_peers(vec![mismatched, expired, local]),
            0
        );
        assert!(node.kademlia.is_empty());
        assert!(node.routing_table.is_empty());
        assert_eq!(node.maintenance.routing_peer_rejects.mismatched_node_id, 1);
        assert_eq!(node.maintenance.routing_peer_rejects.expired, 1);
        assert_eq!(node.maintenance.routing_peer_rejects.local_node, 1);
    }

    #[test]
    fn merge_verified_routing_peers_rejects_address_abuse() {
        let (identity, _) = Identity::create_with_passphrase("routing address limits").unwrap();
        let too_many_announce = NodeConfig {
            peer_id: "too-many-addresses-peer".into(),
            addresses: (0..=MAX_ROUTING_PEER_ADDRESSES)
                .map(|index| format!("transport://too-many-{index}"))
                .collect(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let oversized_announce = NodeConfig {
            peer_id: "oversized-address-peer".into(),
            addresses: vec![format!(
                "transport://{}",
                "a".repeat(MAX_ROUTING_PEER_ADDRESS_BYTES)
            )],
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let make_peer = |announce: PublicPeerAnnounce| RoutingPeer {
            node_id: KademliaNodeId::from_peer_id(&announce.peer_id),
            announce,
            identity_public_key: Some(BASE64.encode(identity.identity_public_key())),
            last_seen_at: current_unix_timestamp(),
        };
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "routing-address-limit-local".into(),
            ..Default::default()
        });
        assert_eq!(
            node.merge_verified_routing_peers(vec![
                make_peer(too_many_announce),
                make_peer(oversized_announce),
            ]),
            0
        );
        assert!(node.kademlia.is_empty());
        assert_eq!(node.maintenance.routing_peer_rejects.too_many_addresses, 1);
        assert_eq!(node.maintenance.routing_peer_rejects.address_too_large, 1);
        assert_eq!(node.maintenance.routing_peer_rejects.total(), 2);
    }

    #[test]
    fn dht_rpc_store_find_value_and_find_node() {
        let (identity, _) = Identity::create_with_passphrase("dht rpc peer").unwrap();
        let announce = NodeConfig {
            peer_id: "rpc-peer".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "rpc-local".into(),
            ..Default::default()
        });
        node.kademlia
            .insert_verified(announce.clone(), &identity.identity_public_key())
            .unwrap();
        let record = DhtRecord::public_peer(&announce, announce.to_export_text().unwrap(), 3600);
        let key = record.key;

        let store = node.handle_dht_rpc(DhtRpcRequest::StoreRecord {
            request_id: "store-1".into(),
            record: record.clone(),
        });
        assert_eq!(
            store,
            DhtRpcResponse::StoreResult {
                request_id: "store-1".into(),
                stored: true,
                inserted: true,
            }
        );
        let found = node.handle_dht_rpc(DhtRpcRequest::FindValue {
            request_id: "find-value-1".into(),
            key,
            limit: 8,
        });
        assert_eq!(
            found,
            DhtRpcResponse::Value {
                request_id: "find-value-1".into(),
                record: Some(record),
                closer_records: Vec::new(),
                closer_nodes: Vec::new(),
            }
        );
        let missing = node.handle_dht_rpc(DhtRpcRequest::FindValue {
            request_id: "find-value-2".into(),
            key: DhtRecordKey::for_public_peer("missing"),
            limit: 1,
        });
        match missing {
            DhtRpcResponse::Value {
                record,
                closer_records,
                closer_nodes,
                ..
            } => {
                assert!(record.is_none());
                assert_eq!(closer_records.len(), 1);
                assert_eq!(closer_nodes.len(), 1);
                assert_eq!(closer_nodes[0].announce.peer_id, "rpc-peer");
            }
            other => panic!("unexpected response: {other:?}"),
        }
        let nodes = node.handle_dht_rpc(DhtRpcRequest::FindNode {
            request_id: "find-node-1".into(),
            target: KademliaNodeId::from_peer_id("rpc-peer"),
            limit: 1,
        });
        match nodes {
            DhtRpcResponse::Nodes { nodes, .. } => {
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].announce.peer_id, "rpc-peer");
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn dht_rpc_rejects_prekey_record_with_mismatched_key() {
        let (identity, _) = Identity::create_with_passphrase("dht prekey invalid key").unwrap();
        let (other, _) = Identity::create_with_passphrase("dht prekey other key").unwrap();
        let (bundle, _) = PreKeyBundle::new(&identity, 7, 0, 3600).unwrap();
        let record = DhtRecord {
            key: DhtRecordKey::for_prekey(other.user_id()),
            kind: DhtRecordKind::PreKey,
            value: bundle.to_export_text().unwrap(),
            created_at: current_unix_timestamp(),
            expires_at: current_unix_timestamp() + 3600,
            republish_at: current_unix_timestamp() + 1800,
        };
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_dht_rpc(DhtRpcRequest::StoreRecord {
            request_id: "prekey-mismatch".into(),
            record,
        });
        assert_eq!(
            response,
            DhtRpcResponse::StoreResult {
                request_id: "prekey-mismatch".into(),
                stored: false,
                inserted: false,
            }
        );
        assert!(node.dht_records.is_empty());
    }

    #[test]
    fn dht_rpc_rejects_empty_mailbox_hint_record() {
        let (identity, _) = Identity::create_with_passphrase("dht mailbox empty").unwrap();
        let record = DhtRecord::mailbox_hint(identity.user_id(), "  ".into(), 3600);
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_dht_rpc(DhtRpcRequest::StoreRecord {
            request_id: "mailbox-empty".into(),
            record,
        });
        assert_eq!(
            response,
            DhtRpcResponse::StoreResult {
                request_id: "mailbox-empty".into(),
                stored: false,
                inserted: false,
            }
        );
        assert!(node.dht_records.is_empty());
    }

    #[test]
    fn dht_rpc_rejects_public_peer_record_with_mismatched_key() {
        let (identity, _) = Identity::create_with_passphrase("dht invalid key").unwrap();
        let announce = NodeConfig {
            peer_id: "valid-peer".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let mut record =
            DhtRecord::public_peer(&announce, announce.to_export_text().unwrap(), 3600);
        record.key = DhtRecordKey::for_public_peer("wrong-peer");
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_dht_rpc(DhtRpcRequest::StoreRecord {
            request_id: "mismatch".into(),
            record,
        });
        assert_eq!(
            response,
            DhtRpcResponse::StoreResult {
                request_id: "mismatch".into(),
                stored: false,
                inserted: false,
            }
        );
        assert!(node.dht_records.is_empty());
    }

    #[test]
    fn dht_rpc_rejects_ttl_above_max() {
        let mut node = NativeNode::new(NodeConfig::default());
        let now = current_unix_timestamp();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("ttl-rpc"),
            kind: DhtRecordKind::PublicPeer,
            value: "record".into(),
            created_at: now,
            expires_at: now + DEFAULT_MAX_DHT_RECORD_TTL_SECONDS + 1,
            republish_at: now + 50,
        };
        let response = node.handle_dht_rpc(DhtRpcRequest::StoreRecord {
            request_id: "ttl".into(),
            record,
        });
        assert_eq!(
            response,
            DhtRpcResponse::StoreResult {
                request_id: "ttl".into(),
                stored: false,
                inserted: false,
            }
        );
        assert!(node.dht_records.is_empty());
    }

    #[test]
    fn dht_rpc_rejects_oversized_store_record() {
        let mut node = NativeNode::new(NodeConfig::default());
        let now = current_unix_timestamp();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("oversized-rpc"),
            kind: DhtRecordKind::PublicPeer,
            value: "A".repeat(DEFAULT_MAX_DHT_RECORD_VALUE_BYTES + 1),
            created_at: now,
            expires_at: now + 100,
            republish_at: now + 50,
        };
        let response = node.handle_dht_rpc(DhtRpcRequest::StoreRecord {
            request_id: "oversized".into(),
            record,
        });
        assert_eq!(
            response,
            DhtRpcResponse::StoreResult {
                request_id: "oversized".into(),
                stored: false,
                inserted: false,
            }
        );
        assert!(node.dht_records.is_empty());
    }

    #[test]
    fn dht_rpc_rejects_expired_store_record() {
        let mut node = NativeNode::new(NodeConfig::default());
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("expired-rpc"),
            kind: DhtRecordKind::PublicPeer,
            value: "expired".into(),
            created_at: 1,
            expires_at: 1,
            republish_at: 1,
        };
        let response = node.handle_dht_rpc(DhtRpcRequest::StoreRecord {
            request_id: "store-expired".into(),
            record,
        });
        assert_eq!(
            response,
            DhtRpcResponse::StoreResult {
                request_id: "store-expired".into(),
                stored: false,
                inserted: false,
            }
        );
        assert!(node.dht_records.is_empty());
    }

    #[test]
    fn dht_record_factories_set_kind_key_ttl_and_republish() {
        let (identity, _) = Identity::create_with_passphrase("dht factory user").unwrap();
        let announce = NodeConfig::default().create_announce(&identity).unwrap();
        let before = current_unix_timestamp();

        let peer = DhtRecord::public_peer(&announce, "announce-text".into(), 60);
        let prekey = DhtRecord::prekey(identity.user_id(), "prekey-text".into(), 60);
        let mailbox = DhtRecord::mailbox_hint(identity.user_id(), "http://node/mailbox".into(), 60);

        assert_eq!(peer.kind, DhtRecordKind::PublicPeer);
        assert_eq!(peer.key, DhtRecordKey::for_public_peer(&announce.peer_id));
        assert_eq!(prekey.kind, DhtRecordKind::PreKey);
        assert_eq!(prekey.key, DhtRecordKey::for_prekey(identity.user_id()));
        assert_eq!(mailbox.kind, DhtRecordKind::MailboxHint);
        assert_eq!(
            mailbox.key,
            DhtRecordKey::for_mailbox_hint(identity.user_id())
        );
        assert!(peer.created_at >= before);
        assert_eq!(peer.expires_at.saturating_sub(peer.created_at), 60);
        assert_eq!(peer.republish_at.saturating_sub(peer.created_at), 30);
        assert!(peer.should_republish_at(peer.republish_at));
        assert!(!peer.should_republish_at(peer.expires_at));
    }

    #[test]
    fn kademlia_distance_orders_closest_peer() {
        let local = KademliaNodeId::from_bytes([0; KADEMLIA_ID_BYTES]);
        let close = KademliaNodeId::from_bytes([
            0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let far = KademliaNodeId::from_bytes([
            0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0,
        ]);
        assert!(local.xor_distance(&close) < local.xor_distance(&far));
        assert_eq!(local.bucket_index(&far), Some(0));
        assert_eq!(local.bucket_index(&local), None);
    }

    #[test]
    fn kademlia_table_returns_closest_verified_peers() {
        let local = KademliaNodeId::from_peer_id("local");
        let mut table = KademliaRoutingTable::new(local, 20);
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let a = NodeConfig {
            peer_id: "peer-a".into(),
            ..Default::default()
        }
        .create_announce(&alice)
        .unwrap();
        let b = NodeConfig {
            peer_id: "peer-b".into(),
            ..Default::default()
        }
        .create_announce(&bob)
        .unwrap();
        table
            .insert_verified(a.clone(), &alice.identity_public_key())
            .unwrap();
        table
            .insert_verified(b.clone(), &bob.identity_public_key())
            .unwrap();
        let closest = table.closest(KademliaNodeId::from_peer_id("peer-a"), 1);
        assert_eq!(closest.len(), 1);
        assert_eq!(closest[0].announce.peer_id, "peer-a");
    }

    #[test]
    fn health_exposes_mailbox_limits() {
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: "/api/health".into(),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["mailbox_take_limit"], DEFAULT_MAX_MAILBOX_TAKE_LIMIT);
        assert_eq!(
            body["mailbox_max_bytes"],
            NodeConfig::default().max_mailbox_bytes.unwrap()
        );
        assert_eq!(
            body["mailbox_max_bytes_per_user"],
            DEFAULT_MAX_MAILBOX_BYTES_PER_USER
        );
        assert_eq!(
            body["mailbox_max_messages_per_user"],
            NodeConfig::default().max_mailbox_messages_per_user.unwrap()
        );
        assert_eq!(body["mailbox_ack_max_ids"], DEFAULT_MAX_MAILBOX_ACK_IDS);
        assert_eq!(
            body["mailbox_ack_id_max_bytes"],
            DEFAULT_MAX_MAILBOX_ACK_ID_BYTES
        );
        assert_eq!(body["state_db_permissions_hardened"], true);
        assert_eq!(body["libp2p_dht_rpc_request_max_bytes"], 1024 * 1024);
        assert_eq!(body["libp2p_dht_rpc_response_max_bytes"], 8 * 1024 * 1024);
        assert_eq!(body["libp2p_dht_rpc_max_concurrent_streams"], 32);
        assert_eq!(
            body["dht_peer_quarantine_consecutive_failures"],
            DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES
        );
        assert_eq!(body["libp2p_dht_pending_incoming_max"], 64);
        assert_eq!(body["libp2p_dht_pending_outgoing_max"], 64);
        assert_eq!(body["libp2p_dht_established_incoming_max"], 128);
        assert_eq!(body["libp2p_dht_established_outgoing_max"], 128);
        assert_eq!(body["libp2p_dht_established_total_max"], 256);
        assert_eq!(body["libp2p_dht_established_per_peer_max"], 4);
    }

    #[test]
    fn control_plane_announces_and_finds_closest() {
        let (local_identity, _) = Identity::create_with_passphrase("local").unwrap();
        let (peer_identity, _) = Identity::create_with_passphrase("peer").unwrap();
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "local-peer".into(),
            ..Default::default()
        });
        let announce = NodeConfig {
            peer_id: "peer-a".into(),
            ..Default::default()
        }
        .create_announce(&peer_identity)
        .unwrap();
        let body = serde_json::json!({
            "announce_text": announce.to_export_text().unwrap(),
            "identity_public_key": BASE64.encode(peer_identity.identity_public_key()),
        })
        .to_string();
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/announce".into(),
            body,
            headers: Vec::new(),
        });
        assert_eq!(response.status, 201);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: "/api/peers/closest?target=peer-a&limit=1".into(),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200);
        assert!(response.body.contains("peer-a"));
        assert_ne!(
            node.kademlia.local_id(),
            KademliaNodeId::from_user_id(local_identity.user_id())
        );
    }

    #[test]
    fn control_plane_derives_dht_record_keys() {
        let (identity, _) = Identity::create_with_passphrase("dht key derive user").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let prekey = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/dht/key?kind=prekey&value={}", identity.user_id()),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(prekey.status, 200, "{}", prekey.body);
        let body: serde_json::Value = serde_json::from_str(&prekey.body).unwrap();
        assert_eq!(body["kind"], "PreKey");
        assert_eq!(
            body["key"],
            DhtRecordKey::for_prekey(identity.user_id()).to_hex()
        );

        let mailbox = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!(
                "/api/dht/key?kind=mailbox-hint&value={}",
                identity.user_id()
            ),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(mailbox.status, 200, "{}", mailbox.body);
        let body: serde_json::Value = serde_json::from_str(&mailbox.body).unwrap();
        assert_eq!(body["kind"], "MailboxHint");
        assert_eq!(
            body["key"],
            DhtRecordKey::for_mailbox_hint(identity.user_id()).to_hex()
        );

        let contact_card = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!(
                "/api/dht/key?kind=contact-card&value={}",
                identity.user_id()
            ),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(contact_card.status, 200, "{}", contact_card.body);
        let body: serde_json::Value = serde_json::from_str(&contact_card.body).unwrap();
        assert_eq!(body["kind"], "ContactCard");
        assert_eq!(
            body["key"],
            DhtRecordKey::for_contact_card(identity.user_id()).to_hex()
        );

        let peer = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: "/api/dht/key?kind=public-peer&value=peer-a".into(),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(peer.status, 200, "{}", peer.body);
        let body: serde_json::Value = serde_json::from_str(&peer.body).unwrap();
        assert_eq!(body["kind"], "PublicPeer");
        assert_eq!(
            body["key"],
            DhtRecordKey::for_public_peer("peer-a").to_hex()
        );
    }

    #[test]
    fn control_plane_dht_plans_are_exposed() {
        let (alice, _) = Identity::create_with_passphrase("control dht plan alice").unwrap();
        let announce = NodeConfig {
            peer_id: "control-plan-peer".into(),
            ..Default::default()
        }
        .create_announce(&alice)
        .unwrap();
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "control-plan-local".into(),
            ..Default::default()
        });
        node.kademlia
            .insert_verified(announce, &alice.identity_public_key())
            .unwrap();
        let now = current_unix_timestamp();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("control-plan-record"),
            kind: DhtRecordKind::PublicPeer,
            value: "plan-record".into(),
            created_at: now,
            expires_at: now + 120,
            republish_at: now,
        };
        assert!(node.dht_records.store(record));

        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: "/api/dht/replication-plan?factor=1".into(),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["replication_factor"], 1);
        assert_eq!(body["records"].as_array().unwrap().len(), 1);
        assert_eq!(
            body["records"][0]["target_nodes"].as_array().unwrap().len(),
            1
        );

        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: "/api/dht/routing-refresh-plan".into(),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(
            body["targets"].as_array().unwrap().len(),
            KADEMLIA_ID_BYTES * 8
        );
    }

    #[test]
    fn control_plane_known_dht_find_value_path_rejects_wrong_method() {
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/dht/find-value?key=00".into(),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 405);
        assert!(response.body.contains("method not allowed"));
    }

    #[test]
    fn control_plane_dht_rpc_handles_store_find_value_and_find_node() {
        let (identity, _) = Identity::create_with_passphrase("control dht rpc peer").unwrap();
        let announce = NodeConfig {
            peer_id: "control-rpc-peer".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "control-rpc-local".into(),
            ..Default::default()
        });
        node.kademlia
            .insert_verified(announce.clone(), &identity.identity_public_key())
            .unwrap();
        let record = DhtRecord::public_peer(&announce, announce.to_export_text().unwrap(), 3600);
        let key = record.key;

        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/dht/rpc".into(),
            body: serde_json::json!({
                "request": {
                    "StoreRecord": {
                        "request_id": "store-control",
                        "record": record,
                    }
                }
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["StoreResult"]["stored"], true);
        assert_eq!(body["StoreResult"]["inserted"], true);

        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/dht/rpc".into(),
            body: serde_json::json!({
                "request": {
                    "FindValue": {
                        "request_id": "find-control",
                        "key": key,
                        "limit": 4,
                    }
                }
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["Value"]["record"]["value"], record.value);

        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/dht/rpc".into(),
            body: serde_json::json!({
                "request": {
                    "FindNode": {
                        "request_id": "find-node-control",
                        "target": KademliaNodeId::from_peer_id("control-rpc-peer"),
                        "limit": 1,
                    }
                }
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(
            body["Nodes"]["nodes"][0]["announce"]["peer_id"],
            "control-rpc-peer"
        );
    }

    #[test]
    fn control_plane_dht_record_rejects_invalid_json_stats() {
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/dht/record".into(),
            body: "{not-json".into(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert_eq!(node.maintenance.dht_record_rejects.invalid_json, 1);
    }

    #[test]
    fn control_plane_dht_record_rejects_public_peer_mismatched_key() {
        let (identity, _) = Identity::create_with_passphrase("control invalid dht key").unwrap();
        let announce = NodeConfig {
            peer_id: "valid-control-peer".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let mut record =
            DhtRecord::public_peer(&announce, announce.to_export_text().unwrap(), 3600);
        record.key = DhtRecordKey::for_public_peer("wrong-control-peer");
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/dht/record".into(),
            body: serde_json::json!({ "record": record }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert!(response.body.contains("invalid dht record"));
        assert!(node.dht_records.is_empty());
        assert_eq!(node.maintenance.dht_record_rejects.invalid_record, 1);
    }

    #[test]
    fn control_plane_dht_record_rejects_ttl_above_max() {
        let mut node = NativeNode::new(NodeConfig::default());
        let now = current_unix_timestamp();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("ttl-control"),
            kind: DhtRecordKind::PublicPeer,
            value: "record".into(),
            created_at: now,
            expires_at: now + DEFAULT_MAX_DHT_RECORD_TTL_SECONDS + 1,
            republish_at: now + 50,
        };
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/dht/record".into(),
            body: serde_json::json!({ "record": record }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert!(response.body.contains("ttl too long"));
        assert!(node.dht_records.is_empty());
        assert_eq!(node.maintenance.dht_record_rejects.ttl_too_long, 1);
    }

    #[test]
    fn control_plane_dht_record_rejects_oversized_values() {
        let mut node = NativeNode::new(NodeConfig::default());
        let now = current_unix_timestamp();
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("oversized-control-record"),
            kind: DhtRecordKind::PublicPeer,
            value: "A".repeat(DEFAULT_MAX_DHT_RECORD_VALUE_BYTES + 1),
            created_at: now,
            expires_at: now + 100,
            republish_at: now + 50,
        };
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/dht/record".into(),
            body: serde_json::json!({ "record": record }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 413);
        assert!(node.dht_records.is_empty());
        assert_eq!(node.maintenance.dht_record_rejects.payload_too_large, 1);
    }

    #[test]
    fn control_plane_dht_record_store_get_closest_and_snapshot() {
        let (identity, _) = Identity::create_with_passphrase("control dht record").unwrap();
        let announce = NodeConfig {
            peer_id: "peer-control".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let record = DhtRecord::public_peer(&announce, announce.to_export_text().unwrap(), 3600);
        let key = record.key;
        let expected_value = record.value.clone();
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/dht/record".into(),
            body: serde_json::json!({ "record": record }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 201, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["inserted"], true);
        assert_eq!(body["records"], 1);

        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/dht/record?key={key}"),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["found"], true);
        assert_eq!(body["record"]["value"], expected_value);

        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/dht/closest?target={key}&limit=1"),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["records"].as_array().unwrap().len(), 1);

        let restored = NativeNode::from_state_snapshot(node.to_state_snapshot());
        assert_eq!(restored.dht_records.len(), 1);
        let mut imported = NativeNode::new(NodeConfig::default());
        let stats = imported.merge_snapshot(restored.to_state_snapshot());
        assert_eq!(stats.dht_records, 1);
        assert_eq!(
            imported.dht_records.find_value(&key).unwrap().value,
            expected_value
        );
    }

    #[test]
    fn control_plane_mailbox_take_supports_limit_and_more_flag() {
        let (alice, _) = Identity::create_with_passphrase("alice take limit").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob take limit").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        for i in 0..3 {
            let message = MailboxMessage::new(
                &alice,
                bob.user_id().clone(),
                MailboxMessageKind::DirectEnvelope,
                format!("ciphertext-{i}"),
                3600,
            )
            .unwrap();
            node.mailbox
                .push_verified(message, &alice.identity_public_key())
                .unwrap();
        }
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/mailbox/take?user_id={}&limit=2", bob.user_id()),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["returned"], 2);
        assert_eq!(body["pending"], 3);
        assert_eq!(body["more"], true);
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 3);
    }

    #[test]
    fn control_plane_mailbox_push_and_take() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let message = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext".into(),
            3600,
        )
        .unwrap();
        let body = serde_json::json!({
            "message_text": message.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        })
        .to_string();
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/push".into(),
            body,
            headers: Vec::new(),
        });
        assert_eq!(response.status, 201);
        let push_body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        let pushed_delivery_id = push_body["delivery_id"].as_str().unwrap().to_string();
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 1);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!(
                "/api/mailbox/status?user_id={}&delivery_id={}",
                bob.user_id(),
                pushed_delivery_id
            ),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let status_body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(status_body["summary"]["total"], 1);
        assert_eq!(status_body["summary"]["undelivered"], 1);
        assert_eq!(status_body["summary"]["delivered_unacked"], 0);
        assert!(status_body["summary"]["bytes"].as_u64().unwrap() > 0);
        assert_eq!(
            status_body["max_bytes_per_user"],
            DEFAULT_MAX_MAILBOX_BYTES_PER_USER
        );
        assert_eq!(status_body["delivery"]["status"], "pending");
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/mailbox/take?user_id={}", bob.user_id()),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200);
        assert!(response.body.contains("ciphertext"));
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 1);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(
            body["max_bytes_per_user"],
            DEFAULT_MAX_MAILBOX_BYTES_PER_USER
        );
        assert!(body["pending_bytes"].as_u64().unwrap() > 0);
        let delivery_id = body["messages"][0]["delivery_id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(delivery_id, pushed_delivery_id);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!(
                "/api/mailbox/status?user_id={}&delivery_id={}",
                bob.user_id(),
                delivery_id
            ),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let status_body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(status_body["summary"]["total"], 1);
        assert_eq!(status_body["summary"]["undelivered"], 0);
        assert_eq!(status_body["summary"]["delivered_unacked"], 1);
        assert!(status_body["summary"]["bytes"].as_u64().unwrap() > 0);
        assert_eq!(status_body["delivery"]["status"], "delivered_unacked");
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/ack".into(),
            body: serde_json::json!({
                "user_id": bob.user_id().to_string(),
                "delivery_ids": [delivery_id],
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200);
        let ack_body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(ack_body["pending_bytes"], 0);
        assert_eq!(
            ack_body["max_bytes_per_user"],
            DEFAULT_MAX_MAILBOX_BYTES_PER_USER
        );
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 0);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!(
                "/api/mailbox/status?user_id={}&delivery_id={}",
                bob.user_id(),
                pushed_delivery_id
            ),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let status_body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(status_body["summary"]["total"], 0);
        assert_eq!(status_body["summary"]["bytes"], 0);
        assert_eq!(status_body["delivery"]["status"], "acked");
        assert!(status_body["delivery"]["acked_at"].as_u64().is_some());
    }

    #[test]
    fn control_plane_mailbox_ack_rejects_excessive_ids() {
        let (bob, _) = Identity::create_with_passphrase("bob ack too many").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/ack".into(),
            body: serde_json::json!({
                "user_id": bob.user_id().to_string(),
                "delivery_ids": vec!["x"; DEFAULT_MAX_MAILBOX_ACK_IDS + 1],
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 413);
        assert_eq!(node.maintenance.mailbox_ack_rejects.too_many_ids, 1);
    }

    #[test]
    fn control_plane_mailbox_ack_rejects_empty_id() {
        let (bob, _) = Identity::create_with_passphrase("bob ack empty").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/ack".into(),
            body: serde_json::json!({
                "user_id": bob.user_id().to_string(),
                "delivery_ids": [""],
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert_eq!(node.maintenance.mailbox_ack_rejects.empty_id, 1);
    }

    #[test]
    fn control_plane_mailbox_ack_rejects_oversized_id() {
        let (bob, _) = Identity::create_with_passphrase("bob ack oversized").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/ack".into(),
            body: serde_json::json!({
                "user_id": bob.user_id().to_string(),
                "delivery_ids": ["x".repeat(DEFAULT_MAX_MAILBOX_ACK_ID_BYTES + 1)],
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 413);
        assert_eq!(node.maintenance.mailbox_ack_rejects.id_too_large, 1);
    }

    #[test]
    fn control_plane_mailbox_ack_records_invalid_json_and_user_id() {
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/ack".into(),
            body: "{bad-json".into(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/ack".into(),
            body: serde_json::json!({ "user_id": "not-a-user", "delivery_ids": [] }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert_eq!(node.maintenance.mailbox_ack_rejects.invalid_json, 1);
        assert_eq!(node.maintenance.mailbox_ack_rejects.invalid_user_id, 1);
        assert_eq!(node.maintenance.mailbox_ack_rejects.total(), 2);
    }

    #[test]
    fn control_plane_mailbox_push_rejects_configured_limits() {
        let (alice, _) = Identity::create_with_passphrase("alice control limit").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob control limit").unwrap();
        let mut node = NativeNode::new(NodeConfig {
            max_mailbox_messages_per_user: Some(0),
            ..Default::default()
        });
        let message = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext".into(),
            3600,
        )
        .unwrap();
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/push".into(),
            body: serde_json::json!({
                "message_text": message.to_export_text().unwrap(),
                "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 413);
        assert!(response.body.contains("MAILBOX_QUOTA_EXCEEDED"));
        assert!(response.body.contains("recovery_hint"));
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 0);
        assert_eq!(node.maintenance.mailbox_push_rejects.payload_too_large, 1);
    }

    #[test]
    fn control_plane_mailbox_push_rate_limits_by_sender_when_configured() {
        let (alice, _) = Identity::create_with_passphrase("alice sender control limit").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob sender control limit").unwrap();
        let mut node = NativeNode::new(NodeConfig {
            mailbox_sender_rate_limit_window_seconds: Some(60),
            mailbox_sender_rate_limit_max_messages: Some(1),
            ..Default::default()
        });
        for expected_status in [201, 429] {
            let message = MailboxMessage::new(
                &alice,
                bob.user_id().clone(),
                MailboxMessageKind::DirectEnvelope,
                "ciphertext".into(),
                3600,
            )
            .unwrap();
            let response = node.handle_control_request(ControlRequest {
                method: "POST".into(),
                path: "/api/mailbox/push".into(),
                body: serde_json::json!({
                    "message_text": message.to_export_text().unwrap(),
                    "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
                })
                .to_string(),
                headers: Vec::new(),
            });
            assert_eq!(response.status, expected_status, "{}", response.body);
            if expected_status == 429 {
                assert!(response.body.contains("MAILBOX_RATE_LIMITED"));
            }
            if expected_status == 429 {
                assert!(response.body.contains("MAILBOX_RATE_LIMITED"));
            }
        }
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 1);
        assert_eq!(node.maintenance.mailbox_push_rejects.sender_rate_limited, 1);
    }

    #[test]
    fn control_plane_mailbox_push_rate_limits_globally_when_configured() {
        let (alice, _) = Identity::create_with_passphrase("alice global control limit").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob global control limit").unwrap();
        let (carol, _) = Identity::create_with_passphrase("carol global control limit").unwrap();
        let mut node = NativeNode::new(NodeConfig {
            mailbox_global_rate_limit_window_seconds: Some(60),
            mailbox_global_rate_limit_max_messages: Some(1),
            ..Default::default()
        });
        for (sender, expected_status) in [(&alice, 201), (&carol, 429)] {
            let message = MailboxMessage::new(
                sender,
                bob.user_id().clone(),
                MailboxMessageKind::DirectEnvelope,
                "ciphertext".into(),
                3600,
            )
            .unwrap();
            let response = node.handle_control_request(ControlRequest {
                method: "POST".into(),
                path: "/api/mailbox/push".into(),
                body: serde_json::json!({
                    "message_text": message.to_export_text().unwrap(),
                    "from_identity_public_key": BASE64.encode(sender.identity_public_key()),
                })
                .to_string(),
                headers: Vec::new(),
            });
            assert_eq!(response.status, expected_status, "{}", response.body);
        }
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 1);
        assert_eq!(node.maintenance.mailbox_push_rejects.global_rate_limited, 1);
    }

    #[test]
    fn control_plane_mailbox_push_records_invalid_payload_stats() {
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/push".into(),
            body: "{not-json".into(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert_eq!(node.maintenance.mailbox_push_rejects.invalid_json, 1);

        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/mailbox/push".into(),
            body: serde_json::json!({
                "message_text": "not-a-mailbox-message",
                "from_identity_public_key": "not-base64"
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert_eq!(
            node.maintenance.mailbox_push_rejects.invalid_message_format,
            1
        );
        assert_eq!(node.maintenance.mailbox_push_rejects.total(), 2);

        let restored = NativeNode::from_state_snapshot(node.to_state_snapshot());
        assert_eq!(restored.maintenance.mailbox_push_rejects.total(), 2);
    }

    #[test]
    fn node_state_snapshot_roundtrip() {
        let (alice, _) = Identity::create_with_passphrase("alice snapshot").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob snapshot").unwrap();
        let mut node = NativeNode::new(NodeConfig {
            peer_id: "local-snapshot".into(),
            ..Default::default()
        });
        let announce = NodeConfig {
            peer_id: "peer-snapshot".into(),
            ..Default::default()
        }
        .create_announce(&alice)
        .unwrap();
        node.routing_table
            .insert_verified(announce.clone(), &alice.identity_public_key())
            .unwrap();
        node.kademlia
            .insert_verified(announce, &alice.identity_public_key())
            .unwrap();
        let msg = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext".into(),
            3600,
        )
        .unwrap();
        let delivery_id = node
            .mailbox
            .push_verified(msg, &alice.identity_public_key())
            .unwrap();
        let deliveries = node.mailbox.take_for(bob.user_id());
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].delivery_id, delivery_id);
        assert_eq!(
            node.mailbox
                .ack(bob.user_id(), std::slice::from_ref(&delivery_id)),
            1
        );

        let snapshot = node.to_state_snapshot();
        assert_eq!(snapshot.routing_peers.len(), 1);
        assert_eq!(snapshot.mailbox_ack_receipts.len(), 1);
        assert_eq!(
            snapshot.routing_peers[0].identity_public_key.as_deref(),
            Some(BASE64.encode(alice.identity_public_key()).as_str())
        );
        let restored = NativeNode::from_state_snapshot(snapshot);
        assert_eq!(restored.routing_table.len(), 1);
        assert_eq!(
            restored
                .routing_table
                .identity_public_key_for("peer-snapshot"),
            Some(BASE64.encode(alice.identity_public_key()).as_str())
        );
        assert_eq!(restored.kademlia.len(), 1);
        assert_eq!(
            restored.kademlia.all_peers()[0]
                .identity_public_key
                .as_deref(),
            Some(BASE64.encode(alice.identity_public_key()).as_str())
        );
        assert_eq!(restored.mailbox.pending_for(bob.user_id()), 0);
        assert_eq!(
            restored
                .mailbox
                .delivery_status(bob.user_id(), &delivery_id)
                .status,
            MailboxDeliveryState::Acked
        );
    }

    #[test]
    fn control_plane_prekey_publish_and_get() {
        let (alice, _) = Identity::create_with_passphrase("alice prekey").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let (bundle, _) = PreKeyBundle::new(&alice, 1, 2, 3600).unwrap();
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/prekey/publish".into(),
            body: serde_json::json!({
                "prekey_bundle_text": bundle.to_export_text().unwrap(),
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 201);
        assert_eq!(node.prekeys.len(), 1);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/prekey/get?user_id={}", alice.user_id()),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200);
        assert!(response.body.contains("lm-prekey-bundle-v1"));
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["found"], true);
    }

    #[test]
    fn prekey_snapshot_roundtrip() {
        let (alice, _) = Identity::create_with_passphrase("alice prekey snapshot").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let (bundle, _, records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&alice, 1, 1, 3600).unwrap();
        node.prekeys
            .publish_verified_with_signed_one_time_prekey_records(bundle, records)
            .unwrap();
        let restored = NativeNode::from_state_snapshot(node.to_state_snapshot());
        assert!(
            restored
                .prekeys
                .get_for_unchecked(alice.user_id())
                .is_some()
        );
        assert_eq!(
            restored
                .prekeys
                .signed_one_time_prekey_records_for(alice.user_id())
                .len(),
            1
        );
    }

    #[test]
    fn control_plane_sync_peer_reset_clears_quarantine_state() {
        let mut node = NativeNode::new(NodeConfig::default());
        let url = "http://peer-reset.example";
        for idx in 0..DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES {
            node.sync_status
                .record_failure(url, format!("synthetic failure {idx}"));
        }
        node.sync_status
            .record_next_attempt(url, current_unix_timestamp().saturating_add(600));
        assert!(
            node.sync_status
                .peers
                .get(url)
                .unwrap()
                .consecutive_failures
                >= DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES
        );

        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/sync/peer/reset".into(),
            body: serde_json::json!({ "url": url }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["reset"], true);
        assert_eq!(body["status"]["consecutive_failures"], 0);
        assert!(body["status"]["last_error"].is_null());
        assert!(body["status"]["next_attempt_at"].is_null());

        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/sync/peer/reset".into(),
            body: serde_json::json!({ "url": "http://missing-peer.example" }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["reset"], false);
        assert!(body["status"].is_null());
    }

    #[test]
    fn control_plane_sync_snapshot_import_merges_prekeys_and_mailbox() {
        let (alice, _) = Identity::create_with_passphrase("alice sync").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob sync").unwrap();
        let mut source = NativeNode::new(NodeConfig {
            peer_id: "source".into(),
            ..Default::default()
        });
        let (bundle, _) = PreKeyBundle::new(&alice, 1, 1, 3600).unwrap();
        source.prekeys.publish_verified(bundle).unwrap();
        let msg = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "sync ciphertext".into(),
            3600,
        )
        .unwrap();
        source
            .mailbox
            .push_verified(msg, &alice.identity_public_key())
            .unwrap();
        let snapshot_response = source.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: "/api/sync/snapshot".into(),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(snapshot_response.status, 200);

        let mut target = NativeNode::new(NodeConfig {
            peer_id: "target".into(),
            ..Default::default()
        });
        let snapshot: NodeStateSnapshot = serde_json::from_str(&snapshot_response.body).unwrap();
        let import_response = target.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/sync/import".into(),
            body: serde_json::json!({ "snapshot": snapshot }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(import_response.status, 200);
        assert!(target.prekeys.get_for_unchecked(alice.user_id()).is_some());
        assert_eq!(target.mailbox.pending_for(bob.user_id()), 1);
    }

    #[test]
    fn sync_snapshot_import_skips_invalid_dht_records() {
        let (identity, _) = Identity::create_with_passphrase("snapshot dht valid").unwrap();
        let announce = NodeConfig {
            peer_id: "snapshot-valid-peer".into(),
            ..Default::default()
        }
        .create_announce(&identity)
        .unwrap();
        let valid = DhtRecord::public_peer(&announce, announce.to_export_text().unwrap(), 3600);
        let mut invalid = valid.clone();
        invalid.key = DhtRecordKey::for_public_peer("snapshot-wrong-peer");
        let mut snapshot = NativeNode::new(NodeConfig::default()).to_state_snapshot();
        snapshot.dht_records = vec![valid.clone(), invalid];

        let mut target = NativeNode::new(NodeConfig::default());
        let stats = target.merge_snapshot(snapshot);
        assert_eq!(stats.dht_records, 1);
        assert_eq!(target.maintenance.dht_record_rejects.invalid_record, 1);
        assert!(target.dht_records.find_value(&valid.key).is_some());
        assert!(
            target
                .dht_records
                .find_value(&DhtRecordKey::for_public_peer("snapshot-wrong-peer"))
                .is_none()
        );
    }

    #[test]
    fn prekey_consume_tracks_individual_one_time_keys() {
        let (alice, _) = Identity::create_with_passphrase("alice consume").unwrap();
        let mut store = PreKeyStore::default();
        let (bundle, _) = PreKeyBundle::new(&alice, 1, 2, 3600).unwrap();
        store.publish_verified(bundle).unwrap();
        let (_, first) = store
            .take_for_with_selected_one_time_prekey(alice.user_id(), true)
            .unwrap();
        let (_, second) = store
            .take_for_with_selected_one_time_prekey(alice.user_id(), true)
            .unwrap();
        assert_eq!(first, Some(0));
        assert_eq!(second, Some(1));
        assert_eq!(store.consumed_for(alice.user_id()), vec![0, 1]);
        assert!(store.get_for(alice.user_id()).is_some());
    }

    #[test]
    fn prekey_publish_rejects_too_many_signed_one_time_records() {
        let (alice, _) = Identity::create_with_passphrase("alice too many signed otk").unwrap();
        let (bundle, _private, records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&alice, 1, 1, 3600).unwrap();
        let too_many = vec![records[0].clone(); lm_core::limits::MAX_ONE_TIME_PREKEYS + 1];
        let mut store = PreKeyStore::default();
        assert_eq!(
            store
                .publish_verified_with_signed_one_time_prekey_records(bundle, too_many)
                .unwrap_err(),
            LmError::PayloadTooLarge
        );
        assert!(store.get_for_unchecked(alice.user_id()).is_none());
    }

    #[test]
    fn prekey_store_prefers_independent_signed_one_time_records() {
        let (alice, _) = Identity::create_with_passphrase("alice signed consume").unwrap();
        let mut store = PreKeyStore::default();
        let (bundle, _, records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&alice, 1, 2, 3600).unwrap();
        store
            .publish_verified_with_signed_one_time_prekey_records(bundle, records)
            .unwrap();

        let (_, first) = store
            .take_for_with_selected_one_time_prekey_record(alice.user_id(), true)
            .unwrap();
        let (_, second) = store
            .take_for_with_selected_one_time_prekey_record(alice.user_id(), true)
            .unwrap();

        assert_eq!(first.as_ref().map(|record| record.key_id), Some(0));
        assert_eq!(second.as_ref().map(|record| record.key_id), Some(1));
        assert_eq!(store.consumed_for(alice.user_id()), vec![0, 1]);
        assert_eq!(
            store.remaining_one_time_prekeys_for(alice.user_id()),
            Some(0)
        );
    }

    #[test]
    fn prekey_rotation_and_consumption_interop_across_snapshots() {
        let (alice, _) = Identity::create_with_passphrase("alice prekey interop").unwrap();
        let mut node_a = NativeNode::new(NodeConfig::default());
        let mut node_b = NativeNode::new(NodeConfig::default());

        let (bundle, _private, records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&alice, 1, 2, 3600).unwrap();
        node_a
            .prekeys
            .publish_verified_with_signed_one_time_prekey_records(bundle, records)
            .unwrap();

        let (_first_bundle, first_record) = node_a
            .prekeys
            .take_for_with_selected_one_time_prekey_record(alice.user_id(), true)
            .unwrap();
        assert_eq!(first_record.as_ref().map(|record| record.key_id), Some(0));
        assert_eq!(node_a.prekeys.consumed_for(alice.user_id()), vec![0]);

        node_b.merge_snapshot(node_a.to_state_snapshot());
        assert_eq!(node_b.prekeys.consumed_for(alice.user_id()), vec![0]);
        let (_second_bundle, second_record) = node_b
            .prekeys
            .take_for_with_selected_one_time_prekey_record(alice.user_id(), true)
            .unwrap();
        assert_eq!(second_record.as_ref().map(|record| record.key_id), Some(1));
        assert_eq!(node_b.prekeys.consumed_for(alice.user_id()), vec![0, 1]);

        node_a.merge_snapshot(node_b.to_state_snapshot());
        assert_eq!(node_a.prekeys.consumed_for(alice.user_id()), vec![0, 1]);

        let (rotated, _rotated_private, rotated_records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&alice, 2, 2, 3600).unwrap();
        node_a
            .prekeys
            .publish_verified_with_signed_one_time_prekey_records(rotated, rotated_records)
            .unwrap();
        assert_eq!(
            node_a.prekeys.consumed_for(alice.user_id()),
            Vec::<u32>::new()
        );

        node_b.merge_snapshot(node_a.to_state_snapshot());
        assert_eq!(
            node_b.prekeys.consumed_for(alice.user_id()),
            Vec::<u32>::new()
        );
        assert_eq!(
            node_b
                .prekeys
                .remaining_one_time_prekeys_for(alice.user_id()),
            Some(2)
        );
    }

    #[test]
    fn prekey_rotation_resets_consumed_one_time_keys() {
        let (alice, _) = Identity::create_with_passphrase("alice prekey rotate").unwrap();
        let mut store = PreKeyStore::default();
        let (first, _) = PreKeyBundle::new(&alice, 1, 2, 3600).unwrap();
        store.publish_verified(first).unwrap();
        let (_, selected) = store
            .take_for_with_selected_one_time_prekey(alice.user_id(), true)
            .unwrap();
        assert_eq!(selected, Some(0));
        assert_eq!(store.consumed_for(alice.user_id()), vec![0]);

        let (rotated, _) = PreKeyBundle::new(&alice, 2, 2, 3600).unwrap();
        store.publish_verified(rotated).unwrap();
        assert_eq!(store.consumed_for(alice.user_id()), Vec::<u32>::new());
        assert_eq!(
            store.remaining_one_time_prekeys_for(alice.user_id()),
            Some(2)
        );
    }

    #[test]
    fn prekey_store_prunes_expired_bundles_and_consumed_state() {
        let (alice, _) = Identity::create_with_passphrase("alice prekey prune").unwrap();
        let mut store = PreKeyStore::default();
        let (bundle, _) = PreKeyBundle::new(&alice, 1, 1, 3600).unwrap();
        store.publish_verified(bundle).unwrap();
        store
            .take_for_with_selected_one_time_prekey(alice.user_id(), true)
            .unwrap();
        assert_eq!(store.len(), 1);
        assert_eq!(store.prune_expired(u64::MAX), 1);
        assert_eq!(store.len(), 0);
        assert_eq!(store.consumed_for(alice.user_id()), Vec::<u32>::new());
    }

    #[test]
    fn control_plane_prekey_get_reports_remaining_and_low_watermark() {
        let (alice, _) = Identity::create_with_passphrase("alice prekey status").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let (bundle, _) = PreKeyBundle::new(&alice, 1, 2, 3600).unwrap();
        let publish_response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/prekey/publish".into(),
            body: serde_json::json!({
                "prekey_bundle_text": bundle.to_export_text().unwrap(),
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(publish_response.status, 201);
        let publish_body: serde_json::Value = serde_json::from_str(&publish_response.body).unwrap();
        assert_eq!(publish_body["one_time_prekeys"], 2);
        assert_eq!(publish_body["remaining_one_time_prekeys"], 2);
        assert_eq!(publish_body["low_one_time_prekeys"], false);
        assert_eq!(publish_body["replenishment_required"], false);
        assert_eq!(publish_body["replenishment_actor"], "client");
        assert_eq!(publish_body["node_generates_user_keys"], false);

        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/prekey/get?user_id={}&consume=true", alice.user_id()),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["selected_one_time_prekey_id"], 0);
        assert_eq!(body["remaining_one_time_prekeys"], 1);
        assert_eq!(body["low_one_time_prekeys"], true);
        assert_eq!(body["replenishment_required"], true);
        assert_eq!(body["replenishment_actor"], "client");
        assert_eq!(body["node_generates_user_keys"], false);

        let status = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/prekey/status?user_id={}", alice.user_id()),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(status.status, 200);
        let status_body: serde_json::Value = serde_json::from_str(&status.body).unwrap();
        assert_eq!(status_body["found"], true);
        assert_eq!(status_body["remaining_one_time_prekeys"], 1);
        assert_eq!(status_body["low_one_time_prekeys"], true);
        assert_eq!(status_body["replenishment_required"], true);
        assert_eq!(status_body["replenishment_actor"], "client");
        assert_eq!(status_body["node_generates_user_keys"], false);

        let (missing, _) = Identity::create_with_passphrase("missing prekey status").unwrap();
        let missing_status = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/prekey/status?user_id={}", missing.user_id()),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(missing_status.status, 200);
        let missing_body: serde_json::Value = serde_json::from_str(&missing_status.body).unwrap();
        assert_eq!(missing_body["found"], false);
        assert_eq!(
            missing_body["remaining_one_time_prekeys"],
            serde_json::Value::Null
        );
        assert_eq!(missing_body["low_one_time_prekeys"], false);
        assert_eq!(missing_body["replenishment_required"], true);
        assert_eq!(missing_body["replenishment_actor"], "client");
        assert_eq!(missing_body["node_generates_user_keys"], false);
    }

    #[test]
    fn control_plane_prekey_publish_rejects_too_many_signed_one_time_records() {
        let (alice, _) =
            Identity::create_with_passphrase("alice control too many signed otk").unwrap();
        let (bundle, _private, records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&alice, 1, 1, 3600).unwrap();
        let too_many =
            vec![records[0].to_export_text().unwrap(); lm_core::limits::MAX_ONE_TIME_PREKEYS + 1];
        let mut node = NativeNode::new(NodeConfig::default());
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/prekey/publish".into(),
            body: serde_json::json!({
                "prekey_bundle_text": bundle.to_export_text().unwrap(),
                "signed_one_time_prekey_record_texts": too_many,
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert!(
            response
                .body
                .to_ascii_lowercase()
                .contains("payload too large")
        );
        assert!(node.prekeys.get_for_unchecked(alice.user_id()).is_none());
    }

    #[test]
    fn control_plane_prekey_publish_and_get_signed_one_time_records() {
        let (alice, _) = Identity::create_with_passphrase("alice prekey signed records").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let (bundle, _, records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&alice, 3, 2, 3600).unwrap();
        let record_texts = records
            .iter()
            .map(|record| record.to_export_text().unwrap())
            .collect::<Vec<_>>();

        let publish_response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/api/prekey/publish".into(),
            body: serde_json::json!({
                "prekey_bundle_text": bundle.to_export_text().unwrap(),
                "signed_one_time_prekey_record_texts": record_texts,
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(publish_response.status, 201, "{}", publish_response.body);
        let publish_body: serde_json::Value = serde_json::from_str(&publish_response.body).unwrap();
        assert_eq!(publish_body["one_time_prekeys"], 2);
        assert_eq!(publish_body["signed_one_time_prekey_records"], 2);
        assert_eq!(publish_body["remaining_one_time_prekeys"], 2);

        let get_response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/api/prekey/get?user_id={}&consume=true", alice.user_id()),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(get_response.status, 200, "{}", get_response.body);
        let body: serde_json::Value = serde_json::from_str(&get_response.body).unwrap();
        assert_eq!(body["selected_one_time_prekey_id"], 0);
        assert!(
            body["selected_signed_one_time_prekey_record_text"]
                .as_str()
                .unwrap()
                .starts_with(lm_core::prekey::SIGNED_ONE_TIME_PREKEY_RECORD_PREFIX)
        );
        assert_eq!(body["remaining_one_time_prekeys"], 1);
        assert_eq!(body["signed_one_time_prekey_records"], 2);

        let restored = NativeNode::from_state_snapshot(node.to_state_snapshot());
        assert_eq!(
            restored
                .prekeys
                .signed_one_time_prekey_records_for(alice.user_id())
                .len(),
            2
        );
        assert_eq!(restored.prekeys.consumed_for(alice.user_id()), vec![0]);
    }

    #[test]
    fn capabilities_csv_parser_accepts_supported_values() {
        assert_eq!(
            parse_capabilities_csv("bootstrap,dht,mailbox").unwrap(),
            vec![
                PublicPeerCapability::Bootstrap,
                PublicPeerCapability::Dht,
                PublicPeerCapability::Mailbox
            ]
        );
    }
}
