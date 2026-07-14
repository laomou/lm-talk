declare module './wasm/lm_wasm.js' {
  export default function init(input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module): Promise<void>
  export function normalize_passphrase(input: string): string
  export function create_identity(passphrase: string): string
  export function reencrypt_identity_backup(backupText: string, oldPassphrase: string, newPassphrase: string): string
  export function export_data_backup(identityBackupText: string, passphrase: string, dataJson: string): string
  export function import_data_backup(identityBackupText: string, passphrase: string, dataBackupText: string): string
  export function restore_identity(backupText: string, passphrase: string): string
  export function create_device_cert(backupText: string, passphrase: string, deviceName?: string): string
  export function create_device_revoke(backupText: string, passphrase: string, deviceId: string, reason?: string): string
  export function inspect_device_revoke(revokeText: string, identityPublicKeyBase64: string): string
  export function export_contact_card(backupText: string, passphrase: string, displayName?: string, deviceCertsJson?: string): string
  export function inspect_contact_card(contactCardText: string): string
  export function create_friend_request(backupText: string, passphrase: string, myContactCardText: string, targetContactCardText: string, note?: string): string
  export function inspect_friend_request(requestText: string): string
  export function accept_friend_request(backupText: string, passphrase: string, requestText: string): string
  export function inspect_friend_response(responseText: string, responderContactCardText: string): string
  export function reject_friend_request(backupText: string, passphrase: string, requestText: string): string
  export function import_contact_as_json(contactCardText: string, trustLevel: string): string
  export function encrypt_text_message(backupText: string, passphrase: string, toContactCardText: string, conversationId: string, text: string): string
  export function decrypt_text_message(backupText: string, passphrase: string, fromContactCardText: string, envelopeJson: string): string
  export function create_file_package(backupText: string, passphrase: string, toContactCardText: string, name: string, mimeType: string, fileBytesBase64: string, chunkSize: number): string
  export function inspect_file_package(filePackageJson: string): string
  export function decrypt_file_package(backupText: string, passphrase: string, fromContactCardText: string, filePackageJson: string): string

  export function create_group_invite(backupText: string, passphrase: string, groupId: string, groupName: string, memberUserIdsJson: string): string
  export function inspect_group_invite(inviteText: string, inviterContactCardText: string): string
  export function create_group_event(backupText: string, passphrase: string, groupId: string, sequence: bigint, actionJson: string): string
  export function inspect_group_event(eventText: string, actorContactCardText: string): string

  export function create_signal_offer(backupText: string, passphrase: string, toUserId: string | undefined, sdp: string, ttlSeconds: number): string
  export function inspect_signal_offer(offerText: string, fromContactCardText: string): string
  export function create_signal_answer(backupText: string, passphrase: string, offerText: string, sdp: string, ttlSeconds: number): string
  export function inspect_signal_answer(answerText: string, fromContactCardText: string): string
  export function create_peer_announce(backupText: string, passphrase: string, addressesJson: string, mailboxKey: string | undefined, ttlSeconds: bigint): string
  export function inspect_peer_announce(text: string, identityPublicKeyBase64: string): string
  export function create_public_peer_announce(backupText: string, passphrase: string, peerId: string, addressesJson: string, capabilitiesJson: string, ttlSeconds: bigint): string
  export function inspect_public_peer_announce(text: string, identityPublicKeyBase64: string): string
  export function create_mailbox_message(backupText: string, passphrase: string, toUserId: string, kind: string, ciphertext: string, ttlSeconds: bigint): string
  export function inspect_mailbox_message(text: string, fromIdentityPublicKeyBase64: string): string
}
