//! Friend request and response protocol objects.

use crate::{Identity, LmError, Result, UserId, contact::ContactCard, limits, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendRequest {
    pub r#type: String,
    pub version: u16,
    pub request_id: Uuid,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub from_contact_card: ContactCard,
    pub note: Option<String>,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FriendRequestSignedFields {
    r#type: String,
    version: u16,
    request_id: Uuid,
    from_user_id: UserId,
    to_user_id: UserId,
    from_contact_card: ContactCard,
    note: Option<String>,
    created_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendResponse {
    pub r#type: String,
    pub version: u16,
    pub request_id: Uuid,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub accepted: bool,
    pub created_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FriendResponseSignedFields {
    r#type: String,
    version: u16,
    request_id: Uuid,
    from_user_id: UserId,
    to_user_id: UserId,
    accepted: bool,
    created_at: u64,
}

impl FriendRequest {
    pub fn new(
        from: &Identity,
        to_user_id: UserId,
        from_contact_card: ContactCard,
        note: Option<String>,
        ttl_seconds: u64,
    ) -> Result<Self> {
        if let Some(note) = &note {
            limits::ensure_len(note, limits::MAX_FRIEND_NOTE_BYTES)?;
        }
        from_contact_card.verify()?;
        if from_contact_card.user_id != *from.user_id() {
            return Err(LmError::InvalidUserId);
        }
        let created_at = current_unix_timestamp();
        let signed = FriendRequestSignedFields {
            r#type: protocol::LM_FRIEND_REQUEST_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            request_id: Uuid::new_v4(),
            from_user_id: from.user_id().clone(),
            to_user_id,
            from_contact_card,
            note,
            created_at,
            expires_at: created_at.saturating_add(ttl_seconds),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = from.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            request_id: signed.request_id,
            from_user_id: signed.from_user_id,
            to_user_id: signed.to_user_id,
            from_contact_card: signed.from_contact_card,
            note: signed.note,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed(crate::codec::FRIEND_REQUEST_TEXT_PREFIX, self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_FRIEND_REQUEST_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed(crate::codec::FRIEND_REQUEST_TEXT_PREFIX, text)
    }

    pub fn verify(&self) -> Result<()> {
        if self.r#type != protocol::LM_FRIEND_REQUEST_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.expires_at <= current_unix_timestamp() {
            return Err(LmError::ExpiredObject);
        }
        self.from_contact_card.verify()?;
        if self.from_contact_card.user_id != self.from_user_id {
            return Err(LmError::InvalidUserId);
        }
        let signed = FriendRequestSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            request_id: self.request_id,
            from_user_id: self.from_user_id.clone(),
            to_user_id: self.to_user_id.clone(),
            from_contact_card: self.from_contact_card.clone(),
            note: self.note.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        verify_with_contact_card(
            &self.from_contact_card,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }
}

impl FriendResponse {
    pub fn accept(identity: &Identity, request: &FriendRequest) -> Result<Self> {
        Self::new(identity, request, true)
    }

    pub fn reject(identity: &Identity, request: &FriendRequest) -> Result<Self> {
        Self::new(identity, request, false)
    }

    fn new(identity: &Identity, request: &FriendRequest, accepted: bool) -> Result<Self> {
        request.verify()?;
        if request.to_user_id != *identity.user_id() {
            return Err(LmError::InvalidUserId);
        }
        let signed = FriendResponseSignedFields {
            r#type: protocol::LM_FRIEND_RESPONSE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            request_id: request.request_id,
            from_user_id: identity.user_id().clone(),
            to_user_id: request.from_user_id.clone(),
            accepted,
            created_at: current_unix_timestamp(),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = identity.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            request_id: signed.request_id,
            from_user_id: signed.from_user_id,
            to_user_id: signed.to_user_id,
            accepted: signed.accepted,
            created_at: signed.created_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed(crate::codec::FRIEND_RESPONSE_TEXT_PREFIX, self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_FRIEND_RESPONSE_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed(crate::codec::FRIEND_RESPONSE_TEXT_PREFIX, text)
    }

    pub fn verify(&self, responder_contact_card: &ContactCard) -> Result<()> {
        if self.r#type != protocol::LM_FRIEND_RESPONSE_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        responder_contact_card.verify()?;
        if responder_contact_card.user_id != self.from_user_id {
            return Err(LmError::InvalidUserId);
        }
        let signed = FriendResponseSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            request_id: self.request_id,
            from_user_id: self.from_user_id.clone(),
            to_user_id: self.to_user_id.clone(),
            accepted: self.accepted,
            created_at: self.created_at,
        };
        verify_with_contact_card(
            responder_contact_card,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }
}

fn verify_with_contact_card(card: &ContactCard, bytes: &[u8], signature: &str) -> Result<()> {
    let public_key_bytes = decode_key_32(&card.identity_public_key)?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key_bytes).map_err(|_| LmError::InvalidSignature)?;
    let sig_bytes = decode_signature(signature)?;
    let signature = Signature::from_bytes(&sig_bytes);
    verifying_key
        .verify(bytes, &signature)
        .map_err(|_| LmError::InvalidSignature)
}

fn decode_key_32(value: &str) -> Result<[u8; 32]> {
    let bytes = BASE64
        .decode(value.as_bytes())
        .map_err(|_| LmError::InvalidSignature)?;
    bytes.try_into().map_err(|_| LmError::InvalidSignature)
}

fn decode_signature(value: &str) -> Result<[u8; 64]> {
    let bytes = BASE64
        .decode(value.as_bytes())
        .map_err(|_| LmError::InvalidSignature)?;
    bytes.try_into().map_err(|_| LmError::InvalidSignature)
}

fn current_unix_timestamp() -> u64 {
    crate::unix_now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friend_request_response_roundtrip() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let alice_card = alice
            .export_contact_card(Some("Alice".into()), None, vec![])
            .unwrap();
        let bob_card = bob
            .export_contact_card(Some("Bob".into()), None, vec![])
            .unwrap();

        let request = FriendRequest::new(
            &bob,
            alice.user_id().clone(),
            bob_card.clone(),
            Some("hi".into()),
            3600,
        )
        .unwrap();
        let text = request.to_export_text().unwrap();
        assert!(text.starts_with(crate::codec::FRIEND_REQUEST_TEXT_PREFIX));
        let request = FriendRequest::from_export_text(&text).unwrap();
        request.verify().unwrap();

        let response = FriendResponse::accept(&alice, &request).unwrap();
        let response_text = response.to_export_text().unwrap();
        assert!(response_text.starts_with(crate::codec::FRIEND_RESPONSE_TEXT_PREFIX));
        let response = FriendResponse::from_export_text(&response_text).unwrap();
        response.verify(&alice_card).unwrap();
        assert!(response.accepted);
        assert_eq!(response.to_user_id, *bob.user_id());
    }

    #[test]
    fn tampered_friend_request_fails() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let bob_card = bob.export_contact_card(None, None, vec![]).unwrap();
        let mut request =
            FriendRequest::new(&bob, alice.user_id().clone(), bob_card, None, 3600).unwrap();
        request.note = Some("changed".into());
        assert_eq!(request.verify().unwrap_err(), LmError::InvalidSignature);
    }
}
