import type { WorkerRpcResponse } from './workers/WorkerRpcClient'

export type IdentityOutput = {
  user_id: string
  identity_public_key: string
  x25519_public_key: string
  backup_text: string
}

export type RestoreOutput = {
  user_id: string
  identity_public_key: string
  x25519_public_key: string
}

export type ReencryptIdentityBackupOutput = {
  user_id: string
  backup_text: string
}

export type DeviceOutput = {
  device_id: string
  device_public_key: string
  device_box_public_key?: string
  device_cert_json: string
  device_backup_text?: string
}

export type DeviceRevokeInfo = {
  user_id: string
  device_id: string
  reason?: string
  created_at: number
}

export type DeviceCertItem = {
  device_id: string
  device_name?: string
  device_public_key?: string
  device_box_public_key?: string
  created_at?: number
}

export type ContactInfo = {
  user_id: string
  display_name?: string
  fingerprint: string
  identity_public_key: string
  x25519_public_key: string
  device_count: number
  device_certs?: DeviceCertItem[]
  avatar_data_url?: string
}

export type ContactItem = ContactInfo & {
  contact_card_text: string
  kind: 'contact' | 'group'
  state: 'LocalOnly' | 'RequestSent' | 'RequestReceived' | 'Friend' | 'Rejected' | 'Blocked'
  pending_request_id?: string
  last_friend_request_error?: string
  last_secure_session_error?: string
  last_secure_session_attempt_at?: number
  last_secure_session_success_at?: number
  secure_session_failure_count?: number
  revoked_device_ids?: string[]
  device_revocations?: DeviceRevokeInfo[]
  device_certs?: DeviceCertItem[]
  block_reason?: string
  read_receipts?: 'default' | 'enabled' | 'disabled'
  mailbox_hint_url?: string
  last_dht_discovery_attempt_at?: number
  last_dht_discovery_success_at?: number
  last_dht_discovery_error?: string
  last_dht_discovery_error_kind?: 'network' | 'not-found' | 'expired' | 'invalid-record' | 'signature' | 'unknown'
  dht_discovery_risk_level?: 'low' | 'medium' | 'high'
  dht_discovery_failure_count?: number
  next_dht_discovery_retry_at?: number
  last_prekey_dht_found_at?: number
  last_mailbox_hint_dht_found_at?: number
  last_contact_card_dht_found_at?: number
  fingerprint_verified_at?: number
  fingerprint_verified_note?: string
  avatar_data_url?: string
}

export type FilterLevel = 'Off' | 'Relaxed' | 'Standard' | 'Strict'
export type FilterAction = 'Allow' | 'Warn' | 'Blur' | 'Hide' | 'Drop'
export type SafetyPolicy = {
  enableTextFilter: boolean
  textFilterLevel: FilterLevel
  warnExternalLinks: boolean
  warnExecutableFiles: boolean
  dropFilteredIncoming: boolean
  requireVerifiedContactsForSend: boolean
  requireVerifiedContactsForReceive: boolean
  requireSealedPerDeviceSlotsForSend: boolean
  requireSealedPerDeviceSlotsForReceive: boolean
}

export type GroupInviteItem = {
  invite_id: string
  group_id: string
  group_name: string
  inviter_user_id: string
  member_user_ids: string[]
  created_at: number
  expires_at: number
  invite_text: string
}

export type FriendRequestItem = {
  request_id: string
  from_user_id: string
  to_user_id: string
  note?: string
  created_at: number
  expires_at: number
  request_text: string
  from_contact_card_text: string
  quarantined?: boolean
  quarantine_reason?: string
}

export type FriendRequestRateRecord = {
  from_user_id: string
  first_seen_at: number
  last_seen_at: number
  count: number
}

export type GroupItem = {
  group_id: string
  name: string
  member_user_ids: string[]
  admin_user_ids?: string[]
  policy_state_json?: string
  created_at: number
  sequence?: number
  last_event_summary?: string
  last_event_actor_user_id?: string
  last_event_at?: number
  last_event_error?: string
  last_event_error_at?: number
  last_event_recovery_hint?: string
  removed_self_at?: number
  removed_self_by?: string
  last_sender_key_error?: string
  last_sender_key_error_at?: number
}

export type GroupSenderKeyItem = {
  key_id: string
  group_id: string
  sender_user_id: string
  state_json: string
  distribution_text?: string
  updated_at: number
}

