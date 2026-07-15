//! File transfer MVP protocol objects.
//!
//! Files are chunked and each chunk is encrypted independently to a recipient
//! using the same static X25519 + HKDF + XChaCha20-Poly1305 MVP construction as
//! direct messages. This is not a streaming transport; Web/UI layers can move
//! the exported chunk JSON over WebRTC, mailbox, QR/text, or any future DHT path.

use crate::{Identity, LmError, Result, UserId, crypto, limits, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const FILE_MANIFEST_TYPE: &str = "lm-file-manifest-v1";
pub const FILE_CHUNK_TYPE: &str = "lm-file-chunk-v1";
pub const FILE_CRYPTO_V1: &str = "x25519-static-hkdf-xchacha20poly1305-file-v1";
const FILE_CHUNK_KEY_INFO: &[u8] = b"lm-talk.file-chunk.v1";
const FILE_NONCE_LEN: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileManifest {
    pub r#type: String,
    pub version: u16,
    pub file_id: Uuid,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub name: String,
    pub mime_type: String,
    pub size: u64,
    pub chunk_size: u32,
    pub chunk_count: u32,
    pub file_hash: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChunkEnvelope {
    pub r#type: String,
    pub version: u16,
    pub crypto: String,
    pub file_id: Uuid,
    pub chunk_index: u32,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FileChunkAad {
    r#type: String,
    version: u16,
    crypto: String,
    file_id: Uuid,
    chunk_index: u32,
    from_user_id: UserId,
    to_user_id: UserId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PlainFileChunk {
    file_id: Uuid,
    chunk_index: u32,
    bytes: Vec<u8>,
}

impl FileManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        from: &Identity,
        to_user_id: UserId,
        name: String,
        mime_type: String,
        size: u64,
        chunk_size: u32,
        chunk_count: u32,
        file_hash: String,
    ) -> Result<Self> {
        if name.trim().is_empty() || chunk_size == 0 || chunk_count == 0 {
            return Err(LmError::InvalidBackupFormat);
        }
        limits::ensure_len(&name, limits::MAX_FILE_NAME_BYTES)?;
        limits::ensure_len(&mime_type, limits::MAX_FILE_MIME_BYTES)?;
        limits::ensure_bytes(size as usize, limits::MAX_FILE_BYTES)?;
        limits::ensure_bytes(chunk_size as usize, limits::MAX_FILE_CHUNK_BYTES)?;
        Ok(Self {
            r#type: FILE_MANIFEST_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            file_id: Uuid::new_v4(),
            from_user_id: from.user_id().clone(),
            to_user_id,
            name,
            mime_type,
            size,
            chunk_size,
            chunk_count,
            file_hash,
            created_at: current_unix_timestamp(),
        })
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-file-manifest-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_FILE_MANIFEST_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-file-manifest-v1:", text)
    }
}

impl FileChunkEnvelope {
    pub fn encrypt_chunk(
        from: &Identity,
        to_user_id: UserId,
        to_x25519_public_key: &[u8; 32],
        file_id: Uuid,
        chunk_index: u32,
        bytes: &[u8],
    ) -> Result<Self> {
        limits::ensure_bytes(bytes.len(), limits::MAX_FILE_CHUNK_BYTES)?;
        let mut nonce = [0u8; FILE_NONCE_LEN];
        getrandom(&mut nonce).map_err(|_| LmError::RandomFailed)?;
        let header = FileChunkAad {
            r#type: FILE_CHUNK_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            crypto: FILE_CRYPTO_V1.to_string(),
            file_id,
            chunk_index,
            from_user_id: from.user_id().clone(),
            to_user_id,
        };
        let plain = PlainFileChunk {
            file_id,
            chunk_index,
            bytes: bytes.to_vec(),
        };
        let key = derive_file_key(
            from,
            to_x25519_public_key,
            &header.from_user_id,
            &header.to_user_id,
            file_id,
        )?;
        let aad = protocol::to_canonical_bytes(&header)?;
        let plaintext = protocol::to_canonical_bytes(&plain)?;
        let ciphertext = crypto::xchacha20poly1305_encrypt(&key, &nonce, &plaintext, &aad)?;
        Ok(Self {
            r#type: header.r#type,
            version: header.version,
            crypto: header.crypto,
            file_id: header.file_id,
            chunk_index: header.chunk_index,
            from_user_id: header.from_user_id,
            to_user_id: header.to_user_id,
            nonce: BASE64.encode(nonce),
            ciphertext: BASE64.encode(ciphertext),
        })
    }

    pub fn decrypt_chunk(
        &self,
        receiver: &Identity,
        sender_x25519_public_key: &[u8; 32],
    ) -> Result<Vec<u8>> {
        self.validate_header()?;
        if self.to_user_id != *receiver.user_id() {
            return Err(LmError::InvalidUserId);
        }
        let nonce = decode_fixed_base64::<FILE_NONCE_LEN>(&self.nonce)?;
        let ciphertext = BASE64
            .decode(self.ciphertext.as_bytes())
            .map_err(|_| LmError::CryptoError)?;
        let header = self.aad_header();
        let key = derive_file_key(
            receiver,
            sender_x25519_public_key,
            &self.from_user_id,
            &self.to_user_id,
            self.file_id,
        )?;
        let aad = protocol::to_canonical_bytes(&header)?;
        let plaintext = crypto::xchacha20poly1305_decrypt(&key, &nonce, &ciphertext, &aad)
            .map_err(|_| LmError::CryptoError)?;
        let plain: PlainFileChunk = protocol::from_canonical_bytes(&plaintext)?;
        if plain.file_id != self.file_id || plain.chunk_index != self.chunk_index {
            return Err(LmError::CryptoError);
        }
        Ok(plain.bytes)
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|_| LmError::SerializationFailed)
    }

    pub fn from_json(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_FILE_CHUNK_JSON_BYTES)?;
        serde_json::from_str(text).map_err(|_| LmError::SerializationFailed)
    }

    fn validate_header(&self) -> Result<()> {
        if self.r#type != FILE_CHUNK_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.crypto != FILE_CRYPTO_V1 {
            return Err(LmError::InvalidBackupFormat);
        }
        limits::ensure_len(&self.nonce, 64)?;
        limits::ensure_len(&self.ciphertext, limits::MAX_FILE_CHUNK_CIPHERTEXT_BYTES)?;
        Ok(())
    }

    fn aad_header(&self) -> FileChunkAad {
        FileChunkAad {
            r#type: self.r#type.clone(),
            version: self.version,
            crypto: self.crypto.clone(),
            file_id: self.file_id,
            chunk_index: self.chunk_index,
            from_user_id: self.from_user_id.clone(),
            to_user_id: self.to_user_id.clone(),
        }
    }
}

