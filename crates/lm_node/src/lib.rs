//! Native node scaffold for LM Talk.
//!
//! This crate intentionally starts as a deterministic, testable scaffold rather
//! than a real Kademlia implementation. It owns the public-peer runtime model
//! that future UDP/TCP/WebSocket transports can plug into: signed public peer
//! announcements, an in-memory routing table, and an optional mailbox queue.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    Identity, IdentityBackupPackage, LmError, MailboxMessage, PreKeyBundle, PublicPeerAnnounce,
    PublicPeerCapability, Result, SignedOneTimePreKeyRecord, UserId,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

pub const KADEMLIA_ID_BYTES: usize = 32;
pub const DEFAULT_K_BUCKET_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KademliaNodeId([u8; KADEMLIA_ID_BYTES]);

impl KademliaNodeId {
    pub fn from_bytes(bytes: [u8; KADEMLIA_ID_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn from_peer_id(peer_id: &str) -> Self {
        Self(*blake3::hash(peer_id.as_bytes()).as_bytes())
    }

    pub fn from_user_id(user_id: &UserId) -> Self {
        Self(*blake3::hash(user_id.as_str().as_bytes()).as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8; KADEMLIA_ID_BYTES] {
        &self.0
    }

    pub fn xor_distance(&self, other: &Self) -> KademliaDistance {
        let mut out = [0u8; KADEMLIA_ID_BYTES];
        for (idx, byte) in out.iter_mut().enumerate() {
            *byte = self.0[idx] ^ other.0[idx];
        }
        KademliaDistance(out)
    }

    pub fn bucket_index(&self, other: &Self) -> Option<usize> {
        self.xor_distance(other).bucket_index()
    }

    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.0)
    }
}

impl std::fmt::Display for KademliaNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KademliaDistance([u8; KADEMLIA_ID_BYTES]);

impl KademliaDistance {
    pub fn bucket_index(&self) -> Option<usize> {
        for (byte_index, byte) in self.0.iter().enumerate() {
            if *byte != 0 {
                let bit_index = byte.leading_zeros() as usize;
                return Some(byte_index * 8 + bit_index);
            }
        }
        None
    }

    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.0)
    }
}

impl Ord for KademliaDistance {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for KademliaDistance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DhtRecordKey([u8; KADEMLIA_ID_BYTES]);

impl DhtRecordKey {
    pub fn from_bytes(bytes: [u8; KADEMLIA_ID_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn from_hex(value: &str) -> Result<Self> {
        let bytes = decode_hex_32(value)?;
        Ok(Self(bytes))
    }

    pub fn for_public_peer(peer_id: &str) -> Self {
        Self::derive("public-peer", peer_id.as_bytes())
    }

    pub fn for_prekey(user_id: &UserId) -> Self {
        Self::derive("prekey", user_id.as_str().as_bytes())
    }

    pub fn for_mailbox_hint(user_id: &UserId) -> Self {
        Self::derive("mailbox-hint", user_id.as_str().as_bytes())
    }

    fn derive(namespace: &str, value: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"lm-talk:dht-record:v1:");
        hasher.update(namespace.as_bytes());
        hasher.update(b":");
        hasher.update(value);
        Self(*hasher.finalize().as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8; KADEMLIA_ID_BYTES] {
        &self.0
    }

    pub fn to_node_id(&self) -> KademliaNodeId {
        KademliaNodeId::from_bytes(self.0)
    }

    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.0)
    }
}

impl std::fmt::Display for DhtRecordKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DhtRecordKind {
    PublicPeer,
    PreKey,
    MailboxHint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhtRecord {
    pub key: DhtRecordKey,
    pub kind: DhtRecordKind,
    pub value: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub republish_at: u64,
}

impl DhtRecord {
    pub fn new(
        key: DhtRecordKey,
        kind: DhtRecordKind,
        value: String,
        ttl_seconds: u64,
        republish_after_seconds: u64,
    ) -> Self {
        let now = current_unix_timestamp();
        Self {
            key,
            kind,
            value,
            created_at: now,
            expires_at: now.saturating_add(ttl_seconds),
            republish_at: now.saturating_add(republish_after_seconds.min(ttl_seconds)),
        }
    }

    pub fn public_peer(peer: &PublicPeerAnnounce, value: String, ttl_seconds: u64) -> Self {
        Self::new(
            DhtRecordKey::for_public_peer(&peer.peer_id),
            DhtRecordKind::PublicPeer,
            value,
            ttl_seconds,
            ttl_seconds / 2,
        )
    }

    pub fn prekey(user_id: &UserId, value: String, ttl_seconds: u64) -> Self {
        Self::new(
            DhtRecordKey::for_prekey(user_id),
            DhtRecordKind::PreKey,
            value,
            ttl_seconds,
            ttl_seconds / 2,
        )
    }

    pub fn mailbox_hint(user_id: &UserId, value: String, ttl_seconds: u64) -> Self {
        Self::new(
            DhtRecordKey::for_mailbox_hint(user_id),
            DhtRecordKind::MailboxHint,
            value,
            ttl_seconds,
            ttl_seconds / 2,
        )
    }

    pub fn is_expired_at(&self, now: u64) -> bool {
        self.expires_at <= now
    }

