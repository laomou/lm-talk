use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    device::DeviceSeed, ContactCard, DeviceCert, DeviceIdentity, DeviceRevoke, DirectEnvelope, FriendRequest, Identity, IdentityBackupPackage,
    IdentitySeed, MailboxMessage, MessageReceipt, MessageReceiptKind, PreKeyBundle, SignedOneTimePreKeyRecord, normalize_passphrase,
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
fn backup_export_text_tamper_is_rejected() {
    let v = fixture("backup_v1.json");
    let tampered = mutate_export_text(v["backup_text"].as_str().unwrap());
    assert!(
        IdentityBackupPackage::from_export_text(&tampered)
            .and_then(|backup| Identity::restore_from_backup(
                &backup,
                v["passphrase"].as_str().unwrap()
            ))
            .is_err()
    );
}


#[test]
fn device_vectors_verify() {
    let v = fixture("device_v1.json");
    let identity = identity(7);
    let device = DeviceIdentity::from_seed(DeviceSeed::from_bytes([9u8; 32])).unwrap();
    assert_eq!(v["identity_seed_hex"], hex(&[7u8; 32]));
    assert_eq!(v["device_seed_hex"], hex(&[9u8; 32]));
    assert_eq!(v["user_id"], identity.user_id().to_string());
    assert_eq!(v["device_id"], device.device_id().to_string());
    assert_eq!(
        v["device_public_key_base64"],
        BASE64.encode(device.device_public_key())
    );
    assert_eq!(
        v["device_box_public_key_base64"],
        BASE64.encode(device.device_box_public_key())
    );

    let cert: DeviceCert = serde_json::from_str(v["device_cert_json"].as_str().unwrap()).unwrap();
    cert.verify(&identity.identity_public_key()).unwrap();
    assert_eq!(cert.device_id.to_string(), v["device_id"]);
    assert_eq!(cert.device_name.as_deref(), Some("phone"));

    let revoke = DeviceRevoke::from_export_text(v["device_revoke_text"].as_str().unwrap()).unwrap();
    revoke.verify(&identity.identity_public_key()).unwrap();
    assert_eq!(revoke.device_id.to_string(), v["device_id"]);
    assert_eq!(revoke.reason.as_deref(), v["revoke_reason"].as_str());
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
fn prekey_vectors_verify() {
    let v = fixture("prekey_v1.json");
    let id = identity(7);
    assert_eq!(v["identity_seed_hex"], hex(&[7u8; 32]));
    assert_eq!(v["user_id"], id.user_id().to_string());
    assert_eq!(
        v["identity_public_key_base64"],
        BASE64.encode(id.identity_public_key())
    );

    let bundle = PreKeyBundle::from_export_text(v["prekey_bundle_text"].as_str().unwrap()).unwrap();
    bundle.verify().unwrap();
    assert_eq!(bundle.user_id.to_string(), v["user_id"]);
    assert_eq!(bundle.signed_prekey_id, v["signed_prekey_id"].as_u64().unwrap() as u32);
    assert_eq!(bundle.one_time_prekeys.len(), 0);

    let records = v["signed_one_time_prekey_record_texts"].as_array().unwrap();
    assert_eq!(records.len(), v["signed_one_time_prekey_count"].as_u64().unwrap() as usize);
    for record_text in records {
        let record = SignedOneTimePreKeyRecord::from_export_text(record_text.as_str().unwrap()).unwrap();
        record.verify_for_bundle(&bundle).unwrap();
    }
    let first = SignedOneTimePreKeyRecord::from_export_text(v["first_signed_one_time_prekey_record_text"].as_str().unwrap()).unwrap();
    assert_eq!(first.key_id, v["first_one_time_prekey_id"].as_u64().unwrap() as u32);
}

#[test]
fn receipt_and_mailbox_vectors_verify() {
    let v = fixture("receipt_mailbox_v1.json");
    let alice = identity(7);
    let bob = identity(8);
    assert_eq!(v["alice_user_id"], alice.user_id().to_string());
    assert_eq!(v["bob_user_id"], bob.user_id().to_string());
    assert_eq!(
        v["alice_identity_public_key_base64"],
        BASE64.encode(alice.identity_public_key())
    );
    assert_eq!(
        v["bob_identity_public_key_base64"],
        BASE64.encode(bob.identity_public_key())
    );

    let delivered = MessageReceipt::from_export_text(v["delivered_receipt_text"].as_str().unwrap()).unwrap();
    delivered.verify(&bob.identity_public_key()).unwrap();
    assert_eq!(delivered.kind, MessageReceiptKind::Delivered);
    assert_eq!(delivered.from_user_id.to_string(), v["bob_user_id"]);
    assert_eq!(delivered.to_user_id.to_string(), v["alice_user_id"]);
    assert_eq!(delivered.target_message_id.to_string(), v["target_message_id"]);
    assert_eq!(delivered.conversation_id, v["conversation_id"]);
    assert_eq!(delivered.mailbox_delivery_id.as_deref(), v["mailbox_delivery_id"].as_str());

    let read = MessageReceipt::from_export_text(v["read_receipt_text"].as_str().unwrap()).unwrap();
    read.verify(&bob.identity_public_key()).unwrap();
    assert_eq!(read.kind, MessageReceiptKind::Read);
    assert_eq!(read.target_message_id.to_string(), v["target_message_id"]);

    let mailbox = MailboxMessage::from_export_text(v["mailbox_message_text"].as_str().unwrap()).unwrap();
    mailbox.verify(&alice.identity_public_key()).unwrap();
    assert_eq!(mailbox.from_user_id.to_string(), v["alice_user_id"]);
    assert_eq!(mailbox.to_user_id.to_string(), v["bob_user_id"]);
    assert_eq!(format!("{:?}", mailbox.kind), v["mailbox_kind"]);
    assert_eq!(mailbox.ciphertext, v["mailbox_ciphertext"]);
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




    let prekey = fixture("prekey_v1.json");
    let tampered_bundle = mutate_export_text(prekey["prekey_bundle_text"].as_str().unwrap());
    assert!(PreKeyBundle::from_export_text(&tampered_bundle).is_err());
    let tampered_otpk = mutate_export_text(prekey["first_signed_one_time_prekey_record_text"].as_str().unwrap());
    assert!(SignedOneTimePreKeyRecord::from_export_text(&tampered_otpk).is_err());

    let receipt_mailbox = fixture("receipt_mailbox_v1.json");
    let tampered_receipt = mutate_export_text(receipt_mailbox["delivered_receipt_text"].as_str().unwrap());
    assert!(
        MessageReceipt::from_export_text(&tampered_receipt)
            .and_then(|r| r.verify(&identity(8).identity_public_key()))
            .is_err()
    );
    let tampered_mailbox = mutate_export_text(receipt_mailbox["mailbox_message_text"].as_str().unwrap());
    assert!(
        MailboxMessage::from_export_text(&tampered_mailbox)
            .and_then(|m| m.verify(&identity(7).identity_public_key()))
            .is_err()
    );

    let device = fixture("device_v1.json");
    let mut cert: DeviceCert = serde_json::from_str(device["device_cert_json"].as_str().unwrap()).unwrap();
    cert.device_name = Some("evil".into());
    assert!(cert.verify(&identity(7).identity_public_key()).is_err());

    let revoke_text = device["device_revoke_text"].as_str().unwrap();
    let tampered_revoke = mutate_export_text(revoke_text);
    assert!(
        DeviceRevoke::from_export_text(&tampered_revoke)
            .and_then(|r| r.verify(&identity(7).identity_public_key()))
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
