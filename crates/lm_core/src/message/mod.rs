//! Direct message envelope encryption.
//!
//! The original MVP static-X25519 envelope remains for compatibility. This
//! module also contains the first `x3dh-double-ratchet-v1` envelope path backed
//! by [`crate::ratchet::RatchetSessionState`].

use crate::{
    Identity, LmError, Result, UserId, crypto, limits, protocol,
    ratchet::{RatchetHeader, RatchetSessionState},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DIRECT_ENVELOPE_TYPE: &str = "lm-direct-envelope-v1";
pub const PLAIN_MESSAGE_TYPE: &str = "lm-message-v1";
pub const MVP_CRYPTO_V1: &str = "x25519-static-hkdf-xchacha20poly1305-v1";
pub const RATCHET_CRYPTO_V1: &str = "x3dh-double-ratchet-v1";
const DIRECT_MESSAGE_KEY_INFO: &[u8] = b"lm-talk.direct-message.v1";
const DIRECT_NONCE_LEN: usize = 24;
const MAX_MESSAGE_AGE_SECONDS: u64 = 7 * 24 * 60 * 60;
const MAX_MESSAGE_FUTURE_SECONDS: u64 = 5 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectEnvelope {
    pub r#type: String,
    pub version: u16,
    pub crypto: String,
    pub message_id: Uuid,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub created_at: u64,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatchetEnvelope {
    pub r#type: String,
    pub version: u16,
    pub crypto: String,
    pub message_id: Uuid,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub created_at: u64,
    pub ratchet_header: RatchetHeader,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RatchetEnvelopeAad {
    r#type: String,
    version: u16,
    crypto: String,
    message_id: Uuid,
    from_user_id: UserId,
    to_user_id: UserId,
    created_at: u64,
    ratchet_header: RatchetHeader,
    nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DirectEnvelopeAad {
    r#type: String,
    version: u16,
    crypto: String,
    message_id: Uuid,
    from_user_id: UserId,
    to_user_id: UserId,
    created_at: u64,
    nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlainMessage {
    pub r#type: String,
    pub version: u16,
    pub message_id: Uuid,
    pub conversation_id: String,
    pub sender_user_id: UserId,
    pub body: MessageBody,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageBody {
    Text { text: String },
}

/// Versioned session encryption abstraction.
///
/// Current implementation is [`MvpSessionCrypto`]. Future X3DH / Double
/// Ratchet implementations should satisfy this trait so application layers do
/// not depend on a specific message crypto protocol.
pub trait SessionCrypto {
    fn crypto_id(&self) -> &'static str;
    fn encrypt_plain(&self, plain: PlainMessage) -> Result<DirectEnvelope>;
    fn decrypt_envelope(&self, envelope: &DirectEnvelope) -> Result<PlainMessage>;
}

pub struct MvpSessionCrypto<'a> {
    identity: &'a Identity,
    peer_user_id: UserId,
    peer_x25519_public_key: [u8; 32],
    direction: SessionDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionDirection {
    Outbound,
    Inbound,
}

impl<'a> MvpSessionCrypto<'a> {
    pub fn outbound(
        identity: &'a Identity,
        peer_user_id: UserId,
        peer_x25519_public_key: [u8; 32],
    ) -> Self {
        Self {
            identity,
            peer_user_id,
            peer_x25519_public_key,
            direction: SessionDirection::Outbound,
        }
    }

    pub fn inbound(
        identity: &'a Identity,
        peer_user_id: UserId,
        peer_x25519_public_key: [u8; 32],
    ) -> Self {
        Self {
            identity,
            peer_user_id,
            peer_x25519_public_key,
            direction: SessionDirection::Inbound,
        }
    }

    pub fn encrypt_text(&self, conversation_id: String, text: String) -> Result<DirectEnvelope> {
        limits::ensure_len(&text, limits::MAX_DIRECT_MESSAGE_TEXT_BYTES)?;
        let plain = PlainMessage {
            r#type: PLAIN_MESSAGE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            message_id: Uuid::new_v4(),
            conversation_id,
            sender_user_id: self.identity.user_id().clone(),
            body: MessageBody::Text { text },
            created_at: current_unix_timestamp(),
        };
        self.encrypt_plain(plain)
    }
}

impl SessionCrypto for MvpSessionCrypto<'_> {
    fn crypto_id(&self) -> &'static str {
        MVP_CRYPTO_V1
    }

    fn encrypt_plain(&self, plain: PlainMessage) -> Result<DirectEnvelope> {
        if self.direction != SessionDirection::Outbound {
            return Err(LmError::CryptoError);
        }
        DirectEnvelope::encrypt_plain(
            self.identity,
            self.peer_user_id.clone(),
            &self.peer_x25519_public_key,
            plain,
        )
    }

    fn decrypt_envelope(&self, envelope: &DirectEnvelope) -> Result<PlainMessage> {
        if self.direction != SessionDirection::Inbound {
            return Err(LmError::CryptoError);
        }
        if envelope.from_user_id != self.peer_user_id {
            return Err(LmError::InvalidUserId);
        }
        envelope.decrypt(self.identity, &self.peer_x25519_public_key)
    }
}

impl RatchetEnvelope {
    pub fn encrypt_text(
        state: &mut RatchetSessionState,
        conversation_id: String,
        text: String,
    ) -> Result<Self> {
        limits::ensure_len(&text, limits::MAX_DIRECT_MESSAGE_TEXT_BYTES)?;
        let plain = PlainMessage {
            r#type: PLAIN_MESSAGE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            message_id: Uuid::new_v4(),
            conversation_id,
            sender_user_id: state.local_user_id.clone(),
            body: MessageBody::Text { text },
            created_at: current_unix_timestamp(),
        };
        Self::encrypt_plain(state, plain)
    }

    pub fn encrypt_plain(state: &mut RatchetSessionState, plain: PlainMessage) -> Result<Self> {
        state.validate()?;
        if plain.sender_user_id != state.local_user_id {
            return Err(LmError::InvalidUserId);
        }
        let MessageBody::Text { text } = &plain.body;
        limits::ensure_len(text, limits::MAX_DIRECT_MESSAGE_TEXT_BYTES)?;
        let key = state.next_sending_key()?;
        let message_key = decode_fixed_base64::<32>(&key.message_key)?;
        let mut nonce = [0u8; DIRECT_NONCE_LEN];
        getrandom(&mut nonce).map_err(|_| LmError::RandomFailed)?;
        let nonce_b64 = BASE64.encode(nonce);
        let header = RatchetEnvelopeAad {
            r#type: DIRECT_ENVELOPE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            crypto: RATCHET_CRYPTO_V1.to_string(),
            message_id: plain.message_id,
            from_user_id: state.local_user_id.clone(),
            to_user_id: state.remote_user_id.clone(),
            created_at: current_unix_timestamp(),
            ratchet_header: key.header.clone(),
            nonce: nonce_b64,
        };
        let aad = protocol::to_canonical_bytes(&header)?;
        let plaintext = protocol::to_canonical_bytes(&plain)?;
        let ciphertext = crypto::xchacha20poly1305_encrypt(&message_key, &nonce, &plaintext, &aad)?;
        Ok(Self {
            r#type: header.r#type,
            version: header.version,
            crypto: header.crypto,
            message_id: header.message_id,
            from_user_id: header.from_user_id,
            to_user_id: header.to_user_id,
            created_at: header.created_at,
            ratchet_header: header.ratchet_header,
            nonce: header.nonce,
            ciphertext: BASE64.encode(ciphertext),
        })
    }

    pub fn decrypt(&self, state: &mut RatchetSessionState) -> Result<PlainMessage> {
        self.validate_header()?;
        state.validate()?;
        if self.to_user_id != state.local_user_id || self.from_user_id != state.remote_user_id {
            return Err(LmError::InvalidUserId);
        }
        let key = state.next_receiving_key(&self.ratchet_header)?;
        let message_key = decode_fixed_base64::<32>(&key.message_key)?;
        let nonce = decode_fixed_base64::<DIRECT_NONCE_LEN>(&self.nonce)?;
        let ciphertext = BASE64
            .decode(self.ciphertext.as_bytes())
            .map_err(|_| LmError::CryptoError)?;
        let aad = protocol::to_canonical_bytes(&self.aad_header())?;
        let plaintext = crypto::xchacha20poly1305_decrypt(&message_key, &nonce, &ciphertext, &aad)
            .map_err(|_| LmError::CryptoError)?;
        let plain: PlainMessage = protocol::from_canonical_bytes(&plaintext)?;
        if plain.message_id != self.message_id || plain.sender_user_id != self.from_user_id {
            return Err(LmError::CryptoError);
        }
        Ok(plain)
    }

    fn validate_header(&self) -> Result<()> {
        if self.r#type != DIRECT_ENVELOPE_TYPE {
            return Err(LmError::InvalidFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.crypto != RATCHET_CRYPTO_V1 {
            return Err(LmError::InvalidFormat);
        }
        validate_timestamp(self.created_at)?;
        Ok(())
    }

    fn aad_header(&self) -> RatchetEnvelopeAad {
        RatchetEnvelopeAad {
            r#type: self.r#type.clone(),
            version: self.version,
            crypto: self.crypto.clone(),
            message_id: self.message_id,
            from_user_id: self.from_user_id.clone(),
            to_user_id: self.to_user_id.clone(),
            created_at: self.created_at,
            ratchet_header: self.ratchet_header.clone(),
            nonce: self.nonce.clone(),
        }
    }
}

impl DirectEnvelope {
    pub fn encrypt_text(
        from: &Identity,
        to_user_id: UserId,
        to_x25519_public_key: &[u8; 32],
        conversation_id: String,
        text: String,
    ) -> Result<Self> {
        let session = MvpSessionCrypto::outbound(from, to_user_id, *to_x25519_public_key);
        session.encrypt_text(conversation_id, text)
    }

    pub fn encrypt_plain(
        from: &Identity,
        to_user_id: UserId,
        to_x25519_public_key: &[u8; 32],
        plain: PlainMessage,
    ) -> Result<Self> {
        if plain.sender_user_id != *from.user_id() {
            return Err(LmError::InvalidUserId);
        }
        let MessageBody::Text { text } = &plain.body;
        limits::ensure_len(text, limits::MAX_DIRECT_MESSAGE_TEXT_BYTES)?;
        let mut nonce = [0u8; DIRECT_NONCE_LEN];
        getrandom(&mut nonce).map_err(|_| LmError::RandomFailed)?;
        let nonce_b64 = BASE64.encode(nonce);
        let header = DirectEnvelopeAad {
            r#type: DIRECT_ENVELOPE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            crypto: MVP_CRYPTO_V1.to_string(),
            message_id: plain.message_id,
            from_user_id: from.user_id().clone(),
            to_user_id,
            created_at: current_unix_timestamp(),
            nonce: nonce_b64,
        };
        let key = derive_direct_key(
            from,
            to_x25519_public_key,
            &header.from_user_id,
            &header.to_user_id,
        )?;
        let aad = protocol::to_canonical_bytes(&header)?;
        let plaintext = protocol::to_canonical_bytes(&plain)?;
        let ciphertext = crypto::xchacha20poly1305_encrypt(&key, &nonce, &plaintext, &aad)?;
        Ok(Self {
            r#type: header.r#type,
            version: header.version,
            crypto: header.crypto,
            message_id: header.message_id,
            from_user_id: header.from_user_id,
            to_user_id: header.to_user_id,
            created_at: header.created_at,
            nonce: header.nonce,
            ciphertext: BASE64.encode(ciphertext),
        })
    }

    pub fn decrypt(
        &self,
        receiver: &Identity,
        sender_x25519_public_key: &[u8; 32],
    ) -> Result<PlainMessage> {
        self.validate_header()?;
        if self.to_user_id != *receiver.user_id() {
            return Err(LmError::InvalidUserId);
        }
        let nonce = decode_fixed_base64::<DIRECT_NONCE_LEN>(&self.nonce)?;
        let ciphertext = BASE64
            .decode(self.ciphertext.as_bytes())
            .map_err(|_| LmError::CryptoError)?;
        let header = self.aad_header();
        let key = derive_direct_key(
            receiver,
            sender_x25519_public_key,
            &self.from_user_id,
            &self.to_user_id,
        )?;
        let aad = protocol::to_canonical_bytes(&header)?;
        let plaintext = crypto::xchacha20poly1305_decrypt(&key, &nonce, &ciphertext, &aad)
            .map_err(|_| LmError::CryptoError)?;
        let plain: PlainMessage = protocol::from_canonical_bytes(&plaintext)?;
        if plain.message_id != self.message_id || plain.sender_user_id != self.from_user_id {
            return Err(LmError::CryptoError);
        }
        Ok(plain)
    }

    fn validate_header(&self) -> Result<()> {
        if self.r#type != DIRECT_ENVELOPE_TYPE {
            return Err(LmError::InvalidFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.crypto != MVP_CRYPTO_V1 {
            return Err(LmError::InvalidFormat);
        }
        validate_timestamp(self.created_at)?;
        Ok(())
    }

    fn aad_header(&self) -> DirectEnvelopeAad {
        DirectEnvelopeAad {
            r#type: self.r#type.clone(),
            version: self.version,
            crypto: self.crypto.clone(),
            message_id: self.message_id,
            from_user_id: self.from_user_id.clone(),
            to_user_id: self.to_user_id.clone(),
            created_at: self.created_at,
            nonce: self.nonce.clone(),
        }
    }
}

fn validate_timestamp(created_at: u64) -> Result<()> {
    let now = current_unix_timestamp();
    if created_at > now.saturating_add(MAX_MESSAGE_FUTURE_SECONDS) {
        return Err(LmError::ExpiredObject);
    }
    if now.saturating_sub(created_at) > MAX_MESSAGE_AGE_SECONDS {
        return Err(LmError::ExpiredObject);
    }
    Ok(())
}

fn derive_direct_key(
    identity: &Identity,
    peer_x25519_public_key: &[u8; 32],
    from_user_id: &UserId,
    to_user_id: &UserId,
) -> Result<[u8; 32]> {
    let shared = identity.x25519_shared_secret(peer_x25519_public_key);
    let mut info = Vec::new();
    info.extend_from_slice(DIRECT_MESSAGE_KEY_INFO);
    info.extend_from_slice(from_user_id.as_str().as_bytes());
    info.push(0);
    info.extend_from_slice(to_user_id.as_str().as_bytes());
    crypto::hkdf_32(&shared, &info)
}

fn decode_fixed_base64<const N: usize>(value: &str) -> Result<[u8; N]> {
    let decoded = BASE64
        .decode(value.as_bytes())
        .map_err(|_| LmError::CryptoError)?;
    decoded.try_into().map_err(|_| LmError::CryptoError)
}

fn current_unix_timestamp() -> u64 {
    crate::unix_now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_message_encrypt_decrypt() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let envelope = DirectEnvelope::encrypt_text(
            &alice,
            bob.user_id().clone(),
            &bob.x25519_public_key(),
            "conv1".into(),
            "hello".into(),
        )
        .unwrap();
        let plain = envelope.decrypt(&bob, &alice.x25519_public_key()).unwrap();
        assert_eq!(plain.sender_user_id, *alice.user_id());
        assert_eq!(
            plain.body,
            MessageBody::Text {
                text: "hello".into()
            }
        );
    }

    #[test]
    fn session_crypto_trait_roundtrip() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let outbound =
            MvpSessionCrypto::outbound(&alice, bob.user_id().clone(), bob.x25519_public_key());
        assert_eq!(outbound.crypto_id(), MVP_CRYPTO_V1);
        let envelope = outbound
            .encrypt_text("conv1".into(), "hello trait".into())
            .unwrap();
        let inbound =
            MvpSessionCrypto::inbound(&bob, alice.user_id().clone(), alice.x25519_public_key());
        let plain = inbound.decrypt_envelope(&envelope).unwrap();
        assert_eq!(
            plain.body,
            MessageBody::Text {
                text: "hello trait".into()
            }
        );
    }

    #[test]
    fn ratchet_envelope_encrypt_decrypt() {
        let alice_id = UserId::from_raw("lm1_alice".to_string()).unwrap();
        let bob_id = UserId::from_raw("lm1_bob".to_string()).unwrap();
        let (mut alice_state, mut bob_state) =
            RatchetSessionState::new_pair(alice_id, bob_id).unwrap();
        let envelope =
            RatchetEnvelope::encrypt_text(&mut alice_state, "conv1".into(), "hello ratchet".into())
                .unwrap();
        assert_eq!(envelope.crypto, RATCHET_CRYPTO_V1);
        let plain = envelope.decrypt(&mut bob_state).unwrap();
        assert_eq!(plain.sender_user_id, envelope.from_user_id);
        assert_eq!(
            plain.body,
            MessageBody::Text {
                text: "hello ratchet".into()
            }
        );
        assert_eq!(alice_state.send_count, 1);
        assert_eq!(bob_state.recv_count, 1);
    }

    #[test]
    fn ratchet_envelope_tamper_fails() {
        let alice_id = UserId::from_raw("lm1_alice".to_string()).unwrap();
        let bob_id = UserId::from_raw("lm1_bob".to_string()).unwrap();
        let (mut alice_state, mut bob_state) =
            RatchetSessionState::new_pair(alice_id, bob_id).unwrap();
        let mut envelope =
            RatchetEnvelope::encrypt_text(&mut alice_state, "conv1".into(), "hello".into())
                .unwrap();
        envelope.created_at = envelope.created_at.saturating_add(1);
        assert_eq!(
            envelope.decrypt(&mut bob_state).unwrap_err(),
            LmError::CryptoError
        );
    }

    #[test]
    fn tampered_direct_message_fails() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let mut envelope = DirectEnvelope::encrypt_text(
            &alice,
            bob.user_id().clone(),
            &bob.x25519_public_key(),
            "conv1".into(),
            "hello".into(),
        )
        .unwrap();
        envelope.created_at = envelope.created_at.saturating_add(1);
        assert_eq!(
            envelope
                .decrypt(&bob, &alice.x25519_public_key())
                .unwrap_err(),
            LmError::CryptoError
        );
    }
}