export type MessageStatus = 'queued' | 'sent' | 'mailbox' | 'delivered' | 'read' | 'copied' | 'received' | 'failed'

export type ChatMessage = {
  id: string
  conversation_id: string
  peer_user_id: string
  group_id?: string
  direction: 'out' | 'in'
  text: string
  envelope_json?: string
  protocol_message_id?: string
  mailbox_delivery_id?: string
  delivered_at?: number
  read_at?: number
  file_downloaded_at?: number
  attachment_name?: string
  attachment_mime?: string
  attachment_size?: number
  attachment_decrypted_at?: number
  attachment_error?: string
  attachment_error_at?: number
  target_device_ids?: string[]
  per_device_envelope_json?: string
  per_device_envelope_version?: number
  status: MessageStatus
  created_at: number
}

export type OutgoingMessageJob = {
  id: string
  message_id: string
  peer_user_id: string
  conversation_id: string
  text: string
  created_at: number
}


export type PerDeviceEnvelopeV1 = {
  type: 'lm-per-device-envelope-v1'
  version: 1
  conversation_id: string
  sender_user_id: string
  target_devices: Array<{
    device_id: string
    slot_id: string
    nonce: string
    aad: string
    crypto: 'placeholder-shared-envelope-v1' | 'x25519-ephemeral-hkdf-xchacha20poly1305-device-slot-v1'
    x25519_ephemeral_public_key?: string
    ciphertext: string
  }>
  fallback_ciphertext?: string
  created_at: number
  signature?: string
}

export type MessageReceiptSyncItem = {
  peer_user_id: string
  protocol_message_id: string
  status: MessageStatus
  mailbox_delivery_id?: string
  delivered_at?: number
  read_at?: number
  created_at?: number
}

export type RatchetSessionItem = {
  peer_user_id: string
  state_text: string
  updated_at: number
}

export type PendingSecureSessionOfferItem = {
  offer_id: string
  peer_user_id: string
  prekey_private_bundle_json: string
  ratchet_dh_private_key: string
  created_at: number
}

export type OutboxItem = {
  id: string
  peer_user_id: string
  envelope_json: string
  message_id?: string
  kind?: 'direct-envelope' | 'group-fanout' | 'file-package' | 'delivery-receipt' | 'read-receipt' | 'contact-update' | 'other'
  status: 'queued' | 'sent' | 'failed'
  created_at: number
  delivery_order?: number
  retry_count: number
  next_retry_at?: number
  expires_at?: number
  last_error?: string
}

export type OutboxSyncSummary = {
  queued: number
  failed: number
  sent: number
  oldest_pending_at?: number
  failed_kinds?: Record<string, number>
}

export type MailboxFailedItem = {
  id: string
  delivery_id?: string
  message_id?: string
  message: any
  reason: string
  first_failed_at: number
  last_failed_at: number
  retry_count: number
}
export type MailboxFailureCategory = 'session' | 'contact' | 'security' | 'expired' | 'decrypt' | 'other'

export type ContactCardUpdateFanoutRecord = {
  peer_user_id: string
  update_id: string
  status: 'sent' | 'queued' | 'acked'
  sent_at: number
  acked_at?: number
  retry_count?: number
  last_retry_at?: number
}

export type ContactCardDhtAutoRefreshRecord = {
  user_id: string
  display_name?: string
  status: 'success' | 'failed'
  refreshed_at: number
  error?: string
}

export type ProcessedMailboxRecord = {
  id: string
  processed_at: number
}


export type EncryptedStringV1 = {
  __lm_enc_v1: true
  alg: 'AES-GCM'
  kdf: 'PBKDF2-SHA-256'
  iv: string
  ct: string
}

