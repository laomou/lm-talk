//! Integration tests for the public LM Talk WASM API.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_wasm::*;
use serde_json::Value;

#[test]
fn wasm_identity_contact_friend_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let alice_backup = alice_v["backup_text"].as_str().unwrap();
    let bob_backup = bob_v["backup_text"].as_str().unwrap();

    let alice_restore = restore_identity(alice_backup, "alice pass").unwrap();
    let alice_restore_v: Value = serde_json::from_str(&alice_restore).unwrap();
    assert_eq!(alice_v["user_id"], alice_restore_v["user_id"]);

    let alice_card =
        export_contact_card(alice_backup, "alice pass", Some("Alice".into()), None).unwrap();
    let bob_card = export_contact_card(bob_backup, "bob pass", Some("Bob".into()), None).unwrap();

    let alice_info: Value =
        serde_json::from_str(&inspect_contact_card(&alice_card).unwrap()).unwrap();
    assert_eq!(alice_v["user_id"], alice_info["user_id"]);

    let req = create_friend_request(
        bob_backup,
        "bob pass",
        &bob_card,
        &alice_card,
        Some("hi".into()),
    )
    .unwrap();
    let resp = accept_friend_request(alice_backup, "alice pass", &req).unwrap();
    assert!(resp.starts_with(lm_core::codec::FRIEND_RESPONSE_TEXT_PREFIX));
}

