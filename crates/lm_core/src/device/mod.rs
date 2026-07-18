//! Device identity and device certificate support.

use crate::{Identity, LmError, Result, UserId, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use data_encoding::BASE32_NOPAD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const DEVICE_SEED_LEN: usize = 32;
pub const DEVICE_ID_HASH_BYTES: usize = 25;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_raw(value: String) -> Result<Self> {
        if value.starts_with("dev1_") && value.len() > 5 {
            Ok(Self(value))
        } else {
            Err(LmError::InvalidDeviceId)
        }
    }

    pub fn from_device_public_key(public_key: &[u8]) -> Self {
        let hash = blake3::hash(public_key);
        let encoded = BASE32_NOPAD.encode(&hash.as_bytes()[..DEVICE_ID_HASH_BYTES]);
        Self(format!("dev1_{}", encoded.to_ascii_lowercase()))
    }

    pub fn verify_public_key(&self, public_key: &[u8]) -> bool {
        self == &Self::from_device_public_key(public_key)
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct DeviceSeed([u8; DEVICE_SEED_LEN]);

impl DeviceSeed {
    pub fn from_bytes(seed: [u8; DEVICE_SEED_LEN]) -> Self {
        Self(seed)
    }

    pub fn random() -> Result<Self> {
        let mut seed = [0u8; DEVICE_SEED_LEN];
        getrandom(&mut seed).map_err(|_| LmError::RandomFailed)?;
        Ok(Self(seed))
    }

    pub fn as_bytes(&self) -> &[u8; DEVICE_SEED_LEN] {
        &self.0
    }
}

impl std::fmt::Debug for DeviceSeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeviceSeed([REDACTED])")
    }
}

#[derive(Clone)]
pub struct DeviceIdentity {
    device_id: DeviceId,
    _seed: DeviceSeed,
    signing_key: SigningKey,
    box_secret: X25519Secret,
}

impl std::fmt::Debug for DeviceIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceIdentity")
            .field("device_id", &self.device_id)
            .field("seed", &"[REDACTED]")
            .field("signing_key", &"[REDACTED]")
            .finish()
    }
}

impl DeviceIdentity {
    pub fn random() -> Result<Self> {
        let seed = DeviceSeed::random()?;
        Self::from_seed(seed)
    }

