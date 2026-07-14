use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    ContactCard, DirectEnvelope, FriendRequest, Identity, IdentityBackupPackage, MessageBody,
};
use proptest::prelude::*;

fn bounded_text() -> impl Strategy<Value = String> {
    ".*".prop_map(|mut text| {
        text.truncate(512);
        text
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
        let expires_at = if offset.is_negative() {
            now.saturating_sub(offset.unsigned_abs())
        } else {
            now.saturating_add(offset as u64)
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
    }
}
