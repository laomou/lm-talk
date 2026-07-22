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

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn hkdf_is_deterministic() {
        let a = hkdf_32(&[7u8; 32], b"info.v1").unwrap();
        let b = hkdf_32(&[7u8; 32], b"info.v1").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn hkdf_info_changes_output() {
        let a = hkdf_32(&[7u8; 32], b"info.a").unwrap();
        let b = hkdf_32(&[7u8; 32], b"info.b").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn hkdf_ikm_changes_output() {
        let a = hkdf_32(&[7u8; 32], b"info").unwrap();
        let b = hkdf_32(&[8u8; 32], b"info").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn hkdf_known_answer_is_stable() {
        // Locks the HKDF-SHA256 (salt=None) output for a fixed input so an
        // accidental change to the algorithm/params is caught.
        let out = hkdf_32(&[0u8; 32], b"lm-talk.test.kat.v1").unwrap();
        assert_eq!(
            hex(&out),
            "ffcec3d8f3357e870cd444c0dbcddb82499233873b9300a7e3578d313c5715bb"
        );
    }

    #[test]
    fn aead_encrypt_decrypt_roundtrip() {
        let key = [1u8; 32];
        let nonce = [2u8; 24];
        let plaintext = b"hello lm-talk";
        let aad = b"aad-context";
        let ct = xchacha20poly1305_encrypt(&key, &nonce, plaintext, aad).unwrap();
        assert_ne!(&ct[..], &plaintext[..]);
        let pt = xchacha20poly1305_decrypt(&key, &nonce, &ct, aad).unwrap();
        assert_eq!(&pt, plaintext);
    }

    #[test]
    fn aead_wrong_key_fails() {
        let ct = xchacha20poly1305_encrypt(&[1u8; 32], &[2u8; 24], b"secret", b"").unwrap();
        assert_eq!(
            xchacha20poly1305_decrypt(&[9u8; 32], &[2u8; 24], &ct, b"").unwrap_err(),
            LmError::DecryptionFailed
        );
    }

    #[test]
    fn aead_wrong_nonce_fails() {
        let ct = xchacha20poly1305_encrypt(&[1u8; 32], &[2u8; 24], b"secret", b"").unwrap();
        assert_eq!(
            xchacha20poly1305_decrypt(&[1u8; 32], &[3u8; 24], &ct, b"").unwrap_err(),
            LmError::DecryptionFailed
        );
    }

    #[test]
    fn aead_wrong_aad_fails() {
        let ct = xchacha20poly1305_encrypt(&[1u8; 32], &[2u8; 24], b"secret", b"aad-1").unwrap();
        assert_eq!(
            xchacha20poly1305_decrypt(&[1u8; 32], &[2u8; 24], &ct, b"aad-2").unwrap_err(),
            LmError::DecryptionFailed
        );
    }

    #[test]
    fn aead_tampered_ciphertext_fails() {
        let mut ct = xchacha20poly1305_encrypt(&[1u8; 32], &[2u8; 24], b"secret", b"").unwrap();
        ct[0] ^= 0xff;
        assert_eq!(
            xchacha20poly1305_decrypt(&[1u8; 32], &[2u8; 24], &ct, b"").unwrap_err(),
            LmError::DecryptionFailed
        );
    }
}
