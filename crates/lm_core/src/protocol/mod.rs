//! Protocol constants and deterministic encoding helpers.

use crate::{LmError, Result};
use serde::{Serialize, de::DeserializeOwned};

pub const LM_IDENTITY_BACKUP_TYPE: &str = "lm-identity-backup-v1";
pub const LM_CONTACT_CARD_TYPE: &str = "lm-contact-card-v1";
pub const LM_FRIEND_REQUEST_TYPE: &str = "lm-friend-request-v1";
pub const LM_FRIEND_RESPONSE_TYPE: &str = "lm-friend-response-v1";
pub const PROTOCOL_VERSION_V1: u16 = 1;

pub fn to_canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    postcard::to_allocvec(value).map_err(|_| LmError::SerializationFailed)
}

pub fn from_canonical_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    postcard::from_bytes(bytes).map_err(|_| LmError::SerializationFailed)
}
