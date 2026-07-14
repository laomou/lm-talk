//! Contact card import/export and verification.

use crate::{Identity, LmError, Result, UserId, device::DeviceCert, limits, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactCard {
    pub r#type: String,
    pub version: u16,
    pub user_id: UserId,
    pub display_name: Option<String>,
    pub identity_public_key: String,
    pub x25519_public_key: String,
    pub device_certs: Vec<DeviceCert>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ContactCardSignedFields {
    r#type: String,
    version: u16,
    user_id: UserId,
    display_name: Option<String>,
    identity_public_key: String,
    x25519_public_key: String,
    device_certs: Vec<DeviceCert>,
    created_at: u64,
    expires_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub user_id: UserId,
    pub display_name: Option<String>,
    pub identity_public_key: String,
    pub x25519_public_key: String,
    pub device_certs: Vec<DeviceCert>,
    pub state: ContactState,
    pub trust_level: TrustLevel,
    pub added_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactState {
    LocalOnly,
    RequestSent,
    RequestReceived,
    Friend,
    Rejected,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    Imported,
    LinkImported,
    QrScanned,
    FingerprintVerified,
}

impl Identity {
    pub fn export_contact_card(
        &self,
        display_name: Option<String>,
        expires_at: Option<u64>,
        device_certs: Vec<DeviceCert>,
    ) -> Result<ContactCard> {
        ContactCard::new(self, display_name, expires_at, device_certs)
    }
}

impl ContactCard {
    pub fn new(
        identity: &Identity,
        display_name: Option<String>,
        expires_at: Option<u64>,
        device_certs: Vec<DeviceCert>,
    ) -> Result<Self> {
        if let Some(name) = &display_name {
            limits::ensure_len(name, limits::MAX_DISPLAY_NAME_BYTES)?;
        }
        limits::ensure_vec_len(&device_certs, limits::MAX_CONTACT_DEVICE_CERTS)?;
        let created_at = current_unix_timestamp();
        let identity_public_key = identity.identity_public_key();
        for cert in &device_certs {
            cert.verify(&identity_public_key)?;
            if cert.user_id != *identity.user_id() {
                return Err(LmError::InvalidUserId);
            }
        }
        let signed = ContactCardSignedFields {
            r#type: protocol::LM_CONTACT_CARD_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            user_id: identity.user_id().clone(),
            display_name,
            identity_public_key: BASE64.encode(identity.identity_public_key()),
            x25519_public_key: BASE64.encode(identity.x25519_public_key()),
            device_certs,
            created_at,
            expires_at,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = identity.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            user_id: signed.user_id,
            display_name: signed.display_name,
            identity_public_key: signed.identity_public_key,
            x25519_public_key: signed.x25519_public_key,
            device_certs: signed.device_certs,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed(crate::codec::CONTACT_CARD_TEXT_PREFIX, self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_CONTACT_CARD_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed(crate::codec::CONTACT_CARD_TEXT_PREFIX, text)
    }

    pub fn verify(&self) -> Result<()> {
        if self.r#type != protocol::LM_CONTACT_CARD_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if let Some(expires_at) = self.expires_at {
            if expires_at <= current_unix_timestamp() {
                return Err(LmError::ExpiredObject);
            }
        }
        let identity_public_key = decode_key_32(&self.identity_public_key)?;
        if !self.user_id.verify_public_key(&identity_public_key) {
            return Err(LmError::InvalidUserId);
        }
        for cert in &self.device_certs {
            cert.verify(&identity_public_key)?;
            if cert.user_id != self.user_id {
                return Err(LmError::InvalidUserId);
            }
        }
        let signed = ContactCardSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            user_id: self.user_id.clone(),
            display_name: self.display_name.clone(),
            identity_public_key: self.identity_public_key.clone(),
            x25519_public_key: self.x25519_public_key.clone(),
            device_certs: self.device_certs.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let verifying_key = VerifyingKey::from_bytes(&identity_public_key)
            .map_err(|_| LmError::InvalidSignature)?;
        let sig_bytes = decode_signature(&self.signature)?;
        let signature = Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(&bytes, &signature)
            .map_err(|_| LmError::InvalidSignature)
    }

    pub fn fingerprint(&self) -> Result<String> {
        let identity_public_key = decode_key_32(&self.identity_public_key)?;
        let hash = blake3::hash(&identity_public_key);
        let bytes = &hash.as_bytes()[..8];
        Ok(format!(
            "{:02X}{:02X} {:02X}{:02X} {:02X}{:02X} {:02X}{:02X}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
        ))
    }

    pub fn into_contact(self, trust_level: TrustLevel) -> Result<Contact> {
        self.verify()?;
        Ok(Contact {
            user_id: self.user_id,
            display_name: self.display_name,
            identity_public_key: self.identity_public_key,
            x25519_public_key: self.x25519_public_key,
            device_certs: self.device_certs,
            state: ContactState::LocalOnly,
            trust_level,
            added_at: current_unix_timestamp(),
        })
    }
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
    fn contact_card_roundtrip() {
        let (identity, _backup) = Identity::create_with_passphrase("pass").unwrap();
        let card = identity
            .export_contact_card(Some("Alice".to_string()), None, vec![])
            .unwrap();
        card.verify().unwrap();
        assert!(!card.fingerprint().unwrap().is_empty());
        let text = card.to_export_text().unwrap();
        assert!(text.starts_with(crate::codec::CONTACT_CARD_TEXT_PREFIX));
        let card = ContactCard::from_export_text(&text).unwrap();
        let contact = card.into_contact(TrustLevel::QrScanned).unwrap();
        assert_eq!(contact.state, ContactState::LocalOnly);
        assert_eq!(contact.trust_level, TrustLevel::QrScanned);
    }

    #[test]
    fn tampered_contact_card_fails() {
        let (identity, _backup) = Identity::create_with_passphrase("pass").unwrap();
        let mut card = identity.export_contact_card(None, None, vec![]).unwrap();
        card.display_name = Some("Mallory".to_string());
        assert_eq!(card.verify().unwrap_err(), LmError::InvalidSignature);
    }

    #[test]
    fn invalid_contact_card_user_id_is_rejected() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (mallory, _m) = Identity::create_with_passphrase("mallory").unwrap();
        let mut card = alice.export_contact_card(None, None, vec![]).unwrap();
        card.user_id = mallory.user_id().clone();
        assert_eq!(card.verify().unwrap_err(), LmError::InvalidUserId);
    }
}