export type PersistedState = {
  backupScope?: 'identity-and-security-v1'
  backupText: string
  contacts: ContactItem[]
  friendRequests: FriendRequestItem[]
  groups: GroupItem[]
  groupInvites: GroupInviteItem[]
  groupSenderKeys?: GroupSenderKeyItem[]
  messages: ChatMessage[]
  outbox: OutboxItem[]
  ratchetSessions?: RatchetSessionItem[]
  pendingSecureSessionOffers?: PendingSecureSessionOfferItem[]
  myContactCardText: string
  myDeviceCertJson?: string
  myDeviceBackupText?: string
  myDeviceId?: string
  prekeyBundleText?: string
  prekeyPrivateBundleJson?: string | EncryptedStringV1
  prekeySignedOneTimeRecordTexts?: string[]
  safetyPolicy?: SafetyPolicy
  nodeControlUrl?: string | EncryptedStringV1
  nodeEnabled?: boolean
  autoMailboxTake?: boolean
  autoReadReceipts?: boolean
  autoPublishPreKey?: boolean
  autoNodeSync?: boolean
  autoSelfMailboxSync?: boolean
  lastNodeSnapshotSyncAt?: number
  processedMailboxIds?: Array<string | ProcessedMailboxRecord>
  mailboxFailedItems?: MailboxFailedItem[]
  contactCardUpdateFanoutRecords?: ContactCardUpdateFanoutRecord[]
  syncRecoveryHistory?: string[]
  dhtOperationHistory?: string[]
  friendRequestRateRecords?: FriendRequestRateRecord[]
  lastFullDataBackupAt?: number
  lastSelfMailboxBackupPushedAt?: number
  lastSelfMailboxBackupReceivedAt?: number
  lastSelfMailboxBackupMergedAt?: number
  processedSelfSyncIds?: string[]
  processedSelfSyncRequestIds?: string[]
  selfSyncMissingRequestRecords?: SelfSyncRequestRecord[]
  selfSyncRequestSentCount?: number
  selfSyncRequestHitCount?: number
  selfSyncRequestMissCount?: number
  selfSyncRecentPackages?: SelfSyncCachedPackage[]
  lastSelfSyncPushedAt?: number
  lastSelfSyncMergedAt?: number
  lastSelfSyncSequenceSent?: number
  lastSelfSyncSequenceMerged?: number
  selfSyncGapCount?: number
  lastSelfSyncGapAt?: number
  lastSelfSyncMissingPreviousId?: string
  lastSelfSyncReceiptStatesSent?: number
  lastSelfSyncReceiptStatesMerged?: number
  totalSelfSyncReceiptStatesMerged?: number
  lastSelfSyncOutboxSummary?: OutboxSyncSummary
  unverifiedIncomingDropCount?: number
  lastUnverifiedIncomingDropAt?: number
  lastUnverifiedIncomingDropFrom?: string
  revokedDeviceIncomingDropCount?: number
  lastRevokedDeviceIncomingDropAt?: number
  lastRevokedDeviceIncomingDropFrom?: string
  perDeviceEnvelopeSentCount?: number
  perDeviceEnvelopeReceivedCount?: number
  perDeviceEnvelopeDropCount?: number
  lastPerDeviceEnvelopeAt?: number
  lastPerDeviceEnvelopeDropAt?: number
  lastPerDeviceEnvelopeDropReason?: string
  contactCardUpdateFanoutCount?: number
  contactCardUpdateFanoutSkipCount?: number
  lastContactCardUpdateFanoutAt?: number
  contactCardDhtAutoRefreshCount?: number
  lastContactCardDhtAutoRefreshAt?: number
  lastContactCardDhtAutoRefreshError?: string
  contactCardDhtAutoRefreshHistory?: ContactCardDhtAutoRefreshRecord[]
  myProfileAvatarDataUrl?: string
  profileSyncPending?: boolean
}

export type IdentityAndSecurityBackupState = Omit<PersistedState, 'messages' | 'outbox'> & {
  backupScope: 'identity-and-security-v1'
}

