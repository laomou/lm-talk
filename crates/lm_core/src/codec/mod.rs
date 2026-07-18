//! Text export/import helpers for protocol objects.

use crate::{LmError, Result};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Serialize, de::DeserializeOwned};

pub const IDENTITY_BACKUP_TEXT_PREFIX: &str = "lm-identity-backup-v1:";
pub const CONTACT_CARD_TEXT_PREFIX: &str = "lm-contact-card-v1:";
pub const FRIEND_REQUEST_TEXT_PREFIX: &str = "lm-friend-request-v1:";
pub const FRIEND_RESPONSE_TEXT_PREFIX: &str = "lm-friend-response-v1:";

pub fn encode_json_prefixed<T: Serialize>(prefix: &str, value: &T) -> Result<String> {
    let json = serde_json::to_vec(value).map_err(|_| LmError::SerializationFailed)?;
    Ok(format!("{}{}", prefix, URL_SAFE_NO_PAD.encode(json)))
}

pub fn decode_json_prefixed<T: DeserializeOwned>(prefix: &str, text: &str) -> Result<T> {
    let payload = text
        .strip_prefix(prefix)
        .ok_or(LmError::InvalidFormat)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .map_err(|_| LmError::InvalidFormat)?;
    serde_json::from_slice(&bytes).map_err(|_| LmError::SerializationFailed)
}