    pub fn from_seed(seed: DeviceSeed) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(seed.as_bytes());
        let box_secret = X25519Secret::from(*seed.as_bytes());
        let device_id = DeviceId::from_device_public_key(signing_key.verifying_key().as_bytes());
        Ok(Self {
            device_id,
            _seed: seed,
            signing_key,
            box_secret,
        })
    }

    pub fn seed_bytes(&self) -> &[u8; DEVICE_SEED_LEN] {
        self._seed.as_bytes()
    }

    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    pub fn device_public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn device_box_public_key(&self) -> [u8; 32] {
        X25519PublicKey::from(&self.box_secret).to_bytes()
    }

    pub fn create_cert(
        &self,
        identity: &Identity,
        device_name: Option<String>,
    ) -> Result<DeviceCert> {
        DeviceCert::new(identity, self, device_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCert {
    pub r#type: String,
    pub version: u16,
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub device_public_key: String,
    pub device_box_public_key: String,
    pub device_name: Option<String>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub signature_by_identity_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DeviceCertSignedFields {
    r#type: String,
    version: u16,
    user_id: UserId,
    device_id: DeviceId,
    device_public_key: String,
    device_box_public_key: String,
    device_name: Option<String>,
    created_at: u64,
    expires_at: Option<u64>,
}

impl DeviceCert {
    pub fn new(
        identity: &Identity,
        device: &DeviceIdentity,
        device_name: Option<String>,
    ) -> Result<Self> {
        let signed = DeviceCertSignedFields {
            r#type: "lm-device-cert-v1".to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            user_id: identity.user_id().clone(),
            device_id: device.device_id().clone(),
            device_public_key: BASE64.encode(device.device_public_key()),
            device_box_public_key: BASE64.encode(device.device_box_public_key()),
            device_name,
            created_at: current_unix_timestamp(),
            expires_at: None,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = identity.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            user_id: signed.user_id,
            device_id: signed.device_id,
            device_public_key: signed.device_public_key,
            device_box_public_key: signed.device_box_public_key,
            device_name: signed.device_name,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature_by_identity_key: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, identity_public_key: &[u8; 32]) -> Result<()> {
        if self.r#type != "lm-device-cert-v1" {
            return Err(LmError::InvalidFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if !self.user_id.verify_public_key(identity_public_key) {
            return Err(LmError::InvalidUserId);
        }
        if let Some(expires_at) = self.expires_at
            && expires_at <= current_unix_timestamp()
        {
            return Err(LmError::ExpiredObject);
        }
        let device_public_key = decode_key_32(&self.device_public_key)?;
        if !self.device_id.verify_public_key(&device_public_key) {
            return Err(LmError::InvalidDeviceId);
        }
        let _device_box_public_key = decode_key_32(&self.device_box_public_key)?;
        let signed = DeviceCertSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            user_id: self.user_id.clone(),
            device_id: self.device_id.clone(),
            device_public_key: self.device_public_key.clone(),
            device_box_public_key: self.device_box_public_key.clone(),
            device_name: self.device_name.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let verifying_key =
            VerifyingKey::from_bytes(identity_public_key).map_err(|_| LmError::InvalidSignature)?;
        let sig_bytes = decode_signature(&self.signature_by_identity_key)?;
        let signature = Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(&bytes, &signature)
            .map_err(|_| LmError::InvalidSignature)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRevoke {
    pub r#type: String,
    pub version: u16,
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub reason: Option<String>,
    pub created_at: u64,
    pub signature_by_identity_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DeviceRevokeSignedFields {
    r#type: String,
    version: u16,
    user_id: UserId,
    device_id: DeviceId,
    reason: Option<String>,
    created_at: u64,
}

impl DeviceRevoke {
    pub fn new(identity: &Identity, device_id: DeviceId, reason: Option<String>) -> Result<Self> {
        let signed = DeviceRevokeSignedFields {
            r#type: "lm-device-revoke-v1".to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            user_id: identity.user_id().clone(),
            device_id,
            reason,
            created_at: current_unix_timestamp(),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = identity.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            user_id: signed.user_id,
            device_id: signed.device_id,
            reason: signed.reason,
            created_at: signed.created_at,
            signature_by_identity_key: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, identity_public_key: &[u8; 32]) -> Result<()> {
        if self.r#type != "lm-device-revoke-v1" {
            return Err(LmError::InvalidFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if !self.user_id.verify_public_key(identity_public_key) {
            return Err(LmError::InvalidUserId);
        }
        let signed = DeviceRevokeSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            user_id: self.user_id.clone(),
            device_id: self.device_id.clone(),
            reason: self.reason.clone(),
            created_at: self.created_at,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let verifying_key =
            VerifyingKey::from_bytes(identity_public_key).map_err(|_| LmError::InvalidSignature)?;
        let sig_bytes = decode_signature(&self.signature_by_identity_key)?;
        let signature = Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(&bytes, &signature)
            .map_err(|_| LmError::InvalidSignature)
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-device-revoke-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        crate::limits::ensure_len(text, crate::limits::MAX_DEVICE_REVOKE_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-device-revoke-v1:", text)
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
    fn device_cert_roundtrip() {
        let (identity, _backup) = Identity::create_with_passphrase("alice").unwrap();
        let device = DeviceIdentity::random().unwrap();
        let cert = device.create_cert(&identity, Some("phone".into())).unwrap();
        cert.verify(&identity.identity_public_key()).unwrap();
    }

    #[test]
    fn device_revoke_roundtrip() {
        let (identity, _backup) = Identity::create_with_passphrase("alice").unwrap();
        let device = DeviceIdentity::random().unwrap();
        let revoke =
            DeviceRevoke::new(&identity, device.device_id().clone(), Some("lost".into())).unwrap();
        revoke.verify(&identity.identity_public_key()).unwrap();
        let text = revoke.to_export_text().unwrap();
        let decoded = DeviceRevoke::from_export_text(&text).unwrap();
        decoded.verify(&identity.identity_public_key()).unwrap();
    }

    #[test]
    fn tampered_device_revoke_fails() {
        let (identity, _backup) = Identity::create_with_passphrase("alice").unwrap();
        let device = DeviceIdentity::random().unwrap();
        let mut revoke = DeviceRevoke::new(&identity, device.device_id().clone(), None).unwrap();
        revoke.reason = Some("changed".into());
        assert_eq!(
            revoke.verify(&identity.identity_public_key()).unwrap_err(),
            LmError::InvalidSignature
        );
    }

    #[test]
    fn tampered_device_cert_fails() {
        let (identity, _backup) = Identity::create_with_passphrase("alice").unwrap();
        let device = DeviceIdentity::random().unwrap();
        let mut cert = device.create_cert(&identity, Some("phone".into())).unwrap();
        cert.device_name = Some("evil".into());
        assert_eq!(
            cert.verify(&identity.identity_public_key()).unwrap_err(),
            LmError::InvalidSignature
        );
    }
}
