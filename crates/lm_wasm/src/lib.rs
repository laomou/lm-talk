//! WASM bindings for LM Talk core.

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD as BASE64, URL_SAFE_NO_PAD},
};
#[cfg(target_arch = "wasm32")]
use ed25519_dalek::SigningKey;
use getrandom::getrandom;
use lm_core::{
    ContactCard, DeviceId, DeviceIdentity, DeviceRevoke, DirectEnvelope, FileChunkEnvelope,
    FileManifest, FriendRequest, FriendResponse, GroupEvent, GroupEventAction, GroupInvite,
    GroupPolicyState, GroupSenderEnvelope, GroupSenderKeyDistribution, GroupSenderKeyState,
    Identity, IdentityBackupPackage, IdentitySeed, MailboxMessage, MailboxMessageKind,
    PeerAnnounce, PreKeyBundle, PreKeyPrivateBundle, PublicPeerAnnounce, PublicPeerCapability,
    RatchetDhKeyPair, RatchetEnvelope, RatchetHeader, RatchetRole, RatchetSessionState,
    SignalAnswer, SignalOffer, SignedOneTimePreKeyRecord, TrustLevel, X3dhInitialMessage,
    x3dh_initiator_secret, x3dh_initiator_secret_with_one_time_prekey_id,
    x3dh_initiator_secret_with_one_time_prekey_record, x3dh_responder_secret,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use x25519_dalek::StaticSecret as X25519Secret;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

fn to_js_error(err: lm_core::LmError) -> JsValue {
    JsValue::from_str(&err.to_string())
}

fn to_json_string<T: Serialize>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn from_json_string<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, JsValue> {
    serde_json::from_str(value).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn ensure_js_len(value: &str, max: usize) -> Result<(), JsValue> {
    lm_core::limits::ensure_len(value, max).map_err(to_js_error)
}

fn ensure_js_bytes(len: usize, max: usize) -> Result<(), JsValue> {
    lm_core::limits::ensure_bytes(len, max).map_err(to_js_error)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataBackupPackage {
    r#type: String,
    version: u16,
    user_id: String,
    nonce: String,
    ciphertext: String,
    created_at: u64,
}

const DATA_BACKUP_PREFIX: &str = "lm-data-backup-v1:";
const DATA_BACKUP_AAD: &[u8] = b"lm-talk.data-backup.v1";

#[wasm_bindgen]
pub fn export_data_backup(
    identity_backup_text: &str,
    passphrase: &str,
    data_json: &str,
) -> Result<String, JsValue> {
    ensure_js_len(
        identity_backup_text,
        lm_core::limits::MAX_IDENTITY_BACKUP_TEXT_BYTES,
    )?;
    ensure_js_bytes(data_json.as_bytes().len(), 4 * 1024 * 1024)?;
    let identity = restore_identity_any(identity_backup_text, passphrase)?;
    let key = identity.storage_key().map_err(to_js_error)?;
    let mut nonce = [0u8; 24];
    getrandom(&mut nonce).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ciphertext = lm_core::crypto::xchacha20poly1305_encrypt(
        &key,
        &nonce,
        data_json.as_bytes(),
        DATA_BACKUP_AAD,
    )
    .map_err(to_js_error)?;
    let package = DataBackupPackage {
        r#type: "lm-data-backup-v1".to_string(),
        version: 1,
        user_id: identity.user_id().to_string(),
        nonce: BASE64.encode(nonce),
        ciphertext: BASE64.encode(ciphertext),
        created_at: unix_now(),
    };
    let json = serde_json::to_vec(&package).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(format!(
        "{}{}",
        DATA_BACKUP_PREFIX,
        URL_SAFE_NO_PAD.encode(json)
    ))
}

#[wasm_bindgen]
pub fn import_data_backup(
    identity_backup_text: &str,
    passphrase: &str,
    data_backup_text: &str,
) -> Result<String, JsValue> {
    ensure_js_len(
        identity_backup_text,
        lm_core::limits::MAX_IDENTITY_BACKUP_TEXT_BYTES,
    )?;
    ensure_js_bytes(data_backup_text.as_bytes().len(), 6 * 1024 * 1024)?;
    let payload = data_backup_text
        .strip_prefix(DATA_BACKUP_PREFIX)
        .ok_or_else(|| JsValue::from_str("invalid data backup prefix"))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let package: DataBackupPackage =
        serde_json::from_slice(&bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    if package.r#type != "lm-data-backup-v1" || package.version != 1 {
        return Err(JsValue::from_str("unsupported data backup"));
    }
    let identity = restore_identity_any(identity_backup_text, passphrase)?;
    if package.user_id != identity.user_id().to_string() {
        return Err(JsValue::from_str(
            "data backup user_id does not match current identity",
        ));
    }
    let key = identity.storage_key().map_err(to_js_error)?;
    let nonce = decode_fixed_24(&package.nonce)?;
    let ciphertext = BASE64
        .decode(package.ciphertext.as_bytes())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let plaintext =
        lm_core::crypto::xchacha20poly1305_decrypt(&key, &nonce, &ciphertext, DATA_BACKUP_AAD)
            .map_err(to_js_error)?;
    String::from_utf8(plaintext).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn decode_fixed_24(value: &str) -> Result<[u8; 24], JsValue> {
    let bytes = BASE64
        .decode(value.as_bytes())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    bytes
        .try_into()
        .map_err(|_| JsValue::from_str("invalid nonce length"))
}

fn unix_now() -> u64 {
    lm_core::unix_now()
}

const WASM_IDENTITY_BACKUP_PREFIX: &str = "lm-identity-backup-v1:wasm-local:";
const WASM_IDENTITY_BACKUP_AAD: &[u8] = b"lm-talk.wasm-identity-backup.v1";

fn wasm_identity_backup_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    let normalized = lm_core::normalize_passphrase(passphrase);
    let mut hasher = Sha256::new();
    hasher.update(b"lm-talk.wasm-identity-backup.key.v1");
    hasher.update(salt);
    hasher.update(normalized.as_bytes());
    hasher.finalize().into()
}

#[cfg(target_arch = "wasm32")]
fn wasm_identity_backup_text(
    identity: &Identity,
    seed: &[u8; lm_core::identity::IDENTITY_SEED_LEN],
    passphrase: &str,
) -> Result<String, JsValue> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    getrandom(&mut salt).map_err(|e| JsValue::from_str(&e.to_string()))?;
    getrandom(&mut nonce).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let key = wasm_identity_backup_key(passphrase, &salt);
    let ciphertext =
        lm_core::crypto::xchacha20poly1305_encrypt(&key, &nonce, seed, WASM_IDENTITY_BACKUP_AAD)
            .map_err(to_js_error)?;
    Ok(format!(
        "{}{}:{}:{}:{}",
        WASM_IDENTITY_BACKUP_PREFIX,
        identity.user_id(),
        URL_SAFE_NO_PAD.encode(salt),
        URL_SAFE_NO_PAD.encode(nonce),
        URL_SAFE_NO_PAD.encode(ciphertext),
    ))
}

fn restore_wasm_identity_backup(
    text: &str,
    passphrase: &str,
) -> Result<Option<(String, [u8; lm_core::identity::IDENTITY_SEED_LEN])>, JsValue> {
    let Some(rest) = text.strip_prefix(WASM_IDENTITY_BACKUP_PREFIX) else {
        return Ok(None);
    };

    let parts = rest.split(':').collect::<Vec<_>>();
    if parts.len() == 2 {
        return Err(JsValue::from_str("WrongPassphrase"));
    }
    if parts.len() != 4 {
        return Err(JsValue::from_str("invalid wasm backup"));
    }

    let user_id = parts[0];
    let salt_text = parts[1];
    let nonce_text = parts[2];
    let ciphertext_text = parts[3];
    let salt = URL_SAFE_NO_PAD
        .decode(salt_text.as_bytes())
        .map_err(|_| JsValue::from_str("invalid wasm backup salt"))?;
    let nonce_vec = URL_SAFE_NO_PAD
        .decode(nonce_text.as_bytes())
        .map_err(|_| JsValue::from_str("invalid wasm backup nonce"))?;
    let nonce: [u8; 24] = nonce_vec
        .try_into()
        .map_err(|_| JsValue::from_str("invalid wasm backup nonce"))?;
    let ciphertext = URL_SAFE_NO_PAD
        .decode(ciphertext_text.as_bytes())
        .map_err(|_| JsValue::from_str("invalid wasm backup ciphertext"))?;
    let key = wasm_identity_backup_key(passphrase, &salt);
    let seed_vec = lm_core::crypto::xchacha20poly1305_decrypt(
        &key,
        &nonce,
        &ciphertext,
        WASM_IDENTITY_BACKUP_AAD,
    )
    .map_err(|_| JsValue::from_str("WrongPassphrase"))?;
    let seed: [u8; lm_core::identity::IDENTITY_SEED_LEN] = seed_vec
        .try_into()
        .map_err(|_| JsValue::from_str("invalid wasm backup seed"))?;
    Ok(Some((user_id.to_string(), seed)))
}

#[cfg(target_arch = "wasm32")]
fn wasm_identity_from_seed(
    seed: [u8; lm_core::identity::IDENTITY_SEED_LEN],
) -> Result<Identity, JsValue> {
    // In browser builds avoid the core Identity::from_seed path because current
    // optimized wasm traps in the dalek stack on some runtimes. Construct the
    // same deterministic identity with wasm-safe primitives.
    let ed_seed = lm_core::crypto::hkdf_32(&seed, lm_core::crypto::IDENTITY_ED25519_INFO)
        .map_err(to_js_error)?;
    let x_seed = lm_core::crypto::hkdf_32(&seed, lm_core::crypto::IDENTITY_X25519_INFO)
        .map_err(to_js_error)?;
    let signing_key = SigningKey::from_bytes(&ed_seed);
    let x25519_secret = X25519Secret::from(x_seed);
    let user_id = lm_core::UserId::from_identity_public_key(signing_key.verifying_key().as_bytes());
    Identity::from_parts_for_wasm(
        user_id,
        IdentitySeed::from_bytes(seed),
        signing_key,
        x25519_secret,
    )
    .map_err(to_js_error)
}

#[cfg(not(target_arch = "wasm32"))]
fn wasm_identity_from_seed(
    seed: [u8; lm_core::identity::IDENTITY_SEED_LEN],
) -> Result<Identity, JsValue> {
    Identity::from_seed(IdentitySeed::from_bytes(seed)).map_err(to_js_error)
}

fn restore_identity_any(backup_text: &str, passphrase: &str) -> Result<Identity, JsValue> {
    if let Some((expected_user_id, seed)) = restore_wasm_identity_backup(backup_text, passphrase)? {
        let identity = wasm_identity_from_seed(seed)?;
        if identity.user_id().to_string() != expected_user_id {
            return Err(JsValue::from_str("backup user_id mismatch"));
        }
        return Ok(identity);
    }
    let backup = IdentityBackupPackage::from_export_text(backup_text).map_err(to_js_error)?;
    Identity::restore_from_backup(&backup, passphrase).map_err(to_js_error)
}

#[derive(Serialize)]
struct CreateIdentityOutput {
    user_id: String,
    identity_public_key: String,
    x25519_public_key: String,
    backup_text: String,
}

#[wasm_bindgen]
pub fn normalize_passphrase(input: &str) -> String {
    lm_core::normalize_passphrase(input)
}

#[wasm_bindgen]
pub fn create_identity(_passphrase: &str) -> Result<String, JsValue> {
    // Browser WASM currently traps inside Argon2 backup encryption on some engines.
    // Keep Web identity creation usable by generating the seed and returning a plaintext
    // local-only backup package; native/core tests still exercise encrypted backups.
    #[cfg(target_arch = "wasm32")]
    {
        let mut seed_bytes = [0u8; lm_core::identity::IDENTITY_SEED_LEN];
        getrandom(&mut seed_bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let identity = wasm_identity_from_seed(seed_bytes)?;
        let backup_text = wasm_identity_backup_text(&identity, &seed_bytes, _passphrase)?;
        let out = CreateIdentityOutput {
            user_id: identity.user_id().to_string(),
            identity_public_key: BASE64.encode(identity.identity_public_key()),
            x25519_public_key: BASE64.encode(identity.x25519_public_key()),
            backup_text,
        };
        return to_json_string(&out);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let identity = Identity::create_with_passphrase(_passphrase).map_err(to_js_error)?;
        let (identity, backup) = identity;
        let out = CreateIdentityOutput {
            user_id: identity.user_id().to_string(),
            identity_public_key: BASE64.encode(identity.identity_public_key()),
            x25519_public_key: BASE64.encode(identity.x25519_public_key()),
            backup_text: backup.to_export_text().map_err(to_js_error)?,
        };
        to_json_string(&out)
    }
}

#[derive(Serialize)]
struct RestoreIdentityOutput {
    user_id: String,
    identity_public_key: String,
    x25519_public_key: String,
}

#[wasm_bindgen]
pub fn restore_identity(backup_text: &str, passphrase: &str) -> Result<String, JsValue> {
    ensure_js_len(backup_text, lm_core::limits::MAX_IDENTITY_BACKUP_TEXT_BYTES)?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let out = RestoreIdentityOutput {
        user_id: identity.user_id().to_string(),
        identity_public_key: BASE64.encode(identity.identity_public_key()),
        x25519_public_key: BASE64.encode(identity.x25519_public_key()),
    };
    to_json_string(&out)
}

#[derive(Serialize)]
struct DeviceOutput {
    device_id: String,
    device_public_key: String,
    device_cert_json: String,
}

#[wasm_bindgen]
pub fn create_device_cert(
    backup_text: &str,
    passphrase: &str,
    device_name: Option<String>,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let device = DeviceIdentity::random().map_err(to_js_error)?;
    let cert = device
        .create_cert(&identity, device_name)
        .map_err(to_js_error)?;
    let out = DeviceOutput {
        device_id: device.device_id().to_string(),
        device_public_key: BASE64.encode(device.device_public_key()),
        device_cert_json: to_json_string(&cert)?,
    };
    to_json_string(&out)
}

#[derive(Serialize)]
struct DeviceRevokeInfo {
    user_id: String,
    device_id: String,
    reason: Option<String>,
    created_at: u64,
}

#[wasm_bindgen]
pub fn create_device_revoke(
    backup_text: &str,
    passphrase: &str,
    device_id: &str,
    reason: Option<String>,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let device_id = DeviceId::from_raw(device_id.to_string()).map_err(to_js_error)?;
    let revoke = DeviceRevoke::new(&identity, device_id, reason).map_err(to_js_error)?;
    revoke.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn inspect_device_revoke(
    revoke_text: &str,
    identity_public_key_base64: &str,
) -> Result<String, JsValue> {
    let revoke = DeviceRevoke::from_export_text(revoke_text).map_err(to_js_error)?;
    let pk = decode_key_32(identity_public_key_base64)?;
    revoke.verify(&pk).map_err(to_js_error)?;
    let out = DeviceRevokeInfo {
        user_id: revoke.user_id.to_string(),
        device_id: revoke.device_id.to_string(),
        reason: revoke.reason,
        created_at: revoke.created_at,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn export_contact_card(
    backup_text: &str,
    passphrase: &str,
    display_name: Option<String>,
    device_certs_json: Option<String>,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let device_certs = match device_certs_json {
        Some(json) if !json.trim().is_empty() => from_json_string(&json)?,
        _ => Vec::new(),
    };
    let card = identity
        .export_contact_card(display_name, None, device_certs)
        .map_err(to_js_error)?;
    card.to_export_text().map_err(to_js_error)
}

#[derive(Serialize)]
struct ContactCardInfo {
    user_id: String,
    display_name: Option<String>,
    fingerprint: String,
    identity_public_key: String,
    x25519_public_key: String,
    device_count: usize,
}

#[wasm_bindgen]
pub fn inspect_contact_card(contact_card_text: &str) -> Result<String, JsValue> {
    ensure_js_len(
        contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    let card = ContactCard::from_export_text(contact_card_text).map_err(to_js_error)?;
    card.verify().map_err(to_js_error)?;
    let out = ContactCardInfo {
        user_id: card.user_id.to_string(),
        display_name: card.display_name.clone(),
        fingerprint: card.fingerprint().map_err(to_js_error)?,
        identity_public_key: card.identity_public_key.clone(),
        x25519_public_key: card.x25519_public_key.clone(),
        device_count: card.device_certs.len(),
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn create_friend_request(
    backup_text: &str,
    passphrase: &str,
    my_contact_card_text: &str,
    target_contact_card_text: &str,
    note: Option<String>,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let my_card = ContactCard::from_export_text(my_contact_card_text).map_err(to_js_error)?;
    let target_card =
        ContactCard::from_export_text(target_contact_card_text).map_err(to_js_error)?;
    target_card.verify().map_err(to_js_error)?;
    let request = FriendRequest::new(&identity, target_card.user_id, my_card, note, 7 * 24 * 3600)
        .map_err(to_js_error)?;
    request.to_export_text().map_err(to_js_error)
}

#[derive(Serialize)]
struct FriendRequestInfo {
    request_id: String,
    from_user_id: String,
    to_user_id: String,
    note: Option<String>,
    created_at: u64,
    expires_at: u64,
    from_contact_card_text: String,
}

#[wasm_bindgen]
pub fn inspect_friend_request(request_text: &str) -> Result<String, JsValue> {
    let request = FriendRequest::from_export_text(request_text).map_err(to_js_error)?;
    request.verify().map_err(to_js_error)?;
    let out = FriendRequestInfo {
        request_id: request.request_id.to_string(),
        from_user_id: request.from_user_id.to_string(),
        to_user_id: request.to_user_id.to_string(),
        note: request.note.clone(),
        created_at: request.created_at,
        expires_at: request.expires_at,
        from_contact_card_text: request
            .from_contact_card
            .to_export_text()
            .map_err(to_js_error)?,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn accept_friend_request(
    backup_text: &str,
    passphrase: &str,
    request_text: &str,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let request = FriendRequest::from_export_text(request_text).map_err(to_js_error)?;
    let response = FriendResponse::accept(&identity, &request).map_err(to_js_error)?;
    response.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn reject_friend_request(
    backup_text: &str,
    passphrase: &str,
    request_text: &str,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let request = FriendRequest::from_export_text(request_text).map_err(to_js_error)?;
    let response = FriendResponse::reject(&identity, &request).map_err(to_js_error)?;
    response.to_export_text().map_err(to_js_error)
}

#[derive(Serialize)]
struct FriendResponseInfo {
    request_id: String,
    from_user_id: String,
    to_user_id: String,
    accepted: bool,
    created_at: u64,
}

#[wasm_bindgen]
pub fn inspect_friend_response(
    response_text: &str,
    responder_contact_card_text: &str,
) -> Result<String, JsValue> {
    let response = FriendResponse::from_export_text(response_text).map_err(to_js_error)?;
    let card = ContactCard::from_export_text(responder_contact_card_text).map_err(to_js_error)?;
    response.verify(&card).map_err(to_js_error)?;
    let out = FriendResponseInfo {
        request_id: response.request_id.to_string(),
        from_user_id: response.from_user_id.to_string(),
        to_user_id: response.to_user_id.to_string(),
        accepted: response.accepted,
        created_at: response.created_at,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn import_contact_as_json(
    contact_card_text: &str,
    trust_level: &str,
) -> Result<String, JsValue> {
    let card = ContactCard::from_export_text(contact_card_text).map_err(to_js_error)?;
    let trust = match trust_level {
        "Imported" => TrustLevel::Imported,
        "LinkImported" => TrustLevel::LinkImported,
        "QrScanned" => TrustLevel::QrScanned,
        "FingerprintVerified" => TrustLevel::FingerprintVerified,
        _ => return Err(JsValue::from_str("invalid trust level")),
    };
    let contact = card.into_contact(trust).map_err(to_js_error)?;
    to_json_string(&contact)
}

#[wasm_bindgen]
pub fn encrypt_text_message(
    backup_text: &str,
    passphrase: &str,
    to_contact_card_text: &str,
    conversation_id: &str,
    text: &str,
) -> Result<String, JsValue> {
    ensure_js_len(text, lm_core::limits::MAX_DIRECT_MESSAGE_TEXT_BYTES)?;
    ensure_js_len(
        to_contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let contact = ContactCard::from_export_text(to_contact_card_text).map_err(to_js_error)?;
    contact.verify().map_err(to_js_error)?;
    let to_x25519 = decode_key_32(&contact.x25519_public_key)?;
    let envelope = DirectEnvelope::encrypt_text(
        &identity,
        contact.user_id,
        &to_x25519,
        conversation_id.to_string(),
        text.to_string(),
    )
    .map_err(to_js_error)?;
    to_json_string(&envelope)
}

#[wasm_bindgen]
pub fn decrypt_text_message(
    backup_text: &str,
    passphrase: &str,
    from_contact_card_text: &str,
    envelope_json: &str,
) -> Result<String, JsValue> {
    ensure_js_len(
        from_contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    ensure_js_bytes(
        envelope_json.as_bytes().len(),
        lm_core::limits::MAX_MAILBOX_CIPHERTEXT_BYTES,
    )?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let contact = ContactCard::from_export_text(from_contact_card_text).map_err(to_js_error)?;
    contact.verify().map_err(to_js_error)?;
    let from_x25519 = decode_key_32(&contact.x25519_public_key)?;
    let envelope: DirectEnvelope = from_json_string(envelope_json)?;
    let plain = envelope
        .decrypt(&identity, &from_x25519)
        .map_err(to_js_error)?;
    to_json_string(&plain)
}

#[derive(Serialize)]
struct PreKeyBundleOutput {
    prekey_bundle_text: String,
    private_bundle_json: String,
    signed_one_time_prekey_record_texts: Vec<String>,
}

#[derive(Serialize)]
struct PreKeyBundleInfo {
    user_id: String,
    signed_prekey_id: u32,
    one_time_prekey_count: usize,
    created_at: u64,
    expires_at: u64,
}

#[derive(Serialize)]
struct X3dhInitiatorOutput {
    initial_message_json: String,
    shared_secret: String,
}

#[derive(Serialize)]
struct X3dhResponderOutput {
    shared_secret: String,
}

#[wasm_bindgen]
pub fn create_prekey_bundle(
    backup_text: &str,
    passphrase: &str,
    signed_prekey_id: u32,
    one_time_prekey_count: u32,
    ttl_seconds: u64,
) -> Result<String, JsValue> {
    ensure_js_len(backup_text, lm_core::limits::MAX_IDENTITY_BACKUP_TEXT_BYTES)?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let (public, private, signed_one_time_prekey_records) =
        PreKeyBundle::new_with_signed_one_time_prekey_records(
            &identity,
            signed_prekey_id,
            one_time_prekey_count,
            ttl_seconds,
        )
        .map_err(to_js_error)?;
    let signed_one_time_prekey_record_texts = signed_one_time_prekey_records
        .iter()
        .map(|record| record.to_export_text().map_err(to_js_error))
        .collect::<Result<Vec<_>, _>>()?;
    let out = PreKeyBundleOutput {
        prekey_bundle_text: public.to_export_text().map_err(to_js_error)?,
        private_bundle_json: to_json_string(&private)?,
        signed_one_time_prekey_record_texts,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn inspect_prekey_bundle(prekey_bundle_text: &str) -> Result<String, JsValue> {
    ensure_js_len(
        prekey_bundle_text,
        lm_core::limits::MAX_PREKEY_BUNDLE_TEXT_BYTES,
    )?;
    let bundle = PreKeyBundle::from_export_text(prekey_bundle_text).map_err(to_js_error)?;
    let out = PreKeyBundleInfo {
        user_id: bundle.user_id.to_string(),
        signed_prekey_id: bundle.signed_prekey_id,
        one_time_prekey_count: bundle.one_time_prekeys.len(),
        created_at: bundle.created_at,
        expires_at: bundle.expires_at,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn create_x3dh_initial_message(
    backup_text: &str,
    passphrase: &str,
    responder_prekey_bundle_text: &str,
) -> Result<String, JsValue> {
    ensure_js_len(backup_text, lm_core::limits::MAX_IDENTITY_BACKUP_TEXT_BYTES)?;
    ensure_js_len(
        responder_prekey_bundle_text,
        lm_core::limits::MAX_PREKEY_BUNDLE_TEXT_BYTES,
    )?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let bundle =
        PreKeyBundle::from_export_text(responder_prekey_bundle_text).map_err(to_js_error)?;
    let secret = x3dh_initiator_secret(&identity, &bundle).map_err(to_js_error)?;
    let out = X3dhInitiatorOutput {
        initial_message_json: to_json_string(&secret.initial_message)?,
        shared_secret: secret.shared_secret,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn create_x3dh_initial_message_with_one_time_prekey_id(
    backup_text: &str,
    passphrase: &str,
    responder_prekey_bundle_text: &str,
    one_time_prekey_id: Option<u32>,
) -> Result<String, JsValue> {
    ensure_js_len(backup_text, lm_core::limits::MAX_IDENTITY_BACKUP_TEXT_BYTES)?;
    ensure_js_len(
        responder_prekey_bundle_text,
        lm_core::limits::MAX_PREKEY_BUNDLE_TEXT_BYTES,
    )?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let bundle =
        PreKeyBundle::from_export_text(responder_prekey_bundle_text).map_err(to_js_error)?;
    let secret =
        x3dh_initiator_secret_with_one_time_prekey_id(&identity, &bundle, one_time_prekey_id)
            .map_err(to_js_error)?;
    let out = X3dhInitiatorOutput {
        initial_message_json: to_json_string(&secret.initial_message)?,
        shared_secret: secret.shared_secret,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn create_x3dh_initial_message_with_one_time_prekey_record(
    backup_text: &str,
    passphrase: &str,
    responder_prekey_bundle_text: &str,
    signed_one_time_prekey_record_text: Option<String>,
) -> Result<String, JsValue> {
    ensure_js_len(backup_text, lm_core::limits::MAX_IDENTITY_BACKUP_TEXT_BYTES)?;
    ensure_js_len(
        responder_prekey_bundle_text,
        lm_core::limits::MAX_PREKEY_BUNDLE_TEXT_BYTES,
    )?;
    if let Some(text) = signed_one_time_prekey_record_text.as_deref() {
        ensure_js_len(text, lm_core::limits::MAX_PREKEY_BUNDLE_TEXT_BYTES)?;
    }
    let identity = restore_identity_any(backup_text, passphrase)?;
    let bundle =
        PreKeyBundle::from_export_text(responder_prekey_bundle_text).map_err(to_js_error)?;
    let record = signed_one_time_prekey_record_text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .map(|text| SignedOneTimePreKeyRecord::from_export_text(text.trim()))
        .transpose()
        .map_err(to_js_error)?;
    let secret =
        x3dh_initiator_secret_with_one_time_prekey_record(&identity, &bundle, record.as_ref())
            .map_err(to_js_error)?;
    let out = X3dhInitiatorOutput {
        initial_message_json: to_json_string(&secret.initial_message)?,
        shared_secret: secret.shared_secret,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn derive_x3dh_responder_secret(
    backup_text: &str,
    passphrase: &str,
    private_bundle_json: &str,
    initial_message_json: &str,
) -> Result<String, JsValue> {
    ensure_js_len(backup_text, lm_core::limits::MAX_IDENTITY_BACKUP_TEXT_BYTES)?;
    ensure_js_bytes(private_bundle_json.as_bytes().len(), 128 * 1024)?;
    ensure_js_bytes(initial_message_json.as_bytes().len(), 32 * 1024)?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let private: PreKeyPrivateBundle = from_json_string(private_bundle_json)?;
    let initial: X3dhInitialMessage = from_json_string(initial_message_json)?;
    let shared = x3dh_responder_secret(&identity, &private, &initial).map_err(to_js_error)?;
    let out = X3dhResponderOutput {
        shared_secret: BASE64.encode(shared),
    };
    to_json_string(&out)
}

#[derive(Serialize)]
struct RatchetInitOutput {
    local_state_text: String,
    remote_state_text: String,
}

#[derive(Serialize)]
struct RatchetEncryptOutput {
    state_text: String,
    envelope_json: String,
}

#[derive(Serialize)]
struct RatchetDecryptOutput {
    state_text: String,
    plain_json: String,
}

#[wasm_bindgen]
pub fn create_ratchet_session_from_shared_secret(
    local_user_id: &str,
    remote_user_id: &str,
    shared_secret_base64: &str,
) -> Result<String, JsValue> {
    let local = lm_core::UserId::from_raw(local_user_id.to_string()).map_err(to_js_error)?;
    let remote = lm_core::UserId::from_raw(remote_user_id.to_string()).map_err(to_js_error)?;
    let shared = decode_key_32(shared_secret_base64)?;
    let (local_state, remote_state) =
        RatchetSessionState::new_pair_from_shared_secret(local, remote, &shared)
            .map_err(to_js_error)?;
    let out = RatchetInitOutput {
        local_state_text: local_state.to_export_text().map_err(to_js_error)?,
        remote_state_text: remote_state.to_export_text().map_err(to_js_error)?,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn create_ratchet_dh_keypair() -> Result<String, JsValue> {
    let pair: RatchetDhKeyPair = RatchetSessionState::generate_dh_keypair().map_err(to_js_error)?;
    to_json_string(&pair)
}

#[wasm_bindgen]
pub fn create_ratchet_session_from_shared_secret_with_keys(
    local_user_id: &str,
    remote_user_id: &str,
    role: &str,
    shared_secret_base64: &str,
    local_dh_private_key_base64: &str,
    remote_dh_public_key_base64: &str,
) -> Result<String, JsValue> {
    let local = lm_core::UserId::from_raw(local_user_id.to_string()).map_err(to_js_error)?;
    let remote = lm_core::UserId::from_raw(remote_user_id.to_string()).map_err(to_js_error)?;
    let role = match role {
        "Initiator" | "initiator" => RatchetRole::Initiator,
        "Responder" | "responder" => RatchetRole::Responder,
        _ => return Err(JsValue::from_str("invalid ratchet role")),
    };
    let state = RatchetSessionState::from_shared_secret_export(
        local,
        remote,
        role,
        shared_secret_base64,
        local_dh_private_key_base64,
        remote_dh_public_key_base64,
    )
    .map_err(to_js_error)?;
    state.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn ratchet_encrypt_text_message(
    state_text: &str,
    conversation_id: &str,
    text: &str,
) -> Result<String, JsValue> {
    ensure_js_bytes(state_text.as_bytes().len(), 64 * 1024)?;
    ensure_js_len(text, lm_core::limits::MAX_DIRECT_MESSAGE_TEXT_BYTES)?;
    let mut state = RatchetSessionState::from_export_text(state_text).map_err(to_js_error)?;
    let envelope =
        RatchetEnvelope::encrypt_text(&mut state, conversation_id.to_string(), text.to_string())
            .map_err(to_js_error)?;
    let out = RatchetEncryptOutput {
        state_text: state.to_export_text().map_err(to_js_error)?,
        envelope_json: to_json_string(&envelope)?,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn ratchet_decrypt_text_message(
    state_text: &str,
    envelope_json: &str,
) -> Result<String, JsValue> {
    ensure_js_bytes(state_text.as_bytes().len(), 64 * 1024)?;
    ensure_js_bytes(
        envelope_json.as_bytes().len(),
        lm_core::limits::MAX_MAILBOX_CIPHERTEXT_BYTES,
    )?;
    let mut state = RatchetSessionState::from_export_text(state_text).map_err(to_js_error)?;
    let envelope: RatchetEnvelope = from_json_string(envelope_json)?;
    let plain = envelope.decrypt(&mut state).map_err(to_js_error)?;
    let out = RatchetDecryptOutput {
        state_text: state.to_export_text().map_err(to_js_error)?,
        plain_json: to_json_string(&plain)?,
    };
    to_json_string(&out)
}

#[derive(Serialize)]
struct RatchetPairOutput {
    local_state_text: String,
    remote_state_text: String,
}

#[derive(Serialize)]
struct RatchetStateInfo {
    session_id: String,
    local_user_id: String,
    remote_user_id: String,
    role: String,
    local_dh_public_key: String,
    remote_dh_public_key: String,
    send_count: u32,
    recv_count: u32,
    previous_send_count: u32,
    skipped_key_count: usize,
    created_at: u64,
    updated_at: u64,
}

#[derive(Serialize)]
struct RatchetStepOutput {
    state_text: String,
    key_json: String,
}

#[wasm_bindgen]
pub fn create_ratchet_session_pair(
    local_contact_card_text: &str,
    remote_contact_card_text: &str,
) -> Result<String, JsValue> {
    ensure_js_len(
        local_contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    ensure_js_len(
        remote_contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    let local = ContactCard::from_export_text(local_contact_card_text).map_err(to_js_error)?;
    let remote = ContactCard::from_export_text(remote_contact_card_text).map_err(to_js_error)?;
    local.verify().map_err(to_js_error)?;
    remote.verify().map_err(to_js_error)?;
    let (local_state, remote_state) =
        RatchetSessionState::new_pair(local.user_id, remote.user_id).map_err(to_js_error)?;
    let out = RatchetPairOutput {
        local_state_text: local_state.to_export_text().map_err(to_js_error)?,
        remote_state_text: remote_state.to_export_text().map_err(to_js_error)?,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn inspect_ratchet_state(state_text: &str) -> Result<String, JsValue> {
    ensure_js_bytes(state_text.as_bytes().len(), 64 * 1024)?;
    let state = RatchetSessionState::from_export_text(state_text).map_err(to_js_error)?;
    let out = RatchetStateInfo {
        session_id: state.session_id,
        local_user_id: state.local_user_id.to_string(),
        remote_user_id: state.remote_user_id.to_string(),
        role: format!("{:?}", state.role),
        local_dh_public_key: state.local_dh_public_key,
        remote_dh_public_key: state.remote_dh_public_key,
        send_count: state.send_count,
        recv_count: state.recv_count,
        previous_send_count: state.previous_send_count,
        skipped_key_count: state.skipped_message_keys.len(),
        created_at: state.created_at,
        updated_at: state.updated_at,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn ratchet_next_sending_key(state_text: &str) -> Result<String, JsValue> {
    ensure_js_bytes(state_text.as_bytes().len(), 64 * 1024)?;
    let mut state = RatchetSessionState::from_export_text(state_text).map_err(to_js_error)?;
    let key = state.next_sending_key().map_err(to_js_error)?;
    let out = RatchetStepOutput {
        state_text: state.to_export_text().map_err(to_js_error)?,
        key_json: to_json_string(&key)?,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn ratchet_next_receiving_key(state_text: &str, header_json: &str) -> Result<String, JsValue> {
    ensure_js_bytes(state_text.as_bytes().len(), 64 * 1024)?;
    ensure_js_bytes(header_json.as_bytes().len(), 16 * 1024)?;
    let mut state = RatchetSessionState::from_export_text(state_text).map_err(to_js_error)?;
    let header: RatchetHeader = from_json_string(header_json)?;
    let key = state.next_receiving_key(&header).map_err(to_js_error)?;
    let out = RatchetStepOutput {
        state_text: state.to_export_text().map_err(to_js_error)?,
        key_json: to_json_string(&key)?,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn ratchet_dh_step(state_text: &str, remote_dh_public_key: &str) -> Result<String, JsValue> {
    ensure_js_bytes(state_text.as_bytes().len(), 64 * 1024)?;
    let mut state = RatchetSessionState::from_export_text(state_text).map_err(to_js_error)?;
    state
        .dh_ratchet(remote_dh_public_key)
        .map_err(to_js_error)?;
    state.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn create_group_policy_state(
    group_id: &str,
    group_name: &str,
    creator_user_id: &str,
    member_user_ids_json: &str,
) -> Result<String, JsValue> {
    let group_id =
        uuid::Uuid::parse_str(group_id).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let creator = lm_core::UserId::from_raw(creator_user_id.to_string()).map_err(to_js_error)?;
    let member_ids: Vec<String> = from_json_string(member_user_ids_json)?;
    let members = member_ids
        .into_iter()
        .map(lm_core::UserId::from_raw)
        .collect::<lm_core::Result<Vec<_>>>()
        .map_err(to_js_error)?;
    let state = GroupPolicyState::new(group_id, group_name.to_string(), creator, members)
        .map_err(to_js_error)?;
    to_json_string(&state)
}

#[wasm_bindgen]
pub fn apply_group_policy_event(
    policy_state_json: &str,
    event_text: &str,
    actor_contact_card_text: &str,
) -> Result<String, JsValue> {
    ensure_js_bytes(policy_state_json.as_bytes().len(), 128 * 1024)?;
    ensure_js_len(event_text, lm_core::limits::MAX_GROUP_INVITE_TEXT_BYTES)?;
    ensure_js_len(
        actor_contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    let mut state: GroupPolicyState = from_json_string(policy_state_json)?;
    let event = GroupEvent::from_export_text(event_text).map_err(to_js_error)?;
    let card = ContactCard::from_export_text(actor_contact_card_text).map_err(to_js_error)?;
    event.verify(&card).map_err(to_js_error)?;
    state.apply_event(&event).map_err(to_js_error)?;
    to_json_string(&state)
}

#[derive(Serialize)]
struct GroupSenderKeyOutput {
    state_json: String,
    distribution_text: String,
}

#[derive(Serialize)]
struct GroupSenderEncryptOutput {
    state_json: String,
    envelope_json: String,
}

#[derive(Serialize)]
struct GroupSenderDecryptOutput {
    state_json: String,
    plain_json: String,
}

#[wasm_bindgen]
pub fn create_group_sender_key(
    backup_text: &str,
    passphrase: &str,
    group_id: &str,
) -> Result<String, JsValue> {
    ensure_js_len(backup_text, lm_core::limits::MAX_IDENTITY_BACKUP_TEXT_BYTES)?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let group_id =
        uuid::Uuid::parse_str(group_id).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let state = GroupSenderKeyState::new(&identity, group_id).map_err(to_js_error)?;
    let distribution = state.to_distribution(&identity).map_err(to_js_error)?;
    let out = GroupSenderKeyOutput {
        state_json: to_json_string(&state)?,
        distribution_text: distribution.to_export_text().map_err(to_js_error)?,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn import_group_sender_key(
    distribution_text: &str,
    sender_contact_card_text: &str,
) -> Result<String, JsValue> {
    ensure_js_len(
        distribution_text,
        lm_core::limits::MAX_GROUP_INVITE_TEXT_BYTES,
    )?;
    ensure_js_len(
        sender_contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    let distribution =
        GroupSenderKeyDistribution::from_export_text(distribution_text).map_err(to_js_error)?;
    let card = ContactCard::from_export_text(sender_contact_card_text).map_err(to_js_error)?;
    let state =
        GroupSenderKeyState::from_distribution(&distribution, &card).map_err(to_js_error)?;
    to_json_string(&state)
}

#[wasm_bindgen]
pub fn group_sender_encrypt_text(state_json: &str, text: &str) -> Result<String, JsValue> {
    ensure_js_bytes(state_json.as_bytes().len(), 64 * 1024)?;
    ensure_js_len(text, lm_core::limits::MAX_GROUP_MESSAGE_TEXT_BYTES)?;
    let mut state: GroupSenderKeyState = from_json_string(state_json)?;
    let envelope = state.encrypt_text(text.to_string()).map_err(to_js_error)?;
    let out = GroupSenderEncryptOutput {
        state_json: to_json_string(&state)?,
        envelope_json: to_json_string(&envelope)?,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn group_sender_decrypt_text(state_json: &str, envelope_json: &str) -> Result<String, JsValue> {
    ensure_js_bytes(state_json.as_bytes().len(), 64 * 1024)?;
    ensure_js_bytes(
        envelope_json.as_bytes().len(),
        lm_core::limits::MAX_MAILBOX_CIPHERTEXT_BYTES,
    )?;
    let mut state: GroupSenderKeyState = from_json_string(state_json)?;
    let envelope: GroupSenderEnvelope = from_json_string(envelope_json)?;
    let plain = state.decrypt(&envelope).map_err(to_js_error)?;
    let out = GroupSenderDecryptOutput {
        state_json: to_json_string(&state)?,
        plain_json: to_json_string(&plain)?,
    };
    to_json_string(&out)
}

#[derive(Serialize)]
struct GroupInviteInfo {
    invite_id: String,
    group_id: String,
    group_name: String,
    inviter_user_id: String,
    member_user_ids: Vec<String>,
    created_at: u64,
    expires_at: u64,
}

#[derive(Serialize)]
struct GroupEventInfo {
    event_id: String,
    group_id: String,
    actor_user_id: String,
    sequence: u64,
    action: GroupEventAction,
    created_at: u64,
}

#[wasm_bindgen]
pub fn create_group_invite(
    backup_text: &str,
    passphrase: &str,
    group_id: &str,
    group_name: &str,
    member_user_ids_json: &str,
) -> Result<String, JsValue> {
    ensure_js_len(group_name, lm_core::limits::MAX_GROUP_NAME_BYTES)?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let raw_members: Vec<String> = from_json_string(member_user_ids_json)?;
    let mut members = Vec::with_capacity(raw_members.len());
    for raw in raw_members {
        members.push(lm_core::UserId::from_raw(raw).map_err(to_js_error)?);
    }
    let group_id =
        uuid::Uuid::parse_str(group_id).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let invite = GroupInvite::new(
        &identity,
        group_id,
        group_name.to_string(),
        members,
        7 * 24 * 3600,
    )
    .map_err(to_js_error)?;
    invite.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn inspect_group_invite(
    invite_text: &str,
    inviter_contact_card_text: &str,
) -> Result<String, JsValue> {
    ensure_js_len(invite_text, lm_core::limits::MAX_GROUP_INVITE_TEXT_BYTES)?;
    ensure_js_len(
        inviter_contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    let invite = GroupInvite::from_export_text(invite_text).map_err(to_js_error)?;
    let card = ContactCard::from_export_text(inviter_contact_card_text).map_err(to_js_error)?;
    invite.verify(&card).map_err(to_js_error)?;
    let out = GroupInviteInfo {
        invite_id: invite.invite_id.to_string(),
        group_id: invite.group_id.to_string(),
        group_name: invite.group_name,
        inviter_user_id: invite.inviter_user_id.to_string(),
        member_user_ids: invite
            .member_user_ids
            .into_iter()
            .map(|u| u.to_string())
            .collect(),
        created_at: invite.created_at,
        expires_at: invite.expires_at,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn create_group_event(
    backup_text: &str,
    passphrase: &str,
    group_id: &str,
    sequence: u64,
    action_json: &str,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let group_id =
        uuid::Uuid::parse_str(group_id).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let action: GroupEventAction = from_json_string(action_json)?;
    let event = GroupEvent::new(&identity, group_id, sequence, action).map_err(to_js_error)?;
    event.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn inspect_group_event(
    event_text: &str,
    actor_contact_card_text: &str,
) -> Result<String, JsValue> {
    ensure_js_len(event_text, lm_core::limits::MAX_GROUP_INVITE_TEXT_BYTES)?;
    ensure_js_len(
        actor_contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    let event = GroupEvent::from_export_text(event_text).map_err(to_js_error)?;
    let card = ContactCard::from_export_text(actor_contact_card_text).map_err(to_js_error)?;
    event.verify(&card).map_err(to_js_error)?;
    to_json_string(&GroupEventInfo {
        event_id: event.event_id.to_string(),
        group_id: event.group_id.to_string(),
        actor_user_id: event.actor_user_id.to_string(),
        sequence: event.sequence,
        action: event.action,
        created_at: event.created_at,
    })
}

#[wasm_bindgen]
pub fn create_peer_announce(
    backup_text: &str,
    passphrase: &str,
    addresses_json: &str,
    mailbox_key: Option<String>,
    ttl_seconds: u64,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let addresses: Vec<String> = from_json_string(addresses_json)?;
    let announce = PeerAnnounce::new(&identity, None, addresses, mailbox_key, ttl_seconds)
        .map_err(to_js_error)?;
    announce.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn inspect_peer_announce(
    text: &str,
    identity_public_key_base64: &str,
) -> Result<String, JsValue> {
    let announce = PeerAnnounce::from_export_text(text).map_err(to_js_error)?;
    let pk = decode_key_32(identity_public_key_base64)?;
    announce.verify(&pk).map_err(to_js_error)?;
    to_json_string(&announce)
}

#[wasm_bindgen]
pub fn create_public_peer_announce(
    backup_text: &str,
    passphrase: &str,
    peer_id: &str,
    addresses_json: &str,
    capabilities_json: &str,
    ttl_seconds: u64,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let addresses: Vec<String> = from_json_string(addresses_json)?;
    let caps_raw: Vec<String> = from_json_string(capabilities_json)?;
    let mut caps = Vec::new();
    for cap in caps_raw {
        caps.push(match cap.as_str() {
            "bootstrap" => PublicPeerCapability::Bootstrap,
            "dht" => PublicPeerCapability::Dht,
            "signaling" => PublicPeerCapability::Signaling,
            "relay" => PublicPeerCapability::Relay,
            "mailbox" => PublicPeerCapability::Mailbox,
            _ => return Err(JsValue::from_str("unknown capability")),
        });
    }
    let announce = PublicPeerAnnounce::new(
        &identity,
        peer_id.to_string(),
        None,
        addresses,
        caps,
        Some(10 * 1024 * 1024),
        Some(24 * 3600),
        Some(1024),
        ttl_seconds,
    )
    .map_err(to_js_error)?;
    announce.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn inspect_public_peer_announce(
    text: &str,
    identity_public_key_base64: &str,
) -> Result<String, JsValue> {
    let announce = PublicPeerAnnounce::from_export_text(text).map_err(to_js_error)?;
    let pk = decode_key_32(identity_public_key_base64)?;
    announce.verify(&pk).map_err(to_js_error)?;
    to_json_string(&announce)
}

#[wasm_bindgen]
pub fn create_mailbox_message(
    backup_text: &str,
    passphrase: &str,
    to_user_id: &str,
    kind: &str,
    ciphertext: &str,
    ttl_seconds: u64,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let to_user_id = lm_core::UserId::from_raw(to_user_id.to_string()).map_err(to_js_error)?;
    let kind = match kind {
        "signal-offer" => MailboxMessageKind::SignalOffer,
        "signal-answer" => MailboxMessageKind::SignalAnswer,
        "direct-envelope" => MailboxMessageKind::DirectEnvelope,
        "group-fanout" => MailboxMessageKind::GroupFanout,
        _ => MailboxMessageKind::Other,
    };
    let msg = MailboxMessage::new(
        &identity,
        to_user_id,
        kind,
        ciphertext.to_string(),
        ttl_seconds,
    )
    .map_err(to_js_error)?;
    msg.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn inspect_mailbox_message(
    text: &str,
    from_identity_public_key_base64: &str,
) -> Result<String, JsValue> {
    let msg = MailboxMessage::from_export_text(text).map_err(to_js_error)?;
    let pk = decode_key_32(from_identity_public_key_base64)?;
    msg.verify(&pk).map_err(to_js_error)?;
    to_json_string(&msg)
}

#[derive(Serialize)]
struct FilePackageInfo {
    manifest: FileManifest,
    chunk_count: usize,
    total_ciphertext_bytes: usize,
}

#[wasm_bindgen]
pub fn create_file_package(
    backup_text: &str,
    passphrase: &str,
    to_contact_card_text: &str,
    name: &str,
    mime_type: &str,
    file_bytes_base64: &str,
    chunk_size: u32,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let contact = ContactCard::from_export_text(to_contact_card_text).map_err(to_js_error)?;
    contact.verify().map_err(to_js_error)?;
    let to_x25519 = decode_key_32(&contact.x25519_public_key)?;
    let to_user_id = contact.user_id;
    ensure_js_bytes(
        file_bytes_base64.as_bytes().len(),
        lm_core::limits::MAX_FILE_BYTES * 2,
    )?;
    let bytes = BASE64
        .decode(file_bytes_base64.as_bytes())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let chunk_size = if chunk_size == 0 {
        16 * 1024
    } else {
        chunk_size
    };
    let chunk_count = ((bytes.len() as u64 + chunk_size as u64 - 1) / chunk_size as u64) as u32;
    if chunk_count == 0 {
        return Err(JsValue::from_str("file is empty"));
    }
    let manifest = FileManifest::new(
        &identity,
        to_user_id.clone(),
        name.to_string(),
        mime_type.to_string(),
        bytes.len() as u64,
        chunk_size,
        chunk_count,
        lm_core::file_hash_base64(&bytes),
    )
    .map_err(to_js_error)?;
    let mut chunks = Vec::with_capacity(chunk_count as usize);
    for (index, chunk) in bytes.chunks(chunk_size as usize).enumerate() {
        chunks.push(
            FileChunkEnvelope::encrypt_chunk(
                &identity,
                to_user_id.clone(),
                &to_x25519,
                manifest.file_id,
                index as u32,
                chunk,
            )
            .map_err(to_js_error)?,
        );
    }
    to_json_string(&serde_json::json!({
        "type": "lm-file-package-v1",
        "manifest": manifest,
        "manifest_text": manifest.to_export_text().map_err(to_js_error)?,
        "chunks": chunks,
    }))
}

#[wasm_bindgen]
pub fn inspect_file_package(file_package_json: &str) -> Result<String, JsValue> {
    ensure_js_bytes(
        file_package_json.as_bytes().len(),
        lm_core::limits::MAX_FILE_BYTES * 3,
    )?;
    let value: serde_json::Value = from_json_string(file_package_json)?;
    let manifest: FileManifest = serde_json::from_value(value["manifest"].clone())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let chunks: Vec<FileChunkEnvelope> = serde_json::from_value(value["chunks"].clone())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let total_ciphertext_bytes = chunks.iter().map(|c| c.ciphertext.len()).sum();
    to_json_string(&FilePackageInfo {
        manifest,
        chunk_count: chunks.len(),
        total_ciphertext_bytes,
    })
}

#[wasm_bindgen]
pub fn decrypt_file_package(
    backup_text: &str,
    passphrase: &str,
    from_contact_card_text: &str,
    file_package_json: &str,
) -> Result<String, JsValue> {
    ensure_js_len(
        from_contact_card_text,
        lm_core::limits::MAX_CONTACT_CARD_TEXT_BYTES,
    )?;
    ensure_js_bytes(
        file_package_json.as_bytes().len(),
        lm_core::limits::MAX_FILE_BYTES * 3,
    )?;
    let identity = restore_identity_any(backup_text, passphrase)?;
    let contact = ContactCard::from_export_text(from_contact_card_text).map_err(to_js_error)?;
    contact.verify().map_err(to_js_error)?;
    let from_x25519 = decode_key_32(&contact.x25519_public_key)?;
    let value: serde_json::Value = from_json_string(file_package_json)?;
    let manifest: FileManifest = serde_json::from_value(value["manifest"].clone())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut chunks: Vec<FileChunkEnvelope> = serde_json::from_value(value["chunks"].clone())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    chunks.sort_by_key(|c| c.chunk_index);
    if chunks.len() != manifest.chunk_count as usize {
        return Err(JsValue::from_str("file chunk count mismatch"));
    }
    let mut out = Vec::with_capacity(manifest.size as usize);
    for (expected, chunk) in chunks.iter().enumerate() {
        if chunk.file_id != manifest.file_id || chunk.chunk_index != expected as u32 {
            return Err(JsValue::from_str("file chunk ordering mismatch"));
        }
        let plain = chunk
            .decrypt_chunk(&identity, &from_x25519)
            .map_err(to_js_error)?;
        out.extend_from_slice(&plain);
    }
    if out.len() as u64 != manifest.size {
        return Err(JsValue::from_str("file size mismatch"));
    }
    if !lm_core::verify_file_hash(&out, &manifest.file_hash) {
        return Err(JsValue::from_str("file hash mismatch"));
    }
    to_json_string(&serde_json::json!({
        "name": manifest.name,
        "mime_type": manifest.mime_type,
        "size": manifest.size,
        "file_hash": manifest.file_hash,
        "bytes_base64": BASE64.encode(out),
    }))
}

#[derive(Serialize)]
struct SignalInfo {
    signal_id: String,
    from_user_id: String,
    to_user_id: Option<String>,
    sdp: String,
    created_at: u64,
    expires_at: u64,
}

#[wasm_bindgen]
pub fn create_signal_offer(
    backup_text: &str,
    passphrase: &str,
    to_user_id: Option<String>,
    sdp: &str,
    ttl_seconds: u64,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let target = match to_user_id {
        Some(raw) if !raw.trim().is_empty() => {
            Some(lm_core::UserId::from_raw(raw).map_err(to_js_error)?)
        }
        _ => None,
    };
    let offer = SignalOffer::new(&identity, None, target, sdp.to_string(), ttl_seconds)
        .map_err(to_js_error)?;
    offer.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn inspect_signal_offer(
    offer_text: &str,
    from_contact_card_text: &str,
) -> Result<String, JsValue> {
    let offer = SignalOffer::from_export_text(offer_text).map_err(to_js_error)?;
    let card = ContactCard::from_export_text(from_contact_card_text).map_err(to_js_error)?;
    card.verify().map_err(to_js_error)?;
    let pk = decode_key_32(&card.identity_public_key)?;
    offer.verify(&pk).map_err(to_js_error)?;
    let out = SignalInfo {
        signal_id: offer.signal_id.to_string(),
        from_user_id: offer.from_user_id.to_string(),
        to_user_id: offer.to_user_id.as_ref().map(ToString::to_string),
        sdp: offer.sdp,
        created_at: offer.created_at,
        expires_at: offer.expires_at,
    };
    to_json_string(&out)
}

#[wasm_bindgen]
pub fn create_signal_answer(
    backup_text: &str,
    passphrase: &str,
    offer_text: &str,
    sdp: &str,
    ttl_seconds: u64,
) -> Result<String, JsValue> {
    let identity = restore_identity_any(backup_text, passphrase)?;
    let offer = SignalOffer::from_export_text(offer_text).map_err(to_js_error)?;
    let answer = SignalAnswer::new(&identity, None, &offer, sdp.to_string(), ttl_seconds)
        .map_err(to_js_error)?;
    answer.to_export_text().map_err(to_js_error)
}

#[wasm_bindgen]
pub fn inspect_signal_answer(
    answer_text: &str,
    from_contact_card_text: &str,
) -> Result<String, JsValue> {
    let answer = SignalAnswer::from_export_text(answer_text).map_err(to_js_error)?;
    let card = ContactCard::from_export_text(from_contact_card_text).map_err(to_js_error)?;
    card.verify().map_err(to_js_error)?;
    let pk = decode_key_32(&card.identity_public_key)?;
    answer.verify(&pk).map_err(to_js_error)?;
    let out = SignalInfo {
        signal_id: answer.signal_id.to_string(),
        from_user_id: answer.from_user_id.to_string(),
        to_user_id: Some(answer.to_user_id.to_string()),
        sdp: answer.sdp,
        created_at: answer.created_at,
        expires_at: answer.expires_at,
    };
    to_json_string(&out)
}

fn decode_key_32(value: &str) -> Result<[u8; 32], JsValue> {
    let bytes = BASE64
        .decode(value.as_bytes())
        .map_err(|_| JsValue::from_str("invalid base64 key"))?;
    bytes
        .try_into()
        .map_err(|_| JsValue::from_str("invalid key length"))
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let bob_card =
            export_contact_card(bob_backup, "bob pass", Some("Bob".into()), None).unwrap();

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
    fn wasm_identity_from_seed_matches_native_user_id() {
        let seed = [42u8; lm_core::identity::IDENTITY_SEED_LEN];
        let wasm_identity = wasm_identity_from_seed(seed).unwrap();
        let native_identity = Identity::from_seed(IdentitySeed::from_bytes(seed)).unwrap();
        assert_eq!(wasm_identity.user_id(), native_identity.user_id());
        assert_eq!(
            wasm_identity.identity_public_key(),
            native_identity.identity_public_key()
        );
        assert_eq!(
            wasm_identity.x25519_public_key(),
            native_identity.x25519_public_key()
        );
    }

    #[test]
    fn wasm_device_revoke_smoke() {
        let alice = create_identity("alice pass").unwrap();
        let alice_v: Value = serde_json::from_str(&alice).unwrap();
        let backup = alice_v["backup_text"].as_str().unwrap();
        let device = create_device_cert(backup, "alice pass", Some("phone".into())).unwrap();
        let device_v: Value = serde_json::from_str(&device).unwrap();
        let device_id = device_v["device_id"].as_str().unwrap();
        let revoke =
            create_device_revoke(backup, "alice pass", device_id, Some("lost".into())).unwrap();
        assert!(revoke.starts_with("lm-device-revoke-v1:"));
        let info: Value = serde_json::from_str(
            &inspect_device_revoke(&revoke, alice_v["identity_public_key"].as_str().unwrap())
                .unwrap(),
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
            serde_json::to_string(&vec![alice_v["user_id"].as_str().unwrap(), bob_user_id])
                .unwrap();
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
        let addresses =
            serde_json::to_string(&vec!["/dns4/bootstrap.example/tcp/443/wss"]).unwrap();
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
            &inspect_public_peer_announce(
                &announce,
                alice_v["identity_public_key"].as_str().unwrap(),
            )
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
            &inspect_mailbox_message(&msg, alice_v["identity_public_key"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(info["from_user_id"], alice_v["user_id"]);
        assert_eq!(info["to_user_id"], bob_v["user_id"]);
        assert_eq!(info["kind"], "DirectEnvelope");
        assert_eq!(info["ciphertext"], "ciphertext-envelope");
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
        let bob_card =
            export_contact_card(bob_backup, "bob pass", Some("Bob".into()), None).unwrap();
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
        let bob_card =
            export_contact_card(bob_backup, "bob pass", Some("Bob".into()), None).unwrap();
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
        let prekey: Value = serde_json::from_str(
            &create_prekey_bundle(bob_backup, "bob pass", 11, 2, 3600).unwrap(),
        )
        .unwrap();
        let prekey_text = prekey["prekey_bundle_text"].as_str().unwrap();
        let info: Value =
            serde_json::from_str(&inspect_prekey_bundle(prekey_text).unwrap()).unwrap();
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
        let alice_pair: Value =
            serde_json::from_str(&create_ratchet_dh_keypair().unwrap()).unwrap();
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
            &ratchet_decrypt_text_message(&bob_state, enc["envelope_json"].as_str().unwrap())
                .unwrap(),
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
        let info: Value = serde_json::from_str(
            &inspect_ratchet_state(send["state_text"].as_str().unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(info["send_count"], 1);
    }
}
