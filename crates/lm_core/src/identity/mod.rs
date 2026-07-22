//! Identity creation, backup, restore, and UserID generation.

use crate::{LmError, Result, crypto, limits, normalize_passphrase, protocol};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use data_encoding::BASE32_NOPAD;
use ed25519_dalek::{SigningKey, VerifyingKey};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const IDENTITY_SEED_LEN: usize = 32;
pub const BACKUP_SALT_LEN: usize = 32;
pub const BACKUP_NONCE_LEN: usize = 24;
pub const USER_ID_HASH_BYTES: usize = 25;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(String);

impl UserId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_raw(value: String) -> Result<Self> {
        if value.starts_with("lm1_") && value.len() > 4 {
            Ok(Self(value))
        } else {
            Err(LmError::InvalidUserId)
        }
    }

    pub fn from_identity_public_key(public_key: &[u8]) -> Self {
        let hash = blake3::hash(public_key);
        let encoded = BASE32_NOPAD.encode(&hash.as_bytes()[..USER_ID_HASH_BYTES]);
        Self(format!("lm1_{}", encoded.to_ascii_lowercase()))
    }

    pub fn verify_public_key(&self, public_key: &[u8]) -> bool {
        self == &Self::from_identity_public_key(public_key)
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct IdentitySeed(pub(crate) [u8; IDENTITY_SEED_LEN]);

impl IdentitySeed {
    pub fn random() -> Result<Self> {
        let mut seed = [0u8; IDENTITY_SEED_LEN];
        getrandom(&mut seed).map_err(|_| LmError::RandomFailed)?;
        Ok(Self(seed))
    }

    pub fn from_bytes(bytes: [u8; IDENTITY_SEED_LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; IDENTITY_SEED_LEN] {
        &self.0
    }
}

impl std::fmt::Debug for IdentitySeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IdentitySeed([REDACTED])")
    }
}

#[derive(Clone)]
pub struct Identity {
    user_id: UserId,
    seed: IdentitySeed,
    signing_key: SigningKey,
    x25519_secret: X25519Secret,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("user_id", &self.user_id)
            .field("seed", &"[REDACTED]")
            .field("signing_key", &"[REDACTED]")
            .field("x25519_secret", &"[REDACTED]")
            .finish()
    }
}

impl Identity {
    pub fn create_with_passphrase(passphrase: &str) -> Result<(Self, IdentityBackupPackage)> {
        let seed = IdentitySeed::random()?;
        let identity = Self::from_seed(seed.clone())?;
        let backup = IdentityBackupPackage::encrypt(&identity, passphrase)?;
        Ok((identity, backup))
    }

    pub fn restore_from_backup(backup: &IdentityBackupPackage, passphrase: &str) -> Result<Self> {
        let seed = backup.decrypt_seed(passphrase)?;
        let identity = Self::from_seed(seed)?;
        if identity.user_id != backup.user_id {
            return Err(LmError::CorruptedBackup);
        }
        Ok(identity)
    }

    pub fn from_seed(seed: IdentitySeed) -> Result<Self> {
        let ed_seed = crypto::hkdf_32(seed.as_bytes(), crypto::IDENTITY_ED25519_INFO)?;
        let x_seed = crypto::hkdf_32(seed.as_bytes(), crypto::IDENTITY_X25519_INFO)?;
        let signing_key = SigningKey::from_bytes(&ed_seed);
        let x25519_secret = X25519Secret::from(x_seed);
        let user_id = UserId::from_identity_public_key(signing_key.verifying_key().as_bytes());
        Ok(Self {
            user_id,
            seed,
            signing_key,
            x25519_secret,
        })
    }

