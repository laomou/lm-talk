//! Core library for LM Talk.
//!
//! This crate intentionally avoids network and UI dependencies. It owns the
//! protocol data types and cryptographic primitives that can be reused by Web,
//! desktop, mobile, and native node runtimes.

pub mod codec;
pub mod contact;
pub mod crypto;
pub mod device;
pub mod error;
pub mod file;
pub mod friend;
pub mod group;
pub mod identity;
pub mod limits;
pub mod message;
pub mod network;
pub mod outbox;
pub mod passphrase;
pub mod policy;
pub mod prekey;
pub mod protocol;
pub mod ratchet;
pub mod storage;

pub use contact::{Contact, ContactCard, ContactState, TrustLevel};
pub use device::{DeviceCert, DeviceId, DeviceIdentity, DeviceRevoke};
pub use error::{LmError, Result};
pub use file::{FileChunkEnvelope, FileManifest, file_hash_base64, verify_file_hash};
pub use friend::{FriendRequest, FriendResponse};
pub use group::{
    GroupEvent, GroupEventAction, GroupInvite, GroupPlainMessage, GroupPolicyState,
    GroupSenderEnvelope, GroupSenderKeyDistribution, GroupSenderKeyState,
};
pub use identity::{Identity, IdentityBackupPackage, IdentitySeed, UserId};
pub use limits::*;
pub use message::{
    DirectEnvelope, MessageBody, MvpSessionCrypto, PlainMessage, RatchetEnvelope, SessionCrypto,
    SessionDirection,
};
pub use network::{
    MailboxMessage, MailboxMessageKind, PeerAnnounce, PublicPeerAnnounce, PublicPeerCapability,
    SignalAnswer, SignalKind, SignalOffer,
};
pub use outbox::{Outbox, OutboxItem, OutboxStatus};
pub use passphrase::normalize_passphrase;
pub use policy::{BlockEntry, FilterAction, FilterLevel, LocalSafetyPolicy, StrangerMessagePolicy};
pub use prekey::{
    OneTimePreKey, OneTimePreKeyPrivate, PreKeyBundle, PreKeyPrivateBundle,
    SignedOneTimePreKeyRecord, X3dhInitialMessage, X3dhInitiatorSecret, x3dh_initiator_secret,
    x3dh_initiator_secret_with_one_time_prekey_id,
    x3dh_initiator_secret_with_one_time_prekey_record, x3dh_responder_secret,
};
pub use ratchet::{
    RatchetDhKeyPair, RatchetHeader, RatchetMessageKey, RatchetRole, RatchetSessionState,
    RatchetSkippedKey,
};
pub use storage::{MemoryStore, StoredMessage};

#[cfg(target_arch = "wasm32")]
pub fn unix_now() -> u64 {
    let millis = js_sys::Date::now();
    if millis.is_finite() && millis > 0.0 {
        (millis / 1000.0) as u64
    } else {
        0
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
