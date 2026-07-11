//! Protocol payload size limits for DoS resistance.

use crate::{LmError, Result};

pub const MAX_IDENTITY_BACKUP_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_CONTACT_CARD_TEXT_BYTES: usize = 32 * 1024;
pub const MAX_FRIEND_REQUEST_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_FRIEND_RESPONSE_TEXT_BYTES: usize = 16 * 1024;
pub const MAX_GROUP_INVITE_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_SIGNAL_TEXT_BYTES: usize = 256 * 1024;
pub const MAX_DIRECT_MESSAGE_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_GROUP_MESSAGE_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_FRIEND_NOTE_BYTES: usize = 1024;
pub const MAX_DISPLAY_NAME_BYTES: usize = 256;
pub const MAX_GROUP_NAME_BYTES: usize = 256;
pub const MAX_GROUP_MEMBERS: usize = 256;
pub const MAX_CONTACT_DEVICE_CERTS: usize = 32;
pub const MAX_PREKEY_BUNDLE_TEXT_BYTES: usize = 128 * 1024;
pub const MAX_ONE_TIME_PREKEYS: usize = 100;
pub const MAX_NETWORK_ADDRESSES: usize = 64;
pub const MAX_NETWORK_ADDRESS_BYTES: usize = 1024;
pub const MAX_MAILBOX_CIPHERTEXT_BYTES: usize = 512 * 1024;
pub const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FILE_NAME_BYTES: usize = 255;
pub const MAX_FILE_MIME_BYTES: usize = 128;
pub const MAX_FILE_CHUNK_BYTES: usize = 256 * 1024;

pub fn ensure_len(value: &str, max: usize) -> Result<()> {
    ensure_bytes(value.as_bytes().len(), max)
}

pub fn ensure_bytes(len: usize, max: usize) -> Result<()> {
    if len > max {
        Err(LmError::PayloadTooLarge)
    } else {
        Ok(())
    }
}

pub fn ensure_vec_len<T>(value: &[T], max: usize) -> Result<()> {
    if value.len() > max {
        Err(LmError::PayloadTooLarge)
    } else {
        Ok(())
    }
}

pub fn ensure_addresses(addresses: &[String]) -> Result<()> {
    ensure_vec_len(addresses, MAX_NETWORK_ADDRESSES)?;
    for address in addresses {
        ensure_len(address, MAX_NETWORK_ADDRESS_BYTES)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DirectEnvelope, FileChunkEnvelope, FileManifest, FriendRequest, GroupInvite, Identity,
        MailboxMessage, MailboxMessageKind, SignalOffer,
    };
    use uuid::Uuid;

    #[test]
    fn oversized_friend_note_is_rejected() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let card = alice.export_contact_card(None, None, vec![]).unwrap();
        let note = "x".repeat(MAX_FRIEND_NOTE_BYTES + 1);
        assert_eq!(
            FriendRequest::new(&alice, bob.user_id().clone(), card, Some(note), 3600).unwrap_err(),
            crate::LmError::PayloadTooLarge
        );
    }

    #[test]
    fn oversized_direct_message_is_rejected() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let text = "x".repeat(MAX_DIRECT_MESSAGE_TEXT_BYTES + 1);
        assert_eq!(
            DirectEnvelope::encrypt_text(
                &alice,
                bob.user_id().clone(),
                &bob.x25519_public_key(),
                "c".into(),
                text
            )
            .unwrap_err(),
            crate::LmError::PayloadTooLarge
        );
    }

    #[test]
    fn oversized_group_members_are_rejected() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let members = (0..=MAX_GROUP_MEMBERS)
            .map(|_| alice.user_id().clone())
            .collect();
        assert_eq!(
            GroupInvite::new(&alice, Uuid::new_v4(), "g".into(), members, 3600).unwrap_err(),
            crate::LmError::PayloadTooLarge
        );
    }

    #[test]
    fn oversized_network_payloads_are_rejected() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        assert_eq!(
            SignalOffer::new(
                &alice,
                None,
                None,
                "x".repeat(MAX_SIGNAL_TEXT_BYTES + 1),
                3600
            )
            .unwrap_err(),
            crate::LmError::PayloadTooLarge
        );
        assert_eq!(
            MailboxMessage::new(
                &alice,
                alice.user_id().clone(),
                MailboxMessageKind::Other,
                "x".repeat(MAX_MAILBOX_CIPHERTEXT_BYTES + 1),
                3600
            )
            .unwrap_err(),
            crate::LmError::PayloadTooLarge
        );
    }

    #[test]
    fn oversized_file_payloads_are_rejected() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        assert_eq!(
            FileManifest::new(
                &alice,
                alice.user_id().clone(),
                "f".into(),
                "application/octet-stream".into(),
                (MAX_FILE_BYTES + 1) as u64,
                1024,
                1,
                "hash".into()
            )
            .unwrap_err(),
            crate::LmError::PayloadTooLarge
        );
        assert_eq!(
            FileChunkEnvelope::encrypt_chunk(
                &alice,
                alice.user_id().clone(),
                &alice.x25519_public_key(),
                Uuid::new_v4(),
                0,
                &vec![0; MAX_FILE_CHUNK_BYTES + 1]
            )
            .unwrap_err(),
            crate::LmError::PayloadTooLarge
        );
    }
}
