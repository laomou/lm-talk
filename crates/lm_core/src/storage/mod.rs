//! Minimal in-memory storage model for tests and Web MVP wiring.
//!
//! This is not persistent storage. Native SQLite/SQLCipher and Web IndexedDB
//! adapters should implement equivalent semantics later.

use crate::{BlockEntry, Contact, DirectEnvelope, LmError, Result, UserId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredMessage {
    pub message_id: Uuid,
    pub conversation_id: String,
    pub sender_user_id: UserId,
    pub receiver_user_id: UserId,
    pub envelope: DirectEnvelope,
    pub received_at: Option<u64>,
}

#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    contacts: HashMap<UserId, Contact>,
    blocks: HashMap<UserId, BlockEntry>,
    messages: HashMap<Uuid, StoredMessage>,
    seen_messages: HashSet<Uuid>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save_contact(&mut self, contact: Contact) -> Result<()> {
        if self.blocks.contains_key(&contact.user_id) {
            return Err(LmError::BlockedSender);
        }
        self.contacts.insert(contact.user_id.clone(), contact);
        Ok(())
    }

    pub fn get_contact(&self, user_id: &UserId) -> Option<&Contact> {
        self.contacts.get(user_id)
    }

    pub fn list_contacts(&self) -> Vec<&Contact> {
        self.contacts.values().collect()
    }

    pub fn block_user(&mut self, entry: BlockEntry) {
        self.contacts.remove(&entry.user_id);
        self.blocks.insert(entry.user_id.clone(), entry);
    }

    pub fn is_blocked(&self, user_id: &UserId) -> bool {
        self.blocks.contains_key(user_id)
    }

    pub fn save_message(&mut self, message: StoredMessage) -> Result<()> {
        if self.is_blocked(&message.sender_user_id) {
            return Err(LmError::BlockedSender);
        }
        if self.seen_messages.contains(&message.message_id) {
            return Err(LmError::DuplicateMessage);
        }
        self.seen_messages.insert(message.message_id);
        self.messages.insert(message.message_id, message);
        Ok(())
    }

    pub fn get_message(&self, message_id: &Uuid) -> Option<&StoredMessage> {
        self.messages.get(message_id)
    }

    pub fn list_messages_for_conversation(&self, conversation_id: &str) -> Vec<&StoredMessage> {
        self.messages
            .values()
            .filter(|m| m.conversation_id == conversation_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContactState, Identity, TrustLevel};

    #[test]
    fn memory_store_contacts_blocks_and_duplicate_messages() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let bob_card = bob
            .export_contact_card(Some("Bob".into()), None, vec![])
            .unwrap();
        let mut bob_contact = bob_card.into_contact(TrustLevel::Imported).unwrap();
        bob_contact.state = ContactState::Friend;

        let mut store = MemoryStore::new();
        store.save_contact(bob_contact).unwrap();
        assert!(store.get_contact(bob.user_id()).is_some());

        let envelope = DirectEnvelope::encrypt_text(
            &alice,
            bob.user_id().clone(),
            &bob.x25519_public_key(),
            "conv1".into(),
            "hello".into(),
        )
        .unwrap();
        let stored = StoredMessage {
            message_id: envelope.message_id,
            conversation_id: "conv1".into(),
            sender_user_id: alice.user_id().clone(),
            receiver_user_id: bob.user_id().clone(),
            envelope,
            received_at: Some(1),
        };
        store.save_message(stored.clone()).unwrap();
        assert_eq!(
            store.save_message(stored).unwrap_err(),
            LmError::DuplicateMessage
        );
    }
}