    #[doc(hidden)]
    pub fn from_parts_for_wasm(
        user_id: UserId,
        seed: IdentitySeed,
        signing_key: SigningKey,
        x25519_secret: X25519Secret,
    ) -> Result<Self> {
        if user_id != UserId::from_identity_public_key(signing_key.verifying_key().as_bytes()) {
            return Err(LmError::InvalidUserId);
        }
        Ok(Self {
            user_id,
            seed,
            signing_key,
            x25519_secret,
        })
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn identity_public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub fn x25519_public_key(&self) -> [u8; 32] {
        X25519PublicKey::from(&self.x25519_secret).to_bytes()
    }

    pub fn storage_key(&self) -> Result<[u8; 32]> {
        crypto::hkdf_32(self.seed.as_bytes(), crypto::STORAGE_KEY_INFO)
    }

    pub(crate) fn x25519_shared_secret(&self, peer_public_key: &[u8; 32]) -> [u8; 32] {
        self.x25519_shared_secret_public(peer_public_key)
    }

    pub fn x25519_shared_secret_public(&self, peer_public_key: &[u8; 32]) -> [u8; 32] {
        let peer = X25519PublicKey::from(*peer_public_key);
        self.x25519_secret.diffie_hellman(&peer).to_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityBackupPackage {
    pub r#type: String,
    pub version: u16,
    pub user_id: UserId,
    pub kdf: KdfParams,
    pub cipher: CipherParams,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    pub name: String,
    pub salt: String,
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CipherParams {
    pub name: String,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
struct BackupSecret {
    identity_seed: [u8; IDENTITY_SEED_LEN],
    created_at: u64,
}

impl IdentityBackupPackage {
    pub fn encrypt(identity: &Identity, passphrase: &str) -> Result<Self> {
        let mut salt = [0u8; BACKUP_SALT_LEN];
        let mut nonce = [0u8; BACKUP_NONCE_LEN];
        getrandom(&mut salt).map_err(|_| LmError::RandomFailed)?;
        getrandom(&mut nonce).map_err(|_| LmError::RandomFailed)?;

        let kdf = KdfParams {
            name: "argon2id".to_string(),
            salt: BASE64.encode(salt),
            memory_kib: default_backup_memory_kib(),
            iterations: default_backup_iterations(),
            parallelism: 1,
        };

        let normalized = normalize_passphrase(passphrase);
        let key = derive_backup_key(&normalized, &kdf)?;
        let created_at = current_unix_timestamp();
        let secret = BackupSecret {
            identity_seed: *identity.seed.as_bytes(),
            created_at,
        };
        let plaintext = protocol::to_canonical_bytes(&secret)?;
        let ciphertext =
            crypto::xchacha20poly1305_encrypt(&key, &nonce, &plaintext, crypto::BACKUP_AEAD_AAD)?;

        Ok(Self {
            r#type: protocol::LM_IDENTITY_BACKUP_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            user_id: identity.user_id.clone(),
            kdf,
            cipher: CipherParams {
                name: "xchacha20poly1305".to_string(),
                nonce: BASE64.encode(nonce),
                ciphertext: BASE64.encode(ciphertext),
            },
            created_at,
        })
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed(crate::codec::IDENTITY_BACKUP_TEXT_PREFIX, self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_IDENTITY_BACKUP_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed(crate::codec::IDENTITY_BACKUP_TEXT_PREFIX, text)
    }

    pub fn decrypt_seed(&self, passphrase: &str) -> Result<IdentitySeed> {
        self.validate_header()?;
        let normalized = normalize_passphrase(passphrase);
        let key = derive_backup_key(&normalized, &self.kdf)?;
        let nonce = decode_fixed_base64::<BACKUP_NONCE_LEN>(&self.cipher.nonce)?;
        let ciphertext = BASE64
            .decode(self.cipher.ciphertext.as_bytes())
            .map_err(|_| LmError::InvalidBackupFormat)?;
        let plaintext =
            crypto::xchacha20poly1305_decrypt(&key, &nonce, &ciphertext, crypto::BACKUP_AEAD_AAD)
                .map_err(|_| LmError::WrongPassphrase)?;
        let secret: BackupSecret =
            protocol::from_canonical_bytes(&plaintext).map_err(|_| LmError::CorruptedBackup)?;
        Ok(IdentitySeed::from_bytes(secret.identity_seed))
    }

    fn validate_header(&self) -> Result<()> {
        if self.r#type != protocol::LM_IDENTITY_BACKUP_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.kdf.name != "argon2id" || self.cipher.name != "xchacha20poly1305" {
            return Err(LmError::InvalidBackupFormat);
        }
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
const DEFAULT_BACKUP_MEMORY_KIB: u32 = 65_536;

#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_BACKUP_MEMORY_KIB: u32 = 65_536;

fn default_backup_memory_kib() -> u32 {
    DEFAULT_BACKUP_MEMORY_KIB
}

fn default_backup_iterations() -> u32 {
    3
}

fn derive_backup_key(passphrase: &str, params: &KdfParams) -> Result<[u8; 32]> {
    if params.name != "argon2id" {
        return Err(LmError::InvalidBackupFormat);
    }
    let salt = BASE64
        .decode(params.salt.as_bytes())
        .map_err(|_| LmError::InvalidBackupFormat)?;
    let argon_params = Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(32),
    )
    .map_err(|_| LmError::InvalidBackupFormat)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), &salt, &mut out)
        .map_err(|_| LmError::CryptoError)?;
    Ok(out)
}

fn decode_fixed_base64<const N: usize>(value: &str) -> Result<[u8; N]> {
    let decoded = BASE64
        .decode(value.as_bytes())
        .map_err(|_| LmError::InvalidBackupFormat)?;
    decoded.try_into().map_err(|_| LmError::InvalidBackupFormat)
}

fn current_unix_timestamp() -> u64 {
    crate::unix_now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_create_and_restore_roundtrip() {
        let passphrase = "correct horse battery staple";
        let (identity, backup) = Identity::create_with_passphrase(passphrase).unwrap();
        let restored = Identity::restore_from_backup(&backup, passphrase).unwrap();

        assert_eq!(identity.user_id(), restored.user_id());
        assert_eq!(
            identity.identity_public_key(),
            restored.identity_public_key()
        );
        assert_eq!(identity.x25519_public_key(), restored.x25519_public_key());
        assert_eq!(
            identity.storage_key().unwrap(),
            restored.storage_key().unwrap()
        );
    }

    #[test]
    fn identity_restore_rejects_wrong_passphrase() {
        let (_identity, backup) = Identity::create_with_passphrase("right").unwrap();
        let err = Identity::restore_from_backup(&backup, "wrong").unwrap_err();
        assert_eq!(err, LmError::WrongPassphrase);
    }

    #[test]
    fn backup_export_text_roundtrip() {
        let (_identity, backup) = Identity::create_with_passphrase(" pass ").unwrap();
        let text = backup.to_export_text().unwrap();
        assert!(text.starts_with(crate::codec::IDENTITY_BACKUP_TEXT_PREFIX));
        let decoded = IdentityBackupPackage::from_export_text(&text).unwrap();
        let restored = Identity::restore_from_backup(&decoded, "pass").unwrap();
        assert_eq!(restored.user_id(), &backup.user_id);
    }

    #[test]
    fn normalized_passphrase_restores() {
        let (_identity, backup) = Identity::create_with_passphrase("ＡＢＣ  １２３").unwrap();
        Identity::restore_from_backup(&backup, "ABC 123").unwrap();
    }

    #[test]
    fn user_id_matches_public_key() {
        let (identity, _backup) = Identity::create_with_passphrase("pass").unwrap();
        assert!(
            identity
                .user_id()
                .verify_public_key(&identity.identity_public_key())
        );
    }
}
