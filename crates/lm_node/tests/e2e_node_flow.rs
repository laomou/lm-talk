use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    Identity, MailboxMessage, MailboxMessageKind, PreKeyBundle, RatchetEnvelope, RatchetRole,
    RatchetSessionState, x3dh_initiator_secret, x3dh_responder_secret,
};
use lm_node::{ControlRequest, NativeNode, NodeConfig, NodeStateSnapshot};

#[test]
fn node_prekey_sync_mailbox_ratchet_e2e() {
    let (alice, _) = Identity::create_with_passphrase("alice node e2e").unwrap();
    let (bob, _) = Identity::create_with_passphrase("bob node e2e").unwrap();
    let mut node_a = NativeNode::new(NodeConfig {
        peer_id: "node-a".into(),
        ..Default::default()
    });
    let mut node_b = NativeNode::new(NodeConfig {
        peer_id: "node-b".into(),
        ..Default::default()
    });

    // Bob publishes a prekey bundle to node B.
    let (bob_prekey, bob_private) = PreKeyBundle::new(&bob, 5, 1, 3600).unwrap();
    let publish = node_b.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/prekey/publish".into(),
        body: serde_json::json!({ "prekey_bundle_text": bob_prekey.to_export_text().unwrap() })
            .to_string(),
    });
    assert_eq!(publish.status, 201);

    // Node A syncs from node B and can serve Bob's prekey.
    let snapshot: NodeStateSnapshot = serde_json::from_str(
        &node_b
            .handle_control_request(ControlRequest {
                method: "GET".into(),
                path: "/sync/snapshot".into(),
                body: String::new(),
            })
            .body,
    )
    .unwrap();
    let import = node_a.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/sync/import".into(),
        body: serde_json::json!({ "snapshot": snapshot }).to_string(),
    });
    assert_eq!(import.status, 200);

    let get = node_a.handle_control_request(ControlRequest {
        method: "GET".into(),
        path: format!("/prekey/get?user_id={}", bob.user_id()),
        body: String::new(),
    });
    assert_eq!(get.status, 200);
    let get_body: serde_json::Value = serde_json::from_str(&get.body).unwrap();
    let fetched_prekey =
        PreKeyBundle::from_export_text(get_body["prekey_bundle_text"].as_str().unwrap()).unwrap();

    // Alice and Bob establish ratchet states using the fetched prekey.
    let x3dh = x3dh_initiator_secret(&alice, &fetched_prekey).unwrap();
    let bob_shared = x3dh_responder_secret(&bob, &bob_private, &x3dh.initial_message).unwrap();
    assert_eq!(x3dh.shared_secret, BASE64.encode(bob_shared));
    let alice_dh = RatchetSessionState::generate_dh_keypair().unwrap();
    let bob_dh = RatchetSessionState::generate_dh_keypair().unwrap();
    let mut alice_state = RatchetSessionState::from_shared_secret_export(
        alice.user_id().clone(),
        bob.user_id().clone(),
        RatchetRole::Initiator,
        &x3dh.shared_secret,
        &alice_dh.private_key,
        &bob_dh.public_key,
    )
    .unwrap();
    let mut bob_state = RatchetSessionState::from_shared_secret_export(
        bob.user_id().clone(),
        alice.user_id().clone(),
        RatchetRole::Responder,
        &x3dh.shared_secret,
        &bob_dh.private_key,
        &alice_dh.public_key,
    )
    .unwrap();

    // Alice sends a ratchet envelope via node A mailbox.
    let envelope = RatchetEnvelope::encrypt_text(
        &mut alice_state,
        "conv-node".into(),
        "hello via node".into(),
    )
    .unwrap();
    let envelope_json = serde_json::to_string(&envelope).unwrap();
    let mailbox = MailboxMessage::new(
        &alice,
        bob.user_id().clone(),
        MailboxMessageKind::DirectEnvelope,
        envelope_json,
        3600,
    )
    .unwrap();
    let push = node_a.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/mailbox/push".into(),
        body: serde_json::json!({
            "message_text": mailbox.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        })
        .to_string(),
    });
    assert_eq!(push.status, 201);

    // Node B syncs from node A, Bob takes and decrypts.
    let snapshot: NodeStateSnapshot = serde_json::from_str(
        &node_a
            .handle_control_request(ControlRequest {
                method: "GET".into(),
                path: "/sync/snapshot".into(),
                body: String::new(),
            })
            .body,
    )
    .unwrap();
    let import = node_b.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/sync/import".into(),
        body: serde_json::json!({ "snapshot": snapshot }).to_string(),
    });
    assert_eq!(import.status, 200);

    let take = node_b.handle_control_request(ControlRequest {
        method: "GET".into(),
        path: format!("/mailbox/take?user_id={}", bob.user_id()),
        body: String::new(),
    });
    assert_eq!(take.status, 200);
    let body: serde_json::Value = serde_json::from_str(&take.body).unwrap();
    let ciphertext = body["messages"][0]["message"]["ciphertext"]
        .as_str()
        .unwrap();
    let received: RatchetEnvelope = serde_json::from_str(ciphertext).unwrap();
    let plain = received.decrypt(&mut bob_state).unwrap();
    assert_eq!(plain.sender_user_id, *alice.user_id());
}
