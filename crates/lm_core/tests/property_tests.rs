use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    ContactCard, DeviceRevoke, DirectEnvelope, FileChunkEnvelope, FileManifest, FriendRequest,
    FriendResponse, GroupEvent, GroupInvite, GroupSenderKeyDistribution, Identity,
    IdentityBackupPackage, LmError, MailboxMessage, MessageBody, MessageReceipt, PeerAnnounce,
    PreKeyBundle, PublicPeerAnnounce, RatchetEnvelope, RatchetSessionState, SignalAnswer,
    SignalOffer, SignedOneTimePreKeyRecord,
};
use proptest::prelude::*;

fn ratchet_pair() -> (RatchetSessionState, RatchetSessionState) {
    let alice = lm_core::UserId::from_raw("lm1_prop_alice".to_string()).unwrap();
    let bob = lm_core::UserId::from_raw("lm1_prop_bob".to_string()).unwrap();
    RatchetSessionState::new_pair_from_shared_secret(alice, bob, &[42u8; 32]).unwrap()
}

fn bounded_text() -> impl Strategy<Value = String> {
    ".*".prop_map(|mut text| {
        text.truncate(512);
        text
    })
}

fn ratchet_texts() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(".*", 1..24).prop_map(|mut texts| {
        for text in &mut texts {
            text.truncate(256);
        }
        texts
    })
}