#[test]
fn wasm_identity_backup_can_be_reencrypted_with_new_passphrase() {
    let alice = create_identity("old pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let old_backup = alice_v["backup_text"].as_str().unwrap();
    let reencrypted = reencrypt_identity_backup(old_backup, "old pass", "new pass").unwrap();
    let reencrypted_v: Value = serde_json::from_str(&reencrypted).unwrap();
    let new_backup = reencrypted_v["backup_text"].as_str().unwrap();

    assert_eq!(alice_v["user_id"], reencrypted_v["user_id"]);
    assert_ne!(old_backup, new_backup);
    let restored = restore_identity(new_backup, "new pass").unwrap();
    let restored_v: Value = serde_json::from_str(&restored).unwrap();
    assert_eq!(alice_v["user_id"], restored_v["user_id"]);
}

#[test]
fn wasm_device_revoke_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let backup = alice_v["backup_text"].as_str().unwrap();
    let device = create_device_cert(backup, "alice pass", Some("phone".into())).unwrap();
    let device_v: Value = serde_json::from_str(&device).unwrap();
    let device_id = device_v["device_id"].as_str().unwrap();
    let device_backup = device_v["device_backup_text"].as_str().unwrap();
    assert!(device_backup.starts_with("lm-device-backup-v1:"));
    let backup_info: Value =
        serde_json::from_str(&inspect_device_backup(backup, "alice pass", device_backup).unwrap())
            .unwrap();
    assert_eq!(backup_info["device_id"], device_id);
    assert_eq!(
        backup_info["device_public_key"],
        device_v["device_public_key"]
    );
    assert_eq!(
        backup_info["device_box_public_key"],
        device_v["device_box_public_key"]
    );
    let sealed = seal_device_slot(
        device_v["device_box_public_key"].as_str().unwrap(),
        "device-slot-aad",
        "secret envelope",
    )
    .unwrap();
    assert_eq!(
        open_device_slot(
            backup,
            "alice pass",
            device_backup,
            "device-slot-aad",
            &sealed
        )
        .unwrap(),
        "secret envelope"
    );
    let revoke =
        create_device_revoke(backup, "alice pass", device_id, Some("lost".into())).unwrap();
    assert!(revoke.starts_with("lm-device-revoke-v1:"));
    let info: Value = serde_json::from_str(
        &inspect_device_revoke(&revoke, alice_v["identity_public_key"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(info["device_id"], device_id);
    assert_eq!(info["reason"], "lost");
}

#[test]
fn wasm_data_backup_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let backup = alice_v["backup_text"].as_str().unwrap();
    let data = r#"{"contacts":[],"messages":[{"text":"hello"}]}"#;
    let encrypted = export_data_backup(backup, "alice pass", data).unwrap();
    assert!(encrypted.starts_with("lm-data-backup-v1:"));
    let decrypted = import_data_backup(backup, "alice pass", &encrypted).unwrap();
    assert_eq!(decrypted, data);
}

#[test]
fn wasm_group_invite_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let alice_backup = alice_v["backup_text"].as_str().unwrap();
    let bob_user_id = bob_v["user_id"].as_str().unwrap();
    let alice_card =
        export_contact_card(alice_backup, "alice pass", Some("Alice".into()), None).unwrap();
    let group_id = uuid::Uuid::new_v4().to_string();
    let members =
        serde_json::to_string(&vec![alice_v["user_id"].as_str().unwrap(), bob_user_id]).unwrap();
    let invite = create_group_invite(
        alice_backup,
        "alice pass",
        &group_id,
        "Test Group",
        &members,
    )
    .unwrap();
    assert!(invite.starts_with("lm-group-invite-v1:"));
    let info: Value =
        serde_json::from_str(&inspect_group_invite(&invite, &alice_card).unwrap()).unwrap();
    assert_eq!(info["group_id"], group_id);
    assert_eq!(info["group_name"], "Test Group");
}

#[test]
fn wasm_peer_announce_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let backup = alice_v["backup_text"].as_str().unwrap();
    let addresses = serde_json::to_string(&vec!["/ip4/127.0.0.1/tcp/4001"]).unwrap();
    let announce = create_peer_announce(
        backup,
        "alice pass",
        &addresses,
        Some("mailbox-key-1".into()),
        3600,
    )
    .unwrap();
    assert!(announce.starts_with("lm-peer-announce-v1:"));
    let info: Value = serde_json::from_str(
        &inspect_peer_announce(&announce, alice_v["identity_public_key"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(info["user_id"], alice_v["user_id"]);
    assert_eq!(info["addresses"][0], "/ip4/127.0.0.1/tcp/4001");
    assert_eq!(info["mailbox_key"], "mailbox-key-1");
}

#[test]
fn wasm_public_peer_announce_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let backup = alice_v["backup_text"].as_str().unwrap();
    let addresses = serde_json::to_string(&vec!["/dns4/bootstrap.example/tcp/443/wss"]).unwrap();
    let caps = serde_json::to_string(&vec!["bootstrap", "dht", "mailbox"]).unwrap();
    let announce = create_public_peer_announce(
        backup,
        "alice pass",
        "public-peer-1",
        &addresses,
        &caps,
        3600,
    )
    .unwrap();
    assert!(announce.starts_with("lm-public-peer-announce-v1:"));
    let info: Value = serde_json::from_str(
        &inspect_public_peer_announce(&announce, alice_v["identity_public_key"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(info["peer_id"], "public-peer-1");
    assert_eq!(info["user_id"], alice_v["user_id"]);
    assert_eq!(info["capabilities"].as_array().unwrap().len(), 3);
}

#[test]
fn wasm_mailbox_message_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let backup = alice_v["backup_text"].as_str().unwrap();
    let msg = create_mailbox_message(
        backup,
        "alice pass",
        bob_v["user_id"].as_str().unwrap(),
        "direct-envelope",
        "ciphertext-envelope",
        3600,
    )
    .unwrap();
    assert!(msg.starts_with("lm-mailbox-message-v1:"));
    let info: Value = serde_json::from_str(
        &inspect_mailbox_message(&msg, alice_v["identity_public_key"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(info["from_user_id"], alice_v["user_id"]);
    assert_eq!(info["to_user_id"], bob_v["user_id"]);
    assert_eq!(info["kind"], "DirectEnvelope");
    assert_eq!(info["ciphertext"], "ciphertext-envelope");
}

#[test]
fn wasm_message_receipt_smoke() {
    let alice = create_identity("alice receipt pass").unwrap();
    let bob = create_identity("bob receipt pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let alice_backup = alice_v["backup_text"].as_str().unwrap();
    let bob_backup = bob_v["backup_text"].as_str().unwrap();
    let msg = create_mailbox_message(
        alice_backup,
        "alice receipt pass",
        bob_v["user_id"].as_str().unwrap(),
        "direct-envelope",
        "ciphertext-envelope",
        3600,
    )
    .unwrap();
    let msg_info: Value = serde_json::from_str(
        &inspect_mailbox_message(&msg, alice_v["identity_public_key"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let receipt = create_message_receipt(
        bob_backup,
        "bob receipt pass",
        alice_v["user_id"].as_str().unwrap(),
        msg_info["message_id"].as_str().unwrap(),
        "conversation-1",
        Some("mailbox-delivery-1".into()),
        "delivered",
        3600,
    )
    .unwrap();
    assert!(receipt.starts_with("lm-message-receipt-v1:"));
    let receipt_info: Value = serde_json::from_str(
        &inspect_message_receipt(&receipt, bob_v["identity_public_key"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt_info["from_user_id"], bob_v["user_id"]);
    assert_eq!(receipt_info["to_user_id"], alice_v["user_id"]);
    assert_eq!(receipt_info["target_message_id"], msg_info["message_id"]);
    assert_eq!(receipt_info["kind"], "Delivered");

    let receipt_msg = create_mailbox_message(
        bob_backup,
        "bob receipt pass",
        alice_v["user_id"].as_str().unwrap(),
        "delivery-receipt",
        &receipt,
        3600,
    )
    .unwrap();
    let receipt_msg_info: Value = serde_json::from_str(
        &inspect_mailbox_message(&receipt_msg, bob_v["identity_public_key"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(receipt_msg_info["kind"], "DeliveryReceipt");
    assert_eq!(receipt_msg_info["ciphertext"], receipt);
}

#[test]
fn wasm_file_package_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let alice_backup = alice_v["backup_text"].as_str().unwrap();
    let bob_backup = bob_v["backup_text"].as_str().unwrap();
    let alice_card =
        export_contact_card(alice_backup, "alice pass", Some("Alice".into()), None).unwrap();
    let bob_card = export_contact_card(bob_backup, "bob pass", Some("Bob".into()), None).unwrap();
    let bytes = BASE64.encode(b"hello file from wasm");
    let package = create_file_package(
        alice_backup,
        "alice pass",
        &bob_card,
        "hello.txt",
        "text/plain",
        &bytes,
        8,
    )
    .unwrap();
    let info: Value = serde_json::from_str(&inspect_file_package(&package).unwrap()).unwrap();
    assert_eq!(info["manifest"]["name"], "hello.txt");
    assert_eq!(info["chunk_count"], 3);
    let plain: Value = serde_json::from_str(
        &decrypt_file_package(bob_backup, "bob pass", &alice_card, &package).unwrap(),
    )
    .unwrap();
    assert_eq!(plain["bytes_base64"], bytes);
    assert_eq!(plain["mime_type"], "text/plain");
}

#[test]
fn wasm_group_policy_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let alice_backup = alice_v["backup_text"].as_str().unwrap();
    let alice_card =
        export_contact_card(alice_backup, "alice pass", Some("Alice".into()), None).unwrap();
    let group_id = uuid::Uuid::new_v4().to_string();
    let policy = create_group_policy_state(
        &group_id,
        "Test",
        alice_v["user_id"].as_str().unwrap(),
        &serde_json::json!([alice_v["user_id"], bob_v["user_id"]]).to_string(),
    )
    .unwrap();
    let event = create_group_event(
        alice_backup,
        "alice pass",
        &group_id,
        1,
        &serde_json::json!({ "PromoteAdmin": { "user_id": bob_v["user_id"] } }).to_string(),
    )
    .unwrap();
    let updated: Value =
        serde_json::from_str(&apply_group_policy_event(&policy, &event, &alice_card).unwrap())
            .unwrap();
    assert_eq!(updated["sequence"], 1);
    assert!(updated["admins"].as_array().unwrap().len() >= 2);
}

#[test]
fn wasm_group_sender_key_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let alice_backup = alice_v["backup_text"].as_str().unwrap();
    let alice_card =
        export_contact_card(alice_backup, "alice pass", Some("Alice".into()), None).unwrap();
    let group_id = uuid::Uuid::new_v4().to_string();
    let created: Value = serde_json::from_str(
        &create_group_sender_key(alice_backup, "alice pass", &group_id).unwrap(),
    )
    .unwrap();
    let mut sender_state = created["state_json"].as_str().unwrap().to_string();
    let mut receiver_state =
        import_group_sender_key(created["distribution_text"].as_str().unwrap(), &alice_card)
            .unwrap();
    let enc: Value = serde_json::from_str(
        &group_sender_encrypt_text(&sender_state, "hello sender key").unwrap(),
    )
    .unwrap();
    sender_state = enc["state_json"].as_str().unwrap().to_string();
    assert!(!sender_state.is_empty());
    let dec: Value = serde_json::from_str(
        &group_sender_decrypt_text(&receiver_state, enc["envelope_json"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    receiver_state = dec["state_json"].as_str().unwrap().to_string();
    assert!(!receiver_state.is_empty());
    let plain: Value = serde_json::from_str(dec["plain_json"].as_str().unwrap()).unwrap();
    assert_eq!(plain["text"], "hello sender key");
}

#[test]
fn wasm_group_event_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let alice_backup = alice_v["backup_text"].as_str().unwrap();
    let alice_card =
        export_contact_card(alice_backup, "alice pass", Some("Alice".into()), None).unwrap();
    let group_id = uuid::Uuid::new_v4().to_string();
    let action = serde_json::json!({ "Rename": { "name": "Renamed" } }).to_string();
    let event = create_group_event(alice_backup, "alice pass", &group_id, 1, &action).unwrap();
    assert!(event.starts_with("lm-group-event-v1:"));
    let info: Value =
        serde_json::from_str(&inspect_group_event(&event, &alice_card).unwrap()).unwrap();
    assert_eq!(info["group_id"], group_id);
    assert_eq!(info["sequence"], 1);
}

#[test]
fn wasm_encrypt_decrypt_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let alice_backup = alice_v["backup_text"].as_str().unwrap();
    let bob_backup = bob_v["backup_text"].as_str().unwrap();
    let alice_card =
        export_contact_card(alice_backup, "alice pass", Some("Alice".into()), None).unwrap();
    let bob_card = export_contact_card(bob_backup, "bob pass", Some("Bob".into()), None).unwrap();
    let envelope =
        encrypt_text_message(alice_backup, "alice pass", &bob_card, "conv1", "hello").unwrap();
    let plain = decrypt_text_message(bob_backup, "bob pass", &alice_card, &envelope).unwrap();
    let plain_v: Value = serde_json::from_str(&plain).unwrap();
    assert_eq!(plain_v["sender_user_id"], alice_v["user_id"]);
    assert_eq!(plain_v["body"]["Text"]["text"], "hello");
}

#[test]
fn wasm_prekey_x3dh_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let alice_backup = alice_v["backup_text"].as_str().unwrap();
    let bob_backup = bob_v["backup_text"].as_str().unwrap();
    let prekey: Value =
        serde_json::from_str(&create_prekey_bundle(bob_backup, "bob pass", 11, 2, 3600).unwrap())
            .unwrap();
    let prekey_text = prekey["prekey_bundle_text"].as_str().unwrap();
    let info: Value = serde_json::from_str(&inspect_prekey_bundle(prekey_text).unwrap()).unwrap();
    assert_eq!(info["signed_prekey_id"], 11);
    assert_eq!(info["one_time_prekey_count"], 0);
    let signed_otks = prekey["signed_one_time_prekey_record_texts"]
        .as_array()
        .unwrap();
    assert_eq!(signed_otks.len(), 2);
    let init: Value = serde_json::from_str(
        &create_x3dh_initial_message_with_one_time_prekey_record(
            alice_backup,
            "alice pass",
            prekey_text,
            Some(signed_otks[0].as_str().unwrap().to_string()),
        )
        .unwrap(),
    )
    .unwrap();
    let initial_message: Value =
        serde_json::from_str(init["initial_message_json"].as_str().unwrap()).unwrap();
    assert_eq!(initial_message["one_time_prekey_id"], 0);
    let resp: Value = serde_json::from_str(
        &derive_x3dh_responder_secret(
            bob_backup,
            "bob pass",
            prekey["private_bundle_json"].as_str().unwrap(),
            init["initial_message_json"].as_str().unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(init["shared_secret"], resp["shared_secret"]);
}

#[test]
fn wasm_ratchet_session_with_keys_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let alice_pair: Value = serde_json::from_str(&create_ratchet_dh_keypair().unwrap()).unwrap();
    let bob_pair: Value = serde_json::from_str(&create_ratchet_dh_keypair().unwrap()).unwrap();
    let alice_state = create_ratchet_session_from_shared_secret_with_keys(
        alice_v["user_id"].as_str().unwrap(),
        bob_v["user_id"].as_str().unwrap(),
        "Initiator",
        &BASE64.encode([8u8; 32]),
        alice_pair["private_key"].as_str().unwrap(),
        bob_pair["public_key"].as_str().unwrap(),
    )
    .unwrap();
    let bob_state = create_ratchet_session_from_shared_secret_with_keys(
        bob_v["user_id"].as_str().unwrap(),
        alice_v["user_id"].as_str().unwrap(),
        "Responder",
        &BASE64.encode([8u8; 32]),
        bob_pair["private_key"].as_str().unwrap(),
        alice_pair["public_key"].as_str().unwrap(),
    )
    .unwrap();
    let enc: Value = serde_json::from_str(
        &ratchet_encrypt_text_message(&alice_state, "conv1", "hello keyed").unwrap(),
    )
    .unwrap();
    let dec: Value = serde_json::from_str(
        &ratchet_decrypt_text_message(&bob_state, enc["envelope_json"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let plain: Value = serde_json::from_str(dec["plain_json"].as_str().unwrap()).unwrap();
    assert_eq!(plain["body"]["Text"]["text"], "hello keyed");
}

#[test]
fn wasm_ratchet_envelope_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let init: Value = serde_json::from_str(
        &create_ratchet_session_from_shared_secret(
            alice_v["user_id"].as_str().unwrap(),
            bob_v["user_id"].as_str().unwrap(),
            &BASE64.encode([9u8; 32]),
        )
        .unwrap(),
    )
    .unwrap();
    let enc: Value = serde_json::from_str(
        &ratchet_encrypt_text_message(
            init["local_state_text"].as_str().unwrap(),
            "conv1",
            "hello ratchet wasm",
        )
        .unwrap(),
    )
    .unwrap();
    let dec: Value = serde_json::from_str(
        &ratchet_decrypt_text_message(
            init["remote_state_text"].as_str().unwrap(),
            enc["envelope_json"].as_str().unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let plain: Value = serde_json::from_str(dec["plain_json"].as_str().unwrap()).unwrap();
    assert_eq!(plain["body"]["Text"]["text"], "hello ratchet wasm");
}

#[test]
fn wasm_ratchet_supports_bidirectional_state_updates() {
    let alice: Value =
        serde_json::from_str(&create_identity("alice ratchet flow").unwrap()).unwrap();
    let bob: Value = serde_json::from_str(&create_identity("bob ratchet flow").unwrap()).unwrap();
    let pair: Value = serde_json::from_str(
        &create_ratchet_session_from_shared_secret(
            alice["user_id"].as_str().unwrap(),
            bob["user_id"].as_str().unwrap(),
            &BASE64.encode([41u8; 32]),
        )
        .unwrap(),
    )
    .unwrap();

    let alice_send: Value = serde_json::from_str(
        &ratchet_encrypt_text_message(
            pair["local_state_text"].as_str().unwrap(),
            "conversation-ratchet-flow",
            "Alice 1",
        )
        .unwrap(),
    )
    .unwrap();
    let bob_receive: Value = serde_json::from_str(
        &ratchet_decrypt_text_message(
            pair["remote_state_text"].as_str().unwrap(),
            alice_send["envelope_json"].as_str().unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let first_plain: Value =
        serde_json::from_str(bob_receive["plain_json"].as_str().unwrap()).unwrap();
    assert_eq!(first_plain["body"]["Text"]["text"], "Alice 1");

    let bob_send: Value = serde_json::from_str(
        &ratchet_encrypt_text_message(
            bob_receive["state_text"].as_str().unwrap(),
            "conversation-ratchet-flow",
            "Bob 1",
        )
        .unwrap(),
    )
    .unwrap();
    let alice_receive: Value = serde_json::from_str(
        &ratchet_decrypt_text_message(
            alice_send["state_text"].as_str().unwrap(),
            bob_send["envelope_json"].as_str().unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let reply_plain: Value =
        serde_json::from_str(alice_receive["plain_json"].as_str().unwrap()).unwrap();
    assert_eq!(reply_plain["body"]["Text"]["text"], "Bob 1");
}

#[test]
fn wasm_ratchet_state_smoke() {
    let alice = create_identity("alice pass").unwrap();
    let bob = create_identity("bob pass").unwrap();
    let alice_v: Value = serde_json::from_str(&alice).unwrap();
    let bob_v: Value = serde_json::from_str(&bob).unwrap();
    let alice_card = export_contact_card(
        alice_v["backup_text"].as_str().unwrap(),
        "alice pass",
        Some("Alice".into()),
        None,
    )
    .unwrap();
    let bob_card = export_contact_card(
        bob_v["backup_text"].as_str().unwrap(),
        "bob pass",
        Some("Bob".into()),
        None,
    )
    .unwrap();
    let pair: Value =
        serde_json::from_str(&create_ratchet_session_pair(&alice_card, &bob_card).unwrap())
            .unwrap();
    let alice_state = pair["local_state_text"].as_str().unwrap();
    let bob_state = pair["remote_state_text"].as_str().unwrap();
    let send: Value =
        serde_json::from_str(&ratchet_next_sending_key(alice_state).unwrap()).unwrap();
    let sent_key: Value = serde_json::from_str(send["key_json"].as_str().unwrap()).unwrap();
    let header_json = serde_json::to_string(&sent_key["header"]).unwrap();
    let recv: Value =
        serde_json::from_str(&ratchet_next_receiving_key(bob_state, &header_json).unwrap())
            .unwrap();
    let recv_key: Value = serde_json::from_str(recv["key_json"].as_str().unwrap()).unwrap();
    assert_eq!(sent_key["message_key"], recv_key["message_key"]);
    let info: Value =
        serde_json::from_str(&inspect_ratchet_state(send["state_text"].as_str().unwrap()).unwrap())
            .unwrap();
    assert_eq!(info["send_count"], 1);
}
