//! Network-adjacent protocol objects.
//!
//! This module only defines signed signaling payloads. It does not implement
//! sockets, WebRTC, DHT, relay, or mailbox networking.

use crate::{DeviceId, Identity, LmError, Result, UserId, limits, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SIGNAL_OFFER_TYPE: &str = "lm-signal-offer-v1";
pub const SIGNAL_ANSWER_TYPE: &str = "lm-signal-answer-v1";
pub const MESSAGE_RECEIPT_TYPE: &str = "lm-message-receipt-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalKind {
    WebRtcOffer,
    WebRtcAnswer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalOffer {
    pub r#type: String,
    pub version: u16,
    pub signal_id: Uuid,
    pub from_user_id: UserId,
    pub from_device_id: Option<DeviceId>,
    pub to_user_id: Option<UserId>,
    pub kind: SignalKind,
    pub sdp: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SignalOfferSignedFields {
    r#type: String,
    version: u16,
    signal_id: Uuid,
    from_user_id: UserId,
    from_device_id: Option<DeviceId>,
    to_user_id: Option<UserId>,
    kind: SignalKind,
    sdp: String,
    created_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalAnswer {
    pub r#type: String,
    pub version: u16,
    pub signal_id: Uuid,
    pub request_signal_id: Uuid,
    pub from_user_id: UserId,
    pub from_device_id: Option<DeviceId>,
    pub to_user_id: UserId,
    pub kind: SignalKind,
    pub sdp: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SignalAnswerSignedFields {
    r#type: String,
    version: u16,
    signal_id: Uuid,
    request_signal_id: Uuid,
    from_user_id: UserId,
    from_device_id: Option<DeviceId>,
    to_user_id: UserId,
    kind: SignalKind,
    sdp: String,
    created_at: u64,
    expires_at: u64,
}

impl SignalOffer {
    pub fn new(
        from: &Identity,
        from_device_id: Option<DeviceId>,
        to_user_id: Option<UserId>,
        sdp: String,
        ttl_seconds: u64,
    ) -> Result<Self> {
        limits::ensure_len(&sdp, limits::MAX_SIGNAL_TEXT_BYTES)?;
        let created_at = current_unix_timestamp();
        let signed = SignalOfferSignedFields {
            r#type: SIGNAL_OFFER_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            signal_id: Uuid::new_v4(),
            from_user_id: from.user_id().clone(),
            from_device_id,
            to_user_id,
            kind: SignalKind::WebRtcOffer,
            sdp,
            created_at,
            expires_at: created_at.saturating_add(ttl_seconds),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = from.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            signal_id: signed.signal_id,
            from_user_id: signed.from_user_id,
            from_device_id: signed.from_device_id,
            to_user_id: signed.to_user_id,
            kind: signed.kind,
            sdp: signed.sdp,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, from_identity_public_key: &[u8; 32]) -> Result<()> {
        if self.r#type != SIGNAL_OFFER_TYPE {
            return Err(LmError::InvalidFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.kind != SignalKind::WebRtcOffer {
            return Err(LmError::InvalidFormat);
        }
        if self.expires_at <= current_unix_timestamp() {
            return Err(LmError::ExpiredObject);
        }
        if !self
            .from_user_id
            .verify_public_key(from_identity_public_key)
        {
            return Err(LmError::InvalidUserId);
        }
        let signed = SignalOfferSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            signal_id: self.signal_id,
            from_user_id: self.from_user_id.clone(),
            from_device_id: self.from_device_id.clone(),
            to_user_id: self.to_user_id.clone(),
            kind: self.kind,
            sdp: self.sdp.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        verify_sig(
            from_identity_public_key,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-signal-offer-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_SIGNAL_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-signal-offer-v1:", text)
    }
}

impl SignalAnswer {
    pub fn new(
        from: &Identity,
        from_device_id: Option<DeviceId>,
        offer: &SignalOffer,
        sdp: String,
        ttl_seconds: u64,
    ) -> Result<Self> {
        limits::ensure_len(&sdp, limits::MAX_SIGNAL_TEXT_BYTES)?;
        let created_at = current_unix_timestamp();
        let signed = SignalAnswerSignedFields {
            r#type: SIGNAL_ANSWER_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            signal_id: Uuid::new_v4(),
            request_signal_id: offer.signal_id,
            from_user_id: from.user_id().clone(),
            from_device_id,
            to_user_id: offer.from_user_id.clone(),
            kind: SignalKind::WebRtcAnswer,
            sdp,
            created_at,
            expires_at: created_at.saturating_add(ttl_seconds),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = from.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            signal_id: signed.signal_id,
            request_signal_id: signed.request_signal_id,
            from_user_id: signed.from_user_id,
            from_device_id: signed.from_device_id,
            to_user_id: signed.to_user_id,
            kind: signed.kind,
            sdp: signed.sdp,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, from_identity_public_key: &[u8; 32]) -> Result<()> {
        if self.r#type != SIGNAL_ANSWER_TYPE {
            return Err(LmError::InvalidFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        if self.kind != SignalKind::WebRtcAnswer {
            return Err(LmError::InvalidFormat);
        }
        if self.expires_at <= current_unix_timestamp() {
            return Err(LmError::ExpiredObject);
        }
        if !self
            .from_user_id
            .verify_public_key(from_identity_public_key)
        {
            return Err(LmError::InvalidUserId);
        }
        let signed = SignalAnswerSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            signal_id: self.signal_id,
            request_signal_id: self.request_signal_id,
            from_user_id: self.from_user_id.clone(),
            from_device_id: self.from_device_id.clone(),
            to_user_id: self.to_user_id.clone(),
            kind: self.kind,
            sdp: self.sdp.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        verify_sig(
            from_identity_public_key,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-signal-answer-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_SIGNAL_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-signal-answer-v1:", text)
    }
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

fn current_unix_timestamp() -> u64 {
    crate::unix_now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_offer_answer_roundtrip() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let offer = SignalOffer::new(
            &alice,
            None,
            Some(bob.user_id().clone()),
            "offer-sdp".into(),
            3600,
        )
        .unwrap();
        offer.verify(&alice.identity_public_key()).unwrap();
        let text = offer.to_export_text().unwrap();
        let offer = SignalOffer::from_export_text(&text).unwrap();
        let answer = SignalAnswer::new(&bob, None, &offer, "answer-sdp".into(), 3600).unwrap();
        answer.verify(&bob.identity_public_key()).unwrap();
        assert_eq!(answer.to_user_id, *alice.user_id());
    }

    #[test]
    fn tampered_signal_fails() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let mut offer = SignalOffer::new(&alice, None, None, "offer-sdp".into(), 3600).unwrap();
        offer.sdp = "changed".into();
        assert_eq!(
            offer.verify(&alice.identity_public_key()).unwrap_err(),
            LmError::InvalidSignature
        );
    }
}

pub const PEER_ANNOUNCE_TYPE: &str = "lm-peer-announce-v1";
pub const PUBLIC_PEER_ANNOUNCE_TYPE: &str = "lm-public-peer-announce-v1";
pub const MAILBOX_MESSAGE_TYPE: &str = "lm-mailbox-message-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerAnnounce {
    pub r#type: String,
    pub version: u16,
    pub user_id: UserId,
    pub device_id: Option<DeviceId>,
    pub addresses: Vec<String>,
    pub mailbox_key: Option<String>,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PeerAnnounceSignedFields {
    r#type: String,
    version: u16,
    user_id: UserId,
    device_id: Option<DeviceId>,
    addresses: Vec<String>,
    mailbox_key: Option<String>,
    created_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicPeerCapability {
    Bootstrap,
    Dht,
    Signaling,
    Relay,
    Mailbox,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicPeerAnnounce {
    pub r#type: String,
    pub version: u16,
    pub peer_id: String,
    pub user_id: UserId,
    pub device_id: Option<DeviceId>,
    pub addresses: Vec<String>,
    pub capabilities: Vec<PublicPeerCapability>,
    pub max_mailbox_bytes: Option<u64>,
    pub max_message_ttl_seconds: Option<u64>,
    pub max_relay_bandwidth_kbps: Option<u64>,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PublicPeerAnnounceSignedFields {
    r#type: String,
    version: u16,
    peer_id: String,
    user_id: UserId,
    device_id: Option<DeviceId>,
    addresses: Vec<String>,
    capabilities: Vec<PublicPeerCapability>,
    max_mailbox_bytes: Option<u64>,
    max_message_ttl_seconds: Option<u64>,
    max_relay_bandwidth_kbps: Option<u64>,
    created_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MailboxMessageKind {
    SignalOffer,
    SignalAnswer,
    DirectEnvelope,
    GroupFanout,
    DeliveryReceipt,
    ReadReceipt,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageReceiptKind {
    Delivered,
    Read,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageReceipt {
    pub r#type: String,
    pub version: u16,
    pub receipt_id: Uuid,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub target_message_id: Uuid,
    pub conversation_id: String,
    pub mailbox_delivery_id: Option<String>,
    pub kind: MessageReceiptKind,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MessageReceiptSignedFields {
    r#type: String,
    version: u16,
    receipt_id: Uuid,
    from_user_id: UserId,
    to_user_id: UserId,
    target_message_id: Uuid,
    conversation_id: String,
    mailbox_delivery_id: Option<String>,
    kind: MessageReceiptKind,
    created_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxMessage {
    pub r#type: String,
    pub version: u16,
    pub message_id: Uuid,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub kind: MailboxMessageKind,
    pub ciphertext: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MailboxMessageSignedFields {
    r#type: String,
    version: u16,
    message_id: Uuid,
    from_user_id: UserId,
    to_user_id: UserId,
    kind: MailboxMessageKind,
    ciphertext: String,
    created_at: u64,
    expires_at: u64,
}

impl PeerAnnounce {
    pub fn new(
        identity: &Identity,
        device_id: Option<DeviceId>,
        addresses: Vec<String>,
        mailbox_key: Option<String>,
        ttl_seconds: u64,
    ) -> Result<Self> {
        limits::ensure_addresses(&addresses)?;
        if let Some(key) = &mailbox_key {
            limits::ensure_len(key, limits::MAX_NETWORK_ADDRESS_BYTES)?;
        }
        let created_at = current_unix_timestamp();
        let signed = PeerAnnounceSignedFields {
            r#type: PEER_ANNOUNCE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            user_id: identity.user_id().clone(),
            device_id,
            addresses,
            mailbox_key,
            created_at,
            expires_at: created_at.saturating_add(ttl_seconds),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = identity.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            user_id: signed.user_id,
            device_id: signed.device_id,
            addresses: signed.addresses,
            mailbox_key: signed.mailbox_key,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, identity_public_key: &[u8; 32]) -> Result<()> {
        if self.r#type != PEER_ANNOUNCE_TYPE {
            return Err(LmError::InvalidFormat);
        }
        verify_common_identity(
            &self.user_id,
            identity_public_key,
            self.version,
            self.expires_at,
        )?;
        let signed = PeerAnnounceSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            user_id: self.user_id.clone(),
            device_id: self.device_id.clone(),
            addresses: self.addresses.clone(),
            mailbox_key: self.mailbox_key.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        verify_sig(
            identity_public_key,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-peer-announce-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_SIGNAL_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-peer-announce-v1:", text)
    }
}

impl PublicPeerAnnounce {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: &Identity,
        peer_id: String,
        device_id: Option<DeviceId>,
        addresses: Vec<String>,
        capabilities: Vec<PublicPeerCapability>,
        max_mailbox_bytes: Option<u64>,
        max_message_ttl_seconds: Option<u64>,
        max_relay_bandwidth_kbps: Option<u64>,
        ttl_seconds: u64,
    ) -> Result<Self> {
        limits::ensure_len(&peer_id, limits::MAX_NETWORK_ADDRESS_BYTES)?;
        limits::ensure_addresses(&addresses)?;
        let created_at = current_unix_timestamp();
        let signed = PublicPeerAnnounceSignedFields {
            r#type: PUBLIC_PEER_ANNOUNCE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            peer_id,
            user_id: identity.user_id().clone(),
            device_id,
            addresses,
            capabilities,
            max_mailbox_bytes,
            max_message_ttl_seconds,
            max_relay_bandwidth_kbps,
            created_at,
            expires_at: created_at.saturating_add(ttl_seconds),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = identity.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            peer_id: signed.peer_id,
            user_id: signed.user_id,
            device_id: signed.device_id,
            addresses: signed.addresses,
            capabilities: signed.capabilities,
            max_mailbox_bytes: signed.max_mailbox_bytes,
            max_message_ttl_seconds: signed.max_message_ttl_seconds,
            max_relay_bandwidth_kbps: signed.max_relay_bandwidth_kbps,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, identity_public_key: &[u8; 32]) -> Result<()> {
        if self.r#type != PUBLIC_PEER_ANNOUNCE_TYPE {
            return Err(LmError::InvalidFormat);
        }
        verify_common_identity(
            &self.user_id,
            identity_public_key,
            self.version,
            self.expires_at,
        )?;
        let signed = PublicPeerAnnounceSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            peer_id: self.peer_id.clone(),
            user_id: self.user_id.clone(),
            device_id: self.device_id.clone(),
            addresses: self.addresses.clone(),
            capabilities: self.capabilities.clone(),
            max_mailbox_bytes: self.max_mailbox_bytes,
            max_message_ttl_seconds: self.max_message_ttl_seconds,
            max_relay_bandwidth_kbps: self.max_relay_bandwidth_kbps,
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        verify_sig(
            identity_public_key,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-public-peer-announce-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_SIGNAL_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-public-peer-announce-v1:", text)
    }
}

impl MessageReceipt {
    pub fn new(
        from: &Identity,
        to_user_id: UserId,
        target_message_id: Uuid,
        conversation_id: String,
        mailbox_delivery_id: Option<String>,
        kind: MessageReceiptKind,
        ttl_seconds: u64,
    ) -> Result<Self> {
        limits::ensure_len(&conversation_id, limits::MAX_NETWORK_ADDRESS_BYTES)?;
        if let Some(delivery_id) = &mailbox_delivery_id {
            if delivery_id.trim().is_empty() {
                return Err(LmError::InvalidFormat);
            }
            limits::ensure_len(delivery_id, limits::MAX_NETWORK_ADDRESS_BYTES)?;
        }
        let created_at = current_unix_timestamp();
        let signed = MessageReceiptSignedFields {
            r#type: MESSAGE_RECEIPT_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            receipt_id: Uuid::new_v4(),
            from_user_id: from.user_id().clone(),
            to_user_id,
            target_message_id,
            conversation_id,
            mailbox_delivery_id,
            kind,
            created_at,
            expires_at: created_at.saturating_add(ttl_seconds),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = from.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            receipt_id: signed.receipt_id,
            from_user_id: signed.from_user_id,
            to_user_id: signed.to_user_id,
            target_message_id: signed.target_message_id,
            conversation_id: signed.conversation_id,
            mailbox_delivery_id: signed.mailbox_delivery_id,
            kind: signed.kind,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, from_identity_public_key: &[u8; 32]) -> Result<()> {
        if self.r#type != MESSAGE_RECEIPT_TYPE {
            return Err(LmError::InvalidFormat);
        }
        verify_common_identity(
            &self.from_user_id,
            from_identity_public_key,
            self.version,
            self.expires_at,
        )?;
        limits::ensure_len(&self.conversation_id, limits::MAX_NETWORK_ADDRESS_BYTES)?;
        if let Some(delivery_id) = &self.mailbox_delivery_id {
            if delivery_id.trim().is_empty() {
                return Err(LmError::InvalidFormat);
            }
            limits::ensure_len(delivery_id, limits::MAX_NETWORK_ADDRESS_BYTES)?;
        }
        let signed = MessageReceiptSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            receipt_id: self.receipt_id,
            from_user_id: self.from_user_id.clone(),
            to_user_id: self.to_user_id.clone(),
            target_message_id: self.target_message_id,
            conversation_id: self.conversation_id.clone(),
            mailbox_delivery_id: self.mailbox_delivery_id.clone(),
            kind: self.kind,
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        verify_sig(
            from_identity_public_key,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }

    pub fn to_export_text(&self) -> Result<String> {
        self.verify_public_fields()?;
        crate::codec::encode_json_prefixed("lm-message-receipt-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_MESSAGE_RECEIPT_TEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-message-receipt-v1:", text)
    }

    fn verify_public_fields(&self) -> Result<()> {
        if self.r#type != MESSAGE_RECEIPT_TYPE {
            return Err(LmError::InvalidFormat);
        }
        if self.version != protocol::PROTOCOL_VERSION_V1 {
            return Err(LmError::UnsupportedVersion(self.version));
        }
        limits::ensure_len(&self.conversation_id, limits::MAX_NETWORK_ADDRESS_BYTES)?;
        if let Some(delivery_id) = &self.mailbox_delivery_id {
            if delivery_id.trim().is_empty() {
                return Err(LmError::InvalidFormat);
            }
            limits::ensure_len(delivery_id, limits::MAX_NETWORK_ADDRESS_BYTES)?;
        }
        Ok(())
    }
}

impl MailboxMessage {
    pub fn new(
        identity: &Identity,
        to_user_id: UserId,
        kind: MailboxMessageKind,
        ciphertext: String,
        ttl_seconds: u64,
    ) -> Result<Self> {
        limits::ensure_len(&ciphertext, limits::MAX_MAILBOX_CIPHERTEXT_BYTES)?;
        let created_at = current_unix_timestamp();
        let signed = MailboxMessageSignedFields {
            r#type: MAILBOX_MESSAGE_TYPE.to_string(),
            version: protocol::PROTOCOL_VERSION_V1,
            message_id: Uuid::new_v4(),
            from_user_id: identity.user_id().clone(),
            to_user_id,
            kind,
            ciphertext,
            created_at,
            expires_at: created_at.saturating_add(ttl_seconds),
        };
        let bytes = protocol::to_canonical_bytes(&signed)?;
        let signature = identity.signing_key().sign(&bytes);
        Ok(Self {
            r#type: signed.r#type,
            version: signed.version,
            message_id: signed.message_id,
            from_user_id: signed.from_user_id,
            to_user_id: signed.to_user_id,
            kind: signed.kind,
            ciphertext: signed.ciphertext,
            created_at: signed.created_at,
            expires_at: signed.expires_at,
            signature: BASE64.encode(signature.to_bytes()),
        })
    }

    pub fn verify(&self, from_identity_public_key: &[u8; 32]) -> Result<()> {
        if self.r#type != MAILBOX_MESSAGE_TYPE {
            return Err(LmError::InvalidFormat);
        }
        verify_common_identity(
            &self.from_user_id,
            from_identity_public_key,
            self.version,
            self.expires_at,
        )?;
        let signed = MailboxMessageSignedFields {
            r#type: self.r#type.clone(),
            version: self.version,
            message_id: self.message_id,
            from_user_id: self.from_user_id.clone(),
            to_user_id: self.to_user_id.clone(),
            kind: self.kind.clone(),
            ciphertext: self.ciphertext.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        verify_sig(
            from_identity_public_key,
            &protocol::to_canonical_bytes(&signed)?,
            &self.signature,
        )
    }

    pub fn to_export_text(&self) -> Result<String> {
        crate::codec::encode_json_prefixed("lm-mailbox-message-v1:", self)
    }

    pub fn from_export_text(text: &str) -> Result<Self> {
        limits::ensure_len(text, limits::MAX_MAILBOX_CIPHERTEXT_BYTES)?;
        crate::codec::decode_json_prefixed("lm-mailbox-message-v1:", text)
    }
}

fn verify_common_identity(
    user_id: &UserId,
    identity_public_key: &[u8; 32],
    version: u16,
    expires_at: u64,
) -> Result<()> {
    if version != protocol::PROTOCOL_VERSION_V1 {
        return Err(LmError::UnsupportedVersion(version));
    }
    if expires_at <= current_unix_timestamp() {
        return Err(LmError::ExpiredObject);
    }
    if !user_id.verify_public_key(identity_public_key) {
        return Err(LmError::InvalidUserId);
    }
    Ok(())
}

#[cfg(test)]
mod network_extra_tests {
    use super::*;

    #[test]
    fn peer_and_public_peer_announce_roundtrip() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let peer = PeerAnnounce::new(
            &alice,
            None,
            vec!["/ip4/127.0.0.1/tcp/4001".into()],
            Some("mailbox".into()),
            3600,
        )
        .unwrap();
        peer.verify(&alice.identity_public_key()).unwrap();
        let public = PublicPeerAnnounce::new(
            &alice,
            "peer1".into(),
            None,
            vec!["/ip4/127.0.0.1/tcp/4001".into()],
            vec![PublicPeerCapability::Bootstrap, PublicPeerCapability::Dht],
            Some(1024),
            Some(3600),
            Some(1024),
            3600,
        )
        .unwrap();
        public.verify(&alice.identity_public_key()).unwrap();
        let text = public.to_export_text().unwrap();
        PublicPeerAnnounce::from_export_text(&text)
            .unwrap()
            .verify(&alice.identity_public_key())
            .unwrap();
    }

    #[test]
    fn mailbox_message_roundtrip_and_tamper() {
        let (alice, _a) = Identity::create_with_passphrase("alice").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob").unwrap();
        let mut msg = MailboxMessage::new(
            &alice,
            bob.user_id().clone(),
            MailboxMessageKind::DirectEnvelope,
            "ciphertext".into(),
            3600,
        )
        .unwrap();
        msg.verify(&alice.identity_public_key()).unwrap();
        msg.ciphertext = "changed".into();
        assert_eq!(
            msg.verify(&alice.identity_public_key()).unwrap_err(),
            LmError::InvalidSignature
        );
    }

    #[test]
    fn message_receipt_roundtrip_and_tamper() {
        let (alice, _a) = Identity::create_with_passphrase("alice receipt").unwrap();
        let (bob, _b) = Identity::create_with_passphrase("bob receipt").unwrap();
        let mut receipt = MessageReceipt::new(
            &bob,
            alice.user_id().clone(),
            Uuid::new_v4(),
            "conversation-1".into(),
            Some("delivery-1".into()),
            MessageReceiptKind::Delivered,
            3600,
        )
        .unwrap();
        receipt.verify(&bob.identity_public_key()).unwrap();
        let text = receipt.to_export_text().unwrap();
        let imported = MessageReceipt::from_export_text(&text).unwrap();
        imported.verify(&bob.identity_public_key()).unwrap();
        receipt.kind = MessageReceiptKind::Read;
        assert_eq!(
            receipt.verify(&bob.identity_public_key()).unwrap_err(),
            LmError::InvalidSignature
        );
    }
}
