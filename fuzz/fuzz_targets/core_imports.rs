#![no_main]

use libfuzzer_sys::fuzz_target;
use lm_core::{
    ContactCard, DeviceRevoke, FileChunkEnvelope, FileManifest, FriendRequest, FriendResponse,
    GroupEvent, GroupInvite, GroupSenderKeyDistribution, IdentityBackupPackage, MailboxMessage, MessageReceipt,
    PeerAnnounce, PreKeyBundle, PublicPeerAnnounce, RatchetSessionState, SignalAnswer,
    SignalOffer, SignedOneTimePreKeyRecord,
};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = ContactCard::from_export_text(&text);
    let _ = FriendRequest::from_export_text(&text);
    let _ = FriendResponse::from_export_text(&text);
    let _ = IdentityBackupPackage::from_export_text(&text);
    let _ = PreKeyBundle::from_export_text(&text);
    let _ = SignedOneTimePreKeyRecord::from_export_text(&text);
    let _ = SignalOffer::from_export_text(&text);
    let _ = SignalAnswer::from_export_text(&text);
    let _ = PeerAnnounce::from_export_text(&text);
    let _ = PublicPeerAnnounce::from_export_text(&text);
    let _ = MailboxMessage::from_export_text(&text);
    let _ = MessageReceipt::from_export_text(&text);
    let _ = GroupInvite::from_export_text(&text);
    let _ = GroupEvent::from_export_text(&text);
    let _ = GroupSenderKeyDistribution::from_export_text(&text);
    let _ = RatchetSessionState::from_export_text(&text);
    let _ = FileManifest::from_export_text(&text);
    let _ = FileChunkEnvelope::from_json(&text);
    let _ = DeviceRevoke::from_export_text(&text);
});
