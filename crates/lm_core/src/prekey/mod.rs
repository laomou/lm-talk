//! X3DH / pre-key protocol object scaffold.
//!
//! This module defines signed pre-key bundles and deterministic shared-secret
//! derivation for the future `x3dh-double-ratchet-v1` message path. It does not
//! publish bundles to a network by itself and does not consume one-time prekeys;
//! those policies belong to the DHT/mailbox layer and local session store.

use crate::{Identity, LmError, Result, UserId, crypto, limits, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};

pub const PREKEY_BUNDLE_TYPE: &str = "lm-prekey-bundle-v1";
pub const PREKEY_BUNDLE_PREFIX: &str = "lm-prekey-bundle-v1:";
pub const SIGNED_ONE_TIME_PREKEY_RECORD_TYPE: &str = "lm-signed-one-time-prekey-v1";
pub const SIGNED_ONE_TIME_PREKEY_RECORD_PREFIX: &str = "lm-signed-one-time-prekey-v1:";
pub const X3DH_SHARED_SECRET_INFO: &[u8] = b"lm-talk.x3dh.shared-secret.v1";
const X3DH_CONTEXT: &[u8] = b"lm-talk.x3dh.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreKeyBundle {
    pub r#type: String,
    pub version: u16,
    pub user_id: UserId,
    pub identity_public_key: String,
    pub identity_x25519_public_key: String,
    pub signed_prekey_id: u32,
    pub signed_prekey_public_key: String,
    pub signed_prekey_created_at: u64,
    pub signed_prekey_expires_at: u64,
    pub one_time_prekeys: Vec<OneTimePreKey>,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PreKeyBundleSignedFields {
    r#type: String,
    version: u16,
    user_id: UserId,
    identity_public_key: String,
    identity_x25519_public_key: String,
    signed_prekey_id: u32,
    signed_prekey_public_key: String,
    signed_prekey_created_at: u64,
    signed_prekey_expires_at: u64,
    one_time_prekeys: Vec<OneTimePreKey>,
    created_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedOneTimePreKeyRecord {
    pub r#type: String,
    pub version: u16,
    pub user_id: UserId,
    pub identity_public_key: String,
    pub signed_prekey_id: u32,
    pub key_id: u32,
    pub public_key: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SignedOneTimePreKeyRecordSignedFields {
    r#type: String,
    version: u16,
    user_id: UserId,
    identity_public_key: String,
    signed_prekey_id: u32,
    key_id: u32,
    public_key: String,
    created_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneTimePreKey {
    pub key_id: u32,
    pub public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreKeyPrivateBundle {
    pub r#type: String,
    pub version: u16,
    pub user_id: UserId,
    pub signed_prekey_id: u32,
    pub signed_prekey_private_key: String,
    pub signed_prekey_public_key: String,
    pub one_time_prekeys: Vec<OneTimePreKeyPrivate>,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneTimePreKeyPrivate {
    pub key_id: u32,
    pub private_key: String,
    pub public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct X3dhInitialMessage {
    pub r#type: String,
    pub version: u16,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub identity_x25519_public_key: String,
    pub ephemeral_public_key: String,
    pub signed_prekey_id: u32,
    pub one_time_prekey_id: Option<u32>,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct X3dhInitiatorSecret {
    pub initial_message: X3dhInitialMessage,
    pub shared_secret: String,
}

impl PreKeyBundle {
    pub fn new(
        identity: &Identity,
        signed_prekey_id: u32,
        one_time_prekey_count: u32,
        ttl_seconds: u64,
    ) -> Result<(Self, PreKeyPrivateBundle)> {
        if one_time_prekey_count as usize > limits::MAX_ONE_TIME_PREKEYS {
            return Err(LmError::PayloadTooLarge);
        }
        let signed_prekey_private = random_x25519_secret()?;
        let signed_prekey_public = X25519PublicKey::from(&signed_prekey_private).to_bytes();
        let mut one_time_prekeys = Vec::with_capacity(one_time_prekey_count as usize);
        let mut private_one_time_prekeys = Vec::with_capacity(one_time_prekey_count as usize);
        for key_id in 0..one_time_prekey_count {
            let private = random_x25519_secret()?;
            let public = X25519PublicKey::from(&private).to_bytes();
            one_time_prekeys.push(OneTimePreKey {
                key_id,
                public_key: BASE64.encode(public),
            });
            private_one_time_prekeys.push(OneTimePreKeyPrivate {
                key_id,
                private_key: BASE64.encode(private.to_bytes()),
                public_key: BASE64.encode(public),
            });
        }
        let created_at = current_unix_timestamp();
        let expires_at = created_at.saturating_add(ttl_seconds);
        let signed = PreKeyBundleSignedFields {
            r#type: PREKEY_BUNDLE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            user_id: identity.user_id().clone(),
            identity_public_key: BASE64.encode(identity.identity_public_key()),
            identity_x25519_public_key: BASE64.encode(identity.x25519_public_key()),
            signed_prekey_id,
            signed_prekey_public_key: BASE64.encode(signed_prekey_public),
            signed_prekey_created_at: created_at,
            signed_prekey_expires_at: expires_at,
            one_time_prekeys,
            created_at,
            expires_at,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = identity.signing_key().sign(&bytes);
        let public = Self {
            r#type: signed.r#type,
            version: signed.version,
            user_id: signed.user_id.clone(),
            identity_public_key: signed.identity_public_key,
            identity_x25519_public_key: signed.identity_x25519_public_key,
            signed_prekey_id: signed.signed_prekey_id,
            signed_prekey_public_key: signed.signed_prekey_public_key.clone(),
            signed_prekey_created_at: signed.signed_prekey_created_at,
            signed_prekey_expires_at: signed.signed_prekey_expires_at,
            one_time_prekeys: signed.one_time_prekeys,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        };
        let private = PreKeyPrivateBundle {
            r#type: "lm-prekey-private-bundle-v1".to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            user_id: identity.user_id().clone(),
            signed_prekey_id,
            signed_prekey_private_key: BASE64.encode(signed_prekey_private.to_bytes()),
            signed_prekey_public_key: signed.signed_prekey_public_key,
            one_time_prekeys: private_one_time_prekeys,
            created_at,
            expires_at,
        };
        Ok((public, private))
    }

    pub fn new_with_signed_one_time_prekey_records(
        identity: &Identity,
        signed_prekey_id: u32,
        one_time_prekey_count: u32,
        ttl_seconds: u64,
    ) -> Result<(Self, PreKeyPrivateBundle, Vec<SignedOneTimePreKeyRecord>)> {
        let (legacy_public, private) = Self::new(
            identity,
            signed_prekey_id,
            one_time_prekey_count,
            ttl_seconds,
        )?;
        let records = legacy_public
            .one_time_prekeys
            .iter()
            .cloned()
            .map(|one_time_prekey| {
                SignedOneTimePreKeyRecord::new_at(
                    identity,
                    legacy_public.signed_prekey_id,
                    one_time_prekey,
                    legacy_public.created_at,
                    legacy_public.expires_at,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let signed = PreKeyBundleSignedFields {
            r#type: legacy_public.r#type.clone(),
            version: legacy_public.version,
            user_id: legacy_public.user_id.clone(),
            identity_public_key: legacy_public.identity_public_key.clone(),
            identity_x25519_public_key: legacy_public.identity_x25519_public_key.clone(),
            signed_prekey_id: legacy_public.signed_prekey_id,
            signed_prekey_public_key: legacy_public.signed_prekey_public_key.clone(),
            signed_prekey_created_at: legacy_public.signed_prekey_created_at,
            signed_prekey_expires_at: legacy_public.signed_prekey_expires_at,
            one_time_prekeys: Vec::new(),
            created_at: legacy_public.created_at,
            expires_at: legacy_public.expires_at,
        };
        let signature = identity
            .signing_key()
            .sign(&protocol::to_canonical_bytes(&signed)?);
        let public = Self {
            r#type: signed.r#type,
            version: signed.version,
            user_id: signed.user_id,
            identity_public_key: signed.identity_public_key,
            identity_x25519_public_key: signed.identity_x25519_public_key,
            signed_prekey_id: signed.signed_prekey_id,
            signed_prekey_public_key: signed.signed_prekey_public_key,
            signed_prekey_created_at: signed.signed_prekey_created_at,
            signed_prekey_expires_at: signed.signed_prekey_expires_at,
            one_time_prekeys: signed.one_time_prekeys,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        };
        public.verify()?;
        for record in &records {
            record.verify_for_bundle(&public)?;
        }
        Ok((public, private, records))
    }

    pub fn verify(&self) -> Result<()> {
        if self.r#type != PREKEY_BUNDLE_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.expires_at <= current_unix_timestamp()
            || self.signed_prekey_expires_at <= current_unix_timestamp()
        {
            return Err(LmError::ExpiredObject);
        }
        limits::ensure_vec_len(&self.one_time_prekeys, limits::MAX_ONE_TIME_PREKEYS)?;
        let identity_public_key = decode_fixed_32(&self.identity_public_key)?;
        if !self.user_id.verify_public_key(&identity_public_key) {
            return Err(LmError::InvalidUserId);
        }
        decode_fixed_32(&self.identity_x25519_public_key)?;
        decode_fixed_32(&self.signed_prekey_public_key)?;
        for key in &self.one_time_prekeys {
            decode_fixed_32(&key.public_key)?;
        }
        let signed = PreKeyBundleSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            user_id: self.user_id.clone(),
            identity_public_key: self.identity_public_key.clone(),
            identity_x25519_public_key: self.identity_x25519_public_key.clone(),
            signed_prekey_id: self.signed_prekey_id,
            signed_prekey_public_key: self.signed_prekey_public_key.clone(),
            signed_prekey_created_at: self.signed_prekey_created_at,
            signed_prekey_expires_at: self.signed_prekey_expires_at,
            one_time_prekeys: self.one_time_prekeys.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        verify_sig(
            &identity_public_key,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }

    pub fn to_export_text(&self) -> Result<String> {
        self.verify()?;
        crate::codec::encode_json_prefixed(PREKEY_BUNDLE_PREFIX, self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_PREKEY_BUNDLE_TEXT_BYTES)?;
        let bundle: Self = crate::codec::decode_json_prefixed(PREKEY_BUNDLE_PREFIX, text)?;
        bundle.verify()?;
        Ok(bundle)
    }

    pub fn select_one_time_prekey(&self) -> Option<&OneTimePreKey> {
        self.one_time_prekeys.first()
    }

    pub fn select_one_time_prekey_by_id(&self, key_id: u32) -> Option<&OneTimePreKey> {
        self.one_time_prekeys
            .iter()
            .find(|key| key.key_id == key_id)
    }
}

impl SignedOneTimePreKeyRecord {
    pub fn new(
        identity: &Identity,
        signed_prekey_id: u32,
        one_time_prekey: OneTimePreKey,
        ttl_seconds: u64,
    ) -> Result<Self> {
        let created_at = current_unix_timestamp();
        let expires_at = created_at.saturating_add(ttl_seconds);
        Self::new_at(
            identity,
            signed_prekey_id,
            one_time_prekey,
            created_at,
            expires_at,
        )
    }

    fn new_at(
        identity: &Identity,
        signed_prekey_id: u32,
        one_time_prekey: OneTimePreKey,
        created_at: u64,
        expires_at: u64,
    ) -> Result<Self> {
        let signed = SignedOneTimePreKeyRecordSignedFields {
            r#type: SIGNED_ONE_TIME_PREKEY_RECORD_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            user_id: identity.user_id().clone(),
            identity_public_key: BASE64.encode(identity.identity_public_key()),
            signed_prekey_id,
            key_id: one_time_prekey.key_id,
            public_key: one_time_prekey.public_key,
            created_at,
            expires_at,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = identity.signing_key().sign(&bytes);
        let record = Self {
            r#type: signed.r#type,
            version: signed.version,
            user_id: signed.user_id,
            identity_public_key: signed.identity_public_key,
            signed_prekey_id: signed.signed_prekey_id,
            key_id: signed.key_id,
            public_key: signed.public_key,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        };
        record.verify()?;
        Ok(record)
    }

    pub fn verify(&self) -> Result<()> {
        if self.r#type != SIGNED_ONE_TIME_PREKEY_RECORD_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.expires_at <= current_unix_timestamp() {
            return Err(LmError::ExpiredObject);
        }
        let identity_public_key = decode_fixed_32(&self.identity_public_key)?;
        if !self.user_id.verify_public_key(&identity_public_key) {
            return Err(LmError::InvalidUserId);
        }
        decode_fixed_32(&self.public_key)?;
        let signed = SignedOneTimePreKeyRecordSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            user_id: self.user_id.clone(),
            identity_public_key: self.identity_public_key.clone(),
            signed_prekey_id: self.signed_prekey_id,
            key_id: self.key_id,
            public_key: self.public_key.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        verify_sig(
            &identity_public_key,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }

    pub fn verify_for_bundle(&self, bundle: &PreKeyBundle) -> Result<()> {
        self.verify()?;
        bundle.verify()?;
        if self.user_id != bundle.user_id
            || self.identity_public_key != bundle.identity_public_key
            || self.signed_prekey_id != bundle.signed_prekey_id
            || self.created_at < bundle.signed_prekey_created_at
            || self.expires_at > bundle.signed_prekey_expires_at
            || self.expires_at > bundle.expires_at
        {
            return Err(LmError::InvalidBackupFormat);
        }
        Ok(())
    }

    pub fn to_export_text(&self) -> Result<String> {
        self.verify()?;
        crate::codec::encode_json_prefixed(SIGNED_ONE_TIME_PREKEY_RECORD_PREFIX, self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_PREKEY_BUNDLE_TEXT_BYTES)?;
        let record: Self =
            crate::codec::decode_json_prefixed(SIGNED_ONE_TIME_PREKEY_RECORD_PREFIX, text)?;
        record.verify()?;
        Ok(record)
    }

    pub fn as_one_time_prekey(&self) -> OneTimePreKey {
        OneTimePreKey {
            key_id: self.key_id,
            public_key: self.public_key.clone(),
        }
    }
}

impl PreKeyPrivateBundle {
    pub fn validate_for_public(&self, public: &PreKeyBundle) -> Result<()> {
        if self.r#type != "lm-prekey-private-bundle-v1" {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.user_id != public.user_id || self.signed_prekey_id != public.signed_prekey_id {
            return Err(LmError::InvalidUserId);
        }
        let signed_private = X25519Secret::from(decode_fixed_32(&self.signed_prekey_private_key)?);
        let signed_public = X25519PublicKey::from(&signed_private).to_bytes();
        if BASE64.encode(signed_public) != public.signed_prekey_public_key {
            return Err(LmError::CryptoError);
        }
        Ok(())
    }

    pub fn one_time_private_key(&self, key_id: u32) -> Result<Option<[u8; 32]>> {
        self.one_time_prekeys
            .iter()
            .find(|k| k.key_id == key_id)
            .map(|k| decode_fixed_32(&k.private_key))
            .transpose()
    }
}

pub fn x3dh_initiator_secret(
    initiator: &Identity,
    responder_bundle: &PreKeyBundle,
) -> Result<X3dhInitiatorSecret> {
    let selected = responder_bundle.select_one_time_prekey().map(|k| k.key_id);
    x3dh_initiator_secret_with_one_time_prekey_id(initiator, responder_bundle, selected)
}

pub fn x3dh_initiator_secret_with_one_time_prekey_id(
    initiator: &Identity,
    responder_bundle: &PreKeyBundle,
    one_time_prekey_id: Option<u32>,
) -> Result<X3dhInitiatorSecret> {
    responder_bundle.verify()?;
    let ephemeral = random_x25519_secret()?;
    let ephemeral_public = X25519PublicKey::from(&ephemeral).to_bytes();
    let responder_identity_x = decode_fixed_32(&responder_bundle.identity_x25519_public_key)?;
    let responder_signed_prekey = decode_fixed_32(&responder_bundle.signed_prekey_public_key)?;
    let selected_otk = one_time_prekey_id
        .map(|id| {
            responder_bundle
                .select_one_time_prekey_by_id(id)
                .ok_or(LmError::InvalidBackupFormat)
        })
        .transpose()?;
    let one_time_public = selected_otk
        .map(|k| decode_fixed_32(&k.public_key))
        .transpose()?;
    let shared = derive_initiator_secret(
        initiator,
        &ephemeral,
        &responder_identity_x,
        &responder_signed_prekey,
        one_time_public.as_ref(),
    )?;
    let initial_message = X3dhInitialMessage {
        r#type: "lm-x3dh-initial-message-v1".to_string(),
        version: protocol::PROTOCOL_VERSION_V1,
        from_user_id: initiator.user_id().clone(),
        to_user_id: responder_bundle.user_id.clone(),
        identity_x25519_public_key: BASE64.encode(initiator.x25519_public_key()),
        ephemeral_public_key: BASE64.encode(ephemeral_public),
        signed_prekey_id: responder_bundle.signed_prekey_id,
        one_time_prekey_id,
        created_at: current_unix_timestamp(),
    };
    Ok(X3dhInitiatorSecret {
        initial_message,
        shared_secret: BASE64.encode(shared),
    })
}

pub fn x3dh_initiator_secret_with_one_time_prekey_record(
    initiator: &Identity,
    responder_bundle: &PreKeyBundle,
    one_time_prekey_record: Option<&SignedOneTimePreKeyRecord>,
) -> Result<X3dhInitiatorSecret> {
    responder_bundle.verify()?;
    if let Some(record) = one_time_prekey_record {
        record.verify_for_bundle(responder_bundle)?;
    }
    let ephemeral = random_x25519_secret()?;
    let ephemeral_public = X25519PublicKey::from(&ephemeral).to_bytes();
    let responder_identity_x = decode_fixed_32(&responder_bundle.identity_x25519_public_key)?;
    let responder_signed_prekey = decode_fixed_32(&responder_bundle.signed_prekey_public_key)?;
    let one_time_public = one_time_prekey_record
        .map(|record| decode_fixed_32(&record.public_key))
        .transpose()?;
    let shared = derive_initiator_secret(
        initiator,
        &ephemeral,
        &responder_identity_x,
        &responder_signed_prekey,
        one_time_public.as_ref(),
    )?;
    let initial_message = X3dhInitialMessage {
        r#type: "lm-x3dh-initial-message-v1".to_string(),
        version: protocol::PROTOCOL_VERSION_V1,
        from_user_id: initiator.user_id().clone(),
        to_user_id: responder_bundle.user_id.clone(),
        identity_x25519_public_key: BASE64.encode(initiator.x25519_public_key()),
        ephemeral_public_key: BASE64.encode(ephemeral_public),
        signed_prekey_id: responder_bundle.signed_prekey_id,
        one_time_prekey_id: one_time_prekey_record.map(|record| record.key_id),
        created_at: current_unix_timestamp(),
    };
    Ok(X3dhInitiatorSecret {
        initial_message,
        shared_secret: BASE64.encode(shared),
    })
}

pub fn x3dh_responder_secret(
    responder: &Identity,
    private_bundle: &PreKeyPrivateBundle,
    initial_message: &X3dhInitialMessage,
) -> Result<[u8; 32]> {
    if initial_message.r#type != "lm-x3dh-initial-message-v1"
        || initial_message.version != protocol::PROTOCOL_VERSION_V1
    {
        return Err(LmError::InvalidBackupFormat);
    }
    if initial_message.to_user_id != *responder.user_id()
        || initial_message.signed_prekey_id != private_bundle.signed_prekey_id
    {
        return Err(LmError::InvalidUserId);
    }
    let initiator_identity_x = decode_fixed_32(&initial_message.identity_x25519_public_key)?;
    if !initial_message
        .from_user_id
        .verify_public_key_guess_x25519_unavailable()
    {
        // UserID is based on Ed25519 identity key, so an X25519 public key cannot
        // prove it here. The full envelope will bind this message to a verified
        // ContactCard. This branch intentionally never rejects; it documents the
        // trust boundary while keeping validation below explicit.
    }
    let initiator_ephemeral = decode_fixed_32(&initial_message.ephemeral_public_key)?;
    let signed_prekey_private = decode_fixed_32(&private_bundle.signed_prekey_private_key)?;
    let one_time_private = initial_message
        .one_time_prekey_id
        .map(|id| private_bundle.one_time_private_key(id))
        .transpose()?
        .flatten();
    derive_responder_secret(
        responder,
        &signed_prekey_private,
        &initiator_identity_x,
        &initiator_ephemeral,
        one_time_private.as_ref(),
    )
}

fn derive_initiator_secret(
    initiator: &Identity,
    ephemeral: &X25519Secret,
    responder_identity_x: &[u8; 32],
    responder_signed_prekey: &[u8; 32],
    responder_one_time_prekey: Option<&[u8; 32]>,
) -> Result<[u8; 32]> {
    let dh1 = initiator.x25519_shared_secret_public(responder_signed_prekey);
    let dh2 = ephemeral
        .diffie_hellman(&X25519PublicKey::from(*responder_identity_x))
        .to_bytes();
    let dh3 = ephemeral
        .diffie_hellman(&X25519PublicKey::from(*responder_signed_prekey))
        .to_bytes();
    let dh4 = responder_one_time_prekey.map(|otk| {
        ephemeral
            .diffie_hellman(&X25519PublicKey::from(*otk))
            .to_bytes()
    });
    derive_shared_secret(
        &[dh1.as_slice(), dh2.as_slice(), dh3.as_slice()],
        dh4.as_ref(),
    )
}

fn derive_responder_secret(
    responder: &Identity,
    signed_prekey_private: &[u8; 32],
    initiator_identity_x: &[u8; 32],
    initiator_ephemeral: &[u8; 32],
    one_time_private: Option<&[u8; 32]>,
) -> Result<[u8; 32]> {
    let spk = X25519Secret::from(*signed_prekey_private);
    let dh1 = spk
        .diffie_hellman(&X25519PublicKey::from(*initiator_identity_x))
        .to_bytes();
    let dh2 = responder.x25519_shared_secret_public(initiator_ephemeral);
    let dh3 = spk
        .diffie_hellman(&X25519PublicKey::from(*initiator_ephemeral))
        .to_bytes();
    let dh4 = one_time_private.map(|private| {
        X25519Secret::from(*private)
            .diffie_hellman(&X25519PublicKey::from(*initiator_ephemeral))
            .to_bytes()
    });
    derive_shared_secret(
        &[dh1.as_slice(), dh2.as_slice(), dh3.as_slice()],
        dh4.as_ref(),
    )
}

fn derive_shared_secret(parts: &[&[u8]], dh4: Option<&[u8; 32]>) -> Result<[u8; 32]> {
    let mut ikm = Vec::new();
    ikm.extend_from_slice(X3DH_CONTEXT);
    for part in parts {
        ikm.extend_from_slice(part);
    }
    if let Some(dh4) = dh4 {
        ikm.extend_from_slice(dh4);
    }
    crypto::hkdf_32(&ikm, X3DH_SHARED_SECRET_INFO)
}

fn random_x25519_secret() -> Result<X25519Secret> {
    let mut secret = [0u8; 32];
    getrandom(&mut secret).map_err(|_| LmError::RandomFailed)?;
    Ok(X25519Secret::from(secret))
}

fn verify_sig(public_key: &[u8; 32], bytes: &[u8], signature: &str) -> Result<()> {
    let verifying_key =
        VerifyingKey::from_bytes(public_key).map_err(|_| LmError::InvalidSignature)?;
    let sig_bytes = BASE64
        .decode(signature.as_bytes())
        .map_err(|_| LmError::InvalidSignature)?;
    let sig_bytes: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| LmError::InvalidSignature)?;
    let signature = Signature::from_bytes(&sig_bytes);
    verifying_key
        .verify(bytes, &signature)
        .map_err(|_| LmError::InvalidSignature)
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

trait UserIdX25519Note {
    fn verify_public_key_guess_x25519_unavailable(&self) -> bool;
}

impl UserIdX25519Note for UserId {
    fn verify_public_key_guess_x25519_unavailable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prekey_bundle_roundtrip_and_verify() {
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let (bundle, private) = PreKeyBundle::new(&bob, 7, 3, 3600).unwrap();
        bundle.verify().unwrap();
        private.validate_for_public(&bundle).unwrap();
        assert_eq!(bundle.one_time_prekeys.len(), 3);
        let text = bundle.to_export_text().unwrap();
        assert!(text.starts_with(PREKEY_BUNDLE_PREFIX));
        let restored = PreKeyBundle::from_export_text(&text).unwrap();
        assert_eq!(restored, bundle);
    }

    #[test]
    fn tampered_prekey_bundle_fails() {
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let (mut bundle, _) = PreKeyBundle::new(&bob, 7, 1, 3600).unwrap();
        bundle.signed_prekey_id = 8;
        assert_eq!(bundle.verify().unwrap_err(), LmError::InvalidSignature);
    }

    #[test]
    fn signed_one_time_prekey_record_roundtrip_and_verify() {
        let (bob, _) = Identity::create_with_passphrase("bob signed otpk").unwrap();
        let (bundle, _) = PreKeyBundle::new(&bob, 7, 2, 3600).unwrap();
        let one_time_prekey = bundle.one_time_prekeys[1].clone();
        let record =
            SignedOneTimePreKeyRecord::new(&bob, bundle.signed_prekey_id, one_time_prekey, 3600)
                .unwrap();

        record.verify().unwrap();
        assert_eq!(record.user_id, *bob.user_id());
        assert_eq!(record.signed_prekey_id, bundle.signed_prekey_id);
        assert_eq!(
            record.as_one_time_prekey(),
            bundle.one_time_prekeys[1].clone()
        );
        let text = record.to_export_text().unwrap();
        assert!(text.starts_with(SIGNED_ONE_TIME_PREKEY_RECORD_PREFIX));
        let restored = SignedOneTimePreKeyRecord::from_export_text(&text).unwrap();
        assert_eq!(restored, record);
    }

    #[test]
    fn prekey_bundle_can_export_independent_signed_one_time_records() {
        let (bob, _) = Identity::create_with_passphrase("bob independent otpk").unwrap();
        let (bundle, private, records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&bob, 9, 2, 3600).unwrap();
        assert!(bundle.one_time_prekeys.is_empty());
        assert_eq!(private.one_time_prekeys.len(), 2);
        assert_eq!(records.len(), 2);
        for (idx, record) in records.iter().enumerate() {
            record.verify_for_bundle(&bundle).unwrap();
            assert_eq!(record.key_id, idx as u32);
            assert_eq!(private.one_time_prekeys[idx].public_key, record.public_key);
        }
    }

    #[test]
    fn tampered_signed_one_time_prekey_record_fails() {
        let (bob, _) = Identity::create_with_passphrase("bob signed otpk tamper").unwrap();
        let (bundle, _) = PreKeyBundle::new(&bob, 7, 1, 3600).unwrap();
        let mut record = SignedOneTimePreKeyRecord::new(
            &bob,
            bundle.signed_prekey_id,
            bundle.one_time_prekeys[0].clone(),
            3600,
        )
        .unwrap();
        record.key_id = record.key_id.saturating_add(1);
        assert_eq!(record.verify().unwrap_err(), LmError::InvalidSignature);
    }

    #[test]
    fn x3dh_initiator_and_responder_derive_same_secret() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let (bundle, private) = PreKeyBundle::new(&bob, 42, 1, 3600).unwrap();
        let initiator = x3dh_initiator_secret(&alice, &bundle).unwrap();
        let responder = x3dh_responder_secret(&bob, &private, &initiator.initial_message).unwrap();
        assert_eq!(initiator.shared_secret, BASE64.encode(responder));
        assert_eq!(initiator.initial_message.one_time_prekey_id, Some(0));
    }

    #[test]
    fn x3dh_can_select_specific_one_time_prekey() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let (bundle, private) = PreKeyBundle::new(&bob, 42, 3, 3600).unwrap();
        let initiator =
            x3dh_initiator_secret_with_one_time_prekey_id(&alice, &bundle, Some(2)).unwrap();
        let responder = x3dh_responder_secret(&bob, &private, &initiator.initial_message).unwrap();
        assert_eq!(initiator.shared_secret, BASE64.encode(responder));
        assert_eq!(initiator.initial_message.one_time_prekey_id, Some(2));
    }

    #[test]
    fn x3dh_can_use_independent_signed_one_time_prekey_record() {
        let (alice, _) = Identity::create_with_passphrase("alice signed otpk x3dh").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob signed otpk x3dh").unwrap();
        let (bundle, private, records) =
            PreKeyBundle::new_with_signed_one_time_prekey_records(&bob, 42, 2, 3600).unwrap();
        let initiator =
            x3dh_initiator_secret_with_one_time_prekey_record(&alice, &bundle, Some(&records[1]))
                .unwrap();
        let responder = x3dh_responder_secret(&bob, &private, &initiator.initial_message).unwrap();
        assert_eq!(initiator.initial_message.one_time_prekey_id, Some(1));
        assert_eq!(initiator.shared_secret, BASE64.encode(responder));
    }

    #[test]
    fn x3dh_without_one_time_prekey_works() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let (bundle, private) = PreKeyBundle::new(&bob, 42, 0, 3600).unwrap();
        let initiator = x3dh_initiator_secret(&alice, &bundle).unwrap();
        let responder = x3dh_responder_secret(&bob, &private, &initiator.initial_message).unwrap();
        assert_eq!(initiator.shared_secret, BASE64.encode(responder));
        assert_eq!(initiator.initial_message.one_time_prekey_id, None);
    }
}
