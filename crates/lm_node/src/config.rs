use lm_core::{Identity, LmError, PublicPeerCapability, Result};
use serde::{Deserialize, Serialize};

use crate::{
    DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES, DEFAULT_MAX_MAILBOX_BYTES_PER_USER,
    MailboxRateLimitConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfig {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub capabilities: Vec<PublicPeerCapability>,
    pub max_mailbox_bytes: Option<u64>,
    pub max_message_ttl_seconds: Option<u64>,
    #[serde(default = "default_max_mailbox_bytes_per_user")]
    pub max_mailbox_bytes_per_user: Option<u64>,
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
    #[serde(default = "default_dht_peer_quarantine_consecutive_failures")]
    pub dht_peer_quarantine_consecutive_failures: u32,
    pub max_relay_bandwidth_kbps: Option<u64>,
    pub announce_ttl_seconds: u64,
}

fn default_max_mailbox_bytes_per_user() -> Option<u64> {
    Some(DEFAULT_MAX_MAILBOX_BYTES_PER_USER)
}

fn default_max_mailbox_messages_per_user() -> Option<usize> {
    Some(1000)
}

fn default_dht_peer_quarantine_consecutive_failures() -> u32 {
    DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            peer_id: "lm-node-dev".to_string(),
            addresses: vec!["/ip4/0.0.0.0/tcp/0".to_string()],
            capabilities: vec![PublicPeerCapability::Bootstrap, PublicPeerCapability::Dht],
            max_mailbox_bytes: Some(10 * 1024 * 1024),
            max_message_ttl_seconds: Some(24 * 3600),
            max_mailbox_bytes_per_user: default_max_mailbox_bytes_per_user(),
            max_mailbox_messages_per_user: default_max_mailbox_messages_per_user(),
            mailbox_sender_rate_limit_window_seconds: None,
            mailbox_sender_rate_limit_max_messages: None,
            mailbox_global_rate_limit_window_seconds: None,
            mailbox_global_rate_limit_max_messages: None,
            dht_peer_quarantine_consecutive_failures:
                DEFAULT_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES,
            max_relay_bandwidth_kbps: Some(1024),
            announce_ttl_seconds: 24 * 3600,
        }
    }
}

impl NodeConfig {
    pub fn create_announce(&self, identity: &Identity) -> Result<lm_core::PublicPeerAnnounce> {
        lm_core::PublicPeerAnnounce::new(
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMaintenanceStats {
    pub prune_runs: u64,
    pub mailbox_expired_deliveries: u64,
    pub prekey_expired_bundles: u64,
    #[serde(default)]
    pub mailbox_push_rejects: MailboxPushRejectStats,
    #[serde(default)]
    pub dht_record_rejects: DhtRecordRejectStats,
    #[serde(default)]
    pub mailbox_ack_rejects: MailboxAckRejectStats,
    #[serde(default)]
    pub routing_peer_rejects: RoutingPeerRejectStats,
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
pub(crate) enum MailboxPushRejectReason {
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
pub struct MailboxAckRejectStats {
    pub invalid_json: u64,
    pub invalid_user_id: u64,
    pub too_many_ids: u64,
    pub empty_id: u64,
    pub id_too_large: u64,
}

impl MailboxAckRejectStats {
    pub fn total(&self) -> u64 {
        self.invalid_json
            .saturating_add(self.invalid_user_id)
            .saturating_add(self.too_many_ids)
            .saturating_add(self.empty_id)
            .saturating_add(self.id_too_large)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MailboxAckRejectReason {
    InvalidJson,
    InvalidUserId,
    TooManyIds,
    EmptyId,
    IdTooLarge,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhtRecordRejectStats {
    pub invalid_json: u64,
    pub expired: u64,
    pub ttl_too_long: u64,
    pub payload_too_large: u64,
    pub invalid_record: u64,
}

impl DhtRecordRejectStats {
    pub fn total(&self) -> u64 {
        self.invalid_json
            .saturating_add(self.expired)
            .saturating_add(self.ttl_too_long)
            .saturating_add(self.payload_too_large)
            .saturating_add(self.invalid_record)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DhtRecordRejectReason {
    InvalidJson,
    Expired,
    TtlTooLong,
    PayloadTooLarge,
    InvalidRecord,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingPeerRejectStats {
    pub expired: u64,
    pub mismatched_node_id: u64,
    pub local_node: u64,
    pub missing_identity_public_key: u64,
    pub invalid_identity_public_key: u64,
    pub invalid_signature: u64,
    #[serde(default)]
    pub too_many_addresses: u64,
    #[serde(default)]
    pub address_too_large: u64,
}

impl RoutingPeerRejectStats {
    pub fn total(&self) -> u64 {
        self.expired
            .saturating_add(self.mismatched_node_id)
            .saturating_add(self.local_node)
            .saturating_add(self.missing_identity_public_key)
            .saturating_add(self.invalid_identity_public_key)
            .saturating_add(self.invalid_signature)
            .saturating_add(self.too_many_addresses)
            .saturating_add(self.address_too_large)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoutingPeerRejectReason {
    Expired,
    MismatchedNodeId,
    LocalNode,
    MissingIdentityPublicKey,
    InvalidIdentityPublicKey,
    InvalidSignature,
    TooManyAddresses,
    AddressTooLarge,
}
