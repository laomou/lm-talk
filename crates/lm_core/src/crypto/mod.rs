//! Cryptographic helper functions used by LM Talk protocol objects.

use crate::{LmError, Result};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use sha2::Sha256;

pub const IDENTITY_ED25519_INFO: &[u8] = b"lm-talk.identity.ed25519.v1";
pub const IDENTITY_X25519_INFO: &[u8] = b"lm-talk.identity.x25519.v1";
pub const STORAGE_KEY_INFO: &[u8] = b"lm-talk.storage-key.v1";
pub const BACKUP_AEAD_AAD: &[u8] = b"lm-talk.identity-backup.v1";

// Salt is intentionally None: all callers provide high-entropy IKM (random
// seeds or DH outputs), and most derivations must be deterministic (both
// parties reproduce the same key). Per RFC 5869 §3.1, omitting salt is safe
// when IKM is already uniformly random.
pub fn hkdf_32(input_key_material: &[u8], info: &[u8]) -> Result<[u8; 32]> {
    let hk = Hkdf::<Sha256>::new(None, input_key_material);
    let mut out = [0u8; 32];
    hk.expand(info, &mut out)
        .map_err(|_| LmError::CryptoError)?;
    Ok(out)
}

pub fn xchacha20poly1305_encrypt(
    key: &[u8; 32],
    nonce: &[u8; 24],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = XNonce::try_from(nonce.as_slice()).map_err(|_| LmError::CryptoError)?;
    cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| LmError::CryptoError)
}

pub fn xchacha20poly1305_decrypt(
    key: &[u8; 32],
    nonce: &[u8; 24],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = XNonce::try_from(nonce.as_slice()).map_err(|_| LmError::DecryptionFailed)?;
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| LmError::DecryptionFailed)
}
