use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    GroupSenderEnvelope, GroupSenderKeyState, Identity, MailboxMessage, MailboxMessageKind,
    PreKeyBundle, RatchetEnvelope, RatchetRole, RatchetSessionState, SignedOneTimePreKeyRecord,
    x3dh_initiator_secret_with_one_time_prekey_record, x3dh_responder_secret,
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
    let (bob_prekey, bob_private, bob_signed_otks) =
        PreKeyBundle::new_with_signed_one_time_prekey_records(&bob, 5, 1, 3600).unwrap();
    let publish = node_b.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/prekey/publish".into(),
        body: serde_json::json!({
            "prekey_bundle_text": bob_prekey.to_export_text().unwrap(),
            "signed_one_time_prekey_record_texts": bob_signed_otks
                .iter()
                .map(|record| record.to_export_text().unwrap())
                .collect::<Vec<_>>(),
        })
        .to_string(),
        headers: Vec::new(),
    });
    assert_eq!(publish.status, 201);

    // Node A syncs from node B and can serve Bob's prekey.
    let snapshot: NodeStateSnapshot = serde_json::from_str(
        &node_b
            .handle_control_request(ControlRequest {
                method: "GET".into(),
                path: "/sync/snapshot".into(),
                body: String::new(),
                headers: Vec::new(),
            })
            .body,
    )
    .unwrap();
    let import = node_a.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/sync/import".into(),
        body: serde_json::json!({ "snapshot": snapshot }).to_string(),
        headers: Vec::new(),
    });
    assert_eq!(import.status, 200);

    let get = node_a.handle_control_request(ControlRequest {
        method: "GET".into(),
        path: format!("/prekey/get?user_id={}", bob.user_id()),
        body: String::new(),
        headers: Vec::new(),
    });
    assert_eq!(get.status, 200);
    let get_body: serde_json::Value = serde_json::from_str(&get.body).unwrap();
    let fetched_prekey =
        PreKeyBundle::from_export_text(get_body["prekey_bundle_text"].as_str().unwrap()).unwrap();
    let selected_record = SignedOneTimePreKeyRecord::from_export_text(
        get_body["selected_signed_one_time_prekey_record_text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();

    // Alice and Bob establish ratchet states using the fetched prekey.
    let x3dh = x3dh_initiator_secret_with_one_time_prekey_record(
        &alice,
        &fetched_prekey,
        Some(&selected_record),
    )
    .unwrap();
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
        headers: Vec::new(),
    });
    assert_eq!(push.status, 201);

    // Node B syncs from node A, Bob takes and decrypts.
    let snapshot: NodeStateSnapshot = serde_json::from_str(
        &node_a
            .handle_control_request(ControlRequest {
                method: "GET".into(),
                path: "/sync/snapshot".into(),
                body: String::new(),
                headers: Vec::new(),
            })
            .body,
    )
    .unwrap();
    let import = node_b.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/sync/import".into(),
        body: serde_json::json!({ "snapshot": snapshot }).to_string(),
        headers: Vec::new(),
    });
    assert_eq!(import.status, 200);

    let take = node_b.handle_control_request(ControlRequest {
        method: "GET".into(),
        path: format!("/mailbox/take?user_id={}", bob.user_id()),
        body: String::new(),
        headers: Vec::new(),
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

#[test]
fn mailbox_pressure_partial_ack_status_and_snapshot_recovery() {
    let (alice, _) = Identity::create_with_passphrase("mailbox pressure alice").unwrap();
    let (bob, _) = Identity::create_with_passphrase("mailbox pressure bob").unwrap();
    let mut node = NativeNode::new(NodeConfig {
        peer_id: "mailbox-pressure-node".into(),
        ..Default::default()
    });

    for i in 0..120 {
        let mailbox = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            format!("pressure-envelope-{i}"),
            3600,
        )
        .unwrap();
        let push = node.handle_control_request(ControlRequest {
            method: "POST".into(),
            path: "/mailbox/push".into(),
            body: serde_json::json!({
                "message_text": mailbox.to_export_text().unwrap(),
                "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
            })
            .to_string(),
            headers: Vec::new(),
        });
        assert_eq!(push.status, 201, "push {i}: {}", push.body);
    }

    let take = node.handle_control_request(ControlRequest {
        method: "GET".into(),
        path: format!("/mailbox/take?user_id={}&limit=25", bob.user_id()),
        body: String::new(),
        headers: Vec::new(),
    });
    assert_eq!(take.status, 200, "{}", take.body);
    let take_body: serde_json::Value = serde_json::from_str(&take.body).unwrap();
    assert_eq!(take_body["returned"], 25);
    assert_eq!(take_body["pending"], 120);
    assert_eq!(take_body["more"], true);
    let deliveries = take_body["messages"].as_array().unwrap();
    let acked_ids = deliveries
        .iter()
        .take(10)
        .map(|delivery| delivery["delivery_id"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    let unacked_id = deliveries[10]["delivery_id"].as_str().unwrap().to_string();

    let ack = node.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/mailbox/ack".into(),
        body: serde_json::json!({
            "user_id": bob.user_id().to_string(),
            "delivery_ids": acked_ids,
        })
        .to_string(),
        headers: Vec::new(),
    });
    assert_eq!(ack.status, 200, "{}", ack.body);
    let ack_body: serde_json::Value = serde_json::from_str(&ack.body).unwrap();
    assert_eq!(ack_body["removed"], 10);
    assert_eq!(ack_body["pending"], 110);

    let acked_status = node.handle_control_request(ControlRequest {
        method: "GET".into(),
        path: format!(
            "/mailbox/status?user_id={}&delivery_id={}",
            bob.user_id(),
            deliveries[0]["delivery_id"].as_str().unwrap()
        ),
        body: String::new(),
        headers: Vec::new(),
    });
    assert_eq!(acked_status.status, 200, "{}", acked_status.body);
    let acked_status_body: serde_json::Value = serde_json::from_str(&acked_status.body).unwrap();
    assert_eq!(acked_status_body["delivery"]["status"], "acked");
    assert!(acked_status_body["delivery"]["acked_at"].as_u64().is_some());

    let unacked_status = node.handle_control_request(ControlRequest {
        method: "GET".into(),
        path: format!(
            "/mailbox/status?user_id={}&delivery_id={unacked_id}",
            bob.user_id()
        ),
        body: String::new(),
        headers: Vec::new(),
    });
    assert_eq!(unacked_status.status, 200, "{}", unacked_status.body);
    let unacked_status_body: serde_json::Value =
        serde_json::from_str(&unacked_status.body).unwrap();
    assert_eq!(
        unacked_status_body["delivery"]["status"],
        "delivered_unacked"
    );
    assert_eq!(unacked_status_body["summary"]["total"], 110);
    assert_eq!(unacked_status_body["summary"]["delivered_unacked"], 15);

    let restored = NativeNode::from_state_snapshot(node.to_state_snapshot());
    let restored_acked = restored.mailbox.delivery_status(
        bob.user_id(),
        deliveries[0]["delivery_id"].as_str().unwrap(),
    );
    assert_eq!(restored_acked.status, lm_node::MailboxDeliveryState::Acked);
    let restored_unacked = restored.mailbox.delivery_status(bob.user_id(), &unacked_id);
    assert_eq!(
        restored_unacked.status,
        lm_node::MailboxDeliveryState::DeliveredUnacked
    );
    assert_eq!(restored.mailbox.pending_for(bob.user_id()), 110);
}

#[test]
fn group_sender_key_fanout_via_mailbox() {
    // 1. Create 3 identities: Alice, Bob, Carol
    let (alice, _) = Identity::create_with_passphrase("alice group fanout").unwrap();
    let (bob, _) = Identity::create_with_passphrase("bob group fanout").unwrap();
    let (carol, _) = Identity::create_with_passphrase("carol group fanout").unwrap();

    let alice_card = alice
        .export_contact_card(Some("Alice".into()), None, vec![])
        .unwrap();

    let mut node = NativeNode::new(NodeConfig {
        peer_id: "group-fanout-node".into(),
        ..Default::default()
    });

    let group_id = uuid::Uuid::new_v4();

    // 2. Alice creates a GroupSenderKeyState and generates her distribution
    let mut alice_sender_state = GroupSenderKeyState::new(&alice, group_id).unwrap();
    let distribution = alice_sender_state.to_distribution(&alice).unwrap();

    // 3. Bob and Carol import Alice's distribution (verify signature using Alice's contact card)
    let mut bob_receiver_state =
        GroupSenderKeyState::from_distribution(&distribution, &alice_card).unwrap();
    let mut carol_receiver_state =
        GroupSenderKeyState::from_distribution(&distribution, &alice_card).unwrap();

    // 4. Alice encrypts a group message using her sender key state
    let envelope = alice_sender_state
        .encrypt_text("hello group via mailbox".into())
        .unwrap();
    let envelope_json = serde_json::to_string(&envelope).unwrap();

    // 5. Alice pushes the encrypted envelope to Bob and Carol's mailboxes
    let bob_mailbox_msg = MailboxMessage::new(
        &alice,
        bob.user_id().clone(),
        MailboxMessageKind::GroupFanout,
        envelope_json.clone(),
        3600,
    )
    .unwrap();
    let push_bob = node.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/mailbox/push".into(),
        body: serde_json::json!({
            "message_text": bob_mailbox_msg.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        })
        .to_string(),
        headers: Vec::new(),
    });
    assert_eq!(push_bob.status, 201, "push to bob: {}", push_bob.body);

    let carol_mailbox_msg = MailboxMessage::new(
        &alice,
        carol.user_id().clone(),
        MailboxMessageKind::GroupFanout,
        envelope_json,
        3600,
    )
    .unwrap();
    let push_carol = node.handle_control_request(ControlRequest {
        method: "POST".into(),
        path: "/mailbox/push".into(),
        body: serde_json::json!({
            "message_text": carol_mailbox_msg.to_export_text().unwrap(),
            "from_identity_public_key": BASE64.encode(alice.identity_public_key()),
        })
        .to_string(),
        headers: Vec::new(),
    });
    assert_eq!(
        push_carol.status, 201,
        "push to carol: {}",
        push_carol.body
    );

    // 6. Bob takes from mailbox, decrypts the group message
    let take_bob = node.handle_control_request(ControlRequest {
        method: "GET".into(),
        path: format!("/mailbox/take?user_id={}", bob.user_id()),
        body: String::new(),
        headers: Vec::new(),
    });
    assert_eq!(take_bob.status, 200, "take bob: {}", take_bob.body);
    let bob_body: serde_json::Value = serde_json::from_str(&take_bob.body).unwrap();
    let bob_ciphertext = bob_body["messages"][0]["message"]["ciphertext"]
        .as_str()
        .unwrap();
    let bob_envelope: GroupSenderEnvelope = serde_json::from_str(bob_ciphertext).unwrap();
    let bob_plain = bob_receiver_state.decrypt(&bob_envelope).unwrap();

    // 7. Carol takes from mailbox, decrypts the same group message
    let take_carol = node.handle_control_request(ControlRequest {
        method: "GET".into(),
        path: format!("/mailbox/take?user_id={}", carol.user_id()),
        body: String::new(),
        headers: Vec::new(),
    });
    assert_eq!(
        take_carol.status, 200,
        "take carol: {}",
        take_carol.body
    );
    let carol_body: serde_json::Value = serde_json::from_str(&take_carol.body).unwrap();
    let carol_ciphertext = carol_body["messages"][0]["message"]["ciphertext"]
        .as_str()
        .unwrap();
    let carol_envelope: GroupSenderEnvelope = serde_json::from_str(carol_ciphertext).unwrap();
    let carol_plain = carol_receiver_state.decrypt(&carol_envelope).unwrap();

    // 8. Assert both get the same plaintext
    assert_eq!(bob_plain.text, "hello group via mailbox");
    assert_eq!(carol_plain.text, "hello group via mailbox");
    assert_eq!(bob_plain.group_id, group_id);
    assert_eq!(carol_plain.group_id, group_id);
    assert_eq!(bob_plain.sender_user_id, *alice.user_id());
    assert_eq!(carol_plain.sender_user_id, *alice.user_id());
    assert_eq!(bob_plain.message_id, carol_plain.message_id);
}
