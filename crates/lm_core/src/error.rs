use thiserror::Error;

pub type Result<T> = std::result::Result<T, LmError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LmError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u16),
    #[error("expired object")]
    ExpiredObject,
    #[error("wrong passphrase or corrupted backup")]
    WrongPassphrase,
    #[error("corrupted backup")]
    CorruptedBackup,
    #[error("invalid backup format")]
    InvalidBackupFormat,
    #[error("invalid protocol format")]
    InvalidFormat,
    #[error("blocked sender")]
    BlockedSender,
    #[error("unknown contact")]
    UnknownContact,
    #[error("contact is not a confirmed friend")]
    NotFriend,
    #[error("item not found")]
    NotFound,
    #[error("replay detected")]
    ReplayDetected,
    #[error("duplicate message")]
    DuplicateMessage,
    #[error("invalid user id")]
    InvalidUserId,
    #[error("invalid device id")]
    InvalidDeviceId,
    #[error("cryptographic operation failed")]
    CryptoError,
    #[error("decryption failed")]
    DecryptionFailed,
    #[error("storage error")]
    StorageError,
    #[error("network error")]
    NetworkError,
    #[error("payload too large")]
    PayloadTooLarge,
    #[error("random number generator failed")]
    RandomFailed,
    #[error("serialization failed")]
    SerializationFailed,
    #[error("ratchet counter exhausted")]
    CounterExhausted,
}