    pub fn should_republish_at(&self, now: u64) -> bool {
        now >= self.republish_at && !self.is_expired_at(now)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DhtRecordStore {
    records: HashMap<DhtRecordKey, DhtRecord>,
}

impl DhtRecordStore {
    pub fn store(&mut self, record: DhtRecord) -> bool {
        if record.is_expired_at(current_unix_timestamp()) {
            return false;
        }
        let is_new = !self.records.contains_key(&record.key);
        self.records.insert(record.key, record);
        is_new
    }

    pub fn find_value(&mut self, key: &DhtRecordKey) -> Option<DhtRecord> {
        self.prune_expired(current_unix_timestamp());
        self.records.get(key).cloned()
    }

    pub fn closest_records(&mut self, target: DhtRecordKey, limit: usize) -> Vec<DhtRecord> {
        self.prune_expired(current_unix_timestamp());
        let target = target.to_node_id();
        let mut records = self.records.values().cloned().collect::<Vec<_>>();
        records.sort_by_key(|record| record.key.to_node_id().xor_distance(&target));
        records.truncate(limit);
        records
    }

    pub fn due_for_republish(&mut self, now: u64) -> Vec<DhtRecord> {
        self.prune_expired(now);
        self.records
            .values()
            .filter(|record| record.should_republish_at(now))
            .cloned()
            .collect()
    }

    pub fn prune_expired(&mut self, now: u64) -> usize {
        let before = self.records.len();
        self.records.retain(|_, record| !record.is_expired_at(now));
        before.saturating_sub(self.records.len())
    }

    pub fn all_records(&self) -> Vec<DhtRecord> {
        self.records.values().cloned().collect()
    }

    pub fn restore_records(&mut self, records: Vec<DhtRecord>) {
        self.records.clear();
        for record in records {
            self.store(record);
        }
    }

    pub fn merge_records(&mut self, records: Vec<DhtRecord>) -> usize {
        let mut inserted = 0;
        for record in records {
            if self.store(record) {
                inserted += 1;
            }
        }
        inserted
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhtRecordReplicationPlan {
    pub record: DhtRecord,
    pub target_nodes: Vec<RoutingPeer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhtReplicationPlan {
    pub generated_at: u64,
    pub replication_factor: usize,
    pub records: Vec<DhtRecordReplicationPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhtRoutingRefreshPlan {
    pub generated_at: u64,
    pub targets: Vec<KademliaNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DhtRpcRequest {
    FindNode {
        request_id: String,
        target: KademliaNodeId,
        limit: usize,
    },
    FindValue {
        request_id: String,
        key: DhtRecordKey,
        limit: usize,
    },
    StoreRecord {
        request_id: String,
        record: DhtRecord,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DhtRpcResponse {
    Nodes {
        request_id: String,
        nodes: Vec<RoutingPeer>,
    },
    Value {
        request_id: String,
        record: Option<DhtRecord>,
        closer_records: Vec<DhtRecord>,
        closer_nodes: Vec<RoutingPeer>,
    },
    StoreResult {
        request_id: String,
        stored: bool,
        inserted: bool,
    },
    Error {
        request_id: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingPeer {
    pub node_id: KademliaNodeId,
    pub announce: PublicPeerAnnounce,
    #[serde(default)]
    pub identity_public_key: Option<String>,
    pub last_seen_at: u64,
}

#[derive(Debug, Clone)]
pub struct KademliaRoutingTable {
    local_id: KademliaNodeId,
    bucket_size: usize,
    buckets: Vec<Vec<RoutingPeer>>,
}

impl KademliaRoutingTable {
    pub fn new(local_id: KademliaNodeId, bucket_size: usize) -> Self {
        Self {
            local_id,
            bucket_size: bucket_size.max(1),
            buckets: vec![Vec::new(); KADEMLIA_ID_BYTES * 8],
        }
    }

    pub fn local_id(&self) -> KademliaNodeId {
        self.local_id
    }

    pub fn insert_verified(
        &mut self,
        announce: PublicPeerAnnounce,
        identity_public_key: &[u8; 32],
    ) -> Result<()> {
        announce.verify(identity_public_key)?;
        let node_id = KademliaNodeId::from_peer_id(&announce.peer_id);
        if node_id == self.local_id {
            return Ok(());
        }
        let Some(bucket_index) = self.local_id.bucket_index(&node_id) else {
            return Ok(());
        };
        let bucket = &mut self.buckets[bucket_index];
        if let Some(existing) = bucket.iter_mut().find(|p| p.node_id == node_id) {
            existing.announce = announce;
            existing.identity_public_key = Some(BASE64.encode(identity_public_key));
            existing.last_seen_at = current_unix_timestamp();
            return Ok(());
        }
        if bucket.len() >= self.bucket_size {
            bucket.remove(0);
        }
        bucket.push(RoutingPeer {
            node_id,
            announce,
            identity_public_key: Some(BASE64.encode(identity_public_key)),
            last_seen_at: current_unix_timestamp(),
        });
        Ok(())
    }

    fn insert_local_snapshot(&mut self, announce: PublicPeerAnnounce) {
        let node_id = KademliaNodeId::from_peer_id(&announce.peer_id);
        if node_id == self.local_id {
            return;
        }
        let Some(bucket_index) = self.local_id.bucket_index(&node_id) else {
            return;
        };
        let bucket = &mut self.buckets[bucket_index];
        if let Some(existing) = bucket.iter_mut().find(|p| p.node_id == node_id) {
            existing.announce = announce;
            existing.identity_public_key = None;
            existing.last_seen_at = current_unix_timestamp();
            return;
        }
        if bucket.len() >= self.bucket_size {
            bucket.remove(0);
        }
        bucket.push(RoutingPeer {
            node_id,
            announce,
            identity_public_key: None,
            last_seen_at: current_unix_timestamp(),
        });
    }

    pub fn all_peers(&self) -> Vec<RoutingPeer> {
        self.buckets
            .iter()
            .flat_map(|bucket| bucket.iter().cloned())
            .collect()
    }

    pub fn closest(&self, target: KademliaNodeId, limit: usize) -> Vec<RoutingPeer> {
        let mut peers: Vec<_> = self
            .buckets
            .iter()
            .flat_map(|bucket| bucket.iter().cloned())
            .collect();
        peers.sort_by_key(|peer| peer.node_id.xor_distance(&target));
        peers.truncate(limit);
        peers
    }

    pub fn refresh_targets(&self) -> Vec<KademliaNodeId> {
        (0..KADEMLIA_ID_BYTES * 8)
            .map(|bucket_index| self.refresh_target_for_bucket(bucket_index))
            .collect()
    }

    pub fn refresh_target_for_bucket(&self, bucket_index: usize) -> KademliaNodeId {
        let mut target = *self.local_id.as_bytes();
        let index = bucket_index.min(KADEMLIA_ID_BYTES * 8 - 1);
        let byte_index = index / 8;
        let bit_index = index % 8;
        target[byte_index] ^= 0x80 >> bit_index;
        KademliaNodeId::from_bytes(target)
    }

    pub fn bucket_len(&self, bucket_index: usize) -> usize {
        self.buckets.get(bucket_index).map(Vec::len).unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.buckets.iter().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn decode_hex_32(value: &str) -> Result<[u8; KADEMLIA_ID_BYTES]> {
    if value.len() != KADEMLIA_ID_BYTES * 2 {
        return Err(LmError::InvalidBackupFormat);
    }
    let mut out = [0u8; KADEMLIA_ID_BYTES];
    let bytes = value.as_bytes();
    for idx in 0..KADEMLIA_ID_BYTES {
        let hi = from_hex(bytes[idx * 2]).ok_or(LmError::InvalidBackupFormat)?;
        let lo = from_hex(bytes[idx * 2 + 1]).ok_or(LmError::InvalidBackupFormat)?;
        out[idx] = (hi << 4) | lo;
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfig {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub capabilities: Vec<PublicPeerCapability>,
    pub max_mailbox_bytes: Option<u64>,
    pub max_message_ttl_seconds: Option<u64>,
    #[serde(default = "default_max_mailbox_messages_per_user")]
    pub max_mailbox_messages_per_user: Option<usize>,
    #[serde(default)]
    pub mailbox_sender_rate_limit_window_seconds: Option<u64>,
    #[serde(default)]
    pub mailbox_sender_rate_limit_max_messages: Option<u32>,
    #[serde(default)]
    pub mailbox_global_rate_limit_window_seconds: Option<u64>,
    #[serde(default)]
    pub mailbox_global_rate_limit_max_messages: Option<u32>,
    pub max_relay_bandwidth_kbps: Option<u64>,
    pub announce_ttl_seconds: u64,
}

fn default_max_mailbox_messages_per_user() -> Option<usize> {
    Some(1000)
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            peer_id: "lm-node-dev".to_string(),
            addresses: vec!["/ip4/0.0.0.0/tcp/0".to_string()],
            capabilities: vec![PublicPeerCapability::Bootstrap, PublicPeerCapability::Dht],
            max_mailbox_bytes: Some(10 * 1024 * 1024),
            max_message_ttl_seconds: Some(24 * 3600),
            max_mailbox_messages_per_user: default_max_mailbox_messages_per_user(),
            mailbox_sender_rate_limit_window_seconds: None,
            mailbox_sender_rate_limit_max_messages: None,
            mailbox_global_rate_limit_window_seconds: None,
            mailbox_global_rate_limit_max_messages: None,
            max_relay_bandwidth_kbps: Some(1024),
            announce_ttl_seconds: 24 * 3600,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailboxRateLimitConfig {
    pub window_seconds: u64,
    pub max_messages: u32,
}

impl NodeConfig {
    pub fn create_announce(&self, identity: &Identity) -> Result<PublicPeerAnnounce> {
        PublicPeerAnnounce::new(
            identity,
            self.peer_id.clone(),
            None,
            self.addresses.clone(),
            self.capabilities.clone(),
            self.max_mailbox_bytes,
            self.max_message_ttl_seconds,
            self.max_relay_bandwidth_kbps,
            self.announce_ttl_seconds,
        )
    }

    pub fn mailbox_sender_rate_limit(&self) -> Option<MailboxRateLimitConfig> {
        let window_seconds = self.mailbox_sender_rate_limit_window_seconds?;
        let max_messages = self.mailbox_sender_rate_limit_max_messages?;
        if window_seconds == 0 || max_messages == 0 {
            return None;
        }
        Some(MailboxRateLimitConfig {
            window_seconds,
            max_messages,
        })
    }

    pub fn mailbox_global_rate_limit(&self) -> Option<MailboxRateLimitConfig> {
        let window_seconds = self.mailbox_global_rate_limit_window_seconds?;
        let max_messages = self.mailbox_global_rate_limit_max_messages?;
        if window_seconds == 0 || max_messages == 0 {
            return None;
        }
        Some(MailboxRateLimitConfig {
            window_seconds,
            max_messages,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct RoutingTable {
    peers: HashMap<String, PublicPeerAnnounce>,
}

impl RoutingTable {
    pub fn insert_verified(
        &mut self,
        announce: PublicPeerAnnounce,
        identity_public_key: &[u8; 32],
    ) -> Result<()> {
        announce.verify(identity_public_key)?;
        self.peers.insert(announce.peer_id.clone(), announce);
        Ok(())
    }

    fn insert_trusted_announce(&mut self, announce: PublicPeerAnnounce) {
        self.peers.insert(announce.peer_id.clone(), announce);
    }

    pub fn get(&self, peer_id: &str) -> Option<&PublicPeerAnnounce> {
        self.peers.get(peer_id)
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxDelivery {
    pub delivery_id: String,
    pub message: MailboxMessage,
    pub created_at: u64,
    pub delivered_at: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct MailboxStore {
    deliveries: HashMap<UserId, Vec<MailboxDelivery>>,
    message_ids: HashMap<UserId, Vec<Uuid>>,
}

impl MailboxStore {
    pub fn push_verified(
        &mut self,
        message: MailboxMessage,
        from_identity_public_key: &[u8; 32],
    ) -> Result<String> {
        self.push_verified_with_limits(message, from_identity_public_key, None, None, None)
    }

    pub fn push_verified_with_limits(
        &mut self,
        message: MailboxMessage,
        from_identity_public_key: &[u8; 32],
        max_total_bytes: Option<u64>,
        max_messages_per_user: Option<usize>,
        max_message_ttl_seconds: Option<u64>,
    ) -> Result<String> {
        message.verify(from_identity_public_key)?;
        let now = current_unix_timestamp();
        if message.expires_at <= now {
            return Err(LmError::ExpiredObject);
        }
        if let Some(max_ttl) = max_message_ttl_seconds {
            if message.expires_at.saturating_sub(now) > max_ttl {
                return Err(LmError::PayloadTooLarge);
            }
        }
        self.prune_expired(now);
        if self.has_message_id(&message.to_user_id, message.message_id) {
            return Err(LmError::DuplicateMessage);
        }
        let message_bytes = mailbox_delivery_size_bytes(&message);
        if let Some(max_total_bytes) = max_total_bytes {
            if self.total_bytes().saturating_add(message_bytes) > max_total_bytes as usize {
                return Err(LmError::PayloadTooLarge);
            }
        }
        if let Some(max_messages) = max_messages_per_user {
            if self.pending_for(&message.to_user_id) >= max_messages {
                return Err(LmError::PayloadTooLarge);
            }
        }
        let delivery_id = Uuid::new_v4().to_string();
        self.message_ids
            .entry(message.to_user_id.clone())
            .or_default()
            .push(message.message_id);
        self.deliveries
            .entry(message.to_user_id.clone())
            .or_default()
            .push(MailboxDelivery {
                delivery_id: delivery_id.clone(),
                message,
                created_at: now,
                delivered_at: None,
            });
        Ok(delivery_id)
    }

    /// Return pending deliveries without deleting them. Clients must call ack
    /// after successful local processing to avoid message loss.
    pub fn take_for(&mut self, user_id: &UserId) -> Vec<MailboxDelivery> {
        let now = current_unix_timestamp();
        self.prune_expired(now);
        let Some(deliveries) = self.deliveries.get_mut(user_id) else {
            return Vec::new();
        };
        for delivery in deliveries.iter_mut() {
            delivery.delivered_at = Some(now);
        }
        deliveries.clone()
    }

    pub fn ack(&mut self, user_id: &UserId, delivery_ids: &[String]) -> usize {
        let Some(deliveries) = self.deliveries.get_mut(user_id) else {
            return 0;
        };
        let before = deliveries.len();
        deliveries.retain(|delivery| !delivery_ids.contains(&delivery.delivery_id));
        let removed = before.saturating_sub(deliveries.len());
        if deliveries.is_empty() {
            self.deliveries.remove(user_id);
            self.message_ids.remove(user_id);
        } else {
            self.rebuild_message_ids_for(user_id);
        }
        removed
    }

    pub fn pending_for(&self, user_id: &UserId) -> usize {
        self.deliveries.get(user_id).map(Vec::len).unwrap_or(0)
    }

    pub fn total_pending(&self) -> usize {
        self.deliveries.values().map(Vec::len).sum()
    }

    pub fn total_bytes(&self) -> usize {
        self.deliveries
            .values()
            .flat_map(|deliveries| deliveries.iter())
            .map(|delivery| mailbox_delivery_size_bytes(&delivery.message))
            .sum()
    }

    pub fn prune_expired(&mut self, now: u64) -> usize {
        let mut removed = 0;
        let users: Vec<_> = self.deliveries.keys().cloned().collect();
        for user_id in users {
            let Some(deliveries) = self.deliveries.get_mut(&user_id) else {
                continue;
            };
            let before = deliveries.len();
            deliveries.retain(|delivery| delivery.message.expires_at > now);
            removed += before.saturating_sub(deliveries.len());
            if deliveries.is_empty() {
                self.deliveries.remove(&user_id);
                self.message_ids.remove(&user_id);
            } else {
                self.rebuild_message_ids_for(&user_id);
            }
        }
        removed
    }

    fn has_message_id(&self, user_id: &UserId, message_id: Uuid) -> bool {
        self.message_ids
            .get(user_id)
            .map(|ids| ids.contains(&message_id))
            .unwrap_or(false)
    }

    fn rebuild_message_ids_for(&mut self, user_id: &UserId) {
        let Some(deliveries) = self.deliveries.get(user_id) else {
            self.message_ids.remove(user_id);
            return;
        };
        self.message_ids.insert(
            user_id.clone(),
            deliveries
                .iter()
                .map(|delivery| delivery.message.message_id)
                .collect(),
        );
    }

    fn rebuild_message_ids(&mut self) {
        self.message_ids.clear();
        let users: Vec<_> = self.deliveries.keys().cloned().collect();
        for user_id in users {
            self.rebuild_message_ids_for(&user_id);
        }
    }

    pub fn all_deliveries(&self) -> Vec<MailboxDelivery> {
        self.deliveries
            .values()
            .flat_map(|deliveries| deliveries.iter().cloned())
            .collect()
    }

    pub fn all_messages(&self) -> Vec<MailboxMessage> {
        self.all_deliveries()
            .into_iter()
            .map(|delivery| delivery.message)
            .collect()
    }

    fn restore_deliveries(&mut self, deliveries: Vec<MailboxDelivery>) {
        self.deliveries.clear();
        for delivery in deliveries {
            self.deliveries
                .entry(delivery.message.to_user_id.clone())
                .or_default()
                .push(delivery);
        }
        self.prune_expired(current_unix_timestamp());
        self.rebuild_message_ids();
    }

    fn restore_messages(&mut self, messages: Vec<MailboxMessage>) {
        self.deliveries.clear();
        for message in messages {
            let delivery_id = Uuid::new_v4().to_string();
            self.deliveries
                .entry(message.to_user_id.clone())
                .or_default()
                .push(MailboxDelivery {
                    delivery_id,
                    message,
                    created_at: current_unix_timestamp(),
                    delivered_at: None,
                });
        }
        self.prune_expired(current_unix_timestamp());
        self.rebuild_message_ids();
    }

    fn merge_deliveries(&mut self, deliveries: Vec<MailboxDelivery>) -> usize {
        self.prune_expired(current_unix_timestamp());
        let mut inserted = 0;
        for delivery in deliveries {
            if delivery.message.expires_at <= current_unix_timestamp() {
                continue;
            }
            if self.has_message_id(&delivery.message.to_user_id, delivery.message.message_id) {
                continue;
            }
            let list = self
                .deliveries
                .entry(delivery.message.to_user_id.clone())
                .or_default();
            if list
                .iter()
                .any(|existing| existing.delivery_id == delivery.delivery_id)
            {
                continue;
            }
            self.message_ids
                .entry(delivery.message.to_user_id.clone())
                .or_default()
                .push(delivery.message.message_id);
            list.push(delivery);
            inserted += 1;
        }
        inserted
    }
}

fn mailbox_delivery_size_bytes(message: &MailboxMessage) -> usize {
    message.ciphertext.len()
        + message.signature.len()
        + message.from_user_id.as_str().len()
        + message.to_user_id.as_str().len()
        + std::mem::size_of::<MailboxMessage>()
}

#[derive(Debug, Clone)]
struct MailboxSenderRateLimitEntry {
    window_started_at: u64,
    count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct MailboxSenderRateLimiter {
    entries: HashMap<UserId, MailboxSenderRateLimitEntry>,
}

impl MailboxSenderRateLimiter {
    pub fn allows(
        &mut self,
        user_id: &UserId,
        now: u64,
        config: Option<MailboxRateLimitConfig>,
    ) -> bool {
        let Some(config) = config else {
            return true;
        };
        let entry = self
            .entries
            .entry(user_id.clone())
            .or_insert(MailboxSenderRateLimitEntry {
                window_started_at: now,
                count: 0,
            });
        if now.saturating_sub(entry.window_started_at) >= config.window_seconds {
            entry.window_started_at = now;
            entry.count = 0;
        }
        if entry.count >= config.max_messages {
            return false;
        }
        true
    }

    pub fn record(&mut self, user_id: &UserId, now: u64, config: Option<MailboxRateLimitConfig>) {
        let Some(config) = config else {
            return;
        };
        let entry = self
            .entries
            .entry(user_id.clone())
            .or_insert(MailboxSenderRateLimitEntry {
                window_started_at: now,
                count: 0,
            });
        if now.saturating_sub(entry.window_started_at) >= config.window_seconds {
            entry.window_started_at = now;
            entry.count = 0;
        }
        entry.count = entry.count.saturating_add(1);
    }

    pub fn check(
        &mut self,
        user_id: &UserId,
        now: u64,
        config: Option<MailboxRateLimitConfig>,
    ) -> bool {
        if !self.allows(user_id, now, config) {
            return false;
        }
        self.record(user_id, now, config);
        true
    }

    pub fn prune(&mut self, now: u64, config: Option<MailboxRateLimitConfig>) {
        let Some(config) = config else {
            self.entries.clear();
            return;
        };
        let ttl = config.window_seconds.saturating_mul(2).max(1);
        self.entries
            .retain(|_, entry| now.saturating_sub(entry.window_started_at) < ttl);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct MailboxGlobalRateLimiter {
    window_started_at: Option<u64>,
    count: u32,
}

impl MailboxGlobalRateLimiter {
    pub fn allows(&mut self, now: u64, config: Option<MailboxRateLimitConfig>) -> bool {
        let Some(config) = config else {
            return true;
        };
        self.rotate_if_needed(now, config.window_seconds);
        self.count < config.max_messages
    }

    pub fn record(&mut self, now: u64, config: Option<MailboxRateLimitConfig>) {
        let Some(config) = config else {
            return;
        };
        self.rotate_if_needed(now, config.window_seconds);
        self.count = self.count.saturating_add(1);
    }

    pub fn check(&mut self, now: u64, config: Option<MailboxRateLimitConfig>) -> bool {
        if !self.allows(now, config) {
            return false;
        }
        self.record(now, config);
        true
    }

    pub fn prune(&mut self, now: u64, config: Option<MailboxRateLimitConfig>) {
        let Some(config) = config else {
            self.window_started_at = None;
            self.count = 0;
            return;
        };
        if let Some(started_at) = self.window_started_at {
            let ttl = config.window_seconds.saturating_mul(2).max(1);
            if now.saturating_sub(started_at) >= ttl {
                self.window_started_at = None;
                self.count = 0;
            }
        }
    }

    fn rotate_if_needed(&mut self, now: u64, window_seconds: u64) {
        match self.window_started_at {
            Some(started_at) if now.saturating_sub(started_at) < window_seconds => {}
            _ => {
                self.window_started_at = Some(now);
                self.count = 0;
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PreKeyStore {
    bundles: HashMap<UserId, PreKeyBundle>,
    signed_one_time_prekey_records: HashMap<UserId, Vec<SignedOneTimePreKeyRecord>>,
    consumed_one_time_prekeys: HashMap<UserId, Vec<u32>>,
}

impl PreKeyStore {
    pub fn publish_verified(&mut self, bundle: PreKeyBundle) -> Result<()> {
        self.publish_verified_with_signed_one_time_prekey_records(bundle, Vec::new())
    }

    pub fn publish_verified_with_signed_one_time_prekey_records(
        &mut self,
        bundle: PreKeyBundle,
        signed_one_time_prekey_records: Vec<SignedOneTimePreKeyRecord>,
    ) -> Result<()> {
        bundle.verify()?;
        for record in &signed_one_time_prekey_records {
            record.verify_for_bundle(&bundle)?;
        }
        let user_id = bundle.user_id.clone();
        let reset_consumed = self
            .bundles
            .get(&user_id)
            .map(|existing| existing.signed_prekey_id != bundle.signed_prekey_id)
            .unwrap_or(true);
        self.bundles.insert(user_id.clone(), bundle);
        if reset_consumed {
            self.consumed_one_time_prekeys.remove(&user_id);
            self.signed_one_time_prekey_records.remove(&user_id);
        }
        self.merge_verified_signed_one_time_prekey_records_for(
            &user_id,
            signed_one_time_prekey_records,
        );
        self.prune_signed_one_time_prekey_records_for(&user_id);
        self.prune_consumed_for(&user_id);
        Ok(())
    }

    pub fn get_for(&mut self, user_id: &UserId) -> Option<PreKeyBundle> {
        self.prune_expired(current_unix_timestamp());
        self.bundles.get(user_id).cloned()
    }

    pub fn get_for_unchecked(&self, user_id: &UserId) -> Option<PreKeyBundle> {
        self.bundles.get(user_id).cloned()
    }

    pub fn take_for(&mut self, user_id: &UserId, consume: bool) -> Option<PreKeyBundle> {
        let (bundle, _, _) =
            self.take_for_with_selected_one_time_prekey_material(user_id, consume)?;
        Some(bundle)
    }

    pub fn take_for_with_selected_one_time_prekey(
        &mut self,
        user_id: &UserId,
        consume: bool,
    ) -> Option<(PreKeyBundle, Option<u32>)> {
        let (bundle, selected_id, _) =
            self.take_for_with_selected_one_time_prekey_material(user_id, consume)?;
        Some((bundle, selected_id))
    }

    pub fn take_for_with_selected_one_time_prekey_record(
        &mut self,
        user_id: &UserId,
        consume: bool,
    ) -> Option<(PreKeyBundle, Option<SignedOneTimePreKeyRecord>)> {
        let (bundle, _, selected_record) =
            self.take_for_with_selected_one_time_prekey_material(user_id, consume)?;
        Some((bundle, selected_record))
    }

    pub fn take_for_with_selected_one_time_prekey_material(
        &mut self,
        user_id: &UserId,
        consume: bool,
    ) -> Option<(PreKeyBundle, Option<u32>, Option<SignedOneTimePreKeyRecord>)> {
        self.prune_expired(current_unix_timestamp());
        let bundle = self.bundles.get(user_id).cloned()?;
        let consumed = self
            .consumed_one_time_prekeys
            .entry(user_id.clone())
            .or_default();
        let selected_record =
            self.signed_one_time_prekey_records
                .get(user_id)
                .and_then(|records| {
                    records
                        .iter()
                        .filter(|record| record.signed_prekey_id == bundle.signed_prekey_id)
                        .find(|record| !consumed.contains(&record.key_id))
                        .cloned()
                });
        let selected_id = selected_record
            .as_ref()
            .map(|record| record.key_id)
            .or_else(|| {
                if self
                    .signed_one_time_prekey_records
                    .get(user_id)
                    .map(|records| !records.is_empty())
                    .unwrap_or(false)
                {
                    None
                } else {
                    bundle
                        .one_time_prekeys
                        .iter()
                        .map(|key| key.key_id)
                        .find(|id| !consumed.contains(id))
                }
            });
        if consume {
            if let Some(id) = selected_id {
                consumed.push(id);
                consumed.sort_unstable();
                consumed.dedup();
            }
        }
        Some((bundle, selected_id, selected_record))
    }

    pub fn consumed_for(&self, user_id: &UserId) -> Vec<u32> {
        self.consumed_one_time_prekeys
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn signed_one_time_prekey_records_for(
        &self,
        user_id: &UserId,
    ) -> Vec<SignedOneTimePreKeyRecord> {
        self.signed_one_time_prekey_records
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn remaining_one_time_prekeys_for(&self, user_id: &UserId) -> Option<usize> {
        let bundle = self.bundles.get(user_id)?;
        let consumed = self.consumed_one_time_prekeys.get(user_id);
        if let Some(records) = self.signed_one_time_prekey_records.get(user_id) {
            let count = records
                .iter()
                .filter(|record| record.signed_prekey_id == bundle.signed_prekey_id)
                .filter(|record| {
                    !consumed
                        .map(|ids| ids.contains(&record.key_id))
                        .unwrap_or(false)
                })
                .count();
            return Some(count);
        }
        Some(
            bundle
                .one_time_prekeys
                .iter()
                .filter(|key| {
                    !consumed
                        .map(|ids| ids.contains(&key.key_id))
                        .unwrap_or(false)
                })
                .count(),
        )
    }

    pub fn published_one_time_prekeys_for(&self, user_id: &UserId) -> Option<usize> {
        let bundle = self.bundles.get(user_id)?;
        if let Some(records) = self.signed_one_time_prekey_records.get(user_id) {
            return Some(
                records
                    .iter()
                    .filter(|record| record.signed_prekey_id == bundle.signed_prekey_id)
                    .count(),
            );
        }
        Some(bundle.one_time_prekeys.len())
    }

    pub fn prune_expired(&mut self, now: u64) -> usize {
        let expired: Vec<_> = self
            .bundles
            .iter()
            .filter(|(_, bundle)| {
                bundle.expires_at <= now || bundle.signed_prekey_expires_at <= now
            })
            .map(|(user_id, _)| user_id.clone())
            .collect();
        let removed = expired.len();
        for user_id in expired {
            self.bundles.remove(&user_id);
            self.signed_one_time_prekey_records.remove(&user_id);
            self.consumed_one_time_prekeys.remove(&user_id);
        }
        let users: Vec<_> = self
            .signed_one_time_prekey_records
            .keys()
            .cloned()
            .collect();
        for user_id in users {
            if let Some(records) = self.signed_one_time_prekey_records.get_mut(&user_id) {
                records.retain(|record| record.expires_at > now);
                if records.is_empty() {
                    self.signed_one_time_prekey_records.remove(&user_id);
                }
            }
            self.prune_consumed_for(&user_id);
        }
        removed
    }

    fn merge_verified_signed_one_time_prekey_records_for(
        &mut self,
        user_id: &UserId,
        records: Vec<SignedOneTimePreKeyRecord>,
    ) -> usize {
        if records.is_empty() {
            return 0;
        }
        let list = self
            .signed_one_time_prekey_records
            .entry(user_id.clone())
            .or_default();
        let mut inserted = 0usize;
        for record in records {
            if let Some(existing) = list.iter_mut().find(|existing| {
                existing.signed_prekey_id == record.signed_prekey_id
                    && existing.key_id == record.key_id
            }) {
                *existing = record;
            } else {
                list.push(record);
                inserted = inserted.saturating_add(1);
            }
        }
        list.sort_by_key(|record| (record.signed_prekey_id, record.key_id));
        inserted
    }

    fn prune_signed_one_time_prekey_records_for(&mut self, user_id: &UserId) {
        let Some(bundle) = self.bundles.get(user_id) else {
            self.signed_one_time_prekey_records.remove(user_id);
            return;
        };
        let Some(records) = self.signed_one_time_prekey_records.get_mut(user_id) else {
            return;
        };
        records.retain(|record| record.verify_for_bundle(bundle).is_ok());
        records.sort_by_key(|record| (record.signed_prekey_id, record.key_id));
        records.dedup_by_key(|record| (record.signed_prekey_id, record.key_id));
        if records.is_empty() {
            self.signed_one_time_prekey_records.remove(user_id);
        }
    }

    fn valid_one_time_key_ids_for(&self, user_id: &UserId) -> Option<Vec<u32>> {
        let bundle = self.bundles.get(user_id)?;
        if let Some(records) = self.signed_one_time_prekey_records.get(user_id) {
            return Some(
                records
                    .iter()
                    .filter(|record| record.signed_prekey_id == bundle.signed_prekey_id)
                    .map(|record| record.key_id)
                    .collect(),
            );
        }
        Some(
            bundle
                .one_time_prekeys
                .iter()
                .map(|key| key.key_id)
                .collect(),
        )
    }

    fn prune_consumed_for(&mut self, user_id: &UserId) {
        let Some(valid_ids) = self.valid_one_time_key_ids_for(user_id) else {
            self.consumed_one_time_prekeys.remove(user_id);
            return;
        };
        if let Some(consumed) = self.consumed_one_time_prekeys.get_mut(user_id) {
            consumed.retain(|id| valid_ids.contains(id));
            consumed.sort_unstable();
            consumed.dedup();
            if consumed.is_empty() {
                self.consumed_one_time_prekeys.remove(user_id);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.bundles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bundles.is_empty()
    }

    pub fn all_bundles(&self) -> Vec<PreKeyBundle> {
        self.bundles.values().cloned().collect()
    }

    pub fn all_signed_one_time_prekey_records(&self) -> Vec<SignedOneTimePreKeyRecord> {
        self.signed_one_time_prekey_records
            .values()
            .flat_map(|records| records.iter().cloned())
            .collect()
    }

    fn restore_bundles(&mut self, bundles: Vec<PreKeyBundle>) {
        self.bundles.clear();
        self.signed_one_time_prekey_records.clear();
        for bundle in bundles {
            if bundle.verify().is_ok() {
                self.bundles.insert(bundle.user_id.clone(), bundle);
            }
        }
        self.prune_expired(current_unix_timestamp());
    }

    fn restore_signed_one_time_prekey_records(&mut self, records: Vec<SignedOneTimePreKeyRecord>) {
        self.signed_one_time_prekey_records.clear();
        self.merge_signed_one_time_prekey_records(records);
    }

    fn merge_bundles(&mut self, bundles: Vec<PreKeyBundle>) -> usize {
        let mut inserted = 0;
        for bundle in bundles {
            if bundle.verify().is_err() {
                continue;
            }
            let user_id = bundle.user_id.clone();
            let reset_consumed = self
                .bundles
                .get(&user_id)
                .map(|existing| existing.signed_prekey_id != bundle.signed_prekey_id)
                .unwrap_or(true);
            let is_new = !self.bundles.contains_key(&user_id);
            self.bundles.insert(user_id.clone(), bundle);
            if reset_consumed {
                self.consumed_one_time_prekeys.remove(&user_id);
                self.signed_one_time_prekey_records.remove(&user_id);
            }
            self.prune_signed_one_time_prekey_records_for(&user_id);
            self.prune_consumed_for(&user_id);
            if is_new {
                inserted += 1;
            }
        }
        inserted
    }

    fn merge_signed_one_time_prekey_records(
        &mut self,
        records: Vec<SignedOneTimePreKeyRecord>,
    ) -> usize {
        let mut grouped: HashMap<UserId, Vec<SignedOneTimePreKeyRecord>> = HashMap::new();
        for record in records {
            let Some(bundle) = self.bundles.get(&record.user_id) else {
                continue;
            };
            if record.verify_for_bundle(bundle).is_err() {
                continue;
            }
            grouped
                .entry(record.user_id.clone())
                .or_default()
                .push(record);
        }
        let mut inserted = 0usize;
        for (user_id, records) in grouped {
            inserted = inserted.saturating_add(
                self.merge_verified_signed_one_time_prekey_records_for(&user_id, records),
            );
            self.prune_consumed_for(&user_id);
        }
        inserted
    }

    fn restore_consumed(&mut self, consumed: Vec<ConsumedOneTimePreKey>) {
        self.consumed_one_time_prekeys.clear();
        self.merge_consumed(consumed);
    }

    fn merge_consumed(&mut self, consumed: Vec<ConsumedOneTimePreKey>) {
        for item in consumed {
            let user_id = item.user_id;
            let ids = self
                .consumed_one_time_prekeys
                .entry(user_id.clone())
                .or_default();
            ids.push(item.key_id);
            ids.sort_unstable();
            ids.dedup();
            self.prune_consumed_for(&user_id);
        }
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMaintenanceStats {
    pub prune_runs: u64,
    pub mailbox_expired_deliveries: u64,
    pub prekey_expired_bundles: u64,
    #[serde(default)]
    pub mailbox_push_rejects: MailboxPushRejectStats,
    pub last_pruned_at: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxPushRejectStats {
    pub invalid_json: u64,
    pub invalid_message_format: u64,
    pub invalid_identity_public_key: u64,
    pub invalid_signature: u64,
    pub expired_object: u64,
    pub duplicate_message: u64,
    pub payload_too_large: u64,
    pub global_rate_limited: u64,
    pub sender_rate_limited: u64,
    pub other: u64,
}

impl MailboxPushRejectStats {
    pub fn total(&self) -> u64 {
        self.invalid_json
            .saturating_add(self.invalid_message_format)
            .saturating_add(self.invalid_identity_public_key)
            .saturating_add(self.invalid_signature)
            .saturating_add(self.expired_object)
            .saturating_add(self.duplicate_message)
            .saturating_add(self.payload_too_large)
            .saturating_add(self.global_rate_limited)
            .saturating_add(self.sender_rate_limited)
            .saturating_add(self.other)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MailboxPushRejectReason {
    InvalidJson,
    InvalidMessageFormat,
    InvalidIdentityPublicKey,
    InvalidSignature,
    ExpiredObject,
    DuplicateMessage,
    PayloadTooLarge,
    GlobalRateLimited,
    SenderRateLimited,
    Other,
}

impl From<LmError> for MailboxPushRejectReason {
    fn from(value: LmError) -> Self {
        match value {
            LmError::InvalidSignature | LmError::InvalidUserId => Self::InvalidSignature,
            LmError::ExpiredObject => Self::ExpiredObject,
            LmError::DuplicateMessage => Self::DuplicateMessage,
            LmError::PayloadTooLarge => Self::PayloadTooLarge,
            _ => Self::Other,
        }
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
    pub mailbox_deliveries: Vec<MailboxDelivery>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumedOneTimePreKey {
    pub user_id: UserId,
    pub key_id: u32,
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
            if peer.announce.expires_at <= now {
                continue;
            }
            let expected_node_id = KademliaNodeId::from_peer_id(&peer.announce.peer_id);
            if peer.node_id != expected_node_id || expected_node_id == self.kademlia.local_id() {
                continue;
            }
            if let Some(identity_public_key) = &peer.identity_public_key {
                let Ok(public_key) = decode_identity_public_key_base64(identity_public_key) else {
                    continue;
                };
                if peer.announce.verify(&public_key).is_err() {
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
            if peer.announce.expires_at <= now {
                continue;
            }
            let expected_node_id = KademliaNodeId::from_peer_id(&peer.announce.peer_id);
            if peer.node_id != expected_node_id || expected_node_id == self.kademlia.local_id() {
                continue;
            }
            let Some(identity_public_key) = &peer.identity_public_key else {
                continue;
            };
            let Ok(public_key) = decode_identity_public_key_base64(identity_public_key) else {
                continue;
            };
            if self
                .routing_table
                .insert_verified(peer.announce.clone(), &public_key)
                .is_err()
            {
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
                let expired = record.is_expired_at(current_unix_timestamp());
                let inserted = if expired {
                    false
                } else {
                    self.dht_records.store(record)
                };
                DhtRpcResponse::StoreResult {
                    request_id,
                    stored: !expired,
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
            mailbox_deliveries: self.mailbox.all_deliveries(),
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
    /// Snapshot restore does not re-verify peer signatures because the snapshot
    /// currently stores public announcements but not the external identity public
    /// keys used to validate them. Network/API insertions are still verified at
    /// ingress before they can be snapshotted.
    pub fn from_state_snapshot(snapshot: NodeStateSnapshot) -> Self {
        let mut node = Self::new(snapshot.config);
        for announce in snapshot.public_peers {
            node.routing_table
                .peers
                .insert(announce.peer_id.clone(), announce.clone());
            node.kademlia.insert_local_snapshot(announce);
        }
        if snapshot.mailbox_deliveries.is_empty() {
            node.mailbox.restore_messages(snapshot.mailbox_messages);
        } else {
            node.mailbox.restore_deliveries(snapshot.mailbox_deliveries);
        }
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
        for announce in snapshot.public_peers {
            if self.routing_table.get(&announce.peer_id).is_none() {
                peers += 1;
            }
            self.routing_table
                .peers
                .insert(announce.peer_id.clone(), announce.clone());
            self.kademlia.insert_local_snapshot(announce);
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
        let prekey_bundles = self.prekeys.merge_bundles(snapshot.prekey_bundles);
        let signed_one_time_prekey_records = self
            .prekeys
            .merge_signed_one_time_prekey_records(snapshot.signed_one_time_prekey_records);
        self.prekeys
            .merge_consumed(snapshot.consumed_one_time_prekeys);
        let dht_records = self.dht_records.merge_records(snapshot.dht_records);
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
        _ => Err(LmError::InvalidBackupFormat),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRequest {
    pub method: String,
    pub path: String,
    pub body: String,
    pub headers: Vec<(String, String)>,
}

impl ControlRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(&name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlResponse {
    pub status: u16,
    pub content_type: String,
    pub body: String,
}

impl ControlResponse {
    pub fn json<T: Serialize>(status: u16, value: &T) -> Self {
        match serde_json::to_string(value) {
            Ok(body) => Self {
                status,
                content_type: "application/json".to_string(),
                body,
            },
            Err(err) => Self::text(500, format!("serialization error: {err}")),
        }
    }

    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: body.into(),
        }
    }

    pub fn to_http_string(&self) -> String {
        let reason = match self.status {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            500 => "Internal Server Error",
            _ => "OK",
        };
        format!(
            "HTTP/1.1 {} {}\r\ncontent-type: {}\r\naccess-control-allow-origin: *\r\naccess-control-allow-methods: GET,POST,OPTIONS\r\naccess-control-allow-headers: content-type\r\naccess-control-allow-private-network: true\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            self.status,
            reason,
            self.content_type,
            self.body.len(),
            self.body
        )
    }
}

#[derive(Debug, Deserialize)]
struct InsertPeerRequest {
    announce_text: String,
    identity_public_key: String,
}

#[derive(Debug, Deserialize)]
struct PushMailboxRequest {
    message_text: String,
    from_identity_public_key: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
    peer_id: &'a str,
    node_id: String,
    peers: usize,
    prekeys: usize,
    mailbox_deliveries: usize,
    mailbox_bytes: usize,
    dht_records: usize,
    maintenance: NodeMaintenanceStats,
    sync: NodeSyncStatus,
}

#[derive(Debug, Serialize)]
struct InsertPeerResponse {
    inserted: bool,
    peer_id: String,
    node_id: String,
    peers: usize,
}

#[derive(Debug, Serialize)]
struct ClosestPeersResponse {
    target: String,
    peers: Vec<PublicPeerAnnounce>,
}

#[derive(Debug, Serialize)]
struct MailboxPushResponse {
    stored: bool,
    delivery_id: String,
    to_user_id: String,
    pending: usize,
}

#[derive(Debug, Serialize)]
struct MailboxTakeResponse {
    user_id: String,
    messages: Vec<MailboxDelivery>,
}

#[derive(Debug, Deserialize)]
struct MailboxAckRequest {
    user_id: String,
    delivery_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct MailboxAckResponse {
    user_id: String,
    removed: usize,
    pending: usize,
}

#[derive(Debug, Deserialize)]
struct PublishPreKeyRequest {
    prekey_bundle_text: String,
    #[serde(default)]
    signed_one_time_prekey_record_texts: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PublishPreKeyResponse {
    stored: bool,
    user_id: String,
    prekey_bundles: usize,
    one_time_prekeys: usize,
    signed_one_time_prekey_records: usize,
    remaining_one_time_prekeys: usize,
    low_one_time_prekeys: bool,
    replenishment_required: bool,
    replenishment_actor: &'static str,
    node_generates_user_keys: bool,
}

#[derive(Debug, Serialize)]
struct GetPreKeyResponse {
    user_id: String,
    found: bool,
    prekey_bundle_text: Option<String>,
    selected_one_time_prekey_id: Option<u32>,
    selected_signed_one_time_prekey_record_text: Option<String>,
    consumed_one_time_prekey_ids: Vec<u32>,
    remaining_one_time_prekeys: Option<usize>,
    signed_one_time_prekey_records: Option<usize>,
    low_one_time_prekeys: bool,
    replenishment_required: bool,
    replenishment_actor: &'static str,
    node_generates_user_keys: bool,
}

#[derive(Debug, Serialize)]
struct PreKeyStatusResponse {
    user_id: String,
    found: bool,
    consumed_one_time_prekey_ids: Vec<u32>,
    remaining_one_time_prekeys: Option<usize>,
    signed_one_time_prekey_records: Option<usize>,
    low_one_time_prekeys: bool,
    replenishment_required: bool,
    replenishment_actor: &'static str,
    node_generates_user_keys: bool,
}

fn prekey_low_one_time_prekeys(remaining: Option<usize>) -> bool {
    remaining.map(|value| value <= 1).unwrap_or(false)
}

fn prekey_replenishment_required(remaining: Option<usize>) -> bool {
    remaining.map(|value| value <= 1).unwrap_or(true)
}

#[derive(Debug, Deserialize)]
struct StoreDhtRecordRequest {
    record: DhtRecord,
}

#[derive(Debug, Deserialize)]
struct DhtRpcControlRequest {
    request: DhtRpcRequest,
}

#[derive(Debug, Serialize)]
struct StoreDhtRecordResponse {
    stored: bool,
    inserted: bool,
    key: String,
    records: usize,
}

#[derive(Debug, Serialize)]
struct GetDhtRecordResponse {
    key: String,
    found: bool,
    record: Option<DhtRecord>,
}

#[derive(Debug, Serialize)]
struct ClosestDhtRecordsResponse {
    target: String,
    records: Vec<DhtRecord>,
}

#[derive(Debug, Deserialize)]
struct ImportSnapshotRequest {
    snapshot: NodeStateSnapshot,
}

#[derive(Debug, Serialize)]
struct ImportSnapshotResponse {
    imported: bool,
    peers: usize,
    mailbox_deliveries: usize,
    prekey_bundles: usize,
    signed_one_time_prekey_records: usize,
    dht_records: usize,
}

impl NativeNode {
    pub fn handle_control_request(&mut self, request: ControlRequest) -> ControlResponse {
        self.prune_expired_records();
        match (request.method.as_str(), path_without_query(&request.path)) {
            ("OPTIONS", _) => ControlResponse::text(200, ""),
            ("GET", "/health") => ControlResponse::json(
                200,
                &HealthResponse {
                    status: "ok",
                    peer_id: &self.config.peer_id,
                    node_id: self.kademlia.local_id().to_hex(),
                    peers: self.kademlia.len(),
                    prekeys: self.prekeys.len(),
                    mailbox_deliveries: self.mailbox.total_pending(),
                    mailbox_bytes: self.mailbox.total_bytes(),
                    dht_records: self.dht_records.len(),
                    maintenance: self.maintenance.clone(),
                    sync: self.sync_status.clone(),
                },
            ),
            ("POST", "/announce") => self.handle_control_announce(&request.body),
            ("GET", "/peers/closest") => self.handle_control_closest(&request.path),
            ("POST", "/mailbox/push") => self.handle_control_mailbox_push(&request.body),
            ("GET", "/mailbox/take") => self.handle_control_mailbox_take(&request.path),
            ("POST", "/mailbox/ack") => self.handle_control_mailbox_ack(&request.body),
            ("POST", "/prekey/publish") => self.handle_control_prekey_publish(&request.body),
            ("GET", "/prekey/get") => self.handle_control_prekey_get(&request.path),
            ("GET", "/prekey/status") => self.handle_control_prekey_status(&request.path),
            ("POST", "/dht/record") => self.handle_control_dht_record_store(&request.body),
            ("GET", "/dht/record") => self.handle_control_dht_record_get(&request.path),
            ("GET", "/dht/closest") => self.handle_control_dht_closest(&request.path),
            ("POST", "/dht/rpc") => self.handle_control_dht_rpc(&request.body),
            ("GET", "/dht/replication-plan") => {
                self.handle_control_dht_replication_plan(&request.path)
            }
            ("GET", "/dht/routing-refresh-plan") => self.handle_control_dht_routing_refresh_plan(),
            ("GET", "/sync/snapshot") => ControlResponse::json(200, &self.to_state_snapshot()),
            ("GET", "/sync/status") => ControlResponse::json(200, &self.sync_status),
            ("POST", "/sync/import") => self.handle_control_sync_import(&request.body),
            (
                _,
                "/health"
                | "/announce"
                | "/peers/closest"
                | "/mailbox/push"
                | "/mailbox/take"
                | "/mailbox/ack"
                | "/prekey/publish"
                | "/prekey/get"
                | "/prekey/status"
                | "/dht/record"
                | "/dht/closest"
                | "/dht/rpc"
                | "/dht/replication-plan"
                | "/dht/routing-refresh-plan"
                | "/sync/snapshot"
                | "/sync/status"
                | "/sync/import",
            ) => ControlResponse::text(405, "method not allowed"),
            _ => ControlResponse::text(404, "not found"),
        }
    }

    fn handle_control_announce(&mut self, body: &str) -> ControlResponse {
        let req: InsertPeerRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let announce = match PublicPeerAnnounce::from_export_text(req.announce_text.trim()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let public_key = match decode_identity_public_key_base64(&req.identity_public_key) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        if let Err(e) = self
            .routing_table
            .insert_verified(announce.clone(), &public_key)
        {
            return ControlResponse::text(400, e.to_string());
        }
        if let Err(e) = self.kademlia.insert_verified(announce.clone(), &public_key) {
            return ControlResponse::text(400, e.to_string());
        }
        ControlResponse::json(
            201,
            &InsertPeerResponse {
                inserted: true,
                peer_id: announce.peer_id.clone(),
                node_id: KademliaNodeId::from_peer_id(&announce.peer_id).to_hex(),
                peers: self.kademlia.len(),
            },
        )
    }

    fn handle_control_closest(&self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(target) = query.get("target") else {
            return ControlResponse::text(400, "missing target");
        };
        let limit = query
            .get("limit")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8)
            .clamp(1, 64);
        let peers = self
            .kademlia
            .closest(KademliaNodeId::from_peer_id(target), limit)
            .into_iter()
            .map(|p| p.announce)
            .collect();
        ControlResponse::json(
            200,
            &ClosestPeersResponse {
                target: target.to_string(),
                peers,
            },
        )
    }

    fn handle_control_mailbox_push(&mut self, body: &str) -> ControlResponse {
        let req: PushMailboxRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                self.record_mailbox_push_reject(MailboxPushRejectReason::InvalidJson);
                return ControlResponse::text(400, format!("invalid json: {e}"));
            }
        };
        let message = match MailboxMessage::from_export_text(req.message_text.trim()) {
            Ok(v) => v,
            Err(e) => {
                self.record_mailbox_push_reject(MailboxPushRejectReason::InvalidMessageFormat);
                return ControlResponse::text(400, e.to_string());
            }
        };
        let public_key = match decode_identity_public_key_base64(&req.from_identity_public_key) {
            Ok(v) => v,
            Err(e) => {
                self.record_mailbox_push_reject(MailboxPushRejectReason::InvalidIdentityPublicKey);
                return ControlResponse::text(400, e.to_string());
            }
        };
        if let Err(e) = message.verify(&public_key) {
            self.record_mailbox_push_reject(MailboxPushRejectReason::from(e.clone()));
            return ControlResponse::text(400, e.to_string());
        }
        let now = current_unix_timestamp();
        let global_rate_limit = self.config.mailbox_global_rate_limit();
        let sender_rate_limit = self.config.mailbox_sender_rate_limit();
        if !self
            .mailbox_global_rate_limiter
            .allows(now, global_rate_limit)
        {
            self.record_mailbox_push_reject(MailboxPushRejectReason::GlobalRateLimited);
            return ControlResponse::text(429, "mailbox global rate limit exceeded");
        }
        if !self
            .mailbox_sender_rate_limiter
            .allows(&message.from_user_id, now, sender_rate_limit)
        {
            self.record_mailbox_push_reject(MailboxPushRejectReason::SenderRateLimited);
            return ControlResponse::text(429, "mailbox sender rate limit exceeded");
        }
        let delivery_id = match self.mailbox.push_verified_with_limits(
            message.clone(),
            &public_key,
            self.config.max_mailbox_bytes,
            self.config.max_mailbox_messages_per_user,
            self.config.max_message_ttl_seconds,
        ) {
            Ok(delivery_id) => delivery_id,
            Err(e) => {
                self.record_mailbox_push_reject(MailboxPushRejectReason::from(e.clone()));
                return ControlResponse::text(400, e.to_string());
            }
        };
        self.mailbox_global_rate_limiter
            .record(now, global_rate_limit);
        self.mailbox_sender_rate_limiter
            .record(&message.from_user_id, now, sender_rate_limit);
        ControlResponse::json(
            201,
            &MailboxPushResponse {
                stored: true,
                delivery_id,
                to_user_id: message.to_user_id.to_string(),
                pending: self.mailbox.pending_for(&message.to_user_id),
            },
        )
    }

    fn handle_control_mailbox_take(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(user_id) = query.get("user_id") else {
            return ControlResponse::text(400, "missing user_id");
        };
        let user_id = match UserId::from_raw(user_id.to_string()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let messages = self.mailbox.take_for(&user_id);
        ControlResponse::json(
            200,
            &MailboxTakeResponse {
                user_id: user_id.to_string(),
                messages,
            },
        )
    }

    fn handle_control_mailbox_ack(&mut self, body: &str) -> ControlResponse {
        let req: MailboxAckRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let user_id = match UserId::from_raw(req.user_id) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let removed = self.mailbox.ack(&user_id, &req.delivery_ids);
        ControlResponse::json(
            200,
            &MailboxAckResponse {
                user_id: user_id.to_string(),
                removed,
                pending: self.mailbox.pending_for(&user_id),
            },
        )
    }

    fn handle_control_prekey_publish(&mut self, body: &str) -> ControlResponse {
        let req: PublishPreKeyRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let bundle = match PreKeyBundle::from_export_text(req.prekey_bundle_text.trim()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let signed_one_time_prekey_records = match req
            .signed_one_time_prekey_record_texts
            .iter()
            .map(|text| SignedOneTimePreKeyRecord::from_export_text(text.trim()))
            .collect::<Result<Vec<_>>>()
        {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let user_id = bundle.user_id.clone();
        let one_time_prekeys = if signed_one_time_prekey_records.is_empty() {
            bundle.one_time_prekeys.len()
        } else {
            signed_one_time_prekey_records.len()
        };
        if let Err(e) = self
            .prekeys
            .publish_verified_with_signed_one_time_prekey_records(
                bundle,
                signed_one_time_prekey_records,
            )
        {
            return ControlResponse::text(400, e.to_string());
        }
        let signed_one_time_prekey_records = self
            .prekeys
            .signed_one_time_prekey_records_for(&user_id)
            .len();
        let remaining = self
            .prekeys
            .remaining_one_time_prekeys_for(&user_id)
            .unwrap_or(0);
        let remaining_status = Some(remaining);
        let low_one_time_prekeys = prekey_low_one_time_prekeys(remaining_status);
        let replenishment_required = prekey_replenishment_required(remaining_status);
        ControlResponse::json(
            201,
            &PublishPreKeyResponse {
                stored: true,
                user_id: user_id.to_string(),
                prekey_bundles: self.prekeys.len(),
                one_time_prekeys,
                signed_one_time_prekey_records,
                remaining_one_time_prekeys: remaining,
                low_one_time_prekeys,
                replenishment_required,
                replenishment_actor: "client",
                node_generates_user_keys: false,
            },
        )
    }

    fn handle_control_prekey_get(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(user_id) = query.get("user_id") else {
            return ControlResponse::text(400, "missing user_id");
        };
        let user_id = match UserId::from_raw(user_id.to_string()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let consume = query
            .get("consume")
            .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        let selected = self
            .prekeys
            .take_for_with_selected_one_time_prekey_material(&user_id, consume);
        let (bundle_text, selected_one_time_prekey_id, selected_record_text) = match selected {
            Some((bundle, selected_id, selected_record)) => {
                let bundle_text = match bundle.to_export_text() {
                    Ok(text) => Some(text),
                    Err(e) => return ControlResponse::text(400, e.to_string()),
                };
                let selected_record_text = match selected_record {
                    Some(record) => match record.to_export_text() {
                        Ok(text) => Some(text),
                        Err(e) => return ControlResponse::text(400, e.to_string()),
                    },
                    None => None,
                };
                (bundle_text, selected_id, selected_record_text)
            }
            None => (None, None, None),
        };
        let remaining = self.prekeys.remaining_one_time_prekeys_for(&user_id);
        let signed_one_time_prekey_records = self.prekeys.published_one_time_prekeys_for(&user_id);
        let low_one_time_prekeys = prekey_low_one_time_prekeys(remaining);
        let replenishment_required = prekey_replenishment_required(remaining);
        ControlResponse::json(
            200,
            &GetPreKeyResponse {
                user_id: user_id.to_string(),
                found: bundle_text.is_some(),
                prekey_bundle_text: bundle_text,
                selected_one_time_prekey_id,
                selected_signed_one_time_prekey_record_text: selected_record_text,
                consumed_one_time_prekey_ids: self.prekeys.consumed_for(&user_id),
                remaining_one_time_prekeys: remaining,
                signed_one_time_prekey_records,
                low_one_time_prekeys,
                replenishment_required,
                replenishment_actor: "client",
                node_generates_user_keys: false,
            },
        )
    }

    fn handle_control_prekey_status(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(user_id) = query.get("user_id") else {
            return ControlResponse::text(400, "missing user_id");
        };
        let user_id = match UserId::from_raw(user_id.to_string()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        self.prekeys.prune_expired(current_unix_timestamp());
        let remaining = self.prekeys.remaining_one_time_prekeys_for(&user_id);
        let signed_one_time_prekey_records = self.prekeys.published_one_time_prekeys_for(&user_id);
        let low_one_time_prekeys = prekey_low_one_time_prekeys(remaining);
        let replenishment_required = prekey_replenishment_required(remaining);
        ControlResponse::json(
            200,
            &PreKeyStatusResponse {
                user_id: user_id.to_string(),
                found: remaining.is_some(),
                consumed_one_time_prekey_ids: self.prekeys.consumed_for(&user_id),
                remaining_one_time_prekeys: remaining,
                signed_one_time_prekey_records,
                low_one_time_prekeys,
                replenishment_required,
                replenishment_actor: "client",
                node_generates_user_keys: false,
            },
        )
    }

    fn handle_control_dht_replication_plan(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let replication_factor = query
            .get("replication_factor")
            .or_else(|| query.get("factor"))
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8)
            .clamp(1, 64);
        ControlResponse::json(200, &self.plan_dht_replication(replication_factor))
    }

    fn handle_control_dht_routing_refresh_plan(&self) -> ControlResponse {
        ControlResponse::json(200, &self.plan_dht_routing_refresh())
    }

    fn handle_control_dht_rpc(&mut self, body: &str) -> ControlResponse {
        let req: DhtRpcControlRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        ControlResponse::json(200, &self.handle_dht_rpc(req.request))
    }

    fn handle_control_dht_record_store(&mut self, body: &str) -> ControlResponse {
        let req: StoreDhtRecordRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let key = req.record.key.to_hex();
        let inserted = self.dht_records.store(req.record);
        ControlResponse::json(
            201,
            &StoreDhtRecordResponse {
                stored: true,
                inserted,
                key,
                records: self.dht_records.len(),
            },
        )
    }

    fn handle_control_dht_record_get(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(key) = query.get("key") else {
            return ControlResponse::text(400, "missing key");
        };
        let key = match DhtRecordKey::from_hex(key) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let record = self.dht_records.find_value(&key);
        ControlResponse::json(
            200,
            &GetDhtRecordResponse {
                key: key.to_hex(),
                found: record.is_some(),
                record,
            },
        )
    }

    fn handle_control_dht_closest(&mut self, path: &str) -> ControlResponse {
        let query = query_params(path);
        let Some(target) = query.get("target") else {
            return ControlResponse::text(400, "missing target");
        };
        let target = match DhtRecordKey::from_hex(target) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let limit = query
            .get("limit")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8)
            .clamp(1, 64);
        let records = self.dht_records.closest_records(target, limit);
        ControlResponse::json(
            200,
            &ClosestDhtRecordsResponse {
                target: target.to_hex(),
                records,
            },
        )
    }

    fn handle_control_sync_import(&mut self, body: &str) -> ControlResponse {
        let req: ImportSnapshotRequest = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let stats = self.merge_snapshot(req.snapshot);
        ControlResponse::json(
            200,
            &ImportSnapshotResponse {
                imported: true,
                peers: stats.peers,
                mailbox_deliveries: stats.mailbox_deliveries,
                prekey_bundles: stats.prekey_bundles,
                signed_one_time_prekey_records: stats.signed_one_time_prekey_records,
                dht_records: stats.dht_records,
            },
        )
    }
}

fn path_without_query(path: &str) -> &str {
    path.split_once('?').map(|(h, _)| h).unwrap_or(path)
}

fn query_params(path: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some((_, query)) = path.split_once('?') else {
        return out;
    };
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        out.insert(percent_decode(key), percent_decode(value));
    }
    out
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 0;
    while idx < bytes.len() {
        match bytes[idx] {
            b'+' => {
                out.push(b' ');
                idx += 1;
            }
            b'%' if idx + 2 < bytes.len() => {
                if let (Some(hi), Some(lo)) = (from_hex(bytes[idx + 1]), from_hex(bytes[idx + 2])) {
                    out.push((hi << 4) | lo);
                    idx += 3;
                } else {
                    out.push(bytes[idx]);
                    idx += 1;
                }
            }
            byte => {
                out.push(byte);
                idx += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lm_core::MailboxMessageKind;

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
                    Some(1),
                )
                .unwrap_err(),
            LmError::PayloadTooLarge
        );
        mailbox
            .push_verified_with_limits(message, &alice.identity_public_key(), None, Some(1), None)
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
        let key = DhtRecordKey::for_public_peer("rpc-record");
        let record = DhtRecord {
            key,
            kind: DhtRecordKind::PublicPeer,
            value: "rpc-value".into(),
            created_at: current_unix_timestamp(),
            expires_at: current_unix_timestamp().saturating_add(3600),
            republish_at: current_unix_timestamp().saturating_add(1800),
        };

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
            path: "/announce".into(),
            body,
            headers: Vec::new(),
        });
        assert_eq!(response.status, 201);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: "/peers/closest?target=peer-a&limit=1".into(),
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
            path: "/dht/replication-plan?factor=1".into(),
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
            path: "/dht/routing-refresh-plan".into(),
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
            .insert_verified(announce, &identity.identity_public_key())
            .unwrap();
        let key = DhtRecordKey::for_public_peer("control-rpc-record");
        let record = DhtRecord {
            key,
            kind: DhtRecordKind::PublicPeer,
            value: "control-rpc-value".into(),
            created_at: current_unix_timestamp(),
            expires_at: current_unix_timestamp().saturating_add(3600),
            republish_at: current_unix_timestamp().saturating_add(1800),
        };

        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/dht/rpc".into(),
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
            path: "/dht/rpc".into(),
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
        assert_eq!(body["Value"]["record"]["value"], "control-rpc-value");

        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/dht/rpc".into(),
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
    fn control_plane_dht_record_store_get_closest_and_snapshot() {
        let mut node = NativeNode::new(NodeConfig::default());
        let record = DhtRecord {
            key: DhtRecordKey::for_public_peer("peer-control"),
            kind: DhtRecordKind::PublicPeer,
            value: "public-peer-record".into(),
            created_at: current_unix_timestamp(),
            expires_at: current_unix_timestamp().saturating_add(3600),
            republish_at: current_unix_timestamp().saturating_add(1800),
        };
        let key = record.key;
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/dht/record".into(),
            body: serde_json::json!({ "record": record }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 201, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["inserted"], true);
        assert_eq!(body["records"], 1);

        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/dht/record?key={key}"),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200, "{}", response.body);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["found"], true);
        assert_eq!(body["record"]["value"], "public-peer-record");

        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/dht/closest?target={key}&limit=1"),
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
            "public-peer-record"
        );
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
            path: "/mailbox/push".into(),
            body,
            headers: Vec::new(),
        });
        assert_eq!(response.status, 201);
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 1);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/mailbox/take?user_id={}", bob.user_id()),
            body: String::new(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200);
        assert!(response.body.contains("ciphertext"));
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 1);
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        let delivery_id = body["messages"][0]["delivery_id"]
            .as_str()
            .unwrap()
            .to_string();
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/mailbox/ack".into(),
            body: serde_json::json!({
                "user_id": bob.user_id().to_string(),
                "delivery_ids": [delivery_id],
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 200);
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 0);
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
            path: "/mailbox/push".into(),
            body: serde_json::json!({
                "message_text": message.to_export_text().unwrap(),
                "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert!(response.body.contains("payload too large"));
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
                path: "/mailbox/push".into(),
                body: serde_json::json!({
                    "message_text": message.to_export_text().unwrap(),
                    "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
                })
                .to_string(),
                headers: Vec::new(),
            });
            assert_eq!(response.status, expected_status, "{}", response.body);
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
                path: "/mailbox/push".into(),
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
            path: "/mailbox/push".into(),
            body: "{not-json".into(),
            headers: Vec::new(),
        });
        assert_eq!(response.status, 400);
        assert_eq!(node.maintenance.mailbox_push_rejects.invalid_json, 1);

        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/mailbox/push".into(),
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
        node.mailbox
            .push_verified(msg, &alice.identity_public_key())
            .unwrap();

        let snapshot = node.to_state_snapshot();
        let restored = NativeNode::from_state_snapshot(snapshot);
        assert_eq!(restored.routing_table.len(), 1);
        assert_eq!(restored.kademlia.len(), 1);
        assert_eq!(restored.mailbox.pending_for(bob.user_id()), 1);
    }

    #[test]
    fn control_plane_prekey_publish_and_get() {
        let (alice, _) = Identity::create_with_passphrase("alice prekey").unwrap();
        let mut node = NativeNode::new(NodeConfig::default());
        let (bundle, _) = PreKeyBundle::new(&alice, 1, 2, 3600).unwrap();
        let response = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/prekey/publish".into(),
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
            path: format!("/prekey/get?user_id={}", alice.user_id()),
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
            path: "/sync/snapshot".into(),
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
            path: "/sync/import".into(),
            body: serde_json::json!({ "snapshot": snapshot }).to_string(),
            headers: Vec::new(),
        });
        assert_eq!(import_response.status, 200);
        assert!(target.prekeys.get_for_unchecked(alice.user_id()).is_some());
        assert_eq!(target.mailbox.pending_for(bob.user_id()), 1);
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
            path: "/prekey/publish".into(),
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
            path: format!("/prekey/get?user_id={}&consume=true", alice.user_id()),
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
            path: format!("/prekey/status?user_id={}", alice.user_id()),
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
            path: format!("/prekey/status?user_id={}", missing.user_id()),
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
            path: "/prekey/publish".into(),
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
            path: format!("/prekey/get?user_id={}&consume=true", alice.user_id()),
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
    fn parse_capabilities_csv_works() {
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
