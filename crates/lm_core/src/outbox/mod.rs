//! Outbox queue for offline / retryable encrypted packets.

use crate::{LmError, Result, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutboxStatus {
    Queued,
    Sending,
    Sent,
    Failed,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxItem {
    pub id: Uuid,
    pub target_user_id: UserId,
    pub target_device_id: Option<String>,
    pub encrypted_packet: String,
    pub retry_count: u32,
    pub next_retry_at: u64,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub status: OutboxStatus,
}

impl OutboxItem {
    pub fn new(
        target_user_id: UserId,
        target_device_id: Option<String>,
        encrypted_packet: String,
        now: u64,
        ttl_seconds: Option<u64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            target_user_id,
            target_device_id,
            encrypted_packet,
            retry_count: 0,
            next_retry_at: now,
            created_at: now,
            expires_at: ttl_seconds.map(|ttl| now.saturating_add(ttl)),
            status: OutboxStatus::Queued,
        }
    }

    pub fn is_due(&self, now: u64) -> bool {
        matches!(self.status, OutboxStatus::Queued | OutboxStatus::Failed)
            && self.next_retry_at <= now
            && !self.is_expired(now)
    }

    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at.is_some_and(|expires| expires <= now)
    }

    pub fn mark_failed_with_backoff(&mut self, now: u64) {
        self.retry_count = self.retry_count.saturating_add(1);
        self.status = OutboxStatus::Failed;
        let delay = retry_delay_seconds(self.retry_count);
        self.next_retry_at = now.saturating_add(delay);
    }
}

#[derive(Debug, Default, Clone)]
pub struct Outbox {
    items: HashMap<Uuid, OutboxItem>,
}

impl Outbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&mut self, item: OutboxItem) -> Uuid {
        let id = item.id;
        self.items.insert(id, item);
        id
    }

    pub fn get(&self, id: &Uuid) -> Option<&OutboxItem> {
        self.items.get(id)
    }

    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut OutboxItem> {
        self.items.get_mut(id)
    }

    pub fn due_items(&self, now: u64) -> Vec<&OutboxItem> {
        self.items
            .values()
            .filter(|item| item.is_due(now))
            .collect()
    }

    pub fn mark_sending(&mut self, id: &Uuid) -> Result<()> {
        let item = self.items.get_mut(id).ok_or(LmError::UnknownContact)?;
        item.status = OutboxStatus::Sending;
        Ok(())
    }

    pub fn mark_sent(&mut self, id: &Uuid) -> Result<()> {
        let item = self.items.get_mut(id).ok_or(LmError::UnknownContact)?;
        item.status = OutboxStatus::Sent;
        Ok(())
    }

    pub fn mark_failed(&mut self, id: &Uuid, now: u64) -> Result<()> {
        let item = self.items.get_mut(id).ok_or(LmError::UnknownContact)?;
        item.mark_failed_with_backoff(now);
        Ok(())
    }

    pub fn expire_old(&mut self, now: u64) {
        for item in self.items.values_mut() {
            if item.is_expired(now) && !matches!(item.status, OutboxStatus::Sent) {
                item.status = OutboxStatus::Expired;
            }
        }
    }

    pub fn cancel(&mut self, id: &Uuid) -> Result<()> {
        let item = self.items.get_mut(id).ok_or(LmError::UnknownContact)?;
        item.status = OutboxStatus::Cancelled;
        Ok(())
    }
}

pub fn retry_delay_seconds(retry_count: u32) -> u64 {
    match retry_count {
        0 | 1 => 30,
        2 => 120,
        3 => 600,
        4 => 3600,
        _ => 21_600,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Identity;

    #[test]
    fn outbox_retry_and_expiry() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let mut outbox = Outbox::new();
        let item = OutboxItem::new(
            alice.user_id().clone(),
            None,
            "packet".into(),
            100,
            Some(1000),
        );
        let id = outbox.enqueue(item);
        assert_eq!(outbox.due_items(100).len(), 1);
        outbox.mark_failed(&id, 100).unwrap();
        let item = outbox.get(&id).unwrap();
        assert_eq!(item.retry_count, 1);
        assert_eq!(item.next_retry_at, 130);
        assert_eq!(outbox.due_items(129).len(), 0);
        assert_eq!(outbox.due_items(130).len(), 1);
        outbox.expire_old(1200);
        assert_eq!(outbox.get(&id).unwrap().status, OutboxStatus::Expired);
    }
}
