//! Mailbox delivery store and rate-limiting types.

use lm_core::{LmError, MailboxMessage, Result, UserId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::kademlia::{DEFAULT_MAILBOX_ACK_RECEIPT_TTL_SECONDS, current_unix_timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailboxRateLimitConfig {
    pub window_seconds: u64,
    pub max_messages: u32,
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
    pub(crate) deliveries: HashMap<UserId, Vec<MailboxDelivery>>,
    message_ids: HashMap<UserId, Vec<Uuid>>,
    ack_receipts: HashMap<UserId, Vec<MailboxAckReceipt>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxDeliveryStatus {
    pub delivery_id: String,
    pub status: MailboxDeliveryState,
    pub created_at: Option<u64>,
    pub delivered_at: Option<u64>,
    pub acked_at: Option<u64>,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailboxDeliveryState {
    Pending,
    DeliveredUnacked,
    Acked,
    AbsentOrExpired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxAckReceipt {
    pub user_id: UserId,
    pub delivery_id: String,
    pub acked_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxUserDeliverySummary {
    pub total: usize,
    pub undelivered: usize,
    pub delivered_unacked: usize,
    pub bytes: usize,
}

impl MailboxStore {
    pub fn push_verified(
        &mut self,
        message: MailboxMessage,
        from_identity_public_key: &[u8; 32],
    ) -> Result<String> {
        self.push_verified_with_limits(message, from_identity_public_key, None, None, None, None)
    }

    pub fn push_verified_with_limits(
        &mut self,
        message: MailboxMessage,
        from_identity_public_key: &[u8; 32],
        max_total_bytes: Option<u64>,
        max_bytes_per_user: Option<u64>,
        max_messages_per_user: Option<usize>,
        max_message_ttl_seconds: Option<u64>,
    ) -> Result<String> {
        message.verify(from_identity_public_key)?;
        let now = current_unix_timestamp();
        if message.expires_at <= now {
            return Err(LmError::ExpiredObject);
        }
        if let Some(max_ttl) = max_message_ttl_seconds
            && message.expires_at.saturating_sub(now) > max_ttl
        {
            return Err(LmError::PayloadTooLarge);
        }
        self.prune_expired(now);
        if self.has_message_id(&message.to_user_id, message.message_id) {
            return Err(LmError::DuplicateMessage);
        }
        let message_bytes = mailbox_delivery_size_bytes(&message);
        if let Some(max_total_bytes) = max_total_bytes
            && self.total_bytes().saturating_add(message_bytes) > max_total_bytes as usize
        {
            return Err(LmError::PayloadTooLarge);
        }
        if let Some(max_bytes_per_user) = max_bytes_per_user
            && self
                .bytes_for(&message.to_user_id)
                .saturating_add(message_bytes)
                > max_bytes_per_user as usize
        {
            return Err(LmError::PayloadTooLarge);
        }
        if let Some(max_messages) = max_messages_per_user
            && self.pending_for(&message.to_user_id) >= max_messages
        {
            return Err(LmError::PayloadTooLarge);
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
        self.take_for_limited(user_id, usize::MAX)
    }

    pub fn take_for_limited(&mut self, user_id: &UserId, limit: usize) -> Vec<MailboxDelivery> {
        let now = current_unix_timestamp();
        self.prune_expired(now);
        let Some(deliveries) = self.deliveries.get_mut(user_id) else {
            return Vec::new();
        };
        let limit = limit.min(deliveries.len());
        for delivery in deliveries.iter_mut().take(limit) {
            delivery.delivered_at = Some(now);
        }
        deliveries.iter().take(limit).cloned().collect()
    }

    pub fn ack(&mut self, user_id: &UserId, delivery_ids: &[String]) -> usize {
        let now = current_unix_timestamp();
        let Some(deliveries) = self.deliveries.get_mut(user_id) else {
            return 0;
        };
        let delivery_ids = delivery_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let mut acked = Vec::new();
        deliveries.retain(|delivery| {
            if delivery_ids.contains(delivery.delivery_id.as_str()) {
                acked.push(delivery.delivery_id.clone());
                false
            } else {
                true
            }
        });
        let removed = acked.len();
        if deliveries.is_empty() {
            self.deliveries.remove(user_id);
            self.message_ids.remove(user_id);
        } else {
            self.rebuild_message_ids_for(user_id);
        }
        for delivery_id in acked {
            self.record_ack_receipt(user_id, delivery_id, now);
        }
        removed
    }

    pub fn pending_for(&self, user_id: &UserId) -> usize {
        self.deliveries.get(user_id).map(Vec::len).unwrap_or(0)
    }

    pub fn delivery_summary_for(&self, user_id: &UserId) -> MailboxUserDeliverySummary {
        let Some(deliveries) = self.deliveries.get(user_id) else {
            return MailboxUserDeliverySummary::default();
        };
        let delivered_unacked = deliveries
            .iter()
            .filter(|delivery| delivery.delivered_at.is_some())
            .count();
        MailboxUserDeliverySummary {
            total: deliveries.len(),
            undelivered: deliveries.len().saturating_sub(delivered_unacked),
            delivered_unacked,
            bytes: self.bytes_for(user_id),
        }
    }

    pub fn delivery_status(&self, user_id: &UserId, delivery_id: &str) -> MailboxDeliveryStatus {
        let Some(delivery) = self
            .deliveries
            .get(user_id)
            .and_then(|deliveries| deliveries.iter().find(|d| d.delivery_id == delivery_id))
        else {
            if let Some(receipt) = self.ack_receipt(user_id, delivery_id) {
                return MailboxDeliveryStatus {
                    delivery_id: receipt.delivery_id.clone(),
                    status: MailboxDeliveryState::Acked,
                    created_at: None,
                    delivered_at: None,
                    acked_at: Some(receipt.acked_at),
                    expires_at: Some(receipt.expires_at),
                };
            }
            return MailboxDeliveryStatus {
                delivery_id: delivery_id.to_string(),
                status: MailboxDeliveryState::AbsentOrExpired,
                created_at: None,
                delivered_at: None,
                acked_at: None,
                expires_at: None,
            };
        };
        MailboxDeliveryStatus {
            delivery_id: delivery.delivery_id.clone(),
            status: if delivery.delivered_at.is_some() {
                MailboxDeliveryState::DeliveredUnacked
            } else {
                MailboxDeliveryState::Pending
            },
            created_at: Some(delivery.created_at),
            delivered_at: delivery.delivered_at,
            acked_at: None,
            expires_at: Some(delivery.message.expires_at),
        }
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

    pub fn bytes_for(&self, user_id: &UserId) -> usize {
        self.deliveries
            .get(user_id)
            .map(|deliveries| {
                deliveries
                    .iter()
                    .map(|delivery| mailbox_delivery_size_bytes(&delivery.message))
                    .sum()
            })
            .unwrap_or(0)
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
        self.prune_ack_receipts(now);
        removed
    }

    fn record_ack_receipt(&mut self, user_id: &UserId, delivery_id: String, acked_at: u64) {
        let expires_at = acked_at.saturating_add(DEFAULT_MAILBOX_ACK_RECEIPT_TTL_SECONDS);
        let receipts = self.ack_receipts.entry(user_id.clone()).or_default();
        if let Some(existing) = receipts
            .iter_mut()
            .find(|receipt| receipt.delivery_id == delivery_id)
        {
            existing.acked_at = acked_at;
            existing.expires_at = expires_at;
            return;
        }
        receipts.push(MailboxAckReceipt {
            user_id: user_id.clone(),
            delivery_id,
            acked_at,
            expires_at,
        });
    }

    fn ack_receipt(&self, user_id: &UserId, delivery_id: &str) -> Option<&MailboxAckReceipt> {
        self.ack_receipts
            .get(user_id)
            .and_then(|receipts| receipts.iter().find(|r| r.delivery_id == delivery_id))
    }

    fn prune_ack_receipts(&mut self, now: u64) {
        let users = self.ack_receipts.keys().cloned().collect::<Vec<_>>();
        for user_id in users {
            let Some(receipts) = self.ack_receipts.get_mut(&user_id) else {
                continue;
            };
            receipts.retain(|receipt| receipt.expires_at > now);
            if receipts.is_empty() {
                self.ack_receipts.remove(&user_id);
            }
        }
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

    pub(crate) fn rebuild_message_ids(&mut self) {
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

    pub fn all_ack_receipts(&self) -> Vec<MailboxAckReceipt> {
        self.ack_receipts
            .values()
            .flat_map(|receipts| receipts.iter().cloned())
            .collect()
    }

    pub fn all_messages(&self) -> Vec<MailboxMessage> {
        self.all_deliveries()
            .into_iter()
            .map(|delivery| delivery.message)
            .collect()
    }

    pub(crate) fn restore_deliveries(&mut self, deliveries: Vec<MailboxDelivery>) {
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

    pub(crate) fn restore_ack_receipts(&mut self, receipts: Vec<MailboxAckReceipt>) {
        self.ack_receipts.clear();
        self.merge_ack_receipts(receipts);
    }

    pub(crate) fn restore_messages(&mut self, messages: Vec<MailboxMessage>) {
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

    pub(crate) fn merge_deliveries(&mut self, deliveries: Vec<MailboxDelivery>) -> usize {
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

    pub(crate) fn merge_ack_receipts(&mut self, receipts: Vec<MailboxAckReceipt>) -> usize {
        let now = current_unix_timestamp();
        self.prune_ack_receipts(now);
        let mut inserted = 0usize;
        for receipt in receipts {
            if receipt.expires_at <= now || receipt.delivery_id.trim().is_empty() {
                continue;
            }
            let list = self
                .ack_receipts
                .entry(receipt.user_id.clone())
                .or_default();
            if let Some(existing) = list
                .iter_mut()
                .find(|existing| existing.delivery_id == receipt.delivery_id)
            {
                if receipt.acked_at > existing.acked_at || receipt.expires_at > existing.expires_at
                {
                    *existing = receipt;
                }
                continue;
            }
            list.push(receipt);
            inserted = inserted.saturating_add(1);
        }
        inserted
    }
}

pub fn mailbox_delivery_size_bytes(message: &MailboxMessage) -> usize {
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
    pub(crate) window_started_at: Option<u64>,
    pub(crate) count: u32,
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
