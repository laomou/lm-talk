use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{DirectEnvelope, Identity, MessageBody};
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
}