export type PersistedMeta = {
  backupText: string
  myContactCardText: string
  myDeviceCertJson?: string
  myDeviceBackupText?: string
  myDeviceId?: string
  prekeyBundleText?: string
  prekeyPrivateBundleJson?: string | EncryptedStringV1
  prekeySignedOneTimeRecordTexts?: string[]
  pendingSecureSessionOffers?: Array<Omit<PendingSecureSessionOfferItem, 'prekey_private_bundle_json' | 'ratchet_dh_private_key'> & {
    prekey_private_bundle_json: string | EncryptedStringV1
    ratchet_dh_private_key: string | EncryptedStringV1
  }>
  safetyPolicy?: SafetyPolicy
  nodeControlUrl?: string | EncryptedStringV1
  nodeEnabled?: boolean
  autoMailboxTake?: boolean
  autoReadReceipts?: boolean
  autoPublishPreKey?: boolean
  autoNodeSync?: boolean
  autoSelfMailboxSync?: boolean
  lastNodeSnapshotSyncAt?: number
  processedMailboxIds?: Array<string | ProcessedMailboxRecord>
  mailboxFailedItems?: MailboxFailedItem[]
  contactCardUpdateFanoutRecords?: ContactCardUpdateFanoutRecord[]
  syncRecoveryHistory?: string[]
  dhtOperationHistory?: string[]
  friendRequestRateRecords?: FriendRequestRateRecord[]
  lastFullDataBackupAt?: number
  lastSelfMailboxBackupPushedAt?: number
  lastSelfMailboxBackupReceivedAt?: number
  lastSelfMailboxBackupMergedAt?: number
  processedSelfSyncIds?: string[]
  processedSelfSyncRequestIds?: string[]
  selfSyncMissingRequestRecords?: SelfSyncRequestRecord[]
  selfSyncRequestSentCount?: number
  selfSyncRequestHitCount?: number
  selfSyncRequestMissCount?: number
  selfSyncRecentPackages?: SelfSyncCachedPackage[]
  lastSelfSyncPushedAt?: number
  lastSelfSyncMergedAt?: number
  lastSelfSyncSequenceSent?: number
  lastSelfSyncSequenceMerged?: number
  selfSyncGapCount?: number
  lastSelfSyncGapAt?: number
  lastSelfSyncMissingPreviousId?: string
  lastSelfSyncReceiptStatesSent?: number
  lastSelfSyncReceiptStatesMerged?: number
  totalSelfSyncReceiptStatesMerged?: number
  lastSelfSyncOutboxSummary?: OutboxSyncSummary
  unverifiedIncomingDropCount?: number
  lastUnverifiedIncomingDropAt?: number
  lastUnverifiedIncomingDropFrom?: string
  revokedDeviceIncomingDropCount?: number
  lastRevokedDeviceIncomingDropAt?: number
  lastRevokedDeviceIncomingDropFrom?: string
  perDeviceEnvelopeSentCount?: number
  perDeviceEnvelopeReceivedCount?: number
  perDeviceEnvelopeDropCount?: number
  lastPerDeviceEnvelopeAt?: number
  lastPerDeviceEnvelopeDropAt?: number
  lastPerDeviceEnvelopeDropReason?: string
  contactCardUpdateFanoutCount?: number
  contactCardUpdateFanoutSkipCount?: number
  lastContactCardUpdateFanoutAt?: number
  contactCardDhtAutoRefreshCount?: number
  lastContactCardDhtAutoRefreshAt?: number
  lastContactCardDhtAutoRefreshError?: string
  contactCardDhtAutoRefreshHistory?: ContactCardDhtAutoRefreshRecord[]
  myProfileAvatarDataUrl?: string
  profileSyncPending?: boolean
  schemaVersion: number
}

export type SelfSyncPackage = {
  type: 'lm-self-sync-v1'
  version: number
  sync_id: string
  sequence: number
  previous_sync_id?: string
  created_at: number
  from_user_id: string
  identity_public_key: string
  from_device_id?: string
  contacts: ContactItem[]
  messageReceiptStates?: MessageReceiptSyncItem[]
  outboxSummary?: OutboxSyncSummary
  myContactCardText?: string
  myDeviceCertJson?: string
  myDeviceId?: string
  myProfileAvatarDataUrl?: string
  signature?: string
  dhtOperationHistory?: string[]
  processedSelfSyncIds?: string[]
  unverifiedIncomingDropCount?: number
  revokedDeviceIncomingDropCount?: number
}

export type SelfSyncRequestPackage = {
  type: 'lm-self-sync-request-v1'
  version: number
  request_id: string
  missing_sync_id: string
  created_at: number
  from_user_id: string
  identity_public_key: string
  from_device_id?: string
  signature?: string
}

export type SelfSyncCachedPackage = {
  sync_id: string
  sequence: number
  created_at: number
  expires_at?: number
  payload: string
}

export type SelfSyncRequestRecord = {
  missing_sync_id: string
  requested_at: number
}

export type LocalIdentityRecord = {
  id: string
  user_id: string
  display_name: string
  backup_text: string
  updated_at: number
}
