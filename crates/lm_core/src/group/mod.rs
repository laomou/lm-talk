//! MVP group invitation protocol.

use crate::{ContactCard, Identity, LmError, Result, UserId, crypto, limits, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const GROUP_INVITE_TYPE: &str = "lm-group-invite-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInvite {
    pub r#type: String,
    pub version: u16,
    pub invite_id: Uuid,
    pub group_id: Uuid,
    pub group_name: String,
    pub inviter_user_id: UserId,
    pub member_user_ids: Vec<UserId>,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GroupInviteSignedFields {
    r#type: String,
    version: u16,
    invite_id: Uuid,
    group_id: Uuid,
    group_name: String,
    inviter_user_id: UserId,
    member_user_ids: Vec<UserId>,
    created_at: u64,
    expires_at: u64,
}

impl GroupInvite {
    pub fn new(
        inviter: &Identity,
        group_id: Uuid,
        group_name: String,
        mut member_user_ids: Vec<UserId>,
        ttl_seconds: u64,
    ) -> Result<Self> {
        limits::ensure_len(&group_name, limits::MAX_GROUP_NAME_BYTES)?;
        limits::ensure_vec_len(&member_user_ids, limits::MAX_GROUP_MEMBERS)?;
        member_user_ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        member_user_ids.dedup();
        let created_at = current_unix_timestamp();
        let signed = GroupInviteSignedFields {
            r#type: GROUP_INVITE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            invite_id: Uuid::new_v4(),
            group_id,
            group_name,
            inviter_user_id: inviter.user_id().clone(),
            member_user_ids,
            created_at,
            expires_at: created_at.saturating_add(ttl_seconds),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = inviter.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            invite_id: signed.invite_id,
            group_id: signed.group_id,
            group_name: signed.group_name,
            inviter_user_id: signed.inviter_user_id,
            member_user_ids: signed.member_user_ids,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, inviter_contact_card: &ContactCard) -> Result<()> {
        if self.r#type != GROUP_INVITE_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.expires_at <= current_unix_timestamp() {
            return Err(LmError::ExpiredObject);
        }
        inviter_contact_card.verify()?;
        if inviter_contact_card.user_id != self.inviter_user_id {
            return Err(LmError::InvalidUserId);
        }
        let public_key = decode_key_32(&inviter_contact_card.identity_public_key)?;
        let signed = GroupInviteSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            invite_id: self.invite_id,
            group_id: self.group_id,
            group_name: self.group_name.clone(),
            inviter_user_id: self.inviter_user_id.clone(),
            member_user_ids: self.member_user_ids.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let verifying_key =
            VerifyingKey::from_bytes(&public_key).map_err(|_| LmError::InvalidSignature)?;
        let sig = decode_signature(&self.signature)?;
        let signature = Signature::from_bytes(&sig);
        verifying_key
            .verify(&bytes, &signature)
            .map_err(|_| LmError::InvalidSignature)
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-group-invite-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_GROUP_INVITE_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-group-invite-v1:", text)
    }
}

pub const GROUP_EVENT_TYPE: &str = "lm-group-event-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupEvent {
    pub r#type: String,
    pub version: u16,
    pub event_id: Uuid,
    pub group_id: Uuid,
    pub actor_user_id: UserId,
    pub sequence: u64,
    pub action: GroupEventAction,
    pub created_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupEventAction {
    Rename { name: String },
    AddMember { user_id: UserId },
    RemoveMember { user_id: UserId },
    PromoteAdmin { user_id: UserId },
    DemoteAdmin { user_id: UserId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GroupEventSignedFields {
    r#type: String,
    version: u16,
    event_id: Uuid,
    group_id: Uuid,
    actor_user_id: UserId,
    sequence: u64,
    action: GroupEventAction,
    created_at: u64,
}

impl GroupEvent {
    pub fn new(
        actor: &Identity,
        group_id: Uuid,
        sequence: u64,
        action: GroupEventAction,
    ) -> Result<Self> {
        match &action {
            GroupEventAction::Rename { name } => {
                limits::ensure_len(name, limits::MAX_GROUP_NAME_BYTES)?
            }
            GroupEventAction::AddMember { .. }
            | GroupEventAction::RemoveMember { .. }
            | GroupEventAction::PromoteAdmin { .. }
            | GroupEventAction::DemoteAdmin { .. } => {}
        }
        let signed = GroupEventSignedFields {
            r#type: GROUP_EVENT_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            event_id: Uuid::new_v4(),
            group_id,
            actor_user_id: actor.user_id().clone(),
            sequence,
            action,
            created_at: current_unix_timestamp(),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = actor.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            event_id: signed.event_id,
            group_id: signed.group_id,
            actor_user_id: signed.actor_user_id,
            sequence: signed.sequence,
            action: signed.action,
            created_at: signed.created_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, actor_contact_card: &ContactCard) -> Result<()> {
        if self.r#type != GROUP_EVENT_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        actor_contact_card.verify()?;
        if actor_contact_card.user_id != self.actor_user_id {
            return Err(LmError::InvalidUserId);
        }
        let public_key = decode_key_32(&actor_contact_card.identity_public_key)?;
        let signed = GroupEventSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            event_id: self.event_id,
            group_id: self.group_id,
            actor_user_id: self.actor_user_id.clone(),
            sequence: self.sequence,
            action: self.action.clone(),
            created_at: self.created_at,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let verifying_key =
            VerifyingKey::from_bytes(&public_key).map_err(|_| LmError::InvalidSignature)?;
        let sig = decode_signature(&self.signature)?;
        let signature = Signature::from_bytes(&sig);
        verifying_key
            .verify(&bytes, &signature)
            .map_err(|_| LmError::InvalidSignature)
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-group-event-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_GROUP_INVITE_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-group-event-v1:", text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupPolicyState {
    pub group_id: Uuid,
    pub name: String,
    pub members: Vec<UserId>,
    pub admins: Vec<UserId>,
    pub sequence: u64,
}

impl GroupPolicyState {
    pub fn new(
        group_id: Uuid,
        name: String,
        creator: UserId,
        mut members: Vec<UserId>,
    ) -> Result<Self> {
        limits::ensure_len(&name, limits::MAX_GROUP_NAME_BYTES)?;
        if !members.contains(&creator) {
            members.push(creator.clone());
        }
        members.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        members.dedup();
        limits::ensure_vec_len(&members, limits::MAX_GROUP_MEMBERS)?;
        Ok(Self {
            group_id,
            name,
            members,
            admins: vec![creator],
            sequence: 0,
        })
    }

    pub fn is_member(&self, user_id: &UserId) -> bool {
        self.members.contains(user_id)
    }

    pub fn is_admin(&self, user_id: &UserId) -> bool {
        self.admins.contains(user_id)
    }

    pub fn event_requires_sender_key_rotation(event: &GroupEvent) -> bool {
        matches!(
            event.action,
            GroupEventAction::AddMember { .. } | GroupEventAction::RemoveMember { .. }
        )
    }

    pub fn apply_event(&mut self, event: &GroupEvent) -> Result<()> {
        if event.group_id != self.group_id {
            return Err(LmError::InvalidBackupFormat);
        }
        if event.sequence != self.sequence.saturating_add(1) {
            return Err(LmError::ReplayDetected);
        }
        if !self.is_member(&event.actor_user_id) {
            return Err(LmError::InvalidUserId);
        }
        match &event.action {
            GroupEventAction::Rename { name } => {
                self.require_admin(&event.actor_user_id)?;
                limits::ensure_len(name, limits::MAX_GROUP_NAME_BYTES)?;
                self.name = name.clone();
            }
            GroupEventAction::AddMember { user_id } => {
                self.require_admin(&event.actor_user_id)?;
                if !self.members.contains(user_id) {
                    self.members.push(user_id.clone());
                    self.members.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                    limits::ensure_vec_len(&self.members, limits::MAX_GROUP_MEMBERS)?;
                }
            }
            GroupEventAction::RemoveMember { user_id } => {
                if &event.actor_user_id != user_id {
                    return Err(LmError::InvalidBackupFormat);
                }
                if self.admins.contains(user_id) && self.admins.len() == 1 && self.members.len() > 1
                {
                    return Err(LmError::InvalidBackupFormat);
                }
                self.members.retain(|id| id != user_id);
                self.admins.retain(|id| id != user_id);
            }
            GroupEventAction::PromoteAdmin { user_id } => {
                self.require_admin(&event.actor_user_id)?;
                if !self.members.contains(user_id) {
                    return Err(LmError::InvalidUserId);
                }
                if !self.admins.contains(user_id) {
                    self.admins.push(user_id.clone());
                    self.admins.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                }
            }
            GroupEventAction::DemoteAdmin { user_id } => {
                self.require_admin(&event.actor_user_id)?;
                if &event.actor_user_id == user_id {
                    return Err(LmError::InvalidBackupFormat);
                }
                if self.admins.len() <= 1 && self.admins.contains(user_id) {
                    return Err(LmError::InvalidBackupFormat);
                }
                self.admins.retain(|id| id != user_id);
            }
        }
        self.sequence = event.sequence;
        Ok(())
    }

    fn require_admin(&self, user_id: &UserId) -> Result<()> {
        if self.is_admin(user_id) {
            Ok(())
        } else {
            Err(LmError::InvalidSignature)
        }
    }
}

pub const GROUP_SENDER_KEY_DISTRIBUTION_TYPE: &str = "lm-group-sender-key-v1";
pub const GROUP_SENDER_KEY_ENVELOPE_TYPE: &str = "lm-group-sender-envelope-v1";
pub const GROUP_SENDER_KEY_CRYPTO_V1: &str = "sender-key-hkdf-xchacha20poly1305-v1";
const GROUP_SENDER_CHAIN_INFO: &[u8] = b"lm-talk.group.sender-chain.v1";
const GROUP_SENDER_MESSAGE_INFO: &[u8] = b"lm-talk.group.sender-message.v1";
const GROUP_SENDER_NEXT_INFO: &[u8] = b"lm-talk.group.sender-next.v1";
const GROUP_SENDER_NONCE_LEN: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupSenderKeyState {
    pub r#type: String,
    pub version: u16,
    pub group_id: Uuid,
    pub sender_user_id: UserId,
    pub chain_key: String,
    pub counter: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupSenderKeyDistribution {
    pub r#type: String,
    pub version: u16,
    pub group_id: Uuid,
    pub sender_user_id: UserId,
    pub chain_key: String,
    pub counter: u32,
    pub created_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GroupSenderKeyDistributionSignedFields {
    r#type: String,
    version: u16,
    group_id: Uuid,
    sender_user_id: UserId,
    chain_key: String,
    counter: u32,
    created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupSenderEnvelope {
    pub r#type: String,
    pub version: u16,
    pub crypto: String,
    pub message_id: Uuid,
    pub group_id: Uuid,
    pub sender_user_id: UserId,
    pub counter: u32,
    pub created_at: u64,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GroupSenderEnvelopeAad {
    r#type: String,
    version: u16,
    crypto: String,
    message_id: Uuid,
    group_id: Uuid,
    sender_user_id: UserId,
    counter: u32,
    created_at: u64,
    nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupPlainMessage {
    pub r#type: String,
    pub version: u16,
    pub message_id: Uuid,
    pub group_id: Uuid,
    pub sender_user_id: UserId,
    pub text: String,
    pub created_at: u64,
}

impl GroupSenderKeyState {
    pub fn new(sender: &Identity, group_id: Uuid) -> Result<Self> {
        let mut seed = [0u8; 32];
        getrandom(&mut seed).map_err(|_| LmError::RandomFailed)?;
        let chain_key = crypto::hkdf_32(&seed, GROUP_SENDER_CHAIN_INFO)?;
        let now = current_unix_timestamp();
        Ok(Self {
            r#type: GROUP_SENDER_KEY_DISTRIBUTION_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            group_id,
            sender_user_id: sender.user_id().clone(),
            chain_key: BASE64.encode(chain_key),
            counter: 0,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn from_distribution(
        distribution: &GroupSenderKeyDistribution,
        sender_card: &ContactCard,
    ) -> Result<Self> {
        distribution.verify(sender_card)?;
        Ok(Self {
            r#type: GROUP_SENDER_KEY_DISTRIBUTION_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            group_id: distribution.group_id,
            sender_user_id: distribution.sender_user_id.clone(),
            chain_key: distribution.chain_key.clone(),
            counter: distribution.counter,
            created_at: distribution.created_at,
            updated_at: current_unix_timestamp(),
        })
    }

    pub fn to_distribution(&self, sender: &Identity) -> Result<GroupSenderKeyDistribution> {
        if self.sender_user_id != *sender.user_id() {
            return Err(LmError::InvalidUserId);
        }
        self.validate()?;
        let signed = GroupSenderKeyDistributionSignedFields {
            r#type: GROUP_SENDER_KEY_DISTRIBUTION_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            group_id: self.group_id,
            sender_user_id: self.sender_user_id.clone(),
            chain_key: self.chain_key.clone(),
            counter: self.counter,
            created_at: current_unix_timestamp(),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = sender.signing_key().sign(&bytes);
        Ok(GroupSenderKeyDistribution {
            r#type: signed.r#type,
            version: signed.version,
            group_id: signed.group_id,
            sender_user_id: signed.sender_user_id,
            chain_key: signed.chain_key,
            counter: signed.counter,
            created_at: signed.created_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.r#type != GROUP_SENDER_KEY_DISTRIBUTION_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        decode_key_32_crypto(&self.chain_key)?;
        Ok(())
    }

    pub fn encrypt_text(&mut self, text: String) -> Result<GroupSenderEnvelope> {
        limits::ensure_len(&text, limits::MAX_GROUP_MESSAGE_TEXT_BYTES)?;
        self.validate()?;
        let chain = decode_key_32_crypto(&self.chain_key)?;
        let key = derive_indexed_key(&chain, GROUP_SENDER_MESSAGE_INFO, self.counter)?;
        let next = derive_indexed_key(&chain, GROUP_SENDER_NEXT_INFO, self.counter)?;
        let mut nonce = [0u8; GROUP_SENDER_NONCE_LEN];
        getrandom(&mut nonce).map_err(|_| LmError::RandomFailed)?;
        let plain = GroupPlainMessage {
            r#type: "lm-group-plain-message-v1".to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            message_id: Uuid::new_v4(),
            group_id: self.group_id,
            sender_user_id: self.sender_user_id.clone(),
            text,
            created_at: current_unix_timestamp(),
        };
        let aad_header = GroupSenderEnvelopeAad {
            r#type: GROUP_SENDER_KEY_ENVELOPE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            crypto: GROUP_SENDER_KEY_CRYPTO_V1.to_string(),
            message_id: plain.message_id,
            group_id: self.group_id,
            sender_user_id: self.sender_user_id.clone(),
            counter: self.counter,
            created_at: current_unix_timestamp(),
            nonce: BASE64.encode(nonce),
        };
        let aad = protocol::to_canonical_bytes(&aad_header)?;
        let ciphertext = crypto::xchacha20poly1305_encrypt(
            &key,
            &nonce,
            &protocol::to_canonical_bytes(&plain)?,
            &aad,
        )?;
        self.counter = self.counter.saturating_add(1);
        self.chain_key = BASE64.encode(next);
        self.updated_at = current_unix_timestamp();
        Ok(GroupSenderEnvelope {
            r#type: aad_header.r#type,
            version: aad_header.version,
            crypto: aad_header.crypto,
            message_id: aad_header.message_id,
            group_id: aad_header.group_id,
            sender_user_id: aad_header.sender_user_id,
            counter: aad_header.counter,
            created_at: aad_header.created_at,
            nonce: aad_header.nonce,
            ciphertext: BASE64.encode(ciphertext),
        })
    }

    pub fn decrypt(&mut self, envelope: &GroupSenderEnvelope) -> Result<GroupPlainMessage> {
        envelope.validate_header()?;
        self.validate()?;
        if envelope.group_id != self.group_id || envelope.sender_user_id != self.sender_user_id {
            return Err(LmError::InvalidUserId);
        }
        if envelope.counter < self.counter {
            return Err(LmError::ReplayDetected);
        }
        if envelope.counter.saturating_sub(self.counter) > 512 {
            return Err(LmError::PayloadTooLarge);
        }
        let mut chain = decode_key_32_crypto(&self.chain_key)?;
        while self.counter < envelope.counter {
            chain = derive_indexed_key(&chain, GROUP_SENDER_NEXT_INFO, self.counter)?;
            self.counter = self.counter.saturating_add(1);
        }
        let key = derive_indexed_key(&chain, GROUP_SENDER_MESSAGE_INFO, self.counter)?;
        let next = derive_indexed_key(&chain, GROUP_SENDER_NEXT_INFO, self.counter)?;
        let nonce = decode_nonce_24(&envelope.nonce)?;
        let ciphertext = BASE64
            .decode(envelope.ciphertext.as_bytes())
            .map_err(|_| LmError::CryptoError)?;
        let aad = protocol::to_canonical_bytes(&envelope.aad_header())?;
        let plaintext = crypto::xchacha20poly1305_decrypt(&key, &nonce, &ciphertext, &aad)
            .map_err(|_| LmError::CryptoError)?;
        let plain: GroupPlainMessage = protocol::from_canonical_bytes(&plaintext)?;
        if plain.message_id != envelope.message_id
            || plain.group_id != envelope.group_id
            || plain.sender_user_id != envelope.sender_user_id
        {
            return Err(LmError::CryptoError);
        }
        self.counter = self.counter.saturating_add(1);
        self.chain_key = BASE64.encode(next);
        self.updated_at = current_unix_timestamp();
        Ok(plain)
    }
}

impl GroupSenderKeyDistribution {
    pub fn verify(&self, sender_card: &ContactCard) -> Result<()> {
        if self.r#type != GROUP_SENDER_KEY_DISTRIBUTION_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        sender_card.verify()?;
        if sender_card.user_id != self.sender_user_id {
            return Err(LmError::InvalidUserId);
        }
        decode_key_32_crypto(&self.chain_key)?;
        let public_key = decode_key_32(&sender_card.identity_public_key)?;
        let signed = GroupSenderKeyDistributionSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            group_id: self.group_id,
            sender_user_id: self.sender_user_id.clone(),
            chain_key: self.chain_key.clone(),
            counter: self.counter,
            created_at: self.created_at,
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let verifying_key =
            VerifyingKey::from_bytes(&public_key).map_err(|_| LmError::InvalidSignature)?;
        let sig = decode_signature(&self.signature)?;
        let signature = Signature::from_bytes(&sig);
        verifying_key
            .verify(&bytes, &signature)
            .map_err(|_| LmError::InvalidSignature)
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-group-sender-key-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_GROUP_INVITE_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-group-sender-key-v1:", text)
    }
}

impl GroupSenderEnvelope {
    fn validate_header(&self) -> Result<()> {
        if self.r#type != GROUP_SENDER_KEY_ENVELOPE_TYPE {
            return Err(LmError::InvalidBackupFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.crypto != GROUP_SENDER_KEY_CRYPTO_V1 {
            return Err(LmError::InvalidBackupFormat);
        }
        Ok(())
    }

    fn aad_header(&self) -> GroupSenderEnvelopeAad {
        GroupSenderEnvelopeAad {
            r#type: self.r#type.clone(),
            version: self.version,
            crypto: self.crypto.clone(),
            message_id: self.message_id,
            group_id: self.group_id,
            sender_user_id: self.sender_user_id.clone(),
            counter: self.counter,
            created_at: self.created_at,
            nonce: self.nonce.clone(),
        }
    }
}

fn derive_indexed_key(chain_key: &[u8; 32], info: &[u8], index: u32) -> Result<[u8; 32]> {
    let mut context = Vec::from(info);
    context.extend_from_slice(&index.to_be_bytes());
    crypto::hkdf_32(chain_key, &context)
}

fn decode_key_32_crypto(value: &str) -> Result<[u8; 32]> {
    let bytes = BASE64
        .decode(value.as_bytes())
        .map_err(|_| LmError::CryptoError)?;
    bytes.try_into().map_err(|_| LmError::CryptoError)
}

fn decode_nonce_24(value: &str) -> Result<[u8; 24]> {
    let bytes = BASE64
        .decode(value.as_bytes())
        .map_err(|_| LmError::CryptoError)?;
    bytes.try_into().map_err(|_| LmError::CryptoError)
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
    fn group_invite_roundtrip() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let alice_card = alice
            .export_contact_card(Some("Alice".into()), None, vec![])
            .unwrap();
        let invite = GroupInvite::new(
            &alice,
            Uuid::new_v4(),
            "Test".into(),
            vec![alice.user_id().clone(), bob.user_id().clone()],
            3600,
        )
        .unwrap();
        invite.verify(&alice_card).unwrap();
        let text = invite.to_export_text().unwrap();
        let decoded = GroupInvite::from_export_text(&text).unwrap();
        decoded.verify(&alice_card).unwrap();
    }

    #[test]
    fn group_event_roundtrip_and_tamper() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let alice_card = alice
            .export_contact_card(Some("Alice".into()), None, vec![])
            .unwrap();
        let event = GroupEvent::new(
            &alice,
            Uuid::new_v4(),
            1,
            GroupEventAction::Rename { name: "New".into() },
        )
        .unwrap();
        event.verify(&alice_card).unwrap();
        let text = event.to_export_text().unwrap();
        let mut decoded = GroupEvent::from_export_text(&text).unwrap();
        decoded.verify(&alice_card).unwrap();
        decoded.sequence += 1;
        assert_eq!(
            decoded.verify(&alice_card).unwrap_err(),
            LmError::InvalidSignature
        );
    }

    #[test]
    fn tampered_group_invite_fails() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let alice_card = alice.export_contact_card(None, None, vec![]).unwrap();
        let mut invite =
            GroupInvite::new(&alice, Uuid::new_v4(), "Test".into(), vec![], 3600).unwrap();
        invite.group_name = "Evil".into();
        assert_eq!(
            invite.verify(&alice_card).unwrap_err(),
            LmError::InvalidSignature
        );
    }

    #[test]
    fn group_policy_requires_admin_for_admin_actions() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let (carol, _) = Identity::create_with_passphrase("carol").unwrap();
        let group_id = Uuid::new_v4();
        let mut state = GroupPolicyState::new(
            group_id,
            "Test".into(),
            alice.user_id().clone(),
            vec![alice.user_id().clone(), bob.user_id().clone()],
        )
        .unwrap();
        let bad = GroupEvent::new(
            &bob,
            group_id,
            1,
            GroupEventAction::AddMember {
                user_id: carol.user_id().clone(),
            },
        )
        .unwrap();
        assert_eq!(
            state.apply_event(&bad).unwrap_err(),
            LmError::InvalidSignature
        );
        let good = GroupEvent::new(
            &alice,
            group_id,
            1,
            GroupEventAction::AddMember {
                user_id: carol.user_id().clone(),
            },
        )
        .unwrap();
        state.apply_event(&good).unwrap();
        assert!(state.is_member(carol.user_id()));
    }

    #[test]
    fn group_policy_admin_promotion_and_demotion() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let group_id = Uuid::new_v4();
        let mut state = GroupPolicyState::new(
            group_id,
            "Test".into(),
            alice.user_id().clone(),
            vec![alice.user_id().clone(), bob.user_id().clone()],
        )
        .unwrap();
        let promote = GroupEvent::new(
            &alice,
            group_id,
            1,
            GroupEventAction::PromoteAdmin {
                user_id: bob.user_id().clone(),
            },
        )
        .unwrap();
        state.apply_event(&promote).unwrap();
        assert!(state.is_admin(bob.user_id()));
        let demote = GroupEvent::new(
            &bob,
            group_id,
            2,
            GroupEventAction::DemoteAdmin {
                user_id: alice.user_id().clone(),
            },
        )
        .unwrap();
        state.apply_event(&demote).unwrap();
        assert!(!state.is_admin(alice.user_id()));
        assert!(state.is_admin(bob.user_id()));
    }

    #[test]
    fn group_policy_rejects_admin_removing_other_members() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let group_id = Uuid::new_v4();
        let mut state = GroupPolicyState::new(
            group_id,
            "Test".into(),
            alice.user_id().clone(),
            vec![alice.user_id().clone(), bob.user_id().clone()],
        )
        .unwrap();
        let remove_bob = GroupEvent::new(
            &alice,
            group_id,
            1,
            GroupEventAction::RemoveMember {
                user_id: bob.user_id().clone(),
            },
        )
        .unwrap();

        assert_eq!(
            state.apply_event(&remove_bob).unwrap_err(),
            LmError::InvalidBackupFormat
        );
        assert!(state.is_member(bob.user_id()));
    }

    #[test]
    fn group_policy_marks_membership_events_for_sender_key_rotation() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _) = Identity::create_with_passphrase("bob").unwrap();
        let group_id = Uuid::new_v4();
        let add = GroupEvent::new(
            &alice,
            group_id,
            1,
            GroupEventAction::AddMember {
                user_id: bob.user_id().clone(),
            },
        )
        .unwrap();
        assert!(GroupPolicyState::event_requires_sender_key_rotation(&add));
        let rename = GroupEvent::new(
            &alice,
            group_id,
            1,
            GroupEventAction::Rename {
                name: "Name".into(),
            },
        )
        .unwrap();
        assert!(!GroupPolicyState::event_requires_sender_key_rotation(
            &rename
        ));
    }

    #[test]
    fn group_sender_key_encrypt_decrypt_roundtrip() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let alice_card = alice
            .export_contact_card(Some("Alice".into()), None, vec![])
            .unwrap();
        let group_id = Uuid::new_v4();
        let mut sender_state = GroupSenderKeyState::new(&alice, group_id).unwrap();
        let distribution = sender_state.to_distribution(&alice).unwrap();
        distribution.verify(&alice_card).unwrap();
        let text = distribution.to_export_text().unwrap();
        let distribution = GroupSenderKeyDistribution::from_export_text(&text).unwrap();
        let mut receiver_state =
            GroupSenderKeyState::from_distribution(&distribution, &alice_card).unwrap();
        let envelope = sender_state.encrypt_text("hello group".into()).unwrap();
        let plain = receiver_state.decrypt(&envelope).unwrap();
        assert_eq!(plain.text, "hello group");
        assert_eq!(plain.group_id, group_id);
        assert_eq!(plain.sender_user_id, *alice.user_id());
    }

    #[test]
    fn group_sender_key_tamper_fails() {
        let (alice, _) = Identity::create_with_passphrase("alice").unwrap();
        let alice_card = alice.export_contact_card(None, None, vec![]).unwrap();
        let mut sender_state = GroupSenderKeyState::new(&alice, Uuid::new_v4()).unwrap();
        let distribution = sender_state.to_distribution(&alice).unwrap();
        let mut receiver_state =
            GroupSenderKeyState::from_distribution(&distribution, &alice_card).unwrap();
        let mut envelope = sender_state.encrypt_text("hello".into()).unwrap();
        envelope.created_at = envelope.created_at.saturating_add(1);
        assert_eq!(
            receiver_state.decrypt(&envelope).unwrap_err(),
            LmError::CryptoError
        );
    }
}