proptest! {
    #[test]
    fn direct_envelope_json_roundtrips_for_arbitrary_text_payload(text in bounded_text()) {
        let alice = Identity::from_seed(lm_core::IdentitySeed::from_bytes([7u8; 32])).unwrap();
        let bob = Identity::from_seed(lm_core::IdentitySeed::from_bytes([8u8; 32])).unwrap();
        let envelope = DirectEnvelope::encrypt_text(
            &alice,
            bob.user_id().clone(),
            &bob.x25519_public_key(),
            "prop-conv".into(),
            text.clone(),
        )
        .unwrap();
        let encoded = serde_json::to_string(&envelope).unwrap();
        let decoded: DirectEnvelope = serde_json::from_str(&encoded).unwrap();
        prop_assert_eq!(&decoded, &envelope);
        let plain = decoded.decrypt(&bob, &alice.x25519_public_key()).unwrap();
        prop_assert_eq!(
            plain.body,
            MessageBody::Text {
                text
            }
        );
    }

    #[test]
    fn direct_envelope_ciphertext_tamper_is_rejected(text in bounded_text(), flip_index in 0usize..512) {
        let alice = Identity::from_seed(lm_core::IdentitySeed::from_bytes([7u8; 32])).unwrap();
        let bob = Identity::from_seed(lm_core::IdentitySeed::from_bytes([8u8; 32])).unwrap();
        let mut envelope = DirectEnvelope::encrypt_text(
            &alice,
            bob.user_id().clone(),
            &bob.x25519_public_key(),
            "prop-conv".into(),
            text,
        )
        .unwrap();
        let mut ciphertext = BASE64.decode(envelope.ciphertext.as_bytes()).unwrap();
        prop_assume!(!ciphertext.is_empty());
        let index = flip_index % ciphertext.len();
        ciphertext[index] ^= 0x01;
        envelope.ciphertext = BASE64.encode(ciphertext);
        prop_assert!(envelope.decrypt(&bob, &alice.x25519_public_key()).is_err());
    }

    #[test]
    fn contact_card_expiration_boundary_is_enforced(offset in -2i64..=2) {
        let alice = Identity::from_seed(lm_core::IdentitySeed::from_bytes([9u8; 32])).unwrap();
        let now = lm_core::unix_now();
        let expires_at = if offset <= 0 {
            now.saturating_sub(offset.unsigned_abs())
        } else {
            // Keep positive-offset cases comfortably in the future so slow CI
            // runners do not turn the "expires in one second" boundary case
            // into an accidental expiration before verification executes.
            now.saturating_add(60 + offset as u64)
        };
        let card = ContactCard::new(&alice, None, Some(expires_at), vec![]).unwrap();
        if expires_at <= now {
            prop_assert_eq!(card.verify().unwrap_err(), lm_core::LmError::ExpiredObject);
        } else {
            prop_assert!(card.verify().is_ok());
        }
    }

    #[test]
    fn import_text_size_limits_are_enforced_at_boundaries(extra in 1usize..=8) {
        let contact = format!(
            "lm-contact-card-v1:{}",
            "A".repeat(lm_core::MAX_CONTACT_CARD_TEXT_BYTES + extra)
        );
        let friend = format!(
            "lm-friend-request-v1:{}",
            "A".repeat(lm_core::MAX_FRIEND_REQUEST_TEXT_BYTES + extra)
        );
        let backup = format!(
            "lm-identity-backup-v1:{}",
            "A".repeat(lm_core::MAX_IDENTITY_BACKUP_TEXT_BYTES + extra)
        );
        let prekey = format!(
            "lm-prekey-bundle-v1:{}",
            "A".repeat(lm_core::MAX_PREKEY_BUNDLE_TEXT_BYTES + extra)
        );
        let signal_offer = format!(
            "lm-signal-offer-v1:{}",
            "A".repeat(lm_core::MAX_SIGNAL_TEXT_BYTES + extra)
        );
        let mailbox = format!(
            "lm-mailbox-message-v1:{}",
            "A".repeat(lm_core::MAX_MAILBOX_CIPHERTEXT_BYTES + extra)
        );
        let receipt = format!(
            "lm-message-receipt-v1:{}",
            "A".repeat(lm_core::MAX_MESSAGE_RECEIPT_TEXT_BYTES + extra)
        );
        let group_invite = format!(
            "lm-group-invite-v1:{}",
            "A".repeat(lm_core::MAX_GROUP_INVITE_TEXT_BYTES + extra)
        );
        let file_manifest = format!(
            "lm-file-manifest-v1:{}",
            "A".repeat(lm_core::MAX_FILE_MANIFEST_TEXT_BYTES + extra)
        );
        let file_chunk_json = "A".repeat(lm_core::MAX_FILE_CHUNK_JSON_BYTES + extra);
        let device_revoke = format!(
            "lm-device-revoke-v1:{}",
            "A".repeat(lm_core::MAX_DEVICE_REVOKE_TEXT_BYTES + extra)
        );
        prop_assert_eq!(
            ContactCard::from_export_text(&contact).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            FriendRequest::from_export_text(&friend).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            IdentityBackupPackage::from_export_text(&backup).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            PreKeyBundle::from_export_text(&prekey).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            SignalOffer::from_export_text(&signal_offer).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            SignalAnswer::from_export_text(&signal_offer.replace("lm-signal-offer-v1:", "lm-signal-answer-v1:")).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            PeerAnnounce::from_export_text(&signal_offer.replace("lm-signal-offer-v1:", "lm-peer-announce-v1:")).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            PublicPeerAnnounce::from_export_text(&signal_offer.replace("lm-signal-offer-v1:", "lm-public-peer-announce-v1:")).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            MailboxMessage::from_export_text(&mailbox).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            MessageReceipt::from_export_text(&receipt).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            GroupInvite::from_export_text(&group_invite).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            GroupEvent::from_export_text(&group_invite.replace("lm-group-invite-v1:", "lm-group-event-v1:")).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            GroupSenderKeyDistribution::from_export_text(&group_invite.replace("lm-group-invite-v1:", "lm-group-sender-key-v1:")).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            FileManifest::from_export_text(&file_manifest).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            FileChunkEnvelope::from_json(&file_chunk_json).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
        prop_assert_eq!(
            DeviceRevoke::from_export_text(&device_revoke).unwrap_err(),
            lm_core::LmError::PayloadTooLarge
        );
    }

    #[test]
    fn malformed_import_text_parsers_do_not_panic(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        let text = String::from_utf8_lossy(&bytes);
        let _ = ContactCard::from_export_text(&text);
        let _ = FriendRequest::from_export_text(&text);
        let _ = FriendResponse::from_export_text(&text);
        let _ = IdentityBackupPackage::from_export_text(&text);
        let _ = PreKeyBundle::from_export_text(&text);
        let _ = SignedOneTimePreKeyRecord::from_export_text(&text);
        let _ = SignalOffer::from_export_text(&text);
        let _ = SignalAnswer::from_export_text(&text);
        let _ = PeerAnnounce::from_export_text(&text);
        let _ = PublicPeerAnnounce::from_export_text(&text);
        let _ = MailboxMessage::from_export_text(&text);
        let _ = MessageReceipt::from_export_text(&text);
        let _ = GroupInvite::from_export_text(&text);
        let _ = GroupEvent::from_export_text(&text);
        let _ = GroupSenderKeyDistribution::from_export_text(&text);
        let _ = RatchetSessionState::from_export_text(&text);
        let _ = FileManifest::from_export_text(&text);
        let _ = FileChunkEnvelope::from_json(&text);
        let _ = DeviceRevoke::from_export_text(&text);
    }

    #[test]
    fn ratchet_rejects_replay_for_any_delivery_order(texts in ratchet_texts()) {
        let (mut alice, mut bob) = ratchet_pair();
        let mut envelopes = Vec::new();
        for text in &texts {
            envelopes.push(RatchetEnvelope::encrypt_text(&mut alice, "prop-ratchet".into(), text.clone()).unwrap());
        }
        for envelope in envelopes.iter().rev() {
            let plain = envelope.decrypt(&mut bob).unwrap();
            let received = match &plain.body { MessageBody::Text { text } => text };
            prop_assert!(texts.contains(received));
            prop_assert_eq!(envelope.decrypt(&mut bob).unwrap_err(), LmError::ReplayDetected);
        }
        prop_assert!(bob.skipped_message_keys.is_empty());
    }

    #[test]
    fn ratchet_skip_window_rejects_unbounded_gap(gap in 513u32..560) {
        let (mut alice, mut bob) = ratchet_pair();
        let mut far = None;
        for i in 0..=gap {
            let envelope = RatchetEnvelope::encrypt_text(&mut alice, "prop-ratchet".into(), format!("msg-{i}")).unwrap();
            if i == gap { far = Some(envelope); }
        }
        prop_assert_eq!(far.unwrap().decrypt(&mut bob).unwrap_err(), LmError::PayloadTooLarge);
        prop_assert!(bob.skipped_message_keys.is_empty());
        prop_assert_eq!(bob.recv_count, 0);
    }

    #[test]
    fn ratchet_out_of_order_within_window_consumes_all_skipped_keys(count in 2usize..32) {
        let (mut alice, mut bob) = ratchet_pair();
        let mut envelopes = Vec::new();
        for i in 0..count {
            envelopes.push(RatchetEnvelope::encrypt_text(&mut alice, "prop-ratchet".into(), format!("msg-{i}")).unwrap());
        }
        let last = envelopes.last().unwrap().clone();
        last.decrypt(&mut bob).unwrap();
        prop_assert_eq!(bob.skipped_message_keys.len(), count - 1);
        for envelope in envelopes.iter().take(count - 1) {
            envelope.decrypt(&mut bob).unwrap();
        }
        prop_assert!(bob.skipped_message_keys.is_empty());
    }
}
