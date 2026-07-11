use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use lm_core::{
    ContactCard, FriendRequest, FriendResponse, Identity, MessageBody, PreKeyBundle,
    RatchetEnvelope, RatchetRole, RatchetSessionState, x3dh_initiator_secret,
    x3dh_responder_secret,
};

#[test]
fn two_users_friend_x3dh_ratchet_message_roundtrip() {
    let (alice, _) = Identity::create_with_passphrase("alice pineapple 2026").unwrap();
    let (bob, _) = Identity::create_with_passphrase("bob pineapple 2026").unwrap();

    let alice_card = alice
        .export_contact_card(Some("Alice".into()), None, vec![])
        .unwrap();
    let bob_card = bob
        .export_contact_card(Some("Bob".into()), None, vec![])
        .unwrap();
    alice_card.verify().unwrap();
    bob_card.verify().unwrap();

    let request = FriendRequest::new(
        &alice,
        bob.user_id().clone(),
        alice_card.clone(),
        Some("hello".into()),
        3600,
    )
    .unwrap();
    request.verify().unwrap();
    let response = FriendResponse::accept(&bob, &request).unwrap();
    response.verify(&bob_card).unwrap();
    assert_eq!(response.to_user_id, *alice.user_id());

    let (bob_prekey_bundle, bob_private_prekeys) = PreKeyBundle::new(&bob, 7, 3, 3600).unwrap();
    bob_prekey_bundle.verify().unwrap();

    let x3dh = x3dh_initiator_secret(&alice, &bob_prekey_bundle).unwrap();
    let bob_shared =
        x3dh_responder_secret(&bob, &bob_private_prekeys, &x3dh.initial_message).unwrap();
    assert_eq!(x3dh.shared_secret, BASE64.encode(bob_shared));
    let shared: [u8; 32] = BASE64
        .decode(x3dh.shared_secret.as_bytes())
        .unwrap()
        .try_into()
        .unwrap();

    let alice_dh = RatchetSessionState::generate_dh_keypair().unwrap();
    let bob_dh = RatchetSessionState::generate_dh_keypair().unwrap();
    let mut alice_state = RatchetSessionState::from_shared_secret_export(
        alice.user_id().clone(),
        bob.user_id().clone(),
        RatchetRole::Initiator,
        &BASE64.encode(shared),
        &alice_dh.private_key,
        &bob_dh.public_key,
    )
    .unwrap();
    let mut bob_state = RatchetSessionState::from_shared_secret_export(
        bob.user_id().clone(),
        alice.user_id().clone(),
        RatchetRole::Responder,
        &BASE64.encode(shared),
        &bob_dh.private_key,
        &alice_dh.public_key,
    )
    .unwrap();

    let envelope = RatchetEnvelope::encrypt_text(
        &mut alice_state,
        format!("conv-{}", bob.user_id()),
        "hello over double ratchet".into(),
    )
    .unwrap();
    let plain = envelope.decrypt(&mut bob_state).unwrap();
    assert_eq!(plain.sender_user_id, *alice.user_id());
    assert_eq!(
        plain.body,
        MessageBody::Text {
            text: "hello over double ratchet".into()
        }
    );

    let reply = RatchetEnvelope::encrypt_text(
        &mut bob_state,
        format!("conv-{}", alice.user_id()),
        "reply over double ratchet".into(),
    )
    .unwrap();
    let plain_reply = reply.decrypt(&mut alice_state).unwrap();
    assert_eq!(plain_reply.sender_user_id, *bob.user_id());
    assert_eq!(
        plain_reply.body,
        MessageBody::Text {
            text: "reply over double ratchet".into()
        }
    );
}

#[test]
fn exported_contact_and_prekey_text_roundtrip() {
    let (alice, _) = Identity::create_with_passphrase("alice export").unwrap();
    let card = alice.export_contact_card(None, None, vec![]).unwrap();
    let card_text = card.to_export_text().unwrap();
    let imported_card = ContactCard::from_export_text(&card_text).unwrap();
    imported_card.verify().unwrap();
    assert_eq!(imported_card.user_id, *alice.user_id());

    let (prekey, private) = PreKeyBundle::new(&alice, 1, 1, 3600).unwrap();
    private.validate_for_public(&prekey).unwrap();
    let text = prekey.to_export_text().unwrap();
    let imported = PreKeyBundle::from_export_text(&text).unwrap();
    assert_eq!(imported.user_id, *alice.user_id());
}
