use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    ContactCard, DirectEnvelope, FriendRequest, Identity, IdentityBackupPackage, IdentitySeed,
    normalize_passphrase,
};
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test-vectors")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn identity(seed_byte: u8) -> Identity {
    Identity::from_seed(IdentitySeed::from_bytes([seed_byte; 32])).unwrap()
}

#[test]
fn identity_vector_matches_fixed_seed() {
    let v = fixture("identity_v1.json");
    let id = identity(7);
    assert_eq!(v["identity_seed_hex"], hex(&[7u8; 32]));
    assert_eq!(v["user_id"], id.user_id().to_string());
    assert_eq!(
        v["identity_public_key_base64"],
        BASE64.encode(id.identity_public_key())
    );
    assert_eq!(
        v["x25519_public_key_base64"],
        BASE64.encode(id.x25519_public_key())
    );
    assert_eq!(v["storage_key_hex"], hex(&id.storage_key().unwrap()));
    assert_eq!(
        v["passphrase_normalized"],
        normalize_passphrase(v["passphrase_input"].as_str().unwrap())
    );
}

#[test]
fn backup_vector_restores_seed_and_user() {
    let v = fixture("backup_v1.json");
    let backup =
        IdentityBackupPackage::from_export_text(v["backup_text"].as_str().unwrap()).unwrap();
    let seed = backup
        .decrypt_seed(v["passphrase"].as_str().unwrap())
        .unwrap();
    assert_eq!(hex(seed.as_bytes()), v["restores_identity_seed_hex"]);
    let restored =
        Identity::restore_from_backup(&backup, v["passphrase"].as_str().unwrap()).unwrap();
    assert_eq!(restored.user_id().to_string(), v["user_id"]);
    assert!(Identity::restore_from_backup(&backup, "wrong passphrase").is_err());
}

#[test]
fn contact_and_friend_vectors_verify() {
    let contact = fixture("contact_card_v1.json");
    let card =
        ContactCard::from_export_text(contact["contact_card_text"].as_str().unwrap()).unwrap();
    card.verify().unwrap();
    assert_eq!(card.user_id.to_string(), contact["user_id"]);
    assert_eq!(
        card.display_name.as_deref(),
        contact["display_name"].as_str()
    );

    let friend = fixture("friend_request_v1.json");
    let req = FriendRequest::from_export_text(friend["request_text"].as_str().unwrap()).unwrap();
    req.verify().unwrap();
    assert_eq!(req.from_user_id.to_string(), friend["from_user_id"]);
    assert_eq!(req.to_user_id.to_string(), friend["to_user_id"]);
    assert_eq!(req.note.as_deref(), friend["note"].as_str());
}

#[test]
fn message_vector_decrypts_and_tamper_fails() {
    let v = fixture("message_crypto_v1.json");
    let envelope: DirectEnvelope =
        serde_json::from_str(v["envelope_json"].as_str().unwrap()).unwrap();
    let bob = identity(8);
    let alice = identity(7);
    let plain = envelope.decrypt(&bob, &alice.x25519_public_key()).unwrap();
    assert_eq!(plain.conversation_id, v["conversation_id"]);
    match plain.body {
        lm_core::MessageBody::Text { text } => assert_eq!(text, v["plaintext"]),
    }

    let mut tampered = envelope.clone();
    tampered.ciphertext.push('A');
    assert!(tampered.decrypt(&bob, &alice.x25519_public_key()).is_err());
}

fn mutate_export_text(input: &str) -> String {
    let mut chars: Vec<char> = input.chars().collect();
    let idx = chars.len() - 1;
    chars[idx] = if chars[idx] == 'A' { 'B' } else { 'A' };
    chars.into_iter().collect()
}

#[test]
fn vector_text_tamper_is_rejected_for_signed_objects() {
    let contact = fixture("contact_card_v1.json");
    let contact_text = contact["contact_card_text"].as_str().unwrap();
    let tampered_contact = mutate_export_text(contact_text);
    assert!(
        ContactCard::from_export_text(&tampered_contact)
            .and_then(|c| c.verify())
            .is_err()
    );

    let friend = fixture("friend_request_v1.json");
    let request_text = friend["request_text"].as_str().unwrap();
    let tampered_request = mutate_export_text(request_text);
    assert!(
        FriendRequest::from_export_text(&tampered_request)
            .and_then(|r| r.verify())
            .is_err()
    );
}

#[test]
fn passphrase_normalization_examples_are_stable() {
    let cases = [
        ("  ＡＢＣ　１２３  ", "ABC 123"),
        ("hello\t\nworld", "hello world"),
        ("  多　空   白  ", "多 空 白"),
        ("① ㍿ Ａ", "1 株式会社 A"),
    ];
    for (input, expected) in cases {
        assert_eq!(normalize_passphrase(input), expected);
    }
}

#[test]
fn vector_size_limits_reject_oversized_import_text() {
    let huge_contact = format!(
        "lm-contact-card-v1:{}",
        "A".repeat(lm_core::MAX_CONTACT_CARD_TEXT_BYTES + 1)
    );
    assert!(ContactCard::from_export_text(&huge_contact).is_err());

    let huge_request = format!(
        "lm-friend-request-v1:{}",
        "A".repeat(lm_core::MAX_FRIEND_REQUEST_TEXT_BYTES + 1)
    );
    assert!(FriendRequest::from_export_text(&huge_request).is_err());

    let huge_backup = format!(
        "lm-identity-backup-v1:{}",
        "A".repeat(lm_core::MAX_IDENTITY_BACKUP_TEXT_BYTES + 1)
    );
    assert!(IdentityBackupPackage::from_export_text(&huge_backup).is_err());
}
