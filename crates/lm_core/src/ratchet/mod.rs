//! Double Ratchet session state scaffold.
//!
//! This module provides the persistent state and deterministic key-advance
//! mechanics needed by a future Signal-style Double Ratchet implementation.
//! It is intentionally not wired into [`DirectEnvelope`] yet: the current
//! production message path remains the MVP static X25519 construction until an
//! X3DH/pre-key handshake and skipped-message-key policy are completed.

use crate::{LmError, Result, UserId, crypto, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const RATCHET_SESSION_TYPE: &str = "lm-ratchet-session-state-v1";
pub const RATCHET_STATE_PREFIX: &str = "lm-ratchet-state-v1:";
pub const RATCHET_MESSAGE_KEY_INFO: &[u8] = b"lm-talk.ratchet.message-key.v1";
const RATCHET_ROOT_INFO: &[u8] = b"lm-talk.ratchet.root-key.v1";
const RATCHET_CHAIN_INFO: &[u8] = b"lm-talk.ratchet.chain-key.v1";
const RATCHET_NEXT_CHAIN_INFO: &[u8] = b"lm-talk.ratchet.next-chain-key.v1";
const MAX_SKIPPED_KEYS: usize = 512;

/// Serializable local Double Ratchet session state.
///
/// Secrets are encoded as base64 strings because this object must round-trip
/// through JSON/WASM/IndexedDB. Treat exported state as sensitive application
/// data and store it only inside the encrypted local database or encrypted data
/// backup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct RatchetSessionState {
    #[zeroize(skip)]
    pub r#type: String,
    #[zeroize(skip)]
    pub version: u16,
    #[zeroize(skip)]
    pub session_id: String,
    #[zeroize(skip)]
    pub local_user_id: UserId,
    #[zeroize(skip)]
    pub remote_user_id: UserId,
    #[zeroize(skip)]
    pub role: RatchetRole,
    /// Root key RK.
    pub root_key: String,
    /// Local DH ratchet private key. Present for a full local state export.
    pub local_dh_private_key: String,
    #[zeroize(skip)]
    pub local_dh_public_key: String,
    #[zeroize(skip)]
    pub remote_dh_public_key: String,
    /// Sending chain key CKs and sending message number Ns.
    pub send_chain_key: String,
    #[zeroize(skip)]
    pub send_count: u32,
    /// Receiving chain key CKr and receiving message number Nr.
    pub recv_chain_key: String,
    #[zeroize(skip)]
    pub recv_count: u32,
    #[zeroize(skip)]
    /// Previous sending-chain length PN.
    pub previous_send_count: u32,
    /// Skipped keys keyed by "remote_dh_public_key_base64:message_number".
    pub skipped_message_keys: BTreeMap<String, String>,
    #[zeroize(skip)]
    pub created_at: u64,
    #[zeroize(skip)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RatchetRole {
    Initiator,
    Responder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatchetHeader {
    pub session_id: String,
    pub dh_public_key: String,
    pub previous_send_count: u32,
    pub message_number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct RatchetMessageKey {
    #[zeroize(skip)]
    pub header: RatchetHeader,
    pub message_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct RatchetSkippedKey {
    #[zeroize(skip)]
    pub header: RatchetHeader,
    pub message_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct RatchetDhKeyPair {
    pub private_key: String,
    #[zeroize(skip)]
    pub public_key: String,
}

impl RatchetSessionState {
    /// Create a matched pair of states for two already-known contacts.
    ///
    /// This is a development scaffold: it simulates the initial shared secret
    /// that a real X3DH/pre-key handshake will produce. Tests and UI debugging
    /// can use it to verify persistent state handling and chain advancement.
    pub fn new_pair(local_user_id: UserId, remote_user_id: UserId) -> Result<(Self, Self)> {
        let mut session_seed = [0u8; 32];
        getrandom(&mut session_seed).map_err(|_| LmError::RandomFailed)?;
        let session_id = BASE64.encode(blake3::hash(&session_seed).as_bytes());
        let root_key = crypto::hkdf_32(&session_seed, RATCHET_ROOT_INFO)?;

        let initiator_dh = random_x25519_secret()?;
        let responder_dh = random_x25519_secret()?;
        let initiator_pub = X25519PublicKey::from(&initiator_dh).to_bytes();
        let responder_pub = X25519PublicKey::from(&responder_dh).to_bytes();
        let dh_out = initiator_dh
            .diffie_hellman(&X25519PublicKey::from(responder_pub))
            .to_bytes();
        let send_chain = derive_chain_key(&root_key, &dh_out, b"initiator-to-responder")?;
        let recv_chain = derive_chain_key(&root_key, &dh_out, b"responder-to-initiator")?;
        let now = current_unix_timestamp();

        let initiator = Self {
            r#type: RATCHET_SESSION_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            session_id: session_id.clone(),
            local_user_id: local_user_id.clone(),
            remote_user_id: remote_user_id.clone(),
            role: RatchetRole::Initiator,
            root_key: BASE64.encode(root_key),
            local_dh_private_key: BASE64.encode(initiator_dh.to_bytes()),
            local_dh_public_key: BASE64.encode(initiator_pub),
            remote_dh_public_key: BASE64.encode(responder_pub),
            send_chain_key: BASE64.encode(send_chain),
            send_count: 0,
            recv_chain_key: BASE64.encode(recv_chain),
            recv_count: 0,
            previous_send_count: 0,
            skipped_message_keys: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        };
        let responder = Self {
            r#type: RATCHET_SESSION_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            session_id,
            local_user_id: remote_user_id,
            remote_user_id: local_user_id,
            role: RatchetRole::Responder,
            root_key: BASE64.encode(root_key),
            local_dh_private_key: BASE64.encode(responder_dh.to_bytes()),
            local_dh_public_key: BASE64.encode(responder_pub),
            remote_dh_public_key: BASE64.encode(initiator_pub),
            send_chain_key: BASE64.encode(recv_chain),
            send_count: 0,
            recv_chain_key: BASE64.encode(send_chain),
            recv_count: 0,
            previous_send_count: 0,
            skipped_message_keys: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        };
        Ok((initiator, responder))
    }

    /// Create a matched pair of states from an already-derived X3DH shared
    /// secret. This is useful for tests and local loopback flows. Production
    /// clients normally create each side with [`Self::from_shared_secret_with_keys`]
    /// because each device only knows its own ratchet private key.
    pub fn new_pair_from_shared_secret(
        local_user_id: UserId,
        remote_user_id: UserId,
        shared_secret: &[u8; 32],
    ) -> Result<(Self, Self)> {
        let initiator_dh = random_x25519_secret()?;
        let responder_dh = random_x25519_secret()?;
        let initiator_pub = X25519PublicKey::from(&initiator_dh).to_bytes();
        let responder_pub = X25519PublicKey::from(&responder_dh).to_bytes();
        let initiator = Self::from_shared_secret_with_keys(
            local_user_id.clone(),
            remote_user_id.clone(),
            RatchetRole::Initiator,
            shared_secret,
            &initiator_dh.to_bytes(),
            &responder_pub,
        )?;
        let responder = Self::from_shared_secret_with_keys(
            remote_user_id,
            local_user_id,
            RatchetRole::Responder,
            shared_secret,
            &responder_dh.to_bytes(),
            &initiator_pub,
        )?;
        Ok((initiator, responder))
    }

    /// Initialize one side of a Double Ratchet session from an X3DH shared
    /// secret and an initial local/remote ratchet DH key pair.
    pub fn from_shared_secret_with_keys(
        local_user_id: UserId,
        remote_user_id: UserId,
        role: RatchetRole,
        shared_secret: &[u8; 32],
        local_dh_private_key: &[u8; 32],
        remote_dh_public_key: &[u8; 32],
    ) -> Result<Self> {
        let root_key = crypto::hkdf_32(shared_secret, RATCHET_ROOT_INFO)?;
        let send_label = match role {
            RatchetRole::Initiator => b"initiator-to-responder".as_slice(),
            RatchetRole::Responder => b"responder-to-initiator".as_slice(),
        };
        let recv_label = match role {
            RatchetRole::Initiator => b"responder-to-initiator".as_slice(),
            RatchetRole::Responder => b"initiator-to-responder".as_slice(),
        };
        let send_chain = derive_initial_chain_key(&root_key, send_label)?;
        let recv_chain = derive_initial_chain_key(&root_key, recv_label)?;
        let local_secret = X25519Secret::from(*local_dh_private_key);
        let local_public = X25519PublicKey::from(&local_secret).to_bytes();
        let session_id = derive_session_id(shared_secret, &local_user_id, &remote_user_id)?;
        let now = current_unix_timestamp();
        let state = Self {
            r#type: RATCHET_SESSION_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            session_id,
            local_user_id,
            remote_user_id,
            role,
            root_key: BASE64.encode(root_key),
            local_dh_private_key: BASE64.encode(local_dh_private_key),
            local_dh_public_key: BASE64.encode(local_public),
            remote_dh_public_key: BASE64.encode(remote_dh_public_key),
            send_chain_key: BASE64.encode(send_chain),
            send_count: 0,
            recv_chain_key: BASE64.encode(recv_chain),
            recv_count: 0,
            previous_send_count: 0,
            skipped_message_keys: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        };
        state.validate()?;
        Ok(state)
    }

    pub fn generate_dh_keypair() -> Result<RatchetDhKeyPair> {
        let secret = random_x25519_secret()?;
        let public = X25519PublicKey::from(&secret).to_bytes();
        Ok(RatchetDhKeyPair {
            private_key: BASE64.encode(secret.to_bytes()),
            public_key: BASE64.encode(public),
        })
    }

    pub fn from_shared_secret_export(
        local_user_id: UserId,
        remote_user_id: UserId,
        role: RatchetRole,
        shared_secret_base64: &str,
        local_dh_private_key_base64: &str,
        remote_dh_public_key_base64: &str,
    ) -> Result<Self> {
        let shared_secret = decode_fixed_32(shared_secret_base64)?;
        let local_private = decode_fixed_32(local_dh_private_key_base64)?;
        let remote_public = decode_fixed_32(remote_dh_public_key_base64)?;
        Self::from_shared_secret_with_keys(
            local_user_id,
            remote_user_id,
            role,
            &shared_secret,
            &local_private,
            &remote_public,
        )
    }

    pub fn to_export_text(&self) -> Result<String> {
        self.validate()?;
        crate::codec::encode_json_prefixed(RATCHET_STATE_PREFIX, self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        let state: Self = crate::codec::decode_json_prefixed(RATCHET_STATE_PREFIX, text)?;
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<()> {
        if self.r#type != RATCHET_SESSION_TYPE {
            return Err(LmError::InvalidFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        decode_fixed_32(&self.root_key)?;
        decode_fixed_32(&self.local_dh_private_key)?;
        decode_fixed_32(&self.local_dh_public_key)?;
        decode_fixed_32(&self.remote_dh_public_key)?;
        decode_fixed_32(&self.send_chain_key)?;
        decode_fixed_32(&self.recv_chain_key)?;
        if self.skipped_message_keys.len() > MAX_SKIPPED_KEYS {
            return Err(LmError::PayloadTooLarge);
        }
        for key in self.skipped_message_keys.values() {
            decode_fixed_32(key)?;
        }
        Ok(())
    }

    /// Advance the sending chain once and return the message key that should be
    /// used for the next outbound payload.
    pub fn next_sending_key(&mut self) -> Result<RatchetMessageKey> {
        self.validate()?;
        let chain = decode_fixed_32(&self.send_chain_key)?;
        let message_key = derive_indexed_key(&chain, RATCHET_MESSAGE_KEY_INFO, self.send_count)?;
        let next_chain = derive_indexed_key(&chain, RATCHET_NEXT_CHAIN_INFO, self.send_count)?;
        let header = RatchetHeader {
            session_id: self.session_id.clone(),
            dh_public_key: self.local_dh_public_key.clone(),
            previous_send_count: self.previous_send_count,
            message_number: self.send_count,
        };
        self.send_count = self.send_count.checked_add(1).ok_or(LmError::CounterExhausted)?;
        self.send_chain_key = BASE64.encode(next_chain);
        self.updated_at = current_unix_timestamp();
        Ok(RatchetMessageKey {
            header,
            message_key: BASE64.encode(message_key),
        })
    }

    /// Advance the receiving chain until `header.message_number` and return the
    /// corresponding key. Earlier skipped keys are retained for later out-of-order
    /// delivery. Reusing the same header after successful receive is rejected.
    pub fn next_receiving_key(&mut self, header: &RatchetHeader) -> Result<RatchetMessageKey> {
        self.validate()?;
        if header.session_id != self.session_id || header.dh_public_key != self.remote_dh_public_key
        {
            return Err(LmError::CryptoError);
        }
        let skipped_id = skipped_key_id(&header.dh_public_key, header.message_number);
        if let Some(message_key) = self.skipped_message_keys.remove(&skipped_id) {
            self.updated_at = current_unix_timestamp();
            return Ok(RatchetMessageKey {
                header: header.clone(),
                message_key,
            });
        }
        if header.message_number < self.recv_count {
            return Err(LmError::ReplayDetected);
        }
        if header.message_number.saturating_sub(self.recv_count) as usize > MAX_SKIPPED_KEYS {
            return Err(LmError::PayloadTooLarge);
        }
        let mut chain = decode_fixed_32(&self.recv_chain_key)?;
        while self.recv_count < header.message_number {
            let skipped_message_key =
                derive_indexed_key(&chain, RATCHET_MESSAGE_KEY_INFO, self.recv_count)?;
            self.skipped_message_keys.insert(
                skipped_key_id(&header.dh_public_key, self.recv_count),
                BASE64.encode(skipped_message_key),
            );
            chain = derive_indexed_key(&chain, RATCHET_NEXT_CHAIN_INFO, self.recv_count)?;
            self.recv_count = self.recv_count.checked_add(1).ok_or(LmError::CounterExhausted)?;
        }
        let message_key = derive_indexed_key(&chain, RATCHET_MESSAGE_KEY_INFO, self.recv_count)?;
        let next_chain = derive_indexed_key(&chain, RATCHET_NEXT_CHAIN_INFO, self.recv_count)?;
        self.recv_count = self.recv_count.checked_add(1).ok_or(LmError::CounterExhausted)?;
        self.recv_chain_key = BASE64.encode(next_chain);
        self.updated_at = current_unix_timestamp();
        Ok(RatchetMessageKey {
            header: header.clone(),
            message_key: BASE64.encode(message_key),
        })
    }

    /// Perform a DH-ratchet step after receiving a new remote DH public key.
    ///
    /// This advances root/send/receive chains and rotates the local DH key. It
    /// is exposed now so persistence and tests can cover the future protocol's
    /// most important state transition, even before message envelopes use it.
    pub fn dh_ratchet(&mut self, remote_dh_public_key: &str) -> Result<()> {
        self.validate()?;
        let remote_pub = decode_fixed_32(remote_dh_public_key)?;
        let old_local_secret = X25519Secret::from(decode_fixed_32(&self.local_dh_private_key)?);
        let old_dh = old_local_secret
            .diffie_hellman(&X25519PublicKey::from(remote_pub))
            .to_bytes();
        let root = decode_fixed_32(&self.root_key)?;
        let new_recv_chain = derive_chain_key(&root, &old_dh, b"recv")?;
        let new_root = crypto::hkdf_32(
            &[root.as_slice(), old_dh.as_slice()].concat(),
            RATCHET_ROOT_INFO,
        )?;

        let new_local_secret = random_x25519_secret()?;
        let new_local_pub = X25519PublicKey::from(&new_local_secret).to_bytes();
        let new_dh = new_local_secret
            .diffie_hellman(&X25519PublicKey::from(remote_pub))
            .to_bytes();
        let new_send_chain = derive_chain_key(&new_root, &new_dh, b"send")?;
        let newer_root = crypto::hkdf_32(
            &[new_root.as_slice(), new_dh.as_slice()].concat(),
            RATCHET_ROOT_INFO,
        )?;

        self.previous_send_count = self.send_count;
        self.send_count = 0;
        self.recv_count = 0;
        self.root_key = BASE64.encode(newer_root);
        self.recv_chain_key = BASE64.encode(new_recv_chain);
        self.send_chain_key = BASE64.encode(new_send_chain);
        self.remote_dh_public_key = remote_dh_public_key.to_string();
        self.local_dh_private_key = BASE64.encode(new_local_secret.to_bytes());
        self.local_dh_public_key = BASE64.encode(new_local_pub);
        self.updated_at = current_unix_timestamp();
        Ok(())
    }
}

fn random_x25519_secret() -> Result<X25519Secret> {
    let mut secret = [0u8; 32];
    getrandom(&mut secret).map_err(|_| LmError::RandomFailed)?;
    let result = X25519Secret::from(secret);
    secret.zeroize();
    Ok(result)
}

fn derive_chain_key(root_key: &[u8; 32], dh_out: &[u8; 32], label: &[u8]) -> Result<[u8; 32]> {
    let mut input = Vec::with_capacity(64);
    input.extend_from_slice(root_key);
    input.extend_from_slice(dh_out);
    let mut info = Vec::from(RATCHET_CHAIN_INFO);
    info.push(0);
    info.extend_from_slice(label);
    crypto::hkdf_32(&input, &info)
}

fn derive_initial_chain_key(root_key: &[u8; 32], label: &[u8]) -> Result<[u8; 32]> {
    let mut info = Vec::from(RATCHET_CHAIN_INFO);
    info.push(0);
    info.extend_from_slice(label);
    crypto::hkdf_32(root_key, &info)
}

fn derive_session_id(shared_secret: &[u8; 32], a: &UserId, b: &UserId) -> Result<String> {
    let (left, right) = if a.as_str() <= b.as_str() {
        (a, b)
    } else {
        (b, a)
    };
    let mut input = Vec::new();
    input.extend_from_slice(shared_secret);
    input.extend_from_slice(left.as_str().as_bytes());
    input.push(0);
    input.extend_from_slice(right.as_str().as_bytes());
    Ok(BASE64.encode(crypto::hkdf_32(&input, b"lm-talk.ratchet.session-id.v1")?))
}

fn derive_indexed_key(chain_key: &[u8; 32], info: &[u8], index: u32) -> Result<[u8; 32]> {
    let mut context = Vec::from(info);
    context.extend_from_slice(&index.to_be_bytes());
    crypto::hkdf_32(chain_key, &context)
}

fn skipped_key_id(dh_public_key: &str, message_number: u32) -> String {
    format!("{}:{}", dh_public_key, message_number)
}

fn decode_fixed_32(value: &str) -> Result<[u8; 32]> {
    let bytes = BASE64
        .decode(value.as_bytes())
        .map_err(|_| LmError::CryptoError)?;
    bytes.try_into().map_err(|_| LmError::CryptoError)
}

fn current_unix_timestamp() -> u64 {
    crate::unix_now()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (UserId, UserId) {
        (
            UserId::from_raw("lm1_alice".to_string()).unwrap(),
            UserId::from_raw("lm1_bob".to_string()).unwrap(),
        )
    }

    #[test]
    fn pair_send_receive_advances_matching_keys() {
        let (alice_id, bob_id) = ids();
        let (mut alice, mut bob) = RatchetSessionState::new_pair(alice_id, bob_id).unwrap();
        let sent = alice.next_sending_key().unwrap();
        assert_eq!(sent.header.message_number, 0);
        assert_eq!(alice.send_count, 1);
        let received = bob.next_receiving_key(&sent.header).unwrap();
        assert_eq!(sent.message_key, received.message_key);
        assert_eq!(bob.recv_count, 1);
    }

    #[test]
    fn out_of_order_receive_stores_and_consumes_skipped_key() {
        let (alice_id, bob_id) = ids();
        let (mut alice, mut bob) = RatchetSessionState::new_pair(alice_id, bob_id).unwrap();
        let first = alice.next_sending_key().unwrap();
        let second = alice.next_sending_key().unwrap();
        let second_recv = bob.next_receiving_key(&second.header).unwrap();
        assert_eq!(second.message_key, second_recv.message_key);
        assert_eq!(bob.skipped_message_keys.len(), 1);
        let first_recv = bob.next_receiving_key(&first.header).unwrap();
        assert_eq!(first.message_key, first_recv.message_key);
        assert!(bob.skipped_message_keys.is_empty());
        assert_eq!(
            bob.next_receiving_key(&first.header).unwrap_err(),
            LmError::ReplayDetected
        );
    }

    #[test]
    fn export_import_roundtrip_preserves_state() {
        let (alice_id, bob_id) = ids();
        let (mut alice, _) = RatchetSessionState::new_pair(alice_id, bob_id).unwrap();
        alice.next_sending_key().unwrap();
        let text = alice.to_export_text().unwrap();
        assert!(text.starts_with(RATCHET_STATE_PREFIX));
        let restored = RatchetSessionState::from_export_text(&text).unwrap();
        assert_eq!(restored, alice);
    }

    #[test]
    fn shared_secret_pair_can_exchange_message_keys() {
        let (alice_id, bob_id) = ids();
        let shared = [7u8; 32];
        let (mut alice, mut bob) =
            RatchetSessionState::new_pair_from_shared_secret(alice_id, bob_id, &shared).unwrap();
        assert_eq!(alice.session_id, bob.session_id);
        let sent = alice.next_sending_key().unwrap();
        let received = bob.next_receiving_key(&sent.header).unwrap();
        assert_eq!(sent.message_key, received.message_key);
    }

    #[test]
    fn dh_ratchet_rotates_keys_and_resets_counters() {
        let (alice_id, bob_id) = ids();
        let (mut alice, bob) = RatchetSessionState::new_pair(alice_id, bob_id).unwrap();
        alice.next_sending_key().unwrap();
        let old_local_pub = alice.local_dh_public_key.clone();
        alice.dh_ratchet(&bob.local_dh_public_key).unwrap();
        assert_ne!(alice.local_dh_public_key, old_local_pub);
        assert_eq!(alice.send_count, 0);
        assert_eq!(alice.recv_count, 0);
        assert_eq!(alice.previous_send_count, 1);
    }
}
