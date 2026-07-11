use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::contact::ContactCard;
use lm_core::friend::FriendRequest;
use lm_core::identity::{IdentityBackupPackage, IdentitySeed};
use lm_core::message::{DirectEnvelope, MessageBody, PlainMessage};
use lm_core::{Identity, normalize_passphrase};
use serde::Serialize;
use uuid::Uuid;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Serialize)]
struct IdentityVector {
    name: String,
    identity_seed_hex: String,
    user_id: String,
    identity_public_key_base64: String,
    x25519_public_key_base64: String,
    storage_key_hex: String,
    passphrase_input: String,
    passphrase_normalized: String,
}

#[derive(Serialize)]
struct ContactVector {
    name: String,
    contact_card_text: String,
    user_id: String,
    identity_public_key_base64: String,
    x25519_public_key_base64: String,
    display_name: String,
    verify: bool,
}

#[derive(Serialize)]
struct FriendVector {
    name: String,
    request_text: String,
    from_user_id: String,
    to_user_id: String,
    note: String,
    verify: bool,
}

#[derive(Serialize)]
struct MessageVector {
    name: String,
    envelope_json: String,
    from_user_id: String,
    to_user_id: String,
    conversation_id: String,
    plaintext: String,
    decrypts: bool,
}

#[derive(Serialize)]
struct BackupVector {
    name: String,
    passphrase: String,
    backup_text: String,
    user_id: String,
    restores_identity_seed_hex: String,
}

fn identity(seed_byte: u8) -> Identity {
    Identity::from_seed(IdentitySeed::from_bytes([seed_byte; 32])).unwrap()
}

fn main() {
    let alice = identity(7);
    let bob = identity(8);
    let identity_vector = IdentityVector {
        name: "identity_v1_fixed_seed_07".into(),
        identity_seed_hex: hex(&[7u8; 32]),
        user_id: alice.user_id().to_string(),
        identity_public_key_base64: BASE64.encode(alice.identity_public_key()),
        x25519_public_key_base64: BASE64.encode(alice.x25519_public_key()),
        storage_key_hex: hex(&alice.storage_key().unwrap()),
        passphrase_input: "  ＡＢＣ　１２３  ".into(),
        passphrase_normalized: normalize_passphrase("  ＡＢＣ　１２３  "),
    };

    let alice_card = ContactCard::new(&alice, Some("Alice Vector".into()), None, vec![]).unwrap();
    let alice_card_text = alice_card.to_export_text().unwrap();
    let contact_vector = ContactVector {
        name: "contact_card_v1_alice_seed_07".into(),
        contact_card_text: alice_card_text.clone(),
        user_id: alice.user_id().to_string(),
        identity_public_key_base64: BASE64.encode(alice.identity_public_key()),
        x25519_public_key_base64: BASE64.encode(alice.x25519_public_key()),
        display_name: "Alice Vector".into(),
        verify: alice_card.verify().is_ok(),
    };

    let friend_request = FriendRequest::new(
        &alice,
        bob.user_id().clone(),
        alice_card,
        Some("hello vector".into()),
        100 * 365 * 24 * 60 * 60,
    )
    .unwrap();
    let friend_vector = FriendVector {
        name: "friend_request_v1_alice_to_bob".into(),
        request_text: friend_request.to_export_text().unwrap(),
        from_user_id: alice.user_id().to_string(),
        to_user_id: bob.user_id().to_string(),
        note: "hello vector".into(),
        verify: friend_request.verify().is_ok(),
    };

    let plain = PlainMessage {
        r#type: "lm-message-v1".into(),
        version: 1,
        message_id: Uuid::parse_str("11111111-2222-4333-8444-555555555555").unwrap(),
        conversation_id: "conv-vector".into(),
        sender_user_id: alice.user_id().clone(),
        body: MessageBody::Text {
            text: "hello encrypted vector".into(),
        },
        created_at: 0,
    };
    let envelope = DirectEnvelope::encrypt_plain(
        &alice,
        bob.user_id().clone(),
        &bob.x25519_public_key(),
        plain,
    )
    .unwrap();
    let decrypts = envelope.decrypt(&bob, &alice.x25519_public_key()).is_ok();
    let message_vector = MessageVector {
        name: "message_crypto_v1_alice_to_bob".into(),
        envelope_json: serde_json::to_string_pretty(&envelope).unwrap(),
        from_user_id: alice.user_id().to_string(),
        to_user_id: bob.user_id().to_string(),
        conversation_id: "conv-vector".into(),
        plaintext: "hello encrypted vector".into(),
        decrypts,
    };

    let backup = IdentityBackupPackage::encrypt(&alice, "vector passphrase").unwrap();
    let backup_vector = BackupVector {
        name: "backup_v1_alice_seed_07".into(),
        passphrase: "vector passphrase".into(),
        backup_text: backup.to_export_text().unwrap(),
        user_id: alice.user_id().to_string(),
        restores_identity_seed_hex: hex(backup
            .decrypt_seed("vector passphrase")
            .unwrap()
            .as_bytes()),
    };

    let out = serde_json::json!({
        "identity_v1": identity_vector,
        "backup_v1": backup_vector,
        "contact_card_v1": contact_vector,
        "friend_request_v1": friend_vector,
        "message_crypto_v1": message_vector,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
