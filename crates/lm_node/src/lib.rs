//! Native node scaffold for LM Talk.
//!
//! This crate intentionally starts as a deterministic, testable scaffold rather
//! than a real Kademlia implementation. It owns the public-peer runtime model
//! that future UDP/TCP/WebSocket transports can plug into: signed public peer
//! announcements, an in-memory routing table, and an optional mailbox queue.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    Identity, IdentityBackupPackage, LmError, MailboxMessage, PreKeyBundle, PublicPeerAnnounce,
    PublicPeerCapability, Result, UserId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingPeer {
    pub node_id: KademliaNodeId,
    pub announce: PublicPeerAnnounce,
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
            existing.last_seen_at = current_unix_timestamp();
            return Ok(());
        }
        if bucket.len() >= self.bucket_size {
            bucket.remove(0);
        }
        bucket.push(RoutingPeer {
            node_id,
            announce,
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
            existing.last_seen_at = current_unix_timestamp();
            return;
        }
        if bucket.len() >= self.bucket_size {
            bucket.remove(0);
        }
        bucket.push(RoutingPeer {
            node_id,
            announce,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfig {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub capabilities: Vec<PublicPeerCapability>,
    pub max_mailbox_bytes: Option<u64>,
    pub max_message_ttl_seconds: Option<u64>,
    #[serde(default = "default_max_mailbox_messages_per_user")]
    pub max_mailbox_messages_per_user: Option<usize>,
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
            max_relay_bandwidth_kbps: Some(1024),
            announce_ttl_seconds: 24 * 3600,
        }
    }
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

#[derive(Debug, Clone, Default)]
pub struct PreKeyStore {
    bundles: HashMap<UserId, PreKeyBundle>,
    consumed_one_time_prekeys: HashMap<UserId, Vec<u32>>,
}

impl PreKeyStore {
    pub fn publish_verified(&mut self, bundle: PreKeyBundle) -> Result<()> {
        bundle.verify()?;
        self.bundles.insert(bundle.user_id.clone(), bundle);
        Ok(())
    }

    pub fn get_for(&self, user_id: &UserId) -> Option<PreKeyBundle> {
        self.bundles.get(user_id).cloned()
    }

    pub fn take_for(&mut self, user_id: &UserId, consume: bool) -> Option<PreKeyBundle> {
        let (bundle, _) = self.take_for_with_selected_one_time_prekey(user_id, consume)?;
        Some(bundle)
    }

    pub fn take_for_with_selected_one_time_prekey(
        &mut self,
        user_id: &UserId,
        consume: bool,
    ) -> Option<(PreKeyBundle, Option<u32>)> {
        let bundle = self.get_for(user_id)?;
        let consumed = self
            .consumed_one_time_prekeys
            .entry(user_id.clone())
            .or_default();
        let selected = bundle
            .one_time_prekeys
            .iter()
            .map(|key| key.key_id)
            .find(|id| !consumed.contains(id));
        if consume {
            if let Some(id) = selected {
                consumed.push(id);
                consumed.sort_unstable();
                consumed.dedup();
            }
        }
        Some((bundle, selected))
    }

    pub fn consumed_for(&self, user_id: &UserId) -> Vec<u32> {
        self.consumed_one_time_prekeys
            .get(user_id)
            .cloned()
            .unwrap_or_default()
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

    fn restore_bundles(&mut self, bundles: Vec<PreKeyBundle>) {
        self.bundles.clear();
        for bundle in bundles {
            self.bundles.insert(bundle.user_id.clone(), bundle);
        }
    }

    fn merge_bundles(&mut self, bundles: Vec<PreKeyBundle>) -> usize {
        let mut inserted = 0;
        for bundle in bundles {
            if bundle.verify().is_err() {
                continue;
            }
            let is_new = !self.bundles.contains_key(&bundle.user_id);
            self.bundles.insert(bundle.user_id.clone(), bundle);
            if is_new {
                inserted += 1;
            }
        }
        inserted
    }

    fn restore_consumed(&mut self, consumed: Vec<ConsumedOneTimePreKey>) {
        self.consumed_one_time_prekeys.clear();
        self.merge_consumed(consumed);
    }

    fn merge_consumed(&mut self, consumed: Vec<ConsumedOneTimePreKey>) {
        for item in consumed {
            let ids = self
                .consumed_one_time_prekeys
                .entry(item.user_id)
                .or_default();
            ids.push(item.key_id);
            ids.sort_unstable();
            ids.dedup();
        }
    }
}

#[derive(Debug, Clone)]
pub struct NativeNode {
    pub config: NodeConfig,
    pub routing_table: RoutingTable,
    pub kademlia: KademliaRoutingTable,
    pub mailbox: MailboxStore,
    pub prekeys: PreKeyStore,
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
    pub consumed_one_time_prekeys: Vec<ConsumedOneTimePreKey>,
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
}

impl NativeNode {
    pub fn new(config: NodeConfig) -> Self {
        let local_id = KademliaNodeId::from_peer_id(&config.peer_id);
        Self {
            config,
            routing_table: RoutingTable::default(),
            kademlia: KademliaRoutingTable::new(local_id, DEFAULT_K_BUCKET_SIZE),
            mailbox: MailboxStore::default(),
            prekeys: PreKeyStore::default(),
        }
    }

    pub fn local_announce(&self, identity: &Identity) -> Result<PublicPeerAnnounce> {
        self.config.create_announce(identity)
    }

    pub fn to_state_snapshot(&self) -> NodeStateSnapshot {
        NodeStateSnapshot {
            version: 1,
            config: self.config.clone(),
            public_peers: self.routing_table.peers().cloned().collect(),
            mailbox_deliveries: self.mailbox.all_deliveries(),
            mailbox_messages: Vec::new(),
            prekey_bundles: self.prekeys.all_bundles(),
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
            .restore_consumed(snapshot.consumed_one_time_prekeys);
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
        self.prekeys
            .merge_consumed(snapshot.consumed_one_time_prekeys);
        NodeMergeStats {
            peers,
            mailbox_deliveries,
            prekey_bundles,
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
}

#[derive(Debug, Serialize)]
struct PublishPreKeyResponse {
    stored: bool,
    user_id: String,
    prekey_bundles: usize,
}

#[derive(Debug, Serialize)]
struct GetPreKeyResponse {
    user_id: String,
    found: bool,
    prekey_bundle_text: Option<String>,
    selected_one_time_prekey_id: Option<u32>,
    consumed_one_time_prekey_ids: Vec<u32>,
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
}

impl NativeNode {
    pub fn handle_control_request(&mut self, request: ControlRequest) -> ControlResponse {
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
                },
            ),
            ("POST", "/announce") => self.handle_control_announce(&request.body),
            ("GET", "/peers/closest") => self.handle_control_closest(&request.path),
            ("POST", "/mailbox/push") => self.handle_control_mailbox_push(&request.body),
            ("GET", "/mailbox/take") => self.handle_control_mailbox_take(&request.path),
            ("POST", "/mailbox/ack") => self.handle_control_mailbox_ack(&request.body),
            ("POST", "/prekey/publish") => self.handle_control_prekey_publish(&request.body),
            ("GET", "/prekey/get") => self.handle_control_prekey_get(&request.path),
            ("GET", "/sync/snapshot") => ControlResponse::json(200, &self.to_state_snapshot()),
            ("POST", "/sync/import") => self.handle_control_sync_import(&request.body),
            (
                _,
                "/health" | "/announce" | "/peers/closest" | "/mailbox/push" | "/mailbox/take"
                | "/mailbox/ack" | "/prekey/publish" | "/prekey/get" | "/sync/snapshot"
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
            Err(e) => return ControlResponse::text(400, format!("invalid json: {e}")),
        };
        let message = match MailboxMessage::from_export_text(req.message_text.trim()) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let public_key = match decode_identity_public_key_base64(&req.from_identity_public_key) {
            Ok(v) => v,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
        let delivery_id = match self.mailbox.push_verified_with_limits(
            message.clone(),
            &public_key,
            self.config.max_mailbox_bytes,
            self.config.max_mailbox_messages_per_user,
            self.config.max_message_ttl_seconds,
        ) {
            Ok(delivery_id) => delivery_id,
            Err(e) => return ControlResponse::text(400, e.to_string()),
        };
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
        let user_id = bundle.user_id.clone();
        if let Err(e) = self.prekeys.publish_verified(bundle) {
            return ControlResponse::text(400, e.to_string());
        }
        ControlResponse::json(
            201,
            &PublishPreKeyResponse {
                stored: true,
                user_id: user_id.to_string(),
                prekey_bundles: self.prekeys.len(),
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
            .take_for_with_selected_one_time_prekey(&user_id, consume);
        let (bundle_text, selected_one_time_prekey_id) = match selected {
            Some((bundle, selected_id)) => match bundle.to_export_text() {
                Ok(text) => (Some(text), selected_id),
                Err(e) => return ControlResponse::text(400, e.to_string()),
            },
            None => (None, None),
        };
        ControlResponse::json(
            200,
            &GetPreKeyResponse {
                user_id: user_id.to_string(),
                found: bundle_text.is_some(),
                prekey_bundle_text: bundle_text,
                selected_one_time_prekey_id,
                consumed_one_time_prekey_ids: self.prekeys.consumed_for(&user_id),
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
        });
        assert_eq!(response.status, 201);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: "/peers/closest?target=peer-a&limit=1".into(),
            body: String::new(),
        });
        assert_eq!(response.status, 200);
        assert!(response.body.contains("peer-a"));
        assert_ne!(
            node.kademlia.local_id(),
            KademliaNodeId::from_user_id(local_identity.user_id())
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
        });
        assert_eq!(response.status, 201);
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 1);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/mailbox/take?user_id={}", bob.user_id()),
            body: String::new(),
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
        });
        assert_eq!(response.status, 400);
        assert!(response.body.contains("payload too large"));
        assert_eq!(node.mailbox.pending_for(bob.user_id()), 0);
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
        });
        assert_eq!(response.status, 201);
        assert_eq!(node.prekeys.len(), 1);
        let response = node.handle_control_request(ControlRequest {
            method: "GET".into(),
            path: format!("/prekey/get?user_id={}", alice.user_id()),
            body: String::new(),
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
        let (bundle, _) = PreKeyBundle::new(&alice, 1, 1, 3600).unwrap();
        node.prekeys.publish_verified(bundle).unwrap();
        let restored = NativeNode::from_state_snapshot(node.to_state_snapshot());
        assert!(restored.prekeys.get_for(alice.user_id()).is_some());
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
        });
        assert_eq!(import_response.status, 200);
        assert!(target.prekeys.get_for(alice.user_id()).is_some());
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