pub fn file_hash_base64(bytes: &[u8]) -> String {
    BASE64.encode(blake3::hash(bytes).as_bytes())
}

pub fn verify_file_hash(bytes: &[u8], expected_base64: &str) -> bool {
    file_hash_base64(bytes) == expected_base64
}

fn derive_file_key(
    identity: &Identity,
    peer_x25519_public_key: &[u8; 32],
    from_user_id: &UserId,
    to_user_id: &UserId,
    file_id: Uuid,
) -> Result<[u8; 32]> {
    let shared = identity.x25519_shared_secret(peer_x25519_public_key);
    let mut info = Vec::new();
    info.extend_from_slice(FILE_CHUNK_KEY_INFO);
    info.extend_from_slice(from_user_id.as_str().as_bytes());
    info.push(0);
    info.extend_from_slice(to_user_id.as_str().as_bytes());
    info.push(0);
    info.extend_from_slice(file_id.as_bytes());
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
    fn file_chunk_encrypt_decrypt() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let data = b"hello file";
        let manifest = FileManifest::new(
            &alice,
            bob.user_id().clone(),
            "hello.txt".into(),
            "text/plain".into(),
            data.len() as u64,
            64,
            1,
            file_hash_base64(data),
        )
        .unwrap();
        let chunk = FileChunkEnvelope::encrypt_chunk(
            &alice,
            bob.user_id().clone(),
            &bob.x25519_public_key(),
            manifest.file_id,
            0,
            data,
        )
        .unwrap();
        let plain = chunk
            .decrypt_chunk(&bob, &alice.x25519_public_key())
            .unwrap();
        assert_eq!(plain, data);
        assert!(verify_file_hash(&plain, &manifest.file_hash));
    }

    #[test]
    fn tampered_file_chunk_fails() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let mut chunk = FileChunkEnvelope::encrypt_chunk(
            &alice,
            bob.user_id().clone(),
            &bob.x25519_public_key(),
            Uuid::new_v4(),
            0,
            b"hello",
        )
        .unwrap();
        chunk.chunk_index = 1;
        assert_eq!(
            chunk
                .decrypt_chunk(&bob, &alice.x25519_public_key())
                .unwrap_err(),
            LmError::CryptoError
        );
    }

    #[test]
    fn oversized_file_chunk_ciphertext_is_rejected_before_decode() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let chunk = FileChunkEnvelope {
            r#type: FILE_CHUNK_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            crypto: FILE_CRYPTO_V1.to_string(),
            file_id: Uuid::new_v4(),
            chunk_index: 0,
            from_user_id: alice.user_id().clone(),
            to_user_id: bob.user_id().clone(),
            nonce: BASE64.encode([0u8; FILE_NONCE_LEN]),
            ciphertext: "A".repeat(limits::MAX_FILE_CHUNK_CIPHERTEXT_BYTES + 1),
        };
        assert_eq!(
            chunk
                .decrypt_chunk(&bob, &alice.x25519_public_key())
                .unwrap_err(),
            LmError::PayloadTooLarge
        );
    }
}
