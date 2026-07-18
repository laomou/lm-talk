//! Kademlia DHT types: node IDs, distance, record storage, routing table, and RPC messages.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{ContactCard, LmError, PreKeyBundle, PublicPeerAnnounce, Result, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const KADEMLIA_ID_BYTES: usize = 32;
pub const DEFAULT_K_BUCKET_SIZE: usize = 20;
pub const DEFAULT_MAX_DHT_RECORDS: usize = 4096;
pub const DEFAULT_MAX_DHT_RECORD_VALUE_BYTES: usize = 256 * 1024;
pub const DEFAULT_MAX_DHT_RECORD_TTL_SECONDS: u64 = 30 * 24 * 60 * 60;
pub const DEFAULT_MAX_MAILBOX_BYTES_PER_USER: u64 = 2 * 1024 * 1024;
pub const DEFAULT_MAX_MAILBOX_ACK_IDS: usize = 2048;
pub const DEFAULT_MAX_MAILBOX_ACK_ID_BYTES: usize = 128;
pub const DEFAULT_MAX_MAILBOX_TAKE_LIMIT: usize = 200;
pub const DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES: u32 = 5;
pub const DEFAULT_MAILBOX_ACK_RECEIPT_TTL_SECONDS: u64 = 30 * 24 * 60 * 60;

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

    pub fn for_contact_card(user_id: &UserId) -> Self {
        Self::derive("contact-card", user_id.as_str().as_bytes())
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
    ContactCard,
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

    pub fn contact_card(user_id: &UserId, value: String, ttl_seconds: u64) -> Self {
        Self::new(
            DhtRecordKey::for_contact_card(user_id),
            DhtRecordKind::ContactCard,
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

    pub fn is_oversized(&self) -> bool {
        self.value.len() > DEFAULT_MAX_DHT_RECORD_VALUE_BYTES
    }

    pub fn ttl_too_long_at(&self, now: u64) -> bool {
        self.expires_at.saturating_sub(now) > DEFAULT_MAX_DHT_RECORD_TTL_SECONDS
    }

    pub fn validate_for_store_at(&self, now: u64) -> Result<()> {
        if self.is_expired_at(now) || self.ttl_too_long_at(now) || self.is_oversized() {
            return Err(LmError::PayloadTooLarge);
        }
        match self.kind {
            DhtRecordKind::PublicPeer => {
                let announce = PublicPeerAnnounce::from_export_text(&self.value)?;
                if DhtRecordKey::for_public_peer(&announce.peer_id) != self.key {
                    return Err(LmError::InvalidSignature);
                }
                if announce.expires_at <= now {
                    return Err(LmError::ExpiredObject);
                }
            }
            DhtRecordKind::PreKey => {
                let bundle = PreKeyBundle::from_export_text(&self.value)?;
                bundle.verify()?;
                if DhtRecordKey::for_prekey(&bundle.user_id) != self.key {
                    return Err(LmError::InvalidSignature);
                }
                if bundle.expires_at <= now {
                    return Err(LmError::ExpiredObject);
                }
            }
            DhtRecordKind::ContactCard => {
                let card = ContactCard::from_export_text(&self.value)?;
                card.verify()?;
                if DhtRecordKey::for_contact_card(&card.user_id) != self.key {
                    return Err(LmError::InvalidSignature);
                }
                if let Some(expires_at) = card.expires_at
                    && expires_at <= now
                {
                    return Err(LmError::ExpiredObject);
                }
            }
            DhtRecordKind::MailboxHint => {
                if self.value.trim().is_empty()
                    || self.value.len() > lm_core::limits::MAX_NETWORK_ADDRESS_BYTES
                {
                    return Err(LmError::PayloadTooLarge);
                }
            }
        }
        Ok(())
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
        let now = current_unix_timestamp();
        if record.is_expired_at(now) || record.is_oversized() || record.ttl_too_long_at(now) {
            return false;
        }
        let is_new = !self.records.contains_key(&record.key);
        self.records.insert(record.key, record);
        self.enforce_capacity(DEFAULT_MAX_DHT_RECORDS);
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
        let now = current_unix_timestamp();
        for record in records {
            if record.validate_for_store_at(now).is_ok() {
                self.store(record);
            }
        }
    }

    pub fn merge_records(&mut self, records: Vec<DhtRecord>) -> usize {
        let mut inserted = 0;
        let now = current_unix_timestamp();
        for record in records {
            if record.validate_for_store_at(now).is_ok() && self.store(record) {
                inserted += 1;
            }
        }
        inserted
    }

    pub fn enforce_capacity(&mut self, max_records: usize) {
        while self.records.len() > max_records {
            let Some(oldest_key) = self
                .records
                .iter()
                .min_by_key(|(_, record)| (record.expires_at, record.created_at))
                .map(|(key, _)| *key)
            else {
                break;
            };
            self.records.remove(&oldest_key);
        }
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

    pub(crate) fn insert_local_snapshot(&mut self, announce: PublicPeerAnnounce) {
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

pub(crate) fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub(crate) fn decode_hex_32(value: &str) -> Result<[u8; KADEMLIA_ID_BYTES]> {
    if value.len() != KADEMLIA_ID_BYTES * 2 {
        return Err(LmError::InvalidFormat);
    }
    let mut out = [0u8; KADEMLIA_ID_BYTES];
    let bytes = value.as_bytes();
    for idx in 0..KADEMLIA_ID_BYTES {
        let hi = from_hex(bytes[idx * 2]).ok_or(LmError::InvalidFormat)?;
        let lo = from_hex(bytes[idx * 2 + 1]).ok_or(LmError::InvalidFormat)?;
        out[idx] = (hi << 4) | lo;
    }
    Ok(out)
}

pub(crate) fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
