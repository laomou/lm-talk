<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import LoginPage from './components/LoginPage.vue'
import ChatPage from './components/ChatPage.vue'
import DiagnosticsPage from './components/DiagnosticsPage.vue'
import ContactsPage from './components/ContactsPage.vue'
import SettingsPage from './components/SettingsPage.vue'
import QRCode from 'qrcode'
import { TABLES, idbDel, idbGet, idbSet, idbTableClear, idbTableGet, idbTableGetAllByPrefix, idbTableReplaceByPrefix } from './idb'
import init, {
  accept_friend_request,
  create_device_cert,
  create_device_revoke,
  create_friend_request,
  create_group_invite,
  create_group_event,
  create_group_policy_state,
  create_group_sender_key,
  create_identity,
  create_file_package,
  create_mailbox_message,
  create_message_receipt,
  create_peer_announce,
  create_prekey_bundle,
  create_public_peer_announce,
  create_ratchet_dh_keypair,
  create_ratchet_session_from_shared_secret,
  create_ratchet_session_from_shared_secret_with_keys,
  create_ratchet_session_pair,
  decrypt_text_message,
  encrypt_text_message,
  export_contact_card,
  export_data_backup,
  import_contact_as_json,
  import_data_backup,
  inspect_contact_card,
  decrypt_file_package,
  inspect_file_package,
  inspect_device_revoke,
  inspect_friend_request,
  inspect_friend_response,
  inspect_group_invite,
  inspect_group_event,
  group_sender_decrypt_text,
  group_sender_encrypt_text,
  import_group_sender_key,
  apply_group_policy_event,
  inspect_mailbox_message,
  inspect_message_receipt,
  sign_identity_text,
  verify_identity_text_signature,
  seal_device_slot,
  open_device_slot,
  inspect_peer_announce,
  inspect_prekey_bundle,
  inspect_public_peer_announce,
  inspect_ratchet_state,
  normalize_passphrase,
  create_signal_offer,
  create_signal_answer,
  create_x3dh_initial_message,
  create_x3dh_initial_message_with_one_time_prekey_id,
  create_x3dh_initial_message_with_one_time_prekey_record,
  derive_x3dh_responder_secret,
  inspect_signal_offer,
  inspect_signal_answer,
  ratchet_next_receiving_key,
  ratchet_next_sending_key,
  ratchet_dh_step,
  ratchet_encrypt_text_message,
  ratchet_decrypt_text_message,
  reencrypt_identity_backup,
  restore_identity,
} from './wasm/lm_wasm.js'

type IdentityOutput = {
  user_id: string
  identity_public_key: string
  x25519_public_key: string
  backup_text: string
}

type RestoreOutput = {
  user_id: string
  identity_public_key: string
  x25519_public_key: string
}

type ReencryptIdentityBackupOutput = {
  user_id: string
  backup_text: string
}

type DeviceOutput = {
  device_id: string
  device_public_key: string
  device_box_public_key?: string
  device_cert_json: string
  device_backup_text?: string
}

type DeviceRevokeInfo = {
  user_id: string
  device_id: string
  reason?: string
  created_at: number
}

type DeviceCertItem = {
  device_id: string
  device_name?: string
  device_public_key?: string
  device_box_public_key?: string
  created_at?: number
}

type ContactInfo = {
  user_id: string
  display_name?: string
  fingerprint: string
  identity_public_key: string
  x25519_public_key: string
  device_count: number
  device_certs?: DeviceCertItem[]
}

type ContactItem = ContactInfo & {
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
}

type FilterLevel = 'Off' | 'Relaxed' | 'Standard' | 'Strict'
type FilterAction = 'Allow' | 'Warn' | 'Blur' | 'Hide' | 'Drop'
type SafetyPolicy = {
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

type GroupInviteItem = {
  invite_id: string
  group_id: string
  group_name: string
  inviter_user_id: string
  member_user_ids: string[]
  created_at: number
  expires_at: number
  invite_text: string
}

type FriendRequestItem = {
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

type FriendRequestRateRecord = {
  from_user_id: string
  first_seen_at: number
  last_seen_at: number
  count: number
}

type GroupItem = {
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

type GroupSenderKeyItem = {
  key_id: string
  group_id: string
  sender_user_id: string
  state_json: string
  distribution_text?: string
  updated_at: number
}

type MessageStatus = 'queued' | 'sent' | 'mailbox' | 'delivered' | 'read' | 'copied' | 'received' | 'failed'

type ChatMessage = {
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
  target_device_ids?: string[]
  per_device_envelope_json?: string
  per_device_envelope_version?: number
  status: MessageStatus
  created_at: number
}


type PerDeviceEnvelopeV1 = {
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

type MessageReceiptSyncItem = {
  peer_user_id: string
  protocol_message_id: string
  status: MessageStatus
  mailbox_delivery_id?: string
  delivered_at?: number
  read_at?: number
  created_at?: number
}

type RatchetSessionItem = {
  peer_user_id: string
  state_text: string
  updated_at: number
}

type OutboxItem = {
  id: string
  peer_user_id: string
  envelope_json: string
  message_id?: string
  kind?: 'direct-envelope' | 'group-fanout' | 'file-package' | 'delivery-receipt' | 'read-receipt' | 'contact-update' | 'other'
  status: 'queued' | 'sent' | 'failed'
  created_at: number
  retry_count: number
  next_retry_at?: number
  expires_at?: number
  last_error?: string
}

type OutboxSyncSummary = {
  queued: number
  failed: number
  sent: number
  oldest_pending_at?: number
  failed_kinds?: Record<string, number>
}

type MailboxFailedItem = {
  id: string
  delivery_id?: string
  message_id?: string
  message: any
  reason: string
  first_failed_at: number
  last_failed_at: number
  retry_count: number
}

type ContactCardUpdateFanoutRecord = {
  peer_user_id: string
  update_id: string
  status: 'sent' | 'queued' | 'acked'
  sent_at: number
  acked_at?: number
  retry_count?: number
  last_retry_at?: number
}

type ContactCardDhtAutoRefreshRecord = {
  user_id: string
  display_name?: string
  status: 'success' | 'failed'
  refreshed_at: number
  error?: string
}

type ProcessedMailboxRecord = {
  id: string
  processed_at: number
}

type PersistedState = {
  backupText: string
  contacts: ContactItem[]
  friendRequests: FriendRequestItem[]
  groups: GroupItem[]
  groupInvites: GroupInviteItem[]
  groupSenderKeys?: GroupSenderKeyItem[]
  messages: ChatMessage[]
  outbox: OutboxItem[]
  ratchetSessions?: RatchetSessionItem[]
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
}

type PersistedMeta = {
  backupText: string
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
  schemaVersion: number
}

type SelfSyncPackage = {
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
  signature?: string
  dhtOperationHistory?: string[]
  processedSelfSyncIds?: string[]
  unverifiedIncomingDropCount?: number
  revokedDeviceIncomingDropCount?: number
}

type SelfSyncRequestPackage = {
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

type SelfSyncCachedPackage = {
  sync_id: string
  sequence: number
  created_at: number
  expires_at?: number
  payload: string
}

type SelfSyncRequestRecord = {
  missing_sync_id: string
  requested_at: number
}

type LocalIdentityRecord = {
  id: string
  user_id: string
  display_name: string
  backup_text: string
  updated_at: number
}

const LOCAL_IDENTITIES_KEY = 'lm-talk-local-identities-v1'

const ready = ref(false)
const loggedIn = ref(false)
const log = ref<string[]>([])
const qrTitle = ref('')
const qrDataUrl = ref('')
const qrRawText = ref('')
const activeFingerprintVerificationText = ref('')
const fingerprintScanOpen = ref(false)
const fingerprintScanStatus = ref('')
const fingerprintScanVideo = ref<HTMLVideoElement | null>(null)
let fingerprintScanStream: MediaStream | null = null
let fingerprintScanStopped = true
const route = useRoute()
const router = useRouter()
const authMode = computed(() => route.path === '/register' ? 'register' : route.path === '/import' ? 'import' : 'login')
const currentPage = computed(() => route.path === '/diagnostics' ? 'diagnostics' : route.path === '/contacts' ? 'contacts' : (route.path === '/me' || route.path === '/settings') ? 'settings' : 'chat')
type ToastKind = 'success' | 'error' | 'warning' | 'info'
type ToastItem = { id: string; kind: ToastKind; text: string }
type ConfirmDialogState = {
  open: boolean
  title: string
  message: string
  danger: boolean
  resolve?: (value: boolean) => void
}
const toasts = ref<ToastItem[]>([])
const alertDialog = ref({ open: false, title: '', message: '', kind: 'info' as ToastKind })
const confirmDialog = ref<ConfirmDialogState>({ open: false, title: '', message: '', danger: false })

const MAX_TEXT_MESSAGE_BYTES = 64 * 1024
const MAX_CONTACT_CARD_BYTES = 32 * 1024
const FRIEND_REQUEST_RATE_WINDOW_MS = 10 * 60 * 1000
const FRIEND_REQUEST_RATE_LIMIT = 3
const FRIEND_REQUEST_LONG_RATE_WINDOW_MS = 24 * 60 * 60 * 1000
const FRIEND_REQUEST_LONG_RATE_LIMIT = 5
const MAILBOX_DEDUPE_MAX_RECORDS = 1000
const DHT_OPERATION_HISTORY_MAX_RECORDS = 8
const DHT_OPERATION_HISTORY_IMPORT_MAX_RECORDS = 32
const DHT_OPERATION_HISTORY_ITEM_MAX_CHARS = 240
const MAILBOX_DEDUPE_RETENTION_MS = 30 * 24 * 60 * 60 * 1000
const MAX_SIGNAL_BYTES = 256 * 1024
const MAX_FILE_BYTES = 16 * 1024 * 1024
const MAX_RTC_TEXT_BYTES = MAX_FILE_BYTES * 3
const MAX_OUTBOX_RETRY_COUNT = 5
const NODE_FETCH_TIMEOUT_MS = 10_000

function nodeFetchTimeoutMs(): number {
  const override = typeof window !== 'undefined' ? Number((window as any).nodeFetchTimeoutMsForTests) : 0
  return Number.isFinite(override) && override > 0 ? override : NODE_FETCH_TIMEOUT_MS
}
const GROUP_EVENT_PAYLOAD_PREFIX = 'lm-group-event-message-v1:'
const GROUP_SENDER_KEY_PAYLOAD_PREFIX = 'lm-group-sender-key-message-v1:'

const passphrase = ref('')
const newIdentityPassphrase = ref('')
const backupText = ref('')
const identity = ref<(IdentityOutput | RestoreOutput) | null>(null)
const displayName = ref('Me')
const localIdentities = ref<LocalIdentityRecord[]>([])
const selectedLocalIdentityId = ref('')
const lastRegisteredIdentity = ref<LocalIdentityRecord | null>(null)
const myContactCardText = ref('')
const myDeviceCertJson = ref('')
const myDeviceBackupText = ref('')
const myDeviceId = ref('')
const revokeDeviceId = ref('')
const revokeReason = ref('')
const deviceRevokeText = ref('')
const incomingDeviceRevokeText = ref('')
const blockReason = ref('')
const safetyPolicy = ref<SafetyPolicy>({
  enableTextFilter: true,
  textFilterLevel: 'Standard',
  warnExternalLinks: true,
  warnExecutableFiles: true,
  dropFilteredIncoming: false,
  requireVerifiedContactsForSend: false,
  requireVerifiedContactsForReceive: false,
  requireSealedPerDeviceSlotsForSend: false,
  requireSealedPerDeviceSlotsForReceive: false,
})

const contacts = ref<ContactItem[]>([])
const friendRequests = ref<FriendRequestItem[]>([])
const friendRequestRateRecords = ref<FriendRequestRateRecord[]>([])
const unverifiedIncomingDropCount = ref(0)
const lastUnverifiedIncomingDropAt = ref<number | null>(null)
const lastUnverifiedIncomingDropFrom = ref('')
const revokedDeviceIncomingDropCount = ref(0)
const lastRevokedDeviceIncomingDropAt = ref<number | null>(null)
const lastRevokedDeviceIncomingDropFrom = ref('')
const perDeviceEnvelopeSentCount = ref(0)
const perDeviceEnvelopeReceivedCount = ref(0)
const perDeviceEnvelopeDropCount = ref(0)
const lastPerDeviceEnvelopeAt = ref<number | null>(null)
const lastPerDeviceEnvelopeDropAt = ref<number | null>(null)
const lastPerDeviceEnvelopeDropReason = ref('')
const contactCardUpdateFanoutCount = ref(0)
const contactCardUpdateFanoutSkipCount = ref(0)
const lastContactCardUpdateFanoutAt = ref<number | null>(null)
const contactCardDhtAutoRefreshCount = ref(0)
const lastContactCardDhtAutoRefreshAt = ref<number | null>(null)
const lastContactCardDhtAutoRefreshError = ref('')
const contactCardDhtAutoRefreshHistory = ref<ContactCardDhtAutoRefreshRecord[]>([])
const groups = ref<GroupItem[]>([])
const groupInvites = ref<GroupInviteItem[]>([])
const groupSenderKeys = ref<GroupSenderKeyItem[]>([])
const messages = ref<ChatMessage[]>([])
const outbox = ref<OutboxItem[]>([])
const ratchetSessions = ref<RatchetSessionItem[]>([])
const processedMailboxIds = ref<ProcessedMailboxRecord[]>([])
const mailboxFailedItems = ref<MailboxFailedItem[]>([])
const CONTACT_CARD_UPDATE_ACK_STALE_MS = 24 * 60 * 60 * 1000
const CONTACT_CARD_DHT_FRESH_MS = 7 * 24 * 60 * 60 * 1000
const contactCardUpdateFanoutRecords = ref<ContactCardUpdateFanoutRecord[]>([])
let outboxRetryTimer: number | undefined
let lastDeliveryError = ''
const runtimeStatusText = ref('尚未检查')
const inAppRuntimePolicyText = computed(() => {
  const visibility = document.visibilityState === 'visible' ? '前台可见' : '后台可能被浏览器暂停'
  const mailbox = autoMailboxTake.value ? '自动收取已开启，切回前台最多 30 秒触发一次' : '自动收取已关闭，需要手动同步'
  return `${visibility}；${mailbox}；只使用页面内提示、红点和日志，不请求系统通知权限。`
})
const activePeerId = ref('')
const activeGroupId = ref('')

const addContactText = ref('')
const friendRequestText = ref('')
const friendResponseText = ref('')
const incomingFriendRequestText = ref('')
const incomingFriendResponseText = ref('')
const inboundEnvelopeText = ref('')
const composerText = ref('')
const newGroupName = ref('')
const selectedGroupMembers = ref<string[]>([])
const groupFanoutJson = ref('')
const dataBackupText = ref('')
const lastFullDataBackupAt = ref<number | null>(null)
const lastSelfMailboxBackupPushedAt = ref<number | null>(null)
const lastSelfMailboxBackupReceivedAt = ref<number | null>(null)
const lastSelfMailboxBackupMergedAt = ref<number | null>(null)
const selfMailboxBackupStatusText = ref('自 Mailbox 备份：尚未运行')
const selfSyncStatusText = ref('自同步：尚未运行')
const processedSelfSyncIds = ref<string[]>([])
const processedSelfSyncRequestIds = ref<string[]>([])
const selfSyncMissingRequestRecords = ref<SelfSyncRequestRecord[]>([])
const selfSyncRequestSentCount = ref(0)
const selfSyncRequestHitCount = ref(0)
const selfSyncRequestMissCount = ref(0)
const selfSyncRecentPackages = ref<SelfSyncCachedPackage[]>([])
const lastSelfSyncPushedAt = ref<number | null>(null)
const lastSelfSyncMergedAt = ref<number | null>(null)
const lastSelfSyncSequenceSent = ref(0)
const lastSelfSyncSequenceMerged = ref(0)
const selfSyncGapCount = ref(0)
const lastSelfSyncGapAt = ref<number | null>(null)
const lastSelfSyncMissingPreviousId = ref('')
const lastSelfSyncReceiptStatesSent = ref(0)
const lastSelfSyncReceiptStatesMerged = ref(0)
const totalSelfSyncReceiptStatesMerged = ref(0)
const lastSelfSyncOutboxSummary = ref<OutboxSyncSummary | null>(null)
const selfMailboxBackupMergePending = computed(() => {
  const receivedAt = lastSelfMailboxBackupReceivedAt.value ?? 0
  const mergedAt = lastSelfMailboxBackupMergedAt.value ?? 0
  return receivedAt > 0 && receivedAt > mergedAt
})
const selfMailboxBackupMergeStatusText = computed(() => {
  if (!lastSelfMailboxBackupReceivedAt.value) return '尚未收到自己的 Mailbox 备份'
  return selfMailboxBackupMergePending.value
    ? '收到自己的 Mailbox 备份，尚未合并'
    : '自己的 Mailbox 备份已合并'
})
const fullDataBackupFreshnessLevel = computed<'ok' | 'warning' | 'danger'>(() => {
  const at = lastFullDataBackupAt.value
  if (!at) return 'danger'
  const ageDays = (Date.now() - at) / 86_400_000
  return ageDays > 30 ? 'danger' : ageDays > 7 ? 'warning' : 'ok'
})
let fullDataBackupFreshnessWarnedThisSession = false
const fullDataBackupFreshnessText = computed(() => {
  const at = lastFullDataBackupAt.value
  if (!at) return '尚未生成完整数据备份'
  const ageDays = Math.max(0, Math.floor((Date.now() - at) / 86_400_000))
  return ageDays === 0 ? '完整数据备份：今天已生成' : `完整数据备份：${ageDays} 天前生成`
})
const groupInviteText = ref('')
const incomingGroupInviteText = ref('')
const groupEventText = ref('')
const groupEventFanoutJson = ref('')
const incomingGroupEventText = ref('')
const groupRenameText = ref('')
const groupEventActorUserId = ref('')
const groupSenderDistributionText = ref('')
const groupSenderDistributionFanoutJson = ref('')
const groupSenderEnvelopeText = ref('')
const groupSenderPlainText = ref('')

const peerAddressesText = ref('/ip4/127.0.0.1/tcp/4001')
const peerMailboxKey = ref('')
const discoveredMailboxHintUrl = ref('')
const peerAnnounceText = ref('')
const peerAnnounceInspectPublicKey = ref('')
const peerAnnounceInfoText = ref('')
const publicPeerId = ref('')
const publicPeerAddressesText = ref('/dns4/example.com/tcp/443/wss')
const publicPeerCapabilities = ref<string[]>(['bootstrap', 'dht'])
const publicPeerAnnounceText = ref('')
const publicPeerAnnounceInspectPublicKey = ref('')
const publicPeerAnnounceInfoText = ref('')
const mailboxKind = ref('direct-envelope')
const mailboxCiphertext = ref('')
const mailboxMessageText = ref('')
const mailboxMessageInspectPublicKey = ref('')
const mailboxMessageInfoText = ref('')
const nodeControlUrl = ref('http://127.0.0.1:8787')
const nodeEnabled = ref(false)
const autoMailboxTake = ref(true)
const autoReadReceipts = ref(false)
const autoPublishPreKey = ref(true)
const autoNodeSync = ref(false)
const autoSelfMailboxSync = ref(false)
let nodeSyncTimer: number | undefined
let selfSyncTimer: number | undefined
let lastVisibilityMailboxTakeAt = 0
const nodeControlStatus = ref('未连接')
const nodeHealthSummaryText = ref('节点健康：尚未检查')
const nodeStateDbSecurityText = ref('state_db：尚未查询')
const nodeStateDbSecurityLevel = ref<'ok' | 'warning' | 'danger'>('warning')
const nodeStateFileSecurityText = ref('state_file：未配置')
const nodeStateFileSecurityLevel = ref<'ok' | 'warning' | 'danger'>('ok')
const nodePeerHealthStatusText = ref('DHT peer：尚未检查')
const nodePeerHealthRiskLevel = ref<'ok' | 'warning' | 'danger'>('ok')
const nodePeerHealthPeers = ref<Array<{ url: string; consecutive_failures: number; failures: number; quarantined: boolean; last_error?: string }>>([])
const nodeClosestTarget = ref('')
const nodeDhtFindValueKey = ref('')
const nodeDhtKeyKind = ref<'prekey' | 'mailbox-hint' | 'public-peer' | 'contact-card'>('prekey')
const nodeDhtKeyValue = ref('')
const nodeDhtFindValueStatusText = ref('DHT 查找：尚未运行')
const nodeClosestInfoText = ref('')
const nodeRoutingRefreshStatusText = ref('DHT 路由刷新：尚未运行')
const nodeDhtReplicationStatusText = ref('DHT 复制：尚未运行')
const nodeDhtMaintenanceStatusText = ref('DHT 维护：尚未运行')
const nodeDhtOperationHistory = ref<string[]>([])
const nodeDhtOperationHistoryImportText = ref('')
const nodeDhtOperationHistoryImportStatus = ref('DHT 历史导入：尚未导入')
const nodeMailboxTakeUserId = ref('')
const nodeMailboxTakeInfoText = ref('')
const syncTriggerPolicyText = computed(() => {
  const parts = ['立即同步优先执行']
  if (autoPublishPreKey.value) parts.push('登录/手动同步先检查 PreKey')
  if (autoMailboxTake.value) parts.push('登录/切回前台收取 Mailbox，前台触发 30 秒节流')
  else parts.push('Mailbox 自动收取关闭')
  parts.push(autoReadReceipts.value ? '当前会话可见时自动发送已读回执' : '已读回执关闭')
  parts.push('Outbox 每 30 秒重试到期项')
  if (autoNodeSync.value) parts.push('节点快照每 60 秒同步')
  if (autoSelfMailboxSync.value) parts.push('手动/定时发送轻量自同步包')
  return parts.join('；')
})
const mailboxInboxStatus = ref('尚未同步')
const mailboxQuotaStatusText = ref('Mailbox 容量：尚未查询')
const mailboxQuotaPressureLevel = ref<'ok' | 'warning' | 'danger'>('ok')
const mailboxInboxErrorText = ref('')
const mailboxFailureSummaryText = ref('')
const mailboxDedupeCount = computed(() => processedMailboxIds.value.length)
const mailboxFailedCount = computed(() => mailboxFailedItems.value.length)
const mailboxDedupeStatusText = computed(() => {
  const records = processedMailboxIds.value
  const range = records.length
    ? `，最新 ${formatDateTime(records[0].processed_at)}，最旧 ${formatDateTime(records[records.length - 1].processed_at)}`
    : ''
  return `本地去重记录 ${mailboxDedupeCount.value}/${MAILBOX_DEDUPE_MAX_RECORDS}，保留 30 天${range}；失败队列 ${mailboxFailedCount.value}`
})
const nodePreKeyUserId = ref('')
const nodePreKeyStatusText = ref('')
const prekeyStatusSummary = ref('尚未发布 PreKey')
const prekeyAutoStateText = ref('尚未检查')
const prekeyAutoErrorText = ref('')
const nodeSyncPeerUrl = ref('http://127.0.0.1:8788')
const nodeSyncSnapshotText = ref('')
const nodeSyncStatusText = ref('')
const lastNodeSnapshotSyncAt = ref<number | null>(null)
const nodeSnapshotSyncFreshnessLevel = computed<'ok' | 'warning' | 'danger'>(() => {
  if (!autoNodeSync.value) return 'ok'
  const at = lastNodeSnapshotSyncAt.value
  if (!at) return 'warning'
  const ageMinutes = (Date.now() - at) / 60_000
  return ageMinutes > 180 ? 'danger' : ageMinutes > 70 ? 'warning' : 'ok'
})
const nodeSnapshotSyncFreshnessText = computed(() => {
  if (!autoNodeSync.value) return '节点快照自动同步未开启'
  const at = lastNodeSnapshotSyncAt.value
  if (!at) return '节点快照自动同步尚未成功'
  const ageMinutes = Math.max(0, Math.floor((Date.now() - at) / 60_000))
  if (ageMinutes < 1) return '节点快照刚刚同步成功'
  if (ageMinutes < 60) return `节点快照 ${ageMinutes} 分钟前同步成功`
  return `节点快照 ${Math.floor(ageMinutes / 60)} 小时前同步成功`
})
const syncRecoveryStatusText = ref('尚未恢复')
const syncRecoveryHistory = ref<string[]>([])
const syncFailureSummaryText = computed(() => {
  const parts: string[] = []
  if (prekeyAutoErrorText.value) parts.push(`PreKey：${prekeyAutoErrorText.value}`)
  if (mailboxInboxErrorText.value) parts.push(`Mailbox：${mailboxInboxErrorText.value.split('\n')[0]}`)
  const failedOutbox = outbox.value.filter((item) => item.status === 'failed')
  if (failedOutbox.length > 0) parts.push(`Outbox：失败 ${failedOutbox.length} 条`)
  if (/failed|失败/i.test(nodeSyncStatusText.value)) parts.push(`节点快照：${nodeSyncStatusText.value}`)
  if (selfSyncGapCount.value > 0) parts.push(`轻量自同步：缺口 ${selfSyncGapCount.value} 次`)
  return parts.length ? parts.join('；') : '暂无同步失败'
})
const storageEstimateText = ref('尚未估算')
const webVersionText = `v${__APP_VERSION__} (${__BUILD_REF__})`
const selectedFile = ref<File | null>(null)
const filePackageText = ref('')
const incomingFilePackageText = ref('')
const pendingFilePackageText = ref('')
const pendingFileMeta = ref('')
const filePackageInfoText = ref('')
const receivedFileName = ref('')
const receivedFileUrl = ref('')
const receivedFileMeta = ref('')
const receivedFileMime = ref('')
const receivedFilePreviewKind = ref('')
const receivedFileMessageId = ref('')
const rtcFileStatus = ref('未发送文件')
const fileTransferPhase = ref('待选择')
const fileProgressText = ref('')
const ratchetStateText = ref('')
const ratchetPeerStateText = ref('')
const ratchetHeaderText = ref('')
const ratchetKeyText = ref('')
const ratchetInfoText = ref('')
const ratchetRemoteDhPublicKey = ref('')
const prekeySignedId = ref(1)
const prekeyOneTimeCount = ref(10)
const prekeyBundleText = ref('')
const prekeyPrivateBundleJson = ref('')
const prekeySignedOneTimeRecordTexts = ref<string[]>([])
const prekeyInfoText = ref('')
const x3dhInitialMessageJson = ref('')
const x3dhSharedSecretText = ref('')
const selectedOneTimePreKeyId = ref<number | null>(null)
const selectedSignedOneTimePreKeyRecordText = ref('')
const secureSessionOfferText = ref('')
const secureSessionResponseText = ref('')
const incomingSecureSessionText = ref('')
const secureSessionStatusText = ref('')
const ratchetLocalDhKeyPairJson = ref('')
const ratchetRemoteDhPublicKeyForInit = ref('')
const ratchetInitRole = ref<'Initiator' | 'Responder'>('Initiator')
const ratchetEnvelopeText = ref('')
const ratchetPlainText = ref('')

const rtcStatus = ref('未连接')
const localSignalText = ref('')
const remoteSignalText = ref('')
let pc: RTCPeerConnection | null = null
let dc: RTCDataChannel | null = null

const normalized = computed(() => ready.value ? normalize_passphrase(passphrase.value) : '')
const activeContact = computed(() => contacts.value.find((c) => c.user_id === activePeerId.value) ?? null)
const activeGroup = computed(() => groups.value.find((g) => g.group_id === activeGroupId.value) ?? null)
const activeRatchetSession = computed(() => activeContact.value ? ratchetSessionFor(activeContact.value.user_id) : null)
const activeRatchetStatusText = computed(() => {
  if (!activeContact.value) return ''
  if (activeContact.value.state !== 'Friend') return '未建链'
  return activeRatchetSession.value ? '已建链' : '未建链'
})

function contactSealedSlotStatusText(contact: ContactItem): string {
  const activeDeviceIds = contactActiveDeviceIds(contact)
  if (activeDeviceIds.length === 0) return '联系人没有活跃设备证书，无法使用 sealed slot 投递。'
  const certs = contact.device_certs ?? []
  const sealed = activeDeviceIds.filter((deviceId) => certs.find((cert) => cert.device_id === deviceId)?.device_box_public_key).length
  if (sealed === activeDeviceIds.length) return `sealed slot 就绪：${sealed}/${activeDeviceIds.length} 个活跃设备支持设备级加密。`
  return `兼容模式风险：${activeDeviceIds.length - sealed}/${activeDeviceIds.length} 个活跃设备缺少 device_box_public_key，将使用 placeholder/fallback；可在设置中开启“仅发送到支持 sealed slot 的设备”阻止降级。`
}


function activeContactSealedSlotRiskFor(contact: ContactItem): 'ok' | 'high' {
  const activeDeviceIds = contactActiveDeviceIds(contact)
  if (activeDeviceIds.length === 0) return 'high'
  const certs = contact.device_certs ?? []
  return activeDeviceIds.some((deviceId) => !certs.find((cert) => cert.device_id === deviceId)?.device_box_public_key) ? 'high' : 'ok'
}

const activeContactSealedSlotStatusText = computed(() => activeContact.value ? contactSealedSlotStatusText(activeContact.value) : '')
const activeContactSealedSlotRiskLevel = computed(() => activeContact.value ? activeContactSealedSlotRiskFor(activeContact.value) : 'none')

function strictE2eeSendRiskReasons(contact: ContactItem): string[] {
  const reasons: string[] = []
  if (!strictE2eePolicyEnabled.value) reasons.push('严格 E2EE 策略未完全启用')
  if (!safetyPolicy.value.requireVerifiedContactsForSend && !contact.fingerprint_verified_at) reasons.push('未强制发送前核验联系人指纹')
  if (!safetyPolicy.value.requireSealedPerDeviceSlotsForSend && activeContactSealedSlotRiskFor(contact) === 'high') reasons.push(contactSealedSlotStatusText(contact))
  if (contactCardDhtDiscoveryIsStale(contact)) reasons.push('ContactCard DHT 发现未刷新，可能缺少最新设备/撤销状态')
  const ack = contactCardUpdateAckStatusFor(contact)
  if (ack.pending) reasons.push(`设备证书更新仍有 ${ack.pending} 条待确认`)
  return Array.from(new Set(reasons))
}

function contactStrictE2eeSendRiskText(contact: ContactItem | null): string {
  if (!contact || contact.state !== 'Friend') return ''
  const reasons = strictE2eeSendRiskReasons(contact)
  return reasons.length ? `发送前风险提示：${reasons.join('；')}` : ''
}

const activeStrictE2eeSendRiskText = computed(() => contactStrictE2eeSendRiskText(activeContact.value))

const activeSecureSessionOutboxCount = computed(() => {
  const peerId = activeContact.value?.user_id
  if (!peerId) return 0
  return outbox.value.filter((item) =>
    item.peer_user_id === peerId
    && item.kind === 'other'
    && item.status !== 'sent'
    && /"lm-secure-session-(offer|response)-v1"/.test(item.envelope_json || '')
  ).length
})
const activeMessages = computed(() => activeGroup.value
  ? messages.value.filter((m) => m.group_id === activeGroup.value?.group_id)
  : messages.value.filter((m) => m.peer_user_id === activePeerId.value)
)
const friendContacts = computed(() => contacts.value.filter((c) => c.state === 'Friend'))
const verifiedFriendContactCount = computed(() => friendContacts.value.filter((c) => c.fingerprint_verified_at).length)
const unverifiedFriendContactCount = computed(() => Math.max(0, friendContacts.value.length - verifiedFriendContactCount.value))


function contactCardUpdateAckStatusFor(contact: ContactItem): { pending: number; stale: number; acked: number } {
  const records = contactCardUpdateFanoutRecords.value.filter((record) => record.peer_user_id === contact.user_id)
  return {
    pending: records.filter((record) => record.status !== 'acked').length,
    stale: records.filter((record) => contactCardUpdateRecordIsStale(record)).length,
    acked: records.filter((record) => record.status === 'acked').length,
  }
}


function contactCardDhtDiscoveryIsStale(contact: ContactItem, now = Date.now()): boolean {
  if (contact.state !== 'Friend') return false
  if (!contact.last_contact_card_dht_found_at) return true
  return now - contact.last_contact_card_dht_found_at > CONTACT_CARD_DHT_FRESH_MS
}

function contactStrictE2eeStatusText(contact: ContactItem): string {
  if (contact.state !== 'Friend') return ''
  const issues: string[] = []
  if (!contact.fingerprint_verified_at) issues.push('指纹未核验')
  if (activeContactSealedSlotRiskFor(contact) === 'high') issues.push('sealed slot 风险')
  if (contactCardDhtDiscoveryIsStale(contact)) issues.push('ContactCard DHT 未刷新')
  const ack = contactCardUpdateAckStatusFor(contact)
  if (ack.pending) issues.push(`设备更新待确认 ${ack.pending}`)
  if (ack.stale) issues.push(`过期 ${ack.stale}`)
  return issues.length ? `严格 E2EE 阻塞：${issues.join('，')}` : '严格 E2EE 就绪'
}

function contactStrictE2eeRiskLevel(contact: ContactItem): 'ok' | 'high' | 'none' {
  if (contact.state !== 'Friend') return 'none'
  return contactStrictE2eeStatusText(contact) === '严格 E2EE 就绪' ? 'ok' : 'high'
}

const sealedSlotCoverageSummary = computed(() => {
  const friends = friendContacts.value
  const totalDevices = friends.reduce((sum, contact) => sum + contactActiveDeviceIds(contact).length, 0)
  const sealedDevices = friends.reduce((sum, contact) => {
    const certs = contact.device_certs ?? []
    return sum + contactActiveDeviceIds(contact).filter((deviceId) => certs.find((cert) => cert.device_id === deviceId)?.device_box_public_key).length
  }, 0)
  const riskyContacts = friends.filter((contact) => activeContactSealedSlotRiskFor(contact) === 'high')
  return {
    friend_count: friends.length,
    total_devices: totalDevices,
    sealed_devices: sealedDevices,
    risky_contacts: riskyContacts.length,
    text: totalDevices === 0
      ? '尚无好友活跃设备证书。'
      : `sealed slot 覆盖 ${sealedDevices}/${totalDevices} 个活跃设备；风险联系人 ${riskyContacts.length}/${friends.length}`,
  }
})

const sealedSlotRiskContacts = computed(() => friendContacts.value
  .filter((contact) => activeContactSealedSlotRiskFor(contact) === 'high')
  .slice(0, 8)
  .map((contact) => ({
    user_id: contact.user_id,
    display_name: contact.display_name || contact.user_id,
    status: contactSealedSlotStatusText(contact),
  }))
)

const strictE2eeReadiness = computed(() => {
  const unverified = friendContacts.value.filter((contact) => !contact.fingerprint_verified_at)
  const sealedRisks = friendContacts.value.filter((contact) => activeContactSealedSlotRiskFor(contact) === 'high')
  const staleContactCardDht = friendContacts.value.filter((contact) => contactCardDhtDiscoveryIsStale(contact))
  const pendingUpdateAcks = contactCardUpdatePendingAckCount.value
  const blockers = [
    ...(unverified.length ? [`未核验指纹联系人 ${unverified.length} 个`] : []),
    ...(sealedRisks.length ? [`sealed slot 风险联系人 ${sealedRisks.length} 个`] : []),
    ...(staleContactCardDht.length ? [`ContactCard DHT 未刷新 ${staleContactCardDht.length} 个`] : []),
    ...(pendingUpdateAcks ? [`设备证书更新待确认 ${pendingUpdateAcks} 条`] : []),
  ]
  return {
    ready: blockers.length === 0,
    unverified_contacts: unverified.length,
    sealed_slot_risk_contacts: sealedRisks.length,
    stale_contact_card_dht_contacts: staleContactCardDht.length,
    pending_contact_update_acks: pendingUpdateAcks,
    text: blockers.length === 0 ? '严格 E2EE 启用前检查通过。' : `严格 E2EE 启用前仍有：${blockers.join('；')}`,
  }
})

const strictE2eeReadinessIssues = computed(() => [
  ...friendContacts.value
    .filter((contact) => !contact.fingerprint_verified_at)
    .slice(0, 8)
    .map((contact) => ({
      user_id: contact.user_id,
      display_name: contact.display_name || contact.user_id,
      issue: '身份指纹未核验',
      issue_kind: 'fingerprint',
    })),
  ...sealedSlotRiskContacts.value.map((contact: any) => ({
    user_id: contact.user_id,
    display_name: contact.display_name,
    issue: contact.status,
    issue_kind: 'sealed-slot',
  })),
  ...friendContacts.value
    .filter((contact) => contactCardDhtDiscoveryIsStale(contact))
    .slice(0, 8)
    .map((contact) => ({
      user_id: contact.user_id,
      display_name: contact.display_name || contact.user_id,
      issue: contact.last_contact_card_dht_found_at ? 'ContactCard DHT 发现已过期' : 'ContactCard DHT 尚未发现',
      issue_kind: 'contact-card-dht',
    })),
  ...contactCardUpdateFanoutRecords.value
    .filter((record) => record.status !== 'acked')
    .slice(0, 8)
    .map((record) => ({
      user_id: record.peer_user_id,
      display_name: contacts.value.find((contact) => contact.user_id === record.peer_user_id)?.display_name || record.peer_user_id,
      issue: contactCardUpdateRecordIsStale(record) ? '设备证书更新确认已过期，等待重试' : '设备证书更新等待对方确认合并',
      issue_kind: 'contact-update-ack',
    })),
].slice(0, 12))


async function repairStrictE2eeBlockers() {
  await runAsync('批量处理严格 E2EE 阻塞联系人', async () => {
    const stale = contactCardUpdateFanoutRecords.value.filter((record) => contactCardUpdateRecordIsStale(record))
    let retried = 0
    for (const record of stale) {
      const contact = contacts.value.find((item) => item.user_id === record.peer_user_id)
      if (!contact) continue
      record.retry_count = Number(record.retry_count ?? 0) + 1
      record.last_retry_at = Date.now()
      await sendContactCardUpdateToContact(contact)
      retried += 1
    }

    const sealedRiskContacts = friendContacts.value
      .filter((contact) => activeContactSealedSlotRiskFor(contact) === 'high' || contactCardDhtDiscoveryIsStale(contact))
      .slice(0, 5)
    let discovered = 0
    for (const contact of sealedRiskContacts) {
      selectContact(contact.user_id)
      try {
        await discoverActiveContactDht()
        discovered += 1
      } catch (error) {
        appendLog(`严格 E2EE 批量处理：${contact.display_name || contact.user_id} DHT 发现失败：${userFacingError(error)}`)
      }
    }

    const unverified = friendContacts.value.filter((contact) => !contact.fingerprint_verified_at)
    if (unverified.length) {
      selectContact(unverified[0].user_id)
      goContactsPage()
      appendLog(`严格 E2EE 批量处理：仍有 ${unverified.length} 个联系人需要人工核验指纹，已打开第一个联系人`)
      try { await showActiveContactFingerprintQr() } catch {}
    }

    appendLog(`严格 E2EE 批量处理完成：重试 ACK ${retried}/${stale.length}，DHT 发现 ${discovered}/${sealedRiskContacts.length}`)
    persist()
  })
}

async function openStrictE2eeReadinessIssue(issue: { user_id: string; issue_kind?: string }) {
  const contact = contacts.value.find((item) => item.user_id === issue.user_id)
  if (!contact) return
  selectContact(contact.user_id)
  goContactsPage()
  await nextTick()
  if (issue.issue_kind === 'fingerprint') {
    appendLog(`严格 E2EE 预检：请核验 ${contact.display_name || contact.user_id} 的身份指纹`)
    try {
      await showActiveContactFingerprintQr()
    } catch (error) {
      appendLog(`严格 E2EE 预检：打开指纹核验码失败：${userFacingError(error)}`)
    }
  } else if (issue.issue_kind === 'sealed-slot') {
    appendLog(`严格 E2EE 预检：正在为 ${contact.display_name || contact.user_id} 重新发现 DHT 记录以刷新设备/投递线索`)
    await discoverActiveContactDht()
  } else if (issue.issue_kind === 'contact-card-dht') {
    appendLog(`严格 E2EE 预检：正在查找 ${contact.display_name || contact.user_id} 的 ContactCard DHT 记录`)
    await findActiveContactContactCard()
  } else if (issue.issue_kind === 'contact-update-ack') {
    appendLog(`严格 E2EE 预检：正在重新向 ${contact.display_name || contact.user_id} 分发设备证书更新`)
    if (!myContactCardText.value.trim()) refreshMyContactCard()
    await sendContactCardUpdateToContact(contact)
    persist()
  }
}

const visibleFriendRequests = computed(() => friendRequests.value.filter((req) => !req.quarantined))
const quarantinedFriendRequests = computed(() => friendRequests.value.filter((req) => req.quarantined))
const friendRequestRateSummaryText = computed(() => {
  const parts: string[] = []
  const counts = new Map<string, number>()
  for (const req of friendRequests.value) counts.set(req.from_user_id, (counts.get(req.from_user_id) ?? 0) + 1)
  const hot = [...counts.entries()].filter(([, count]) => count >= FRIEND_REQUEST_RATE_LIMIT)
  if (hot.length > 0) parts.push(`未处理：${hot.map(([userId, count]) => `${userId} ${count} 条`).join('，')}`)
  const now = Date.now()
  const longHot = friendRequestRateRecords.value
    .filter((record) => now - record.first_seen_at <= FRIEND_REQUEST_LONG_RATE_WINDOW_MS && record.count >= FRIEND_REQUEST_LONG_RATE_LIMIT)
  if (longHot.length > 0) parts.push(`24小时：${longHot.map((record) => `${record.from_user_id} ${record.count} 条`).join('，')}`)
  return parts.join('；')
})
const activeGroupMembers = computed(() => activeGroup.value
  ? activeGroup.value.member_user_ids.map((id) => contacts.value.find((c) => c.user_id === id)).filter(Boolean) as ContactItem[]
  : []
)
const activeGroupWarningText = computed(() => {
  if (!activeGroup.value || !identity.value) return ''
  const missing = activeGroup.value.member_user_ids.filter((id) => !contacts.value.some((c) => c.user_id === id))
  const notFriend = activeGroup.value.member_user_ids.filter((id) => {
    const contact = contacts.value.find((c) => c.user_id === id)
    return contact && contact.state !== 'Friend'
  })
  const blocked = activeGroupMembers.value.filter((c) => c.state === 'Blocked').map((c) => c.display_name || c.user_id)
  const warnings = []
  if (activeGroup.value.removed_self_at) warnings.push(`你已被移出群聊：${formatDateTime(activeGroup.value.removed_self_at)}`)
  if (missing.length > 0) warnings.push(`缺少联系人：${missing.join(', ')}`)
  if (notFriend.length > 0) warnings.push(`非好友成员：${notFriend.join(', ')}`)
  if (blocked.length > 0) warnings.push(`已拉黑成员：${blocked.join(', ')}`)
  if (!getGroupSenderKey(activeGroup.value.group_id, identity.value.user_id)) warnings.push('未启用本群 Sender Key，将回退逐个加密')
  if (activeGroup.value.last_sender_key_error) warnings.push(`Sender Key 异常：${activeGroup.value.last_sender_key_error}`)
  return warnings.join('；')
})

function groupStrictE2eeRiskReasons(group: GroupItem | null): string[] {
  if (!group) return []
  const reasons: string[] = []
  if (!strictE2eePolicyEnabled.value) reasons.push('严格 E2EE 策略未完全启用')
  for (const memberId of group.member_user_ids) {
    const contact = contacts.value.find((c) => c.user_id === memberId)
    if (!contact) { reasons.push(`缺少群成员联系人：${memberId}`); continue }
    if (contact.state !== 'Friend') reasons.push(`群成员不是好友：${contact.display_name || contact.user_id}`)
    if (!contact.fingerprint_verified_at) reasons.push(`群成员指纹未核验：${contact.display_name || contact.user_id}`)
    if (activeContactSealedSlotRiskFor(contact) === 'high') reasons.push(`群成员 sealed slot 风险：${contact.display_name || contact.user_id}`)
    if (contactCardDhtDiscoveryIsStale(contact)) reasons.push(`群成员 ContactCard DHT 未刷新：${contact.display_name || contact.user_id}`)
    const ack = contactCardUpdateAckStatusFor(contact)
    if (ack.pending) reasons.push(`群成员设备更新待确认：${contact.display_name || contact.user_id} ${ack.pending} 条`)
  }
  return Array.from(new Set(reasons))
}

function groupStrictE2eeRiskTextFor(group: GroupItem | null): string {
  const reasons = groupStrictE2eeRiskReasons(group)
  return reasons.length ? `群聊严格 E2EE 风险：${reasons.join('；')}` : ''
}

const activeGroupStrictE2eeRiskText = computed(() => groupStrictE2eeRiskTextFor(activeGroup.value))

function buildCreateGroupStrictE2eeRiskText(): string {
  const members = [...new Set(selectedGroupMembers.value)].filter(Boolean)
  if (members.length === 0) return ''
  const reasons: string[] = []
  for (const memberId of members) {
    const contact = contacts.value.find((c) => c.user_id === memberId)
    if (!contact) { reasons.push(`缺少联系人：${memberId}`); continue }
    if (contact.state !== 'Friend') reasons.push(`非好友成员：${contact.display_name || contact.user_id}`)
    if (!contact.fingerprint_verified_at) reasons.push(`指纹未核验：${contact.display_name || contact.user_id}`)
    if (activeContactSealedSlotRiskFor(contact) === 'high') reasons.push(`sealed slot 风险：${contact.display_name || contact.user_id}`)
    if (contactCardDhtDiscoveryIsStale(contact)) reasons.push(`ContactCard DHT 未刷新：${contact.display_name || contact.user_id}`)
    const ack = contactCardUpdateAckStatusFor(contact)
    if (ack.pending) reasons.push(`设备更新待确认：${contact.display_name || contact.user_id} ${ack.pending} 条`)
  }
  return reasons.length ? `新建群聊严格 E2EE 预检：${Array.from(new Set(reasons)).join('；')}` : ''
}

const createGroupStrictE2eeRiskText = computed(() => buildCreateGroupStrictE2eeRiskText())

function groupInviteStrictE2eeRiskText(invite: GroupInviteItem): string {
  const reasons: string[] = []
  if (!strictE2eePolicyEnabled.value) reasons.push('严格 E2EE 策略未完全启用')
  for (const memberId of invite.member_user_ids) {
    if (memberId === identity.value?.user_id) continue
    const contact = contacts.value.find((c) => c.user_id === memberId)
    if (!contact) { reasons.push(`邀请成员不在联系人中：${memberId}`); continue }
    if (contact.state !== 'Friend') reasons.push(`邀请成员不是好友：${contact.display_name || contact.user_id}`)
    if (!contact.fingerprint_verified_at) reasons.push(`邀请成员指纹未核验：${contact.display_name || contact.user_id}`)
    if (activeContactSealedSlotRiskFor(contact) === 'high') reasons.push(`邀请成员 sealed slot 风险：${contact.display_name || contact.user_id}`)
    if (contactCardDhtDiscoveryIsStale(contact)) reasons.push(`邀请成员 ContactCard DHT 未刷新：${contact.display_name || contact.user_id}`)
  }
  return reasons.length ? `群邀请严格 E2EE 风险：${Array.from(new Set(reasons)).join('；')}` : ''
}

const fanoutItems = computed(() => {
  try {
    const parsed = JSON.parse(groupFanoutJson.value || '[]') as Array<{ to_user_id: string; envelope: string }>
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
})

const groupEventFanoutItems = computed(() => {
  try {
    const parsed = JSON.parse(groupEventFanoutJson.value || '[]') as Array<{ to_user_id: string; envelope: string }>
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
})

const groupSenderDistributionFanoutItems = computed(() => {
  try {
    const parsed = JSON.parse(groupSenderDistributionFanoutJson.value || '[]') as Array<{ to_user_id: string; envelope: string }>
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
})

router.afterEach((to) => {
  if (to.path === '/import') backupText.value = ''
})

onMounted(async () => {
  try {
    await init()
    loadLocalIdentityList()
    startOutboxRetryLoop()
    startNodeSyncLoop()
    startSelfSyncLoop()
    document.addEventListener('visibilitychange', () => {
      refreshRuntimeStatus()
      if (document.visibilityState === 'visible' && loggedIn.value && nodeEnabled.value && autoMailboxTake.value) {
        const now = Date.now()
        if (now - lastVisibilityMailboxTakeAt >= 30_000) {
          lastVisibilityMailboxTakeAt = now
          void takeMailboxFromNode()
        }
      }
    })
    window.addEventListener('online', refreshRuntimeStatus)
    window.addEventListener('offline', refreshRuntimeStatus)
    void refreshRuntimeStatus()
    ready.value = true
  } catch (e) {
    appendLog(`WASM 初始化失败：${String(e)}`)
  }
})

function appendLog(line: string) {
  log.value = [`${formatTime(Date.now())} ${line}`, ...log.value].slice(0, 50)
}

function newId(): string {
  const webCrypto = globalThis.crypto
  if (typeof webCrypto?.randomUUID === 'function') return webCrypto.randomUUID()
  if (typeof webCrypto?.getRandomValues === 'function') {
    const bytes = webCrypto.getRandomValues(new Uint8Array(16))
    bytes[6] = (bytes[6] & 0x0f) | 0x40
    bytes[8] = (bytes[8] & 0x3f) | 0x80
    const hex = [...bytes].map((b) => b.toString(16).padStart(2, '0'))
    return `${hex.slice(0, 4).join('')}-${hex.slice(4, 6).join('')}-${hex.slice(6, 8).join('')}-${hex.slice(8, 10).join('')}-${hex.slice(10, 16).join('')}`
  }
  return `id-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`
}

function randomBase64Url(byteLength: number): string {
  const bytes = new Uint8Array(byteLength)
  const webCrypto = globalThis.crypto
  if (typeof webCrypto?.getRandomValues === 'function') webCrypto.getRandomValues(bytes)
  else for (let i = 0; i < bytes.length; i += 1) bytes[i] = Math.floor(Math.random() * 256)
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
}

function perDeviceSlotAad(conversationId: string, senderUserId: string, deviceId: string, createdAt: number): string {
  return JSON.stringify({
    type: 'lm-per-device-envelope-slot-aad-v1',
    conversation_id: conversationId,
    sender_user_id: senderUserId,
    target_device_id: deviceId,
    created_at: createdAt,
  })
}

function fallbackCopyText(value: string) {
  const textarea = document.createElement('textarea')
  textarea.value = value
  textarea.setAttribute('readonly', 'true')
  textarea.style.position = 'fixed'
  textarea.style.left = '-9999px'
  document.body.appendChild(textarea)
  textarea.select()
  document.execCommand('copy')
  document.body.removeChild(textarea)
}

async function writeClipboardText(value: string) {
  if ((navigator.clipboard as Clipboard | undefined)?.writeText) {
    await navigator.clipboard.writeText(value)
    return
  }
  fallbackCopyText(value)
}

function toast(text: string, kind: ToastKind = 'info') {
  const id = newId()
  toasts.value.push({ id, kind, text })
  window.setTimeout(() => {
    toasts.value = toasts.value.filter((item) => item.id !== id)
  }, 2800)
}

async function refreshRuntimeStatus() {
  const online = navigator.onLine ? '在线' : '离线'
  const visibility = document.visibilityState === 'visible' ? '前台' : '后台'
  let battery = ''
  const nav = navigator as Navigator & { getBattery?: () => Promise<{ charging: boolean; level: number }> }
  if (typeof nav.getBattery === 'function') {
    const info = await nav.getBattery().catch(() => null)
    if (info) {
      const percent = Math.round(info.level * 100)
      battery = info.charging ? ` · 电量 ${percent}% 充电中` : percent <= 20 ? ` · 低电量 ${percent}%` : ` · 电量 ${percent}%`
    }
  }
  runtimeStatusText.value = `${online} · ${visibility}${battery}`
}

function showAlert(title: string, message: string, kind: ToastKind = 'info') {
  alertDialog.value = { open: true, title, message, kind }
}

function closeAlert() {
  alertDialog.value.open = false
}

function showConfirm(title: string, message: string, danger = false): Promise<boolean> {
  return new Promise((resolve) => {
    confirmDialog.value = { open: true, title, message, danger, resolve }
  })
}

function closeConfirm(value: boolean) {
  const resolve = confirmDialog.value.resolve
  confirmDialog.value = { open: false, title: '', message: '', danger: false }
  resolve?.(value)
}

function loadLocalIdentityList() {
  try {
    const parsed = JSON.parse(localStorage.getItem(LOCAL_IDENTITIES_KEY) || '[]') as LocalIdentityRecord[]
    localIdentities.value = Array.isArray(parsed) ? parsed.filter((x) => x?.backup_text && x?.user_id) : []
    if (!selectedLocalIdentityId.value && localIdentities.value.length) selectedLocalIdentityId.value = localIdentities.value[0].id
  } catch {
    localIdentities.value = []
  }
}

function saveLocalIdentityList() {
  localStorage.setItem(LOCAL_IDENTITIES_KEY, JSON.stringify(localIdentities.value))
}

function rememberLocalIdentity(userId: string, name: string, backup: string): LocalIdentityRecord {
  const id = userId
  const item: LocalIdentityRecord = {
    id,
    user_id: userId,
    display_name: name || userId,
    backup_text: backup,
    updated_at: Date.now(),
  }
  localIdentities.value = [item, ...localIdentities.value.filter((x) => x.id !== id)]
  selectedLocalIdentityId.value = id
  saveLocalIdentityList()
  return item
}

function selectedLocalIdentity(): LocalIdentityRecord | undefined {
  return localIdentities.value.find((x) => x.id === selectedLocalIdentityId.value) ?? localIdentities.value[0]
}

function userFacingError(e: unknown): string {
  const raw = String(e)
  if (raw.includes('WrongPassphrase')) return '提示词不正确，请重新输入。'
  if (raw.includes('invalid wasm backup')) return '身份文本格式不正确。'
  if (raw.includes('backup user_id mismatch')) return '身份文本校验失败。'
  if (raw.includes('请粘贴身份文本')) return '请粘贴身份文本。'
  if (raw.includes('请输入提示词')) return '请输入提示词。'
  if (raw.includes('AbortError') || raw.includes('同步服务请求超时')) return '同步服务请求超时，请稍后重试或切换同步服务。'
  if (raw.includes('Failed to fetch')) return '无法连接同步服务。请确认 lm_node 已启动；如果在 GitHub Pages 上连接 127.0.0.1 或局域网 IP，请确认 lm_node 已启动、地址可访问，并使用最新 lm_node（已支持浏览器 HTTPS 访问本机/局域网服务所需的权限头）。IPv6 地址请写成 http://[fd00::1234]:8787 这种带方括号的形式。'
  return raw.replace(/^Error:\s*/, '')
}

function run(label: string, fn: () => void) {
  try {
    fn()
    appendLog(`✅ ${label}`)
  } catch (e) {
    const message = userFacingError(e)
    appendLog(`❌ ${label}: ${message}`)
    showAlert(label, message, 'error')
  }
}

function utf8Bytes(value: string): number {
  return new TextEncoder().encode(value).length
}

function ensureUiTextSize(label: string, value: string, max: number) {
  const size = utf8Bytes(value)
  if (size > max) throw new Error(`${label} 过大：${formatBytes(size)} > ${formatBytes(max)}`)
}

function safeJson<T>(value: string): T {
  return JSON.parse(value) as T
}

function base64UrlToString(value: string): string {
  const padded = value.replace(/-/g, '+').replace(/_/g, '/') + '==='.slice((value.length + 3) % 4)
  return atob(padded)
}

function contactCardDeviceCerts(cardText: string): DeviceCertItem[] {
  try {
    const payload = cardText.slice('lm-contact-card-v1:'.length)
    const parsed = JSON.parse(base64UrlToString(payload)) as { device_certs?: DeviceCertItem[] }
    return Array.isArray(parsed.device_certs) ? parsed.device_certs : []
  } catch { return [] }
}


type EncryptedStringV1 = {
  __lm_enc_v1: true
  alg: 'AES-GCM'
  kdf: 'PBKDF2-SHA-256'
  iv: string
  ct: string
}

function isEncryptedString(value: unknown): value is EncryptedStringV1 {
  return Boolean(value && typeof value === 'object' && (value as any).__lm_enc_v1 === true)
}

function maybePlainText(value: unknown): string {
  if (typeof value === 'string') return value
  if (isEncryptedString(value)) return '[本地加密]'
  return ''
}

function bytesToBase64Raw(bytes: Uint8Array): string {
  let binary = ''
  const step = 0x8000
  for (let i = 0; i < bytes.length; i += step) {
    binary += String.fromCharCode(...bytes.subarray(i, i + step))
  }
  return btoa(binary)
}

function base64ToBytesRaw(value: string): Uint8Array {
  const binary = atob(value)
  const out = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i += 1) out[i] = binary.charCodeAt(i)
  return out
}

async function localStorageCryptoKey(): Promise<CryptoKey | null> {
  if (!identity.value || !passphrase.value) return null
  const webCrypto = globalThis.crypto
  if (!webCrypto?.subtle) {
    appendLog(`⚠️ 当前页面没有 WebCrypto subtle，无法解密/加密本地敏感数据；当前地址：${window.location.protocol}//${window.location.host}。请使用 https 或 http://127.0.0.1 访问`)
    return null
  }
  const material = await webCrypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(normalize_passphrase(passphrase.value)),
    'PBKDF2',
    false,
    ['deriveKey'],
  )
  return webCrypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: new TextEncoder().encode(`lm-talk-local-store-v1:${identity.value.user_id}`),
      iterations: 210_000,
      hash: 'SHA-256',
    },
    material,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  )
}

async function encryptLocalString(value: string, key: CryptoKey | null): Promise<string | EncryptedStringV1> {
  if (!key || !value) return value
  const webCrypto = globalThis.crypto
  if (!webCrypto?.subtle) return value
  const iv = webCrypto.getRandomValues(new Uint8Array(12))
  const ct = new Uint8Array(await webCrypto.subtle.encrypt(
    { name: 'AES-GCM', iv: iv as BufferSource },
    key,
    new TextEncoder().encode(value),
  ))
  return { __lm_enc_v1: true, alg: 'AES-GCM', kdf: 'PBKDF2-SHA-256', iv: bytesToBase64Raw(iv), ct: bytesToBase64Raw(ct) }
}

async function decryptLocalString(value: unknown, key: CryptoKey | null): Promise<string> {
  if (typeof value === 'string') return value
  if (!isEncryptedString(value)) return ''
  if (!key) return value as any
  const webCrypto = globalThis.crypto
  if (!webCrypto?.subtle) return value as any
  const plain = await webCrypto.subtle.decrypt(
    { name: 'AES-GCM', iv: base64ToBytesRaw(value.iv) as BufferSource },
    key,
    base64ToBytesRaw(value.ct) as BufferSource,
  )
  return new TextDecoder().decode(plain)
}

async function encryptContactForStore(contact: ContactItem, key: CryptoKey | null): Promise<any> {
  return {
    ...contact,
    display_name: contact.display_name ? await encryptLocalString(contact.display_name, key) : contact.display_name,
    contact_card_text: await encryptLocalString(contact.contact_card_text, key),
    block_reason: contact.block_reason ? await encryptLocalString(contact.block_reason, key) : contact.block_reason,
  }
}

async function decryptContactFromStore(contact: any, key: CryptoKey | null): Promise<ContactItem> {
  return {
    ...contact,
    state: contact.state ?? 'LocalOnly',
    display_name: contact.display_name ? await decryptLocalString(contact.display_name, key) : contact.display_name,
    contact_card_text: await decryptLocalString(contact.contact_card_text, key),
    block_reason: contact.block_reason ? await decryptLocalString(contact.block_reason, key) : contact.block_reason,
  }
}

async function encryptGroupForStore(group: GroupItem, key: CryptoKey | null): Promise<any> {
  return {
    ...group,
    name: await encryptLocalString(group.name, key),
    policy_state_json: group.policy_state_json ? await encryptLocalString(group.policy_state_json, key) : group.policy_state_json,
  }
}

async function decryptGroupFromStore(group: any, key: CryptoKey | null): Promise<GroupItem> {
  return {
    ...group,
    sequence: group.sequence ?? 0,
    admin_user_ids: group.admin_user_ids ?? [],
    name: await decryptLocalString(group.name, key),
    policy_state_json: group.policy_state_json ? await decryptLocalString(group.policy_state_json, key) : group.policy_state_json,
  }
}

async function encryptMessageForStore(message: ChatMessage, key: CryptoKey | null): Promise<any> {
  return {
    ...message,
    text: await encryptLocalString(message.text, key),
    envelope_json: message.envelope_json ? await encryptLocalString(message.envelope_json, key) : message.envelope_json,
  }
}

async function decryptMessageFromStore(message: any, key: CryptoKey | null): Promise<ChatMessage> {
  return {
    ...message,
    status: message.status ?? (message.direction === 'in' ? 'received' : 'queued'),
    text: await decryptLocalString(message.text, key),
    envelope_json: message.envelope_json ? await decryptLocalString(message.envelope_json, key) : message.envelope_json,
  }
}

async function encryptOutboxForStore(item: OutboxItem, key: CryptoKey | null): Promise<any> {
  return { ...item, envelope_json: await encryptLocalString(item.envelope_json, key) }
}

async function decryptOutboxFromStore(item: any, key: CryptoKey | null): Promise<OutboxItem> {
  return { ...item, status: item.status ?? 'queued', retry_count: item.retry_count ?? 0, envelope_json: await decryptLocalString(item.envelope_json, key) }
}

async function encryptFriendRequestForStore(item: FriendRequestItem, key: CryptoKey | null): Promise<any> {
  return {
    ...item,
    note: item.note ? await encryptLocalString(item.note, key) : item.note,
    request_text: await encryptLocalString(item.request_text, key),
    from_contact_card_text: await encryptLocalString(item.from_contact_card_text, key),
    quarantine_reason: item.quarantine_reason ? await encryptLocalString(item.quarantine_reason, key) : item.quarantine_reason,
  }
}

async function decryptFriendRequestFromStore(item: any, key: CryptoKey | null): Promise<FriendRequestItem> {
  return {
    ...item,
    note: item.note ? await decryptLocalString(item.note, key) : item.note,
    request_text: await decryptLocalString(item.request_text, key),
    from_contact_card_text: await decryptLocalString(item.from_contact_card_text, key),
    quarantine_reason: item.quarantine_reason ? await decryptLocalString(item.quarantine_reason, key) : item.quarantine_reason,
  }
}

async function encryptGroupInviteForStore(item: GroupInviteItem, key: CryptoKey | null): Promise<any> {
  return {
    ...item,
    group_name: await encryptLocalString(item.group_name, key),
    invite_text: await encryptLocalString(item.invite_text, key),
  }
}

async function decryptGroupInviteFromStore(item: any, key: CryptoKey | null): Promise<GroupInviteItem> {
  return {
    ...item,
    group_name: await decryptLocalString(item.group_name, key),
    invite_text: await decryptLocalString(item.invite_text, key),
  }
}

async function encryptGroupSenderKeyForStore(item: GroupSenderKeyItem, key: CryptoKey | null): Promise<any> {
  return {
    ...item,
    state_json: await encryptLocalString(item.state_json, key),
    distribution_text: item.distribution_text ? await encryptLocalString(item.distribution_text, key) : item.distribution_text,
  }
}

async function decryptGroupSenderKeyFromStore(item: any, key: CryptoKey | null): Promise<GroupSenderKeyItem> {
  return {
    ...item,
    state_json: await decryptLocalString(item.state_json, key),
    distribution_text: item.distribution_text ? await decryptLocalString(item.distribution_text, key) : item.distribution_text,
  }
}

async function encryptMailboxFailedItemForStore(item: MailboxFailedItem, key: CryptoKey | null): Promise<any> {
  return {
    ...item,
    message: await encryptLocalString(JSON.stringify(item.message ?? null), key),
    reason: await encryptLocalString(item.reason, key),
  }
}

async function decryptMailboxFailedItemFromStore(item: any, key: CryptoKey | null): Promise<MailboxFailedItem> {
  const messageText = await decryptLocalString(item.message, key)
  let message: any = null
  try { message = messageText ? JSON.parse(messageText) : null } catch { message = item.message }
  return {
    ...item,
    message,
    reason: await decryptLocalString(item.reason, key),
    retry_count: item.retry_count ?? 0,
  }
}

function normalizeProcessedMailboxRecords(records: Array<string | ProcessedMailboxRecord> | undefined): ProcessedMailboxRecord[] {
  const now = Date.now()
  const seen = new Set<string>()
  const normalized: ProcessedMailboxRecord[] = []
  for (const record of records ?? []) {
    const id = typeof record === 'string' ? record : record.id
    if (!id || seen.has(id)) continue
    const processedAt = typeof record === 'string' ? now : record.processed_at || now
    if (now - processedAt > MAILBOX_DEDUPE_RETENTION_MS) continue
    seen.add(id)
    normalized.push({ id, processed_at: processedAt })
  }
  return normalized
    .sort((a, b) => b.processed_at - a.processed_at)
    .slice(0, MAILBOX_DEDUPE_MAX_RECORDS)
}

async function encryptRatchetForStore(item: RatchetSessionItem, key: CryptoKey | null): Promise<any> {
  return { ...item, state_text: await encryptLocalString(item.state_text, key) }
}

async function decryptRatchetFromStore(item: any, key: CryptoKey | null): Promise<RatchetSessionItem> {
  return { ...item, state_text: await decryptLocalString(item.state_text, key) }
}

async function decryptSensitiveStateInMemory() {
  const key = await localStorageCryptoKey()
  if (!key) return
  contacts.value = await Promise.all(contacts.value.map((c: any) => decryptContactFromStore(c, key)))
  friendRequests.value = await Promise.all(friendRequests.value.map((r: any) => decryptFriendRequestFromStore(r, key)))
  groups.value = await Promise.all(groups.value.map((g: any) => decryptGroupFromStore(g, key)))
  groupInvites.value = await Promise.all(groupInvites.value.map((g: any) => decryptGroupInviteFromStore(g, key)))
  groupSenderKeys.value = await Promise.all(groupSenderKeys.value.map((g: any) => decryptGroupSenderKeyFromStore(g, key)))
  messages.value = await Promise.all(messages.value.map((m: any) => decryptMessageFromStore(m, key)))
  outbox.value = await Promise.all(outbox.value.map((o: any) => decryptOutboxFromStore(o, key)))
  mailboxFailedItems.value = await Promise.all(mailboxFailedItems.value.map((m: any) => decryptMailboxFailedItemFromStore(m, key)))
  ratchetSessions.value = await Promise.all(ratchetSessions.value.map((r: any) => decryptRatchetFromStore(r, key)))
}

function currentPersistedState(): PersistedState {
  return {
    backupText: backupText.value,
    contacts: contacts.value,
    friendRequests: friendRequests.value,
    groups: groups.value,
    groupInvites: groupInvites.value,
    groupSenderKeys: groupSenderKeys.value,
    messages: messages.value,
    outbox: outbox.value,
    ratchetSessions: ratchetSessions.value,
    myContactCardText: myContactCardText.value,
    myDeviceCertJson: myDeviceCertJson.value,
    myDeviceBackupText: myDeviceBackupText.value,
    myDeviceId: myDeviceId.value,
    prekeyBundleText: prekeyBundleText.value,
    prekeyPrivateBundleJson: prekeyPrivateBundleJson.value,
    prekeySignedOneTimeRecordTexts: prekeySignedOneTimeRecordTexts.value,
    safetyPolicy: safetyPolicy.value,
    nodeControlUrl: nodeControlUrl.value,
    nodeEnabled: nodeEnabled.value,
    autoMailboxTake: autoMailboxTake.value,
    autoReadReceipts: autoReadReceipts.value,
    autoPublishPreKey: autoPublishPreKey.value,
    autoNodeSync: autoNodeSync.value,
    autoSelfMailboxSync: autoSelfMailboxSync.value,
    lastNodeSnapshotSyncAt: lastNodeSnapshotSyncAt.value ?? undefined,
    processedMailboxIds: processedMailboxIds.value,
    mailboxFailedItems: mailboxFailedItems.value,
    contactCardUpdateFanoutRecords: contactCardUpdateFanoutRecords.value,
    syncRecoveryHistory: syncRecoveryHistory.value,
    dhtOperationHistory: nodeDhtOperationHistory.value,
    friendRequestRateRecords: friendRequestRateRecords.value,
    lastFullDataBackupAt: lastFullDataBackupAt.value ?? undefined,
    lastSelfMailboxBackupPushedAt: lastSelfMailboxBackupPushedAt.value ?? undefined,
    lastSelfMailboxBackupReceivedAt: lastSelfMailboxBackupReceivedAt.value ?? undefined,
    lastSelfMailboxBackupMergedAt: lastSelfMailboxBackupMergedAt.value ?? undefined,
    processedSelfSyncIds: processedSelfSyncIds.value,
    processedSelfSyncRequestIds: processedSelfSyncRequestIds.value,
    selfSyncMissingRequestRecords: selfSyncMissingRequestRecords.value,
    selfSyncRequestSentCount: selfSyncRequestSentCount.value || undefined,
    selfSyncRequestHitCount: selfSyncRequestHitCount.value || undefined,
    selfSyncRequestMissCount: selfSyncRequestMissCount.value || undefined,
    selfSyncRecentPackages: selfSyncRecentPackages.value,
    lastSelfSyncPushedAt: lastSelfSyncPushedAt.value ?? undefined,
    lastSelfSyncMergedAt: lastSelfSyncMergedAt.value ?? undefined,
    lastSelfSyncSequenceSent: lastSelfSyncSequenceSent.value || undefined,
    lastSelfSyncSequenceMerged: lastSelfSyncSequenceMerged.value || undefined,
    selfSyncGapCount: selfSyncGapCount.value || undefined,
    lastSelfSyncGapAt: lastSelfSyncGapAt.value ?? undefined,
    lastSelfSyncMissingPreviousId: lastSelfSyncMissingPreviousId.value || undefined,
    lastSelfSyncReceiptStatesSent: lastSelfSyncReceiptStatesSent.value || undefined,
    lastSelfSyncReceiptStatesMerged: lastSelfSyncReceiptStatesMerged.value || undefined,
    totalSelfSyncReceiptStatesMerged: totalSelfSyncReceiptStatesMerged.value || undefined,
    lastSelfSyncOutboxSummary: lastSelfSyncOutboxSummary.value || undefined,
    unverifiedIncomingDropCount: unverifiedIncomingDropCount.value,
    lastUnverifiedIncomingDropAt: lastUnverifiedIncomingDropAt.value ?? undefined,
    lastUnverifiedIncomingDropFrom: lastUnverifiedIncomingDropFrom.value,
    revokedDeviceIncomingDropCount: revokedDeviceIncomingDropCount.value,
    lastRevokedDeviceIncomingDropAt: lastRevokedDeviceIncomingDropAt.value ?? undefined,
    lastRevokedDeviceIncomingDropFrom: lastRevokedDeviceIncomingDropFrom.value,
    perDeviceEnvelopeSentCount: perDeviceEnvelopeSentCount.value || undefined,
    perDeviceEnvelopeReceivedCount: perDeviceEnvelopeReceivedCount.value || undefined,
    perDeviceEnvelopeDropCount: perDeviceEnvelopeDropCount.value || undefined,
    lastPerDeviceEnvelopeAt: lastPerDeviceEnvelopeAt.value ?? undefined,
    lastPerDeviceEnvelopeDropAt: lastPerDeviceEnvelopeDropAt.value ?? undefined,
    lastPerDeviceEnvelopeDropReason: lastPerDeviceEnvelopeDropReason.value || undefined,
    contactCardUpdateFanoutCount: contactCardUpdateFanoutCount.value || undefined,
    contactCardUpdateFanoutSkipCount: contactCardUpdateFanoutSkipCount.value || undefined,
    lastContactCardUpdateFanoutAt: lastContactCardUpdateFanoutAt.value ?? undefined,
    contactCardDhtAutoRefreshCount: contactCardDhtAutoRefreshCount.value || undefined,
    lastContactCardDhtAutoRefreshAt: lastContactCardDhtAutoRefreshAt.value ?? undefined,
    lastContactCardDhtAutoRefreshError: lastContactCardDhtAutoRefreshError.value || undefined,
    contactCardDhtAutoRefreshHistory: contactCardDhtAutoRefreshHistory.value,
  }
}

function persistedMeta(): PersistedMeta {
  return {
    backupText: backupText.value,
    myContactCardText: myContactCardText.value,
    myDeviceCertJson: myDeviceCertJson.value,
    myDeviceBackupText: myDeviceBackupText.value,
    myDeviceId: myDeviceId.value,
    prekeyBundleText: prekeyBundleText.value,
    prekeyPrivateBundleJson: prekeyPrivateBundleJson.value,
    prekeySignedOneTimeRecordTexts: prekeySignedOneTimeRecordTexts.value,
    safetyPolicy: safetyPolicy.value,
    nodeControlUrl: nodeControlUrl.value,
    nodeEnabled: nodeEnabled.value,
    autoMailboxTake: autoMailboxTake.value,
    autoReadReceipts: autoReadReceipts.value,
    autoPublishPreKey: autoPublishPreKey.value,
    autoNodeSync: autoNodeSync.value,
    autoSelfMailboxSync: autoSelfMailboxSync.value,
    lastNodeSnapshotSyncAt: lastNodeSnapshotSyncAt.value ?? undefined,
    processedMailboxIds: processedMailboxIds.value,
    mailboxFailedItems: mailboxFailedItems.value,
    contactCardUpdateFanoutRecords: contactCardUpdateFanoutRecords.value,
    syncRecoveryHistory: syncRecoveryHistory.value,
    dhtOperationHistory: nodeDhtOperationHistory.value,
    friendRequestRateRecords: friendRequestRateRecords.value,
    lastFullDataBackupAt: lastFullDataBackupAt.value ?? undefined,
    lastSelfMailboxBackupPushedAt: lastSelfMailboxBackupPushedAt.value ?? undefined,
    lastSelfMailboxBackupReceivedAt: lastSelfMailboxBackupReceivedAt.value ?? undefined,
    lastSelfMailboxBackupMergedAt: lastSelfMailboxBackupMergedAt.value ?? undefined,
    processedSelfSyncIds: processedSelfSyncIds.value,
    processedSelfSyncRequestIds: processedSelfSyncRequestIds.value,
    selfSyncMissingRequestRecords: selfSyncMissingRequestRecords.value,
    selfSyncRequestSentCount: selfSyncRequestSentCount.value || undefined,
    selfSyncRequestHitCount: selfSyncRequestHitCount.value || undefined,
    selfSyncRequestMissCount: selfSyncRequestMissCount.value || undefined,
    selfSyncRecentPackages: selfSyncRecentPackages.value,
    lastSelfSyncPushedAt: lastSelfSyncPushedAt.value ?? undefined,
    lastSelfSyncMergedAt: lastSelfSyncMergedAt.value ?? undefined,
    lastSelfSyncSequenceSent: lastSelfSyncSequenceSent.value || undefined,
    lastSelfSyncSequenceMerged: lastSelfSyncSequenceMerged.value || undefined,
    selfSyncGapCount: selfSyncGapCount.value || undefined,
    lastSelfSyncGapAt: lastSelfSyncGapAt.value ?? undefined,
    lastSelfSyncMissingPreviousId: lastSelfSyncMissingPreviousId.value || undefined,
    lastSelfSyncReceiptStatesSent: lastSelfSyncReceiptStatesSent.value || undefined,
    lastSelfSyncReceiptStatesMerged: lastSelfSyncReceiptStatesMerged.value || undefined,
    totalSelfSyncReceiptStatesMerged: totalSelfSyncReceiptStatesMerged.value || undefined,
    lastSelfSyncOutboxSummary: lastSelfSyncOutboxSummary.value || undefined,
    unverifiedIncomingDropCount: unverifiedIncomingDropCount.value,
    lastUnverifiedIncomingDropAt: lastUnverifiedIncomingDropAt.value ?? undefined,
    lastUnverifiedIncomingDropFrom: lastUnverifiedIncomingDropFrom.value,
    revokedDeviceIncomingDropCount: revokedDeviceIncomingDropCount.value,
    lastRevokedDeviceIncomingDropAt: lastRevokedDeviceIncomingDropAt.value ?? undefined,
    lastRevokedDeviceIncomingDropFrom: lastRevokedDeviceIncomingDropFrom.value,
    schemaVersion: 3,
  }
}

function ownerId(): string {
  return identity.value?.user_id || selectedLocalIdentityId.value || 'anonymous'
}

function ownerKey(key: string): string {
  return `${ownerId()}::${key}`
}

function ownerPrefix(): string {
  return `${ownerId()}::`
}

function accountPrefix(userId: string): string {
  return `${userId}::`
}

async function purgeAccountTables(userId: string) {
  const prefix = accountPrefix(userId)
  await Promise.all(Object.values(TABLES).map((table) => idbTableReplaceByPrefix(table, prefix, [])))
}

type TableLoadResult<T> = {
  items: T[]
  failed: number
}

function recordTableLoadFailure(table: string, error: unknown) {
  appendLog(`⚠️ IndexedDB ${table} 部分记录加载失败：${userFacingError(error)}`)
}

async function loadTableByPrefixSafe<T>(table: typeof TABLES[keyof typeof TABLES], prefix: string, decode: (value: any) => Promise<T>): Promise<TableLoadResult<T>> {
  const rows = await idbTableGetAllByPrefix<any>(table, prefix)
  const items: T[] = []
  let failed = 0
  for (const row of rows) {
    try {
      items.push(await decode(row))
    } catch (e) {
      failed += 1
      recordTableLoadFailure(table, e)
    }
  }
  return { items, failed }
}

function summarizePartialLoadFailures(failures: Array<[string, number]>) {
  const parts = failures.filter(([, count]) => count > 0).map(([name, count]) => `${name} ${count}`)
  if (parts.length === 0) return
  appendLog(`⚠️ 已跳过损坏的本地记录：${parts.join('，')}；其余数据已加载，请从备份或同步服务恢复缺失内容`)
  toast('已跳过损坏的本地记录，其余数据已加载', 'warning')
}

async function persistStateTables() {
  if (!identity.value) return
  const key = await localStorageCryptoKey()
  const storedContacts = await Promise.all(contacts.value.map((c) => encryptContactForStore(c, key)))
  const storedFriendRequests = await Promise.all(friendRequests.value.map((r) => encryptFriendRequestForStore(r, key)))
  const storedGroups = await Promise.all(groups.value.map((g) => encryptGroupForStore(g, key)))
  const storedGroupInvites = await Promise.all(groupInvites.value.map((g) => encryptGroupInviteForStore(g, key)))
  const storedGroupSenderKeys = await Promise.all(groupSenderKeys.value.map((g) => encryptGroupSenderKeyForStore(g, key)))
  const storedMessages = await Promise.all(messages.value.map((m) => encryptMessageForStore(m, key)))
  const storedOutbox = await Promise.all(outbox.value.map((o) => encryptOutboxForStore(o, key)))
  const storedRatchets = await Promise.all(ratchetSessions.value.map((r) => encryptRatchetForStore(r, key)))
  const storedMailboxFailedItems = await Promise.all(mailboxFailedItems.value.map((m) => encryptMailboxFailedItemForStore(m, key)))
  const meta = persistedMeta()
  meta.nodeControlUrl = await encryptLocalString(nodeControlUrl.value, key)
  meta.prekeyPrivateBundleJson = await encryptLocalString(prekeyPrivateBundleJson.value, key)
  meta.mailboxFailedItems = storedMailboxFailedItems
  const prefix = ownerPrefix()
  await idbTableReplaceByPrefix(TABLES.meta, prefix, [[ownerKey('main'), meta]])
  await idbTableReplaceByPrefix(TABLES.contacts, prefix, storedContacts.map((c) => [ownerKey(c.user_id), c]))
  await idbTableReplaceByPrefix(TABLES.friendRequests, prefix, storedFriendRequests.map((r) => [ownerKey(r.request_id), r]))
  await idbTableReplaceByPrefix(TABLES.groups, prefix, storedGroups.map((g) => [ownerKey(g.group_id), g]))
  await idbTableReplaceByPrefix(TABLES.groupInvites, prefix, storedGroupInvites.map((g) => [ownerKey(g.invite_id), g]))
  await idbTableReplaceByPrefix(TABLES.groupSenderKeys, prefix, storedGroupSenderKeys.map((k) => [ownerKey(k.key_id), k]))
  await idbTableReplaceByPrefix(TABLES.messages, prefix, storedMessages.map((m) => [ownerKey(m.id), m]))
  await idbTableReplaceByPrefix(TABLES.outbox, prefix, storedOutbox.map((o) => [ownerKey(o.id), o]))
  await idbTableReplaceByPrefix(TABLES.ratchetSessions, prefix, storedRatchets.map((r) => [ownerKey(r.peer_user_id), r]))
  await idbSet('chat-state-schema-v2', true)
}

let persistChain: Promise<void> = Promise.resolve()

function persist() {
  persistChain = persistChain
    .catch(() => undefined)
    .then(() => persistStateTables())
    .catch((e) => appendLog(`❌ IndexedDB 保存失败：${String(e)}`))
}

async function flushPersistForTests() {
  await persistChain
}

if (typeof window !== 'undefined') {
  ;(window as any).flushPersistForTests = flushPersistForTests
  ;(window as any).appendLogForTests = appendLog
  ;(window as any).setDhtDiagnosticsForTests = (status: string, history: string[] = []) => {
    nodeDhtFindValueStatusText.value = status
    nodeDhtOperationHistory.value = history
    persist()
  }
  ;(window as any).mergeMessagesForTests = mergeMessagesForState
  ;(window as any).mergeContactDeviceAndTrustStateForTests = mergeContactDeviceAndTrustState
  ;(window as any).contactAllKnownDevicesRevokedForTests = contactAllKnownDevicesRevoked
}

async function writeStateToTables(state: PersistedState) {
  backupText.value = state.backupText ?? ''
  const key = await localStorageCryptoKey()
  contacts.value = await Promise.all((state.contacts ?? []).map((c: any) => decryptContactFromStore(c, key)))
  friendRequests.value = await Promise.all((state.friendRequests ?? []).map((r: any) => decryptFriendRequestFromStore(r, key)))
  groups.value = await Promise.all((state.groups ?? []).map((g: any) => decryptGroupFromStore(g, key)))
  groupInvites.value = await Promise.all((state.groupInvites ?? []).map((g: any) => decryptGroupInviteFromStore(g, key)))
  groupSenderKeys.value = await Promise.all((state.groupSenderKeys ?? []).map((g: any) => decryptGroupSenderKeyFromStore(g, key)))
  messages.value = await Promise.all((state.messages ?? []).map((m: any) => decryptMessageFromStore(m, key)))
  outbox.value = await Promise.all((state.outbox ?? []).map((o: any) => decryptOutboxFromStore(o, key)))
  ratchetSessions.value = await Promise.all((state.ratchetSessions ?? []).map((r: any) => decryptRatchetFromStore(r, key)))
  myContactCardText.value = state.myContactCardText ?? ''
  myDeviceCertJson.value = state.myDeviceCertJson ?? ''
  myDeviceBackupText.value = state.myDeviceBackupText ?? ''
  myDeviceId.value = state.myDeviceId ?? ''
  prekeyBundleText.value = state.prekeyBundleText ?? ''
  prekeyPrivateBundleJson.value = state.prekeyPrivateBundleJson ? await decryptLocalString(state.prekeyPrivateBundleJson, key) : ''
  prekeySignedOneTimeRecordTexts.value = state.prekeySignedOneTimeRecordTexts ?? []
  safetyPolicy.value = { ...safetyPolicy.value, ...(state.safetyPolicy ?? {}) }
  nodeControlUrl.value = state.nodeControlUrl ? await decryptLocalString(state.nodeControlUrl, key) : nodeControlUrl.value
  nodeEnabled.value = state.nodeEnabled ?? false
  autoMailboxTake.value = state.autoMailboxTake ?? true
  autoReadReceipts.value = state.autoReadReceipts ?? false
  autoPublishPreKey.value = state.autoPublishPreKey ?? true
  autoNodeSync.value = state.autoNodeSync ?? false
  autoSelfMailboxSync.value = state.autoSelfMailboxSync ?? false
  lastNodeSnapshotSyncAt.value = typeof state.lastNodeSnapshotSyncAt === 'number' ? state.lastNodeSnapshotSyncAt : null
  processedMailboxIds.value = normalizeProcessedMailboxRecords(state.processedMailboxIds)
  mailboxFailedItems.value = await Promise.all((state.mailboxFailedItems ?? []).map((m: any) => decryptMailboxFailedItemFromStore(m, key)))
  contactCardUpdateFanoutRecords.value = normalizeContactCardUpdateFanoutRecords(state.contactCardUpdateFanoutRecords ?? [])
  syncRecoveryHistory.value = state.syncRecoveryHistory ?? []
  nodeDhtOperationHistory.value = state.dhtOperationHistory ?? []
  friendRequestRateRecords.value = state.friendRequestRateRecords ?? []
  lastFullDataBackupAt.value = typeof state.lastFullDataBackupAt === 'number' ? state.lastFullDataBackupAt : null
  lastSelfMailboxBackupPushedAt.value = typeof state.lastSelfMailboxBackupPushedAt === 'number' ? state.lastSelfMailboxBackupPushedAt : null
  lastSelfMailboxBackupReceivedAt.value = typeof state.lastSelfMailboxBackupReceivedAt === 'number' ? state.lastSelfMailboxBackupReceivedAt : null
  lastSelfMailboxBackupMergedAt.value = typeof state.lastSelfMailboxBackupMergedAt === 'number' ? state.lastSelfMailboxBackupMergedAt : null
  processedSelfSyncIds.value = state.processedSelfSyncIds ?? []
  processedSelfSyncRequestIds.value = state.processedSelfSyncRequestIds ?? []
  selfSyncMissingRequestRecords.value = state.selfSyncMissingRequestRecords ?? []
  selfSyncRequestSentCount.value = Number(state.selfSyncRequestSentCount ?? 0)
  selfSyncRequestHitCount.value = Number(state.selfSyncRequestHitCount ?? 0)
  selfSyncRequestMissCount.value = Number(state.selfSyncRequestMissCount ?? 0)
  selfSyncRecentPackages.value = normalizeSelfSyncRecentPackages(state.selfSyncRecentPackages ?? [])
  lastSelfSyncPushedAt.value = typeof state.lastSelfSyncPushedAt === 'number' ? state.lastSelfSyncPushedAt : null
  lastSelfSyncMergedAt.value = typeof state.lastSelfSyncMergedAt === 'number' ? state.lastSelfSyncMergedAt : null
  lastSelfSyncSequenceSent.value = Number(state.lastSelfSyncSequenceSent ?? 0)
  lastSelfSyncSequenceMerged.value = Number(state.lastSelfSyncSequenceMerged ?? 0)
  selfSyncGapCount.value = Number(state.selfSyncGapCount ?? 0)
  lastSelfSyncGapAt.value = typeof state.lastSelfSyncGapAt === 'number' ? state.lastSelfSyncGapAt : null
  lastSelfSyncMissingPreviousId.value = state.lastSelfSyncMissingPreviousId ?? ''
  lastSelfSyncReceiptStatesSent.value = Number(state.lastSelfSyncReceiptStatesSent ?? 0)
  lastSelfSyncReceiptStatesMerged.value = Number(state.lastSelfSyncReceiptStatesMerged ?? 0)
  totalSelfSyncReceiptStatesMerged.value = Number(state.totalSelfSyncReceiptStatesMerged ?? 0)
  lastSelfSyncOutboxSummary.value = state.lastSelfSyncOutboxSummary ?? null
  unverifiedIncomingDropCount.value = Number(state.unverifiedIncomingDropCount ?? 0)
  lastUnverifiedIncomingDropAt.value = typeof state.lastUnverifiedIncomingDropAt === 'number' ? state.lastUnverifiedIncomingDropAt : null
  lastUnverifiedIncomingDropFrom.value = state.lastUnverifiedIncomingDropFrom ?? ''
  revokedDeviceIncomingDropCount.value = Number(state.revokedDeviceIncomingDropCount ?? 0)
  lastRevokedDeviceIncomingDropAt.value = typeof state.lastRevokedDeviceIncomingDropAt === 'number' ? state.lastRevokedDeviceIncomingDropAt : null
  lastRevokedDeviceIncomingDropFrom.value = state.lastRevokedDeviceIncomingDropFrom ?? ''
  perDeviceEnvelopeSentCount.value = Number(state.perDeviceEnvelopeSentCount ?? 0)
  perDeviceEnvelopeReceivedCount.value = Number(state.perDeviceEnvelopeReceivedCount ?? 0)
  perDeviceEnvelopeDropCount.value = Number(state.perDeviceEnvelopeDropCount ?? 0)
  lastPerDeviceEnvelopeAt.value = typeof state.lastPerDeviceEnvelopeAt === 'number' ? state.lastPerDeviceEnvelopeAt : null
  lastPerDeviceEnvelopeDropAt.value = typeof state.lastPerDeviceEnvelopeDropAt === 'number' ? state.lastPerDeviceEnvelopeDropAt : null
  lastPerDeviceEnvelopeDropReason.value = state.lastPerDeviceEnvelopeDropReason ?? ''
  contactCardUpdateFanoutCount.value = Number(state.contactCardUpdateFanoutCount ?? 0)
  contactCardUpdateFanoutSkipCount.value = Number(state.contactCardUpdateFanoutSkipCount ?? 0)
  lastContactCardUpdateFanoutAt.value = typeof state.lastContactCardUpdateFanoutAt === 'number' ? state.lastContactCardUpdateFanoutAt : null
  contactCardDhtAutoRefreshCount.value = Number(state.contactCardDhtAutoRefreshCount ?? 0)
  lastContactCardDhtAutoRefreshAt.value = typeof state.lastContactCardDhtAutoRefreshAt === 'number' ? state.lastContactCardDhtAutoRefreshAt : null
  lastContactCardDhtAutoRefreshError.value = state.lastContactCardDhtAutoRefreshError ?? ''
  contactCardDhtAutoRefreshHistory.value = (state.contactCardDhtAutoRefreshHistory ?? []).slice(0, 20)
  await persistStateTables()
}

async function loadStateFromTables(): Promise<boolean> {
  if (!identity.value) return false
  const meta = await idbTableGet<PersistedMeta>(TABLES.meta, ownerKey('main'))
  if (!meta) return false
  backupText.value = meta.backupText ?? ''
  myContactCardText.value = meta.myContactCardText ?? ''
  myDeviceCertJson.value = meta.myDeviceCertJson ?? ''
  myDeviceBackupText.value = meta.myDeviceBackupText ?? ''
  myDeviceId.value = meta.myDeviceId ?? ''
  const key = await localStorageCryptoKey()
  prekeyBundleText.value = meta.prekeyBundleText ?? ''
  prekeyPrivateBundleJson.value = meta.prekeyPrivateBundleJson ? await decryptLocalString(meta.prekeyPrivateBundleJson, key) : ''
  prekeySignedOneTimeRecordTexts.value = meta.prekeySignedOneTimeRecordTexts ?? []
  safetyPolicy.value = { ...safetyPolicy.value, ...(meta.safetyPolicy ?? {}) }
  nodeControlUrl.value = meta.nodeControlUrl ? await decryptLocalString(meta.nodeControlUrl, key) : nodeControlUrl.value
  nodeEnabled.value = meta.nodeEnabled ?? false
  autoMailboxTake.value = meta.autoMailboxTake ?? true
  autoReadReceipts.value = meta.autoReadReceipts ?? false
  autoPublishPreKey.value = meta.autoPublishPreKey ?? true
  autoNodeSync.value = meta.autoNodeSync ?? false
  autoSelfMailboxSync.value = meta.autoSelfMailboxSync ?? false
  lastNodeSnapshotSyncAt.value = typeof meta.lastNodeSnapshotSyncAt === 'number' ? meta.lastNodeSnapshotSyncAt : null
  processedMailboxIds.value = normalizeProcessedMailboxRecords(meta.processedMailboxIds)
  mailboxFailedItems.value = await Promise.all((meta.mailboxFailedItems ?? []).map((m: any) => decryptMailboxFailedItemFromStore(m, key)))
  contactCardUpdateFanoutRecords.value = normalizeContactCardUpdateFanoutRecords(meta.contactCardUpdateFanoutRecords ?? [])
  syncRecoveryHistory.value = meta.syncRecoveryHistory ?? []
  nodeDhtOperationHistory.value = meta.dhtOperationHistory ?? []
  friendRequestRateRecords.value = meta.friendRequestRateRecords ?? []
  lastFullDataBackupAt.value = typeof meta.lastFullDataBackupAt === 'number' ? meta.lastFullDataBackupAt : null
  lastSelfMailboxBackupPushedAt.value = typeof meta.lastSelfMailboxBackupPushedAt === 'number' ? meta.lastSelfMailboxBackupPushedAt : null
  lastSelfMailboxBackupReceivedAt.value = typeof meta.lastSelfMailboxBackupReceivedAt === 'number' ? meta.lastSelfMailboxBackupReceivedAt : null
  lastSelfMailboxBackupMergedAt.value = typeof meta.lastSelfMailboxBackupMergedAt === 'number' ? meta.lastSelfMailboxBackupMergedAt : null
  processedSelfSyncIds.value = meta.processedSelfSyncIds ?? []
  processedSelfSyncRequestIds.value = meta.processedSelfSyncRequestIds ?? []
  selfSyncMissingRequestRecords.value = meta.selfSyncMissingRequestRecords ?? []
  selfSyncRequestSentCount.value = Number(meta.selfSyncRequestSentCount ?? 0)
  selfSyncRequestHitCount.value = Number(meta.selfSyncRequestHitCount ?? 0)
  selfSyncRequestMissCount.value = Number(meta.selfSyncRequestMissCount ?? 0)
  selfSyncRecentPackages.value = normalizeSelfSyncRecentPackages(meta.selfSyncRecentPackages ?? [])
  lastSelfSyncPushedAt.value = typeof meta.lastSelfSyncPushedAt === 'number' ? meta.lastSelfSyncPushedAt : null
  lastSelfSyncMergedAt.value = typeof meta.lastSelfSyncMergedAt === 'number' ? meta.lastSelfSyncMergedAt : null
  lastSelfSyncSequenceSent.value = Number(meta.lastSelfSyncSequenceSent ?? 0)
  lastSelfSyncSequenceMerged.value = Number(meta.lastSelfSyncSequenceMerged ?? 0)
  selfSyncGapCount.value = Number(meta.selfSyncGapCount ?? 0)
  lastSelfSyncGapAt.value = typeof meta.lastSelfSyncGapAt === 'number' ? meta.lastSelfSyncGapAt : null
  lastSelfSyncMissingPreviousId.value = meta.lastSelfSyncMissingPreviousId ?? ''
  lastSelfSyncReceiptStatesSent.value = Number(meta.lastSelfSyncReceiptStatesSent ?? 0)
  lastSelfSyncReceiptStatesMerged.value = Number(meta.lastSelfSyncReceiptStatesMerged ?? 0)
  totalSelfSyncReceiptStatesMerged.value = Number(meta.totalSelfSyncReceiptStatesMerged ?? 0)
  lastSelfSyncOutboxSummary.value = meta.lastSelfSyncOutboxSummary ?? null
  unverifiedIncomingDropCount.value = Number(meta.unverifiedIncomingDropCount ?? 0)
  lastUnverifiedIncomingDropAt.value = typeof meta.lastUnverifiedIncomingDropAt === 'number' ? meta.lastUnverifiedIncomingDropAt : null
  lastUnverifiedIncomingDropFrom.value = meta.lastUnverifiedIncomingDropFrom ?? ''
  revokedDeviceIncomingDropCount.value = Number(meta.revokedDeviceIncomingDropCount ?? 0)
  lastRevokedDeviceIncomingDropAt.value = typeof meta.lastRevokedDeviceIncomingDropAt === 'number' ? meta.lastRevokedDeviceIncomingDropAt : null
  lastRevokedDeviceIncomingDropFrom.value = meta.lastRevokedDeviceIncomingDropFrom ?? ''
  perDeviceEnvelopeSentCount.value = Number(meta.perDeviceEnvelopeSentCount ?? 0)
  perDeviceEnvelopeReceivedCount.value = Number(meta.perDeviceEnvelopeReceivedCount ?? 0)
  perDeviceEnvelopeDropCount.value = Number(meta.perDeviceEnvelopeDropCount ?? 0)
  lastPerDeviceEnvelopeAt.value = typeof meta.lastPerDeviceEnvelopeAt === 'number' ? meta.lastPerDeviceEnvelopeAt : null
  lastPerDeviceEnvelopeDropAt.value = typeof meta.lastPerDeviceEnvelopeDropAt === 'number' ? meta.lastPerDeviceEnvelopeDropAt : null
  lastPerDeviceEnvelopeDropReason.value = meta.lastPerDeviceEnvelopeDropReason ?? ''
  contactCardUpdateFanoutCount.value = Number(meta.contactCardUpdateFanoutCount ?? 0)
  contactCardUpdateFanoutSkipCount.value = Number(meta.contactCardUpdateFanoutSkipCount ?? 0)
  lastContactCardUpdateFanoutAt.value = typeof meta.lastContactCardUpdateFanoutAt === 'number' ? meta.lastContactCardUpdateFanoutAt : null
  contactCardDhtAutoRefreshCount.value = Number(meta.contactCardDhtAutoRefreshCount ?? 0)
  lastContactCardDhtAutoRefreshAt.value = typeof meta.lastContactCardDhtAutoRefreshAt === 'number' ? meta.lastContactCardDhtAutoRefreshAt : null
  lastContactCardDhtAutoRefreshError.value = meta.lastContactCardDhtAutoRefreshError ?? ''
  contactCardDhtAutoRefreshHistory.value = (meta.contactCardDhtAutoRefreshHistory ?? []).slice(0, 20)
  const prefix = ownerPrefix()
  const loadedContacts = await loadTableByPrefixSafe(TABLES.contacts, prefix, (c) => decryptContactFromStore(c, key))
  const loadedFriendRequests = await loadTableByPrefixSafe(TABLES.friendRequests, prefix, (r) => decryptFriendRequestFromStore(r, key))
  const loadedGroups = await loadTableByPrefixSafe(TABLES.groups, prefix, (g) => decryptGroupFromStore(g, key))
  const loadedGroupInvites = await loadTableByPrefixSafe(TABLES.groupInvites, prefix, (g) => decryptGroupInviteFromStore(g, key))
  const loadedGroupSenderKeys = await loadTableByPrefixSafe(TABLES.groupSenderKeys, prefix, (g) => decryptGroupSenderKeyFromStore(g, key))
  const loadedMessages = await loadTableByPrefixSafe(TABLES.messages, prefix, (m) => decryptMessageFromStore(m, key))
  const loadedOutbox = await loadTableByPrefixSafe(TABLES.outbox, prefix, (o) => decryptOutboxFromStore(o, key))
  const loadedRatchets = await loadTableByPrefixSafe(TABLES.ratchetSessions, prefix, (r) => decryptRatchetFromStore(r, key))
  contacts.value = loadedContacts.items
  friendRequests.value = loadedFriendRequests.items
  groups.value = loadedGroups.items
  groupInvites.value = loadedGroupInvites.items
  groupSenderKeys.value = loadedGroupSenderKeys.items
  messages.value = loadedMessages.items
  outbox.value = loadedOutbox.items
  ratchetSessions.value = loadedRatchets.items
  summarizePartialLoadFailures([
    ['联系人', loadedContacts.failed],
    ['好友请求', loadedFriendRequests.failed],
    ['群', loadedGroups.failed],
    ['群邀请', loadedGroupInvites.failed],
    ['Sender Key', loadedGroupSenderKeys.failed],
    ['消息', loadedMessages.failed],
    ['Outbox', loadedOutbox.failed],
    ['Ratchet', loadedRatchets.failed],
  ])
  if (backupText.value && myContactCardText.value) {
    try {
      const info = safeJson<ContactInfo>(inspect_contact_card(myContactCardText.value))
      rememberLocalIdentity(info.user_id, info.display_name || displayName.value, backupText.value)
    } catch { /* ignore old/incomplete local identity */ }
  }
  appendLog('✅ 本地状态已加载；若存在损坏记录已自动跳过')
  return true
}

async function loadPersistedState() {
  try {
    if (await loadStateFromTables()) return

    let state = await idbGet<PersistedState>('chat-state-v1')

    // One-time migration from the old localStorage demo state.
    let migratedFromLocalStorage = false
    if (!state) {
      const raw = localStorage.getItem('lm-talk-chat-state-v1')
      if (raw) {
        state = JSON.parse(raw) as PersistedState
        migratedFromLocalStorage = true
      }
    } else {
      appendLog('✅ 已从旧 IndexedDB 单对象迁移到分表')
    }

    if (!state) return
    await writeStateToTables(state)
    if (migratedFromLocalStorage) {
      localStorage.removeItem('lm-talk-chat-state-v1')
      appendLog('✅ 已从 localStorage 迁移到 IndexedDB 分表')
    }
    await idbDel('chat-state-v1')
  } catch (e) {
    appendLog(`❌ IndexedDB 加载失败：${String(e)}`)
  }
}

function resetAccountScopedState() {
  contacts.value = []
  friendRequests.value = []
  groups.value = []
  groupInvites.value = []
  groupSenderKeys.value = []
  messages.value = []
  outbox.value = []
  ratchetSessions.value = []
  processedMailboxIds.value = []
  mailboxFailedItems.value = []
  contactCardUpdateFanoutRecords.value = []
  syncRecoveryHistory.value = []
  nodeDhtOperationHistory.value = []
  friendRequestRateRecords.value = []
  activePeerId.value = ''
  activeGroupId.value = ''
  myContactCardText.value = ''
  myDeviceCertJson.value = ''
  myDeviceBackupText.value = ''
  myDeviceId.value = ''
  prekeyBundleText.value = ''
  prekeyPrivateBundleJson.value = ''
  prekeySignedOneTimeRecordTexts.value = []
  selectedSignedOneTimePreKeyRecordText.value = ''
  selectedOneTimePreKeyId.value = null
  safetyPolicy.value = {
    enableTextFilter: true,
    textFilterLevel: 'Standard',
    warnExternalLinks: true,
    warnExecutableFiles: true,
    dropFilteredIncoming: false,
    requireVerifiedContactsForSend: false,
    requireVerifiedContactsForReceive: false,
    requireSealedPerDeviceSlotsForSend: false,
    requireSealedPerDeviceSlotsForReceive: false,
  }
  nodeEnabled.value = false
  autoMailboxTake.value = true
  autoReadReceipts.value = false
  autoPublishPreKey.value = true
  autoNodeSync.value = false
  autoSelfMailboxSync.value = false
  lastNodeSnapshotSyncAt.value = null
  perDeviceEnvelopeSentCount.value = 0
  perDeviceEnvelopeReceivedCount.value = 0
  perDeviceEnvelopeDropCount.value = 0
  lastPerDeviceEnvelopeAt.value = null
  lastPerDeviceEnvelopeDropAt.value = null
  lastPerDeviceEnvelopeDropReason.value = ''
  contactCardUpdateFanoutCount.value = 0
  contactCardUpdateFanoutSkipCount.value = 0
  lastContactCardUpdateFanoutAt.value = null
  contactCardDhtAutoRefreshCount.value = 0
  lastContactCardDhtAutoRefreshAt.value = null
  lastContactCardDhtAutoRefreshError.value = ''
  contactCardDhtAutoRefreshHistory.value = []
  nodeControlStatus.value = '未连接'
}

async function clearPersisted() {
  await idbDel('chat-state-v1')
  await idbDel('chat-state-schema-v2')
  await Promise.all(Object.values(TABLES).map((table) => idbTableClear(table)))
  localStorage.removeItem('lm-talk-chat-state-v1')
  localStorage.removeItem(LOCAL_IDENTITIES_KEY)
  localIdentities.value = []
  selectedLocalIdentityId.value = ''
  lastRegisteredIdentity.value = null
  backupText.value = ''
  identity.value = null
  contacts.value = []
  friendRequests.value = []
  groups.value = []
  groupInvites.value = []
  groupSenderKeys.value = []
  messages.value = []
  outbox.value = []
  ratchetSessions.value = []
  processedMailboxIds.value = []
  mailboxFailedItems.value = []
  contactCardUpdateFanoutRecords.value = []
  syncRecoveryHistory.value = []
  nodeDhtOperationHistory.value = []
  friendRequestRateRecords.value = []
  lastFullDataBackupAt.value = null
  lastSelfMailboxBackupPushedAt.value = null
  lastSelfMailboxBackupReceivedAt.value = null
  lastSelfMailboxBackupMergedAt.value = null
  processedSelfSyncIds.value = []
  processedSelfSyncRequestIds.value = []
  selfSyncMissingRequestRecords.value = []
  selfSyncRequestSentCount.value = 0
  selfSyncRequestHitCount.value = 0
  selfSyncRequestMissCount.value = 0
  selfSyncRecentPackages.value = []
  lastSelfSyncPushedAt.value = null
  lastSelfSyncMergedAt.value = null
  lastSelfSyncSequenceSent.value = 0
  lastSelfSyncSequenceMerged.value = 0
  selfSyncGapCount.value = 0
  lastSelfSyncGapAt.value = null
  lastSelfSyncMissingPreviousId.value = ''
  lastSelfSyncReceiptStatesSent.value = 0
  lastSelfSyncReceiptStatesMerged.value = 0
  totalSelfSyncReceiptStatesMerged.value = 0
  lastSelfSyncOutboxSummary.value = null
  unverifiedIncomingDropCount.value = 0
  lastUnverifiedIncomingDropAt.value = null
  lastUnverifiedIncomingDropFrom.value = ''
  revokedDeviceIncomingDropCount.value = 0
  lastRevokedDeviceIncomingDropAt.value = null
  lastRevokedDeviceIncomingDropFrom.value = ''
  perDeviceEnvelopeSentCount.value = 0
  perDeviceEnvelopeReceivedCount.value = 0
  perDeviceEnvelopeDropCount.value = 0
  lastPerDeviceEnvelopeAt.value = null
  lastPerDeviceEnvelopeDropAt.value = null
  lastPerDeviceEnvelopeDropReason.value = ''
  contactCardUpdateFanoutCount.value = 0
  contactCardUpdateFanoutSkipCount.value = 0
  lastContactCardUpdateFanoutAt.value = null
  contactCardDhtAutoRefreshCount.value = 0
  lastContactCardDhtAutoRefreshAt.value = null
  lastContactCardDhtAutoRefreshError.value = ''
  contactCardDhtAutoRefreshHistory.value = []
  myContactCardText.value = ''
  myDeviceCertJson.value = ''
  myDeviceBackupText.value = ''
  myDeviceId.value = ''
  prekeyBundleText.value = ''
  prekeyPrivateBundleJson.value = ''
  prekeySignedOneTimeRecordTexts.value = []
  selectedSignedOneTimePreKeyRecordText.value = ''
  selectedOneTimePreKeyId.value = null
  safetyPolicy.value = {
    enableTextFilter: true,
    textFilterLevel: 'Standard',
    warnExternalLinks: true,
    warnExecutableFiles: true,
    dropFilteredIncoming: false,
    requireVerifiedContactsForSend: false,
    requireVerifiedContactsForReceive: false,
    requireSealedPerDeviceSlotsForSend: false,
    requireSealedPerDeviceSlotsForReceive: false,
  }
  nodeEnabled.value = false
  autoMailboxTake.value = true
  autoReadReceipts.value = false
  autoPublishPreKey.value = true
  autoNodeSync.value = false
  nodeControlStatus.value = '未连接'
  loggedIn.value = false
  appendLog('已清空本地状态')
}


async function clearBrowserCaches() {
  const ok = await showConfirm('清理浏览器缓存', '清理浏览器缓存并注销旧版 Service Worker？这不会删除身份、联系人或聊天数据。', true)
  if (!ok) return
  const cacheKeys = typeof caches !== 'undefined' ? await caches.keys().catch(() => []) : []
  await Promise.all(cacheKeys.map((key) => caches.delete(key)))
  const nav = navigator as Navigator & { serviceWorker?: ServiceWorkerContainer }
  const registrations = nav.serviceWorker?.getRegistrations ? await nav.serviceWorker.getRegistrations().catch(() => []) : []
  await Promise.all(registrations.map((registration) => registration.unregister()))
  appendLog(`已清理浏览器缓存：cache=${cacheKeys.length}, service_worker=${registrations.length}`)
  toast('已清理浏览器缓存', 'success')
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`
}

function isDangerousFileName(name: string): boolean {
  return /\.(exe|bat|cmd|com|scr|ps1|vbs|js|jar|msi|apk|dmg|pkg|sh)$/i.test(name)
}

async function refreshStorageEstimate() {
  const estimate = navigator.storage?.estimate ? await navigator.storage.estimate() : {}
  const usage = estimate.usage ?? 0
  const quota = estimate.quota ?? 0
  const ratio = quota > 0 ? `，${Math.round((usage / quota) * 100)}%` : ''
  storageEstimateText.value = `已用 ${formatBytes(usage)} / 可用 ${formatBytes(quota)}${ratio}`
}

async function warnIfLowStorageForFile(fileSize: number) {
  if (!navigator.storage?.estimate) return
  const estimate = await navigator.storage.estimate()
  const usage = estimate.usage ?? 0
  const quota = estimate.quota ?? 0
  if (quota <= 0) return
  const remaining = quota - usage
  if (remaining < fileSize * 2) {
    const message = `浏览器剩余存储约 ${formatBytes(Math.max(remaining, 0))}，发送大文件可能失败`
    rtcFileStatus.value = message
    appendLog(`⚠️ ${message}`)
  }
}

function exportFullDataBackup() {
  run('导出完整数据备份', () => {
    if (!backupText.value || !passphrase.value) throw new Error('需要当前身份备份包和提示词')
    dataBackupText.value = export_data_backup(
      backupText.value,
      passphrase.value,
      JSON.stringify(currentPersistedState()),
    )
    lastFullDataBackupAt.value = Date.now()
    persist()
  })
}

async function pushFullDataBackupToOwnMailbox() {
  await runAsync('备份到自己的 Mailbox', async () => {
    if (!identity.value?.user_id) throw new Error('需要先登录身份')
    if (!nodeEnabled.value) throw new Error('节点未启用')
    if (!dataBackupText.value.trim()) exportFullDataBackup()
    if (!dataBackupText.value.trim()) throw new Error('请先生成完整数据备份')
    const msg = create_mailbox_message(
      backupText.value,
      passphrase.value,
      identity.value.user_id,
      'data-backup',
      dataBackupText.value,
      BigInt(7 * 24 * 3600),
    )
    const body = await nodeFetchJson('/mailbox/push', {
      method: 'POST',
      body: JSON.stringify({
        message_text: msg,
        from_identity_public_key: identity.value.identity_public_key,
      }),
    })
    nodeControlStatus.value = JSON.stringify(body, null, 2)
    lastSelfMailboxBackupPushedAt.value = Date.now()
    selfMailboxBackupStatusText.value = `完整数据备份已投递到自己的 Mailbox${body?.delivery_id ? '：' + body.delivery_id : ''}`
    persist()
    appendLog(`✅ ${selfMailboxBackupStatusText.value}`)
    toast('完整数据备份已投递到自己的 Mailbox', 'success')
  })
}

function selfSyncSigningPayload(pkg: SelfSyncPackage): string {
  return JSON.stringify({
    type: pkg.type,
    version: pkg.version,
    sync_id: pkg.sync_id,
    sequence: pkg.sequence,
    previous_sync_id: pkg.previous_sync_id,
    created_at: pkg.created_at,
    from_user_id: pkg.from_user_id,
    identity_public_key: pkg.identity_public_key,
    from_device_id: pkg.from_device_id,
    contacts: pkg.contacts,
    messageReceiptStates: pkg.messageReceiptStates,
    outboxSummary: pkg.outboxSummary,
    myContactCardText: pkg.myContactCardText,
    myDeviceCertJson: pkg.myDeviceCertJson,
    myDeviceId: pkg.myDeviceId,
    dhtOperationHistory: pkg.dhtOperationHistory,
    processedSelfSyncIds: pkg.processedSelfSyncIds,
    unverifiedIncomingDropCount: pkg.unverifiedIncomingDropCount,
    revokedDeviceIncomingDropCount: pkg.revokedDeviceIncomingDropCount,
  })
}

function selfSyncRequestSigningPayload(pkg: SelfSyncRequestPackage): string {
  return JSON.stringify({
    type: pkg.type,
    version: pkg.version,
    request_id: pkg.request_id,
    missing_sync_id: pkg.missing_sync_id,
    created_at: pkg.created_at,
    from_user_id: pkg.from_user_id,
    identity_public_key: pkg.identity_public_key,
    from_device_id: pkg.from_device_id,
  })
}

function currentSelfSyncRequestPackage(missingSyncId: string): SelfSyncRequestPackage {
  const pkg: SelfSyncRequestPackage = {
    type: 'lm-self-sync-request-v1',
    version: 1,
    request_id: newId(),
    missing_sync_id: missingSyncId,
    created_at: Date.now(),
    from_user_id: identity.value?.user_id || '',
    identity_public_key: identity.value?.identity_public_key || '',
    from_device_id: myDeviceId.value || undefined,
  }
  pkg.signature = sign_identity_text(backupText.value, passphrase.value, selfSyncRequestSigningPayload(pkg))
  return pkg
}


function currentMessageReceiptSyncItems(): MessageReceiptSyncItem[] {
  return messages.value
    .filter((message) => message.direction === 'out' && message.peer_user_id && message.protocol_message_id)
    .filter((message) =>
      message.mailbox_delivery_id
      || message.delivered_at
      || message.read_at
      || ['sent', 'mailbox', 'delivered', 'read'].includes(message.status))
    .map((message) => ({
      peer_user_id: message.peer_user_id,
      protocol_message_id: message.protocol_message_id!,
      status: message.status,
      mailbox_delivery_id: message.mailbox_delivery_id,
      delivered_at: message.delivered_at,
      read_at: message.read_at,
      created_at: message.created_at,
    }))
    .slice(-200)
}

function applyMessageReceiptSyncItems(items: MessageReceiptSyncItem[] | undefined): number {
  let merged = 0
  for (const item of items ?? []) {
    if (!item.peer_user_id || !item.protocol_message_id) continue
    const message = messages.value.find((m) =>
      m.direction === 'out'
      && m.peer_user_id === item.peer_user_id
      && m.protocol_message_id === item.protocol_message_id)
    if (!message) continue
    const before = `${message.status}:${message.delivered_at || ''}:${message.read_at || ''}`
    mergeMessageStateInto(message, {
      ...message,
      status: item.status,
      mailbox_delivery_id: item.mailbox_delivery_id,
      delivered_at: item.delivered_at,
      read_at: item.read_at,
      created_at: item.created_at ?? message.created_at,
    })
    const after = `${message.status}:${message.delivered_at || ''}:${message.read_at || ''}`
    if (after !== before) merged += 1
  }
  return merged
}


function currentOutboxSyncSummary(): OutboxSyncSummary {
  const pending = outbox.value.filter((item) => item.status !== 'sent')
  const failedKinds: Record<string, number> = {}
  for (const item of outbox.value.filter((x) => x.status === 'failed')) {
    const key = item.kind || 'unknown'
    failedKinds[key] = (failedKinds[key] ?? 0) + 1
  }
  return {
    queued: outbox.value.filter((item) => item.status === 'queued').length,
    failed: outbox.value.filter((item) => item.status === 'failed').length,
    sent: outbox.value.filter((item) => item.status === 'sent').length,
    oldest_pending_at: pending.length ? Math.min(...pending.map((item) => item.created_at || Date.now())) : undefined,
    failed_kinds: Object.keys(failedKinds).length ? failedKinds : undefined,
  }
}

function currentSelfSyncPackage(): SelfSyncPackage {
  const sequence = lastSelfSyncSequenceSent.value + 1
  const messageReceiptStates = currentMessageReceiptSyncItems()
  lastSelfSyncReceiptStatesSent.value = messageReceiptStates.length
  const pkg: SelfSyncPackage = {
    type: 'lm-self-sync-v1',
    version: 1,
    sync_id: newId(),
    sequence,
    previous_sync_id: processedSelfSyncIds.value[0],
    created_at: Date.now(),
    from_user_id: identity.value?.user_id || '',
    identity_public_key: identity.value?.identity_public_key || '',
    from_device_id: myDeviceId.value || undefined,
    contacts: contacts.value,
    messageReceiptStates,
    outboxSummary: currentOutboxSyncSummary(),
    myContactCardText: myContactCardText.value || undefined,
    myDeviceCertJson: myDeviceCertJson.value || undefined,
    myDeviceId: myDeviceId.value || undefined,
    dhtOperationHistory: nodeDhtOperationHistory.value,
    processedSelfSyncIds: processedSelfSyncIds.value,
    unverifiedIncomingDropCount: unverifiedIncomingDropCount.value,
    revokedDeviceIncomingDropCount: revokedDeviceIncomingDropCount.value,
  }
  pkg.signature = sign_identity_text(backupText.value, passphrase.value, selfSyncSigningPayload(pkg))
  return pkg
}


function mergeOwnDeviceCertsFromSelfSync(pkg: SelfSyncPackage): boolean {
  if (!identity.value?.user_id) return false
  const incomingCerts = [
    ...(pkg.myContactCardText ? contactCardDeviceCerts(pkg.myContactCardText) : []),
    ...(pkg.myDeviceCertJson ? [safeJson<DeviceCertItem>(pkg.myDeviceCertJson)] : []),
  ].filter((cert) => cert?.device_id)
  if (incomingCerts.length === 0) return false
  const before = myContactCardText.value || ''
  const byId = new Map<string, DeviceCertItem>()
  for (const cert of contactCardDeviceCerts(myContactCardText.value || '')) byId.set(cert.device_id, cert)
  if (myDeviceCertJson.value) {
    const cert = safeJson<DeviceCertItem>(myDeviceCertJson.value)
    if (cert?.device_id) byId.set(cert.device_id, cert)
  }
  for (const cert of incomingCerts) byId.set(cert.device_id, cert)
  const certJson = JSON.stringify([...byId.values()])
  myContactCardText.value = export_contact_card(backupText.value, passphrase.value, displayName.value || undefined, certJson)
  const changed = myContactCardText.value !== before
  appendLog(`✅ 自同步已合并自己的设备证书：${incomingCerts.length} 个新增/更新来源`)
  return changed
}

function normalizeSelfSyncRecentPackages(items: SelfSyncCachedPackage[]): SelfSyncCachedPackage[] {
  const now = Date.now()
  return items
    .filter((item) => item?.sync_id && item?.payload && (item.expires_at ?? item.created_at + 7 * 24 * 3600 * 1000) > now)
    .filter((item, index, all) => all.findIndex((x) => x.sync_id === item.sync_id) === index)
    .sort((a, b) => Number(b.created_at ?? 0) - Number(a.created_at ?? 0))
    .slice(0, 10)
}

function rememberSelfSyncPackage(pkg: SelfSyncPackage, payload: string) {
  selfSyncRecentPackages.value = normalizeSelfSyncRecentPackages([
    { sync_id: pkg.sync_id, sequence: pkg.sequence, created_at: pkg.created_at, expires_at: pkg.created_at + 7 * 24 * 3600 * 1000, payload },
    ...selfSyncRecentPackages.value.filter((item) => item.sync_id !== pkg.sync_id),
  ])
}

async function pushSelfSyncPayloadToOwnMailbox(payload: string, label: string, kind = 'self-sync') {
  if (!identity.value?.user_id) throw new Error('需要先登录身份')
  if (!nodeEnabled.value) throw new Error('节点未启用')
  const msg = create_mailbox_message(
    backupText.value,
    passphrase.value,
    identity.value.user_id,
    kind,
    payload,
    BigInt(7 * 24 * 3600),
  )
  const body = await nodeFetchJson('/mailbox/push', {
    method: 'POST',
    body: JSON.stringify({
      message_text: msg,
      from_identity_public_key: identity.value.identity_public_key,
    }),
  })
  lastSelfSyncPushedAt.value = Date.now()
  selfSyncStatusText.value = `${label}${body?.delivery_id ? '：' + body.delivery_id : ''}`
  appendLog(`✅ ${selfSyncStatusText.value}`)
  persist()
}

function applySelfSyncPackage(pkg: SelfSyncPackage) {
  if (pkg.version !== 1) throw new Error('self-sync version 不支持')
  if (!pkg.sync_id) throw new Error('self-sync 缺少 sync_id')
  const sequence = Number(pkg.sequence ?? 0)
  if (!Number.isFinite(sequence) || sequence <= 0) throw new Error('self-sync sequence 无效')
  const createdAt = Number(pkg.created_at ?? 0)
  if (!Number.isFinite(createdAt) || createdAt <= 0) throw new Error('self-sync created_at 无效')
  const ageMs = Math.abs(Date.now() - createdAt)
  if (ageMs > 30 * 24 * 3600 * 1000) throw new Error('self-sync 已超过 30 天时间窗口')
  if (pkg.from_user_id !== identity.value?.user_id) throw new Error('self-sync user_id 与当前身份不匹配')
  if (pkg.identity_public_key !== identity.value?.identity_public_key) throw new Error('self-sync identity_public_key 与当前身份不匹配')
  if (!pkg.signature || !verify_identity_text_signature(pkg.identity_public_key, selfSyncSigningPayload(pkg), pkg.signature)) throw new Error('self-sync 签名无效')
  if (pkg.from_device_id && myDeviceId.value && pkg.from_device_id === myDeviceId.value) {
    processedSelfSyncIds.value = [pkg.sync_id, ...processedSelfSyncIds.value.filter((id) => id !== pkg.sync_id)].slice(0, 100)
    selfSyncStatusText.value = `自同步：已跳过本设备包 ${pkg.sync_id}`
    appendLog(selfSyncStatusText.value)
    persist()
    return
  }
  if (processedSelfSyncIds.value.includes(pkg.sync_id)) {
    selfSyncStatusText.value = `自同步：已跳过重复包 ${pkg.sync_id}`
    appendLog(selfSyncStatusText.value)
    persist()
    return
  }
  if (sequence <= lastSelfSyncSequenceMerged.value) {
    processedSelfSyncIds.value = [pkg.sync_id, ...processedSelfSyncIds.value.filter((id) => id !== pkg.sync_id)].slice(0, 100)
    selfSyncStatusText.value = `自同步：已跳过旧序列包 #${sequence} ${pkg.sync_id}`
    appendLog(selfSyncStatusText.value)
    persist()
    return
  }
  if (pkg.previous_sync_id && processedSelfSyncIds.value.length > 0 && !processedSelfSyncIds.value.includes(pkg.previous_sync_id)) {
    selfSyncGapCount.value += 1
    lastSelfSyncGapAt.value = Date.now()
    lastSelfSyncMissingPreviousId.value = pkg.previous_sync_id
    appendLog(`⚠️ 自同步可能存在缺口：previous_sync_id ${pkg.previous_sync_id} 未见过`)
    void requestMissingSelfSyncPackage(pkg.previous_sync_id).catch((error) => appendLog(`⚠️ 自同步缺包请求失败：${userFacingError(error)}`))
  }
  processedSelfSyncIds.value = [pkg.sync_id, ...processedSelfSyncIds.value.filter((id) => id !== pkg.sync_id), ...(pkg.processedSelfSyncIds ?? [])].filter(Boolean).slice(0, 100)
  processedSelfSyncIds.value = [...new Set(processedSelfSyncIds.value)].slice(0, 100)
  contacts.value = mergeContactDeviceAndTrustState(contacts.value, pkg.contacts ?? [])
  const receiptStatesMerged = applyMessageReceiptSyncItems(pkg.messageReceiptStates)
  lastSelfSyncOutboxSummary.value = pkg.outboxSummary ?? null
  lastSelfSyncReceiptStatesMerged.value = receiptStatesMerged
  totalSelfSyncReceiptStatesMerged.value += receiptStatesMerged
  const ownDeviceCertsChanged = mergeOwnDeviceCertsFromSelfSync(pkg)
  nodeDhtOperationHistory.value = [...new Set([...(pkg.dhtOperationHistory ?? []), ...nodeDhtOperationHistory.value])].slice(0, DHT_OPERATION_HISTORY_MAX_RECORDS)
  unverifiedIncomingDropCount.value = Math.max(unverifiedIncomingDropCount.value, Number(pkg.unverifiedIncomingDropCount ?? 0))
  revokedDeviceIncomingDropCount.value = Math.max(revokedDeviceIncomingDropCount.value, Number(pkg.revokedDeviceIncomingDropCount ?? 0))
  lastSelfSyncMergedAt.value = Date.now()
  lastSelfSyncSequenceMerged.value = Math.max(lastSelfSyncSequenceMerged.value, sequence)
  selfSyncStatusText.value = `自同步：已合并 ${pkg.contacts?.length ?? 0} 个联系人状态、${receiptStatesMerged} 条回执状态（#${sequence} ${pkg.sync_id.slice(0, 8)}）`
  appendLog(`✅ ${selfSyncStatusText.value}`)
  persist()
  if (ownDeviceCertsChanged && friendContacts.value.length) {
    appendLog('自同步合并了新的本机设备证书，正在向好友分发联系人更新')
    void fanoutMyContactCardUpdateToFriends()
    if (nodeEnabled.value) void ensureOwnContactCardDhtRecord()
  }
}

async function autoPushSelfSyncPackageToOwnMailbox() {
  if (!loggedIn.value || !nodeEnabled.value || !autoSelfMailboxSync.value) return
  if (lastSelfSyncPushedAt.value && Date.now() - lastSelfSyncPushedAt.value < 5 * 60_000) return
  try {
    if (contactCardUpdateStaleAckCount.value > 0) {
      const retried = await retryStaleContactCardUpdateAcksRaw()
      if (retried.retried) appendLog(`自同步巡检：已自动重试过期设备证书更新确认 ${retried.retried}/${retried.stale} 条`)
    }
    if (selfSyncGapCount.value > 0) await repairSelfSyncGapNow()
    else await pushSelfSyncPackageToOwnMailbox()
  } catch (error) {
    selfSyncStatusText.value = `自同步：自动投递失败：${userFacingError(error)}`
  }
}

function clearSelfSyncGapStats() {
  selfSyncGapCount.value = 0
  lastSelfSyncGapAt.value = null
  lastSelfSyncMissingPreviousId.value = ''
  selfSyncStatusText.value = '自同步：已清空缺口统计'
  appendLog('已清空轻量自同步缺口统计')
  persist()
}

async function requestMissingSelfSyncPackage(missingSyncId: string) {
  const now = Date.now()
  const recent = selfSyncMissingRequestRecords.value.find((item) => item.missing_sync_id === missingSyncId)
  if (recent && now - recent.requested_at < 5 * 60_000) {
    selfSyncStatusText.value = `自同步：缺包请求已节流 ${missingSyncId.slice(0, 8)}`
    appendLog(selfSyncStatusText.value)
    return
  }
  selfSyncMissingRequestRecords.value = [
    { missing_sync_id: missingSyncId, requested_at: now },
    ...selfSyncMissingRequestRecords.value.filter((item) => item.missing_sync_id !== missingSyncId),
  ].slice(0, 100)
  const pkg = currentSelfSyncRequestPackage(missingSyncId)
  const payload = JSON.stringify(pkg)
  await pushSelfSyncPayloadToOwnMailbox(payload, `自同步：已请求缺失包 ${missingSyncId.slice(0, 8)}`, 'self-sync-request')
  selfSyncRequestSentCount.value += 1
  persist()
}

async function applySelfSyncRequestPackage(pkg: SelfSyncRequestPackage) {
  if (pkg.version !== 1) throw new Error('self-sync request version 不支持')
  if (!pkg.request_id || !pkg.missing_sync_id) throw new Error('self-sync request 缺少 request_id 或 missing_sync_id')
  if (processedSelfSyncRequestIds.value.includes(pkg.request_id)) {
    selfSyncStatusText.value = `自同步：已跳过重复缺包请求 ${pkg.request_id}`
    appendLog(selfSyncStatusText.value)
    persist()
    return
  }
  processedSelfSyncRequestIds.value = [pkg.request_id, ...processedSelfSyncRequestIds.value.filter((id) => id !== pkg.request_id)].slice(0, 100)
  if (pkg.from_user_id !== identity.value?.user_id) throw new Error('self-sync request user_id 与当前身份不匹配')
  if (pkg.identity_public_key !== identity.value?.identity_public_key) throw new Error('self-sync request identity_public_key 与当前身份不匹配')
  if (!pkg.signature || !verify_identity_text_signature(pkg.identity_public_key, selfSyncRequestSigningPayload(pkg), pkg.signature)) throw new Error('self-sync request 签名无效')
  if (pkg.from_device_id && myDeviceId.value && pkg.from_device_id === myDeviceId.value) {
    selfSyncStatusText.value = `自同步：已跳过本设备缺包请求 ${pkg.missing_sync_id.slice(0, 8)}`
    appendLog(selfSyncStatusText.value)
    return
  }
  const cached = selfSyncRecentPackages.value.find((item) => item.sync_id === pkg.missing_sync_id)
  if (!cached?.payload) {
    selfSyncRequestMissCount.value += 1
    selfSyncStatusText.value = `自同步：收到缺包请求但本机无缓存 ${pkg.missing_sync_id.slice(0, 8)}`
    appendLog(`⚠️ ${selfSyncStatusText.value}`)
    persist()
    return
  }
  await pushSelfSyncPayloadToOwnMailbox(cached.payload, `自同步：响应缺包请求，重发 #${cached.sequence} 到自己的 Mailbox`)
  selfSyncRequestHitCount.value += 1
  persist()
}

async function repairSelfSyncGapNow() {
  await runAsync('补发轻量自同步包', async () => {
    selfSyncRecentPackages.value = normalizeSelfSyncRecentPackages(selfSyncRecentPackages.value)
    const missing = lastSelfSyncMissingPreviousId.value
    const cached = missing ? selfSyncRecentPackages.value.find((item) => item.sync_id === missing) : undefined
    if (cached?.payload) {
      await pushSelfSyncPayloadToOwnMailbox(cached.payload, `自同步：已补发缺失包 #${cached.sequence} 到自己的 Mailbox`)
    } else {
      await pushSelfSyncPackageToOwnMailbox()
      selfSyncStatusText.value = `${selfSyncStatusText.value}；未找到缺失包缓存，已补发当前状态`
    }
    appendLog('✅ 已补发轻量自同步包用于缺口补偿')
  })
}

async function pushSelfSyncPackageToOwnMailbox() {
  await runAsync('同步状态到自己的 Mailbox', async () => {
    const pkg = currentSelfSyncPackage()
    const payload = JSON.stringify(pkg)
    rememberSelfSyncPackage(pkg, payload)
    lastSelfSyncSequenceSent.value = Math.max(lastSelfSyncSequenceSent.value, pkg.sequence)
    await pushSelfSyncPayloadToOwnMailbox(payload, `自同步：已投递 #${pkg.sequence} 到自己的 Mailbox`)
  })
}

async function resendLatestSelfSyncPackageToOwnMailbox() {
  await runAsync('重发最近轻量自同步包', async () => {
    selfSyncRecentPackages.value = normalizeSelfSyncRecentPackages(selfSyncRecentPackages.value)
    const latest = selfSyncRecentPackages.value[0]
    if (!latest?.payload) throw new Error('暂无可重发的轻量自同步包')
    await pushSelfSyncPayloadToOwnMailbox(latest.payload, `自同步：已重发 #${latest.sequence} 到自己的 Mailbox`)
  })
}

function clearSelfSyncRecentPackages() {
  selfSyncRecentPackages.value = []
  selfSyncStatusText.value = '自同步：已清空最近轻量包缓存'
  appendLog('已清空最近轻量自同步包缓存')
  persist()
}

async function importFullDataBackup() {
  try {
    if (!backupText.value || !passphrase.value) throw new Error('需要当前身份备份包和提示词')
    if (!dataBackupText.value.trim()) throw new Error('请粘贴完整数据备份')
    const hasLocalData = contacts.value.length + groups.value.length + messages.value.length + outbox.value.length > 0
    if (hasLocalData) {
      const ok = await showConfirm('导入完整数据备份', '导入会覆盖当前身份的本地联系人、群聊、消息和待发送队列。继续导入？', true)
      if (!ok) return
    }
    const json = import_data_backup(backupText.value, passphrase.value, dataBackupText.value)
    const state = JSON.parse(json) as PersistedState
    await writeStateToTables(state)
    appendLog('✅ 已导入完整数据备份')
    toast('完整数据备份已导入', 'success')
  } catch (e) {
    appendLog(`❌ 导入完整数据备份失败：${String(e)}`)
  }
}

function mergeUniqueBy<T>(current: T[], incoming: T[], keyOf: (item: T) => string | undefined): { items: T[]; added: number; skipped: number } {
  const seen = new Set(current.map(keyOf).filter(Boolean) as string[])
  const items = [...current]
  let added = 0
  let skipped = 0
  for (const item of incoming) {
    const key = keyOf(item)
    if (!key) { skipped += 1; continue }
    if (seen.has(key)) { skipped += 1; continue }
    seen.add(key)
    items.push(item)
    added += 1
  }
  return { items, added, skipped }
}


function messageStateRank(message: ChatMessage): number {
  if (message.status === 'read' || message.read_at) return 5
  if (message.status === 'delivered' || message.delivered_at) return 4
  if (message.status === 'mailbox') return 3
  if (message.status === 'sent') return 2
  if (message.status === 'failed') return 1
  return 0
}

function messageMergeKey(message: ChatMessage): string | undefined {
  if (message.protocol_message_id && message.peer_user_id) return `protocol:${message.peer_user_id}:${message.protocol_message_id}`
  if (message.id) return `id:${message.id}`
  return undefined
}

function mergeMessageStateInto(target: ChatMessage, incoming: ChatMessage): boolean {
  let changed = false
  const targetRank = messageStateRank(target)
  const incomingRank = messageStateRank(incoming)
  if (incomingRank > targetRank) {
    target.status = incoming.status
    changed = true
  }
  if (!target.delivered_at && incoming.delivered_at) { target.delivered_at = incoming.delivered_at; changed = true }
  if (!target.read_at && incoming.read_at) { target.read_at = incoming.read_at; changed = true }
  if (!target.mailbox_delivery_id && incoming.mailbox_delivery_id) { target.mailbox_delivery_id = incoming.mailbox_delivery_id; changed = true }
  if (!target.protocol_message_id && incoming.protocol_message_id) { target.protocol_message_id = incoming.protocol_message_id; changed = true }
  if (target.read_at && target.status !== 'read') { target.status = 'read'; changed = true }
  else if (target.delivered_at && messageStateRank(target) < 4) { target.status = 'delivered'; changed = true }
  return changed
}

function mergeMessagesForState(current: ChatMessage[], incoming: ChatMessage[]): { items: ChatMessage[]; added: number; merged: number; skipped: number } {
  const items = [...current]
  const byKey = new Map<string, ChatMessage>()
  for (const item of items) {
    const key = messageMergeKey(item)
    if (key) byKey.set(key, item)
    if (item.id) byKey.set(`id:${item.id}`, item)
  }
  let added = 0
  let merged = 0
  let skipped = 0
  for (const item of incoming) {
    const key = messageMergeKey(item)
    const existing = key ? byKey.get(key) : item.id ? byKey.get(`id:${item.id}`) : undefined
    if (existing) {
      if (mergeMessageStateInto(existing, item)) merged += 1
      else skipped += 1
      continue
    }
    if (!key && !item.id) { skipped += 1; continue }
    items.push(item)
    if (key) byKey.set(key, item)
    if (item.id) byKey.set(`id:${item.id}`, item)
    added += 1
  }
  return { items, added, merged, skipped }
}

function mergeContactDeviceAndTrustState(current: ContactItem[], incoming: ContactItem[]): ContactItem[] {
  const byId = new Map(incoming.map((contact) => [contact.user_id, contact]))
  return current.map((contact) => {
    const other = byId.get(contact.user_id)
    if (!other || other.identity_public_key !== contact.identity_public_key) return contact
    const revoked = [...new Set([...(contact.revoked_device_ids ?? []), ...(other.revoked_device_ids ?? [])])]
    const revocationById = new Map<string, DeviceRevokeInfo>()
    for (const item of [...(other.device_revocations ?? []), ...(contact.device_revocations ?? [])]) {
      const existing = revocationById.get(item.device_id)
      if (!existing || item.created_at >= existing.created_at) revocationById.set(item.device_id, item)
    }
    const deviceById = new Map<string, DeviceCertItem>()
    for (const cert of [...(contact.device_certs ?? []), ...(other.device_certs ?? [])]) deviceById.set(cert.device_id, cert)
    return {
      ...contact,
      revoked_device_ids: revoked.length ? revoked : contact.revoked_device_ids,
      device_revocations: revocationById.size ? [...revocationById.values()] : contact.device_revocations,
      device_certs: deviceById.size ? [...deviceById.values()] : contact.device_certs,
      fingerprint_verified_at: contact.fingerprint_verified_at ?? other.fingerprint_verified_at,
      fingerprint_verified_note: contact.fingerprint_verified_note ?? other.fingerprint_verified_note,
      mailbox_hint_url: contact.mailbox_hint_url ?? other.mailbox_hint_url,
      last_prekey_dht_found_at: Math.max(contact.last_prekey_dht_found_at ?? 0, other.last_prekey_dht_found_at ?? 0) || contact.last_prekey_dht_found_at,
      last_mailbox_hint_dht_found_at: Math.max(contact.last_mailbox_hint_dht_found_at ?? 0, other.last_mailbox_hint_dht_found_at ?? 0) || contact.last_mailbox_hint_dht_found_at,
    }
  })
}

function mergeProcessedMailboxRecords(current: ProcessedMailboxRecord[], incoming: Array<string | ProcessedMailboxRecord> | undefined): ProcessedMailboxRecord[] {
  return normalizeProcessedMailboxRecords([...current, ...normalizeProcessedMailboxRecords(incoming)])
}

async function mergeSelfMailboxBackupNow() {
  if (!lastSelfMailboxBackupReceivedAt.value) throw new Error('尚未从自己的 Mailbox 收到完整备份')
  await importFullDataBackupMerge()
  lastSelfMailboxBackupMergedAt.value = Date.now()
  persist()
}

async function importFullDataBackupMerge() {
  try {
    if (!backupText.value || !passphrase.value) throw new Error('需要当前身份备份包和提示词')
    if (!dataBackupText.value.trim()) throw new Error('请粘贴完整数据备份')
    const json = import_data_backup(backupText.value, passphrase.value, dataBackupText.value)
    const state = JSON.parse(json) as PersistedState
    const contactMerge = mergeUniqueBy(contacts.value, state.contacts ?? [], (x) => x.user_id)
    const requestMerge = mergeUniqueBy(friendRequests.value, state.friendRequests ?? [], (x) => x.request_id)
    const groupMerge = mergeUniqueBy(groups.value, state.groups ?? [], (x) => x.group_id)
    const inviteMerge = mergeUniqueBy(groupInvites.value, state.groupInvites ?? [], (x) => x.invite_id)
    const senderKeyMerge = mergeUniqueBy(groupSenderKeys.value, state.groupSenderKeys ?? [], (x) => x.key_id)
    const messageMerge = mergeMessagesForState(messages.value, state.messages ?? [])
    const outboxMerge = mergeUniqueBy(outbox.value, state.outbox ?? [], (x) => x.id)
    const ratchetMerge = mergeUniqueBy(ratchetSessions.value, state.ratchetSessions ?? [], (x) => x.peer_user_id)
    contacts.value = mergeContactDeviceAndTrustState(contactMerge.items, state.contacts ?? [])
    friendRequests.value = requestMerge.items
    groups.value = groupMerge.items
    groupInvites.value = inviteMerge.items
    groupSenderKeys.value = senderKeyMerge.items
    messages.value = messageMerge.items
    outbox.value = outboxMerge.items
    ratchetSessions.value = ratchetMerge.items
    processedMailboxIds.value = mergeProcessedMailboxRecords(processedMailboxIds.value, state.processedMailboxIds)
    mailboxFailedItems.value = mergeUniqueBy(mailboxFailedItems.value, state.mailboxFailedItems ?? [], (x) => x.id).items
    syncRecoveryHistory.value = [...new Set([...syncRecoveryHistory.value, ...(state.syncRecoveryHistory ?? [])])].slice(0, 5)
    nodeDhtOperationHistory.value = [...new Set([...nodeDhtOperationHistory.value, ...(state.dhtOperationHistory ?? [])])].slice(0, DHT_OPERATION_HISTORY_MAX_RECORDS)
    friendRequestRateRecords.value = mergeUniqueBy(friendRequestRateRecords.value, state.friendRequestRateRecords ?? [], (x) => x.from_user_id).items
    if (typeof state.lastFullDataBackupAt === 'number') lastFullDataBackupAt.value = Math.max(lastFullDataBackupAt.value ?? 0, state.lastFullDataBackupAt)
    if (typeof state.lastSelfMailboxBackupPushedAt === 'number') lastSelfMailboxBackupPushedAt.value = Math.max(lastSelfMailboxBackupPushedAt.value ?? 0, state.lastSelfMailboxBackupPushedAt)
    if (typeof state.lastSelfMailboxBackupReceivedAt === 'number') lastSelfMailboxBackupReceivedAt.value = Math.max(lastSelfMailboxBackupReceivedAt.value ?? 0, state.lastSelfMailboxBackupReceivedAt)
    if (typeof state.lastSelfMailboxBackupMergedAt === 'number') lastSelfMailboxBackupMergedAt.value = Math.max(lastSelfMailboxBackupMergedAt.value ?? 0, state.lastSelfMailboxBackupMergedAt)
    processedSelfSyncIds.value = [...new Set([...processedSelfSyncIds.value, ...(state.processedSelfSyncIds ?? [])])].slice(0, 100)
    processedSelfSyncRequestIds.value = [...new Set([...processedSelfSyncRequestIds.value, ...(state.processedSelfSyncRequestIds ?? [])])].slice(0, 100)
    selfSyncMissingRequestRecords.value = [...(state.selfSyncMissingRequestRecords ?? []), ...selfSyncMissingRequestRecords.value]
      .filter((item, index, all) => item?.missing_sync_id && all.findIndex((x) => x.missing_sync_id === item.missing_sync_id) === index)
      .slice(0, 100)
    selfSyncRequestSentCount.value += Number(state.selfSyncRequestSentCount ?? 0)
    selfSyncRequestHitCount.value += Number(state.selfSyncRequestHitCount ?? 0)
    selfSyncRequestMissCount.value += Number(state.selfSyncRequestMissCount ?? 0)
    selfSyncRecentPackages.value = normalizeSelfSyncRecentPackages([...(state.selfSyncRecentPackages ?? []), ...selfSyncRecentPackages.value])
    if (typeof state.lastSelfSyncPushedAt === 'number') lastSelfSyncPushedAt.value = Math.max(lastSelfSyncPushedAt.value ?? 0, state.lastSelfSyncPushedAt)
    if (typeof state.lastSelfSyncMergedAt === 'number') lastSelfSyncMergedAt.value = Math.max(lastSelfSyncMergedAt.value ?? 0, state.lastSelfSyncMergedAt)
    lastSelfSyncSequenceSent.value = Math.max(lastSelfSyncSequenceSent.value, Number(state.lastSelfSyncSequenceSent ?? 0))
    lastSelfSyncSequenceMerged.value = Math.max(lastSelfSyncSequenceMerged.value, Number(state.lastSelfSyncSequenceMerged ?? 0))
    selfSyncGapCount.value += Number(state.selfSyncGapCount ?? 0)
    if (typeof state.lastSelfSyncGapAt === 'number' && state.lastSelfSyncGapAt > (lastSelfSyncGapAt.value ?? 0)) {
      lastSelfSyncGapAt.value = state.lastSelfSyncGapAt
      lastSelfSyncMissingPreviousId.value = state.lastSelfSyncMissingPreviousId ?? lastSelfSyncMissingPreviousId.value
    }
    unverifiedIncomingDropCount.value += Number(state.unverifiedIncomingDropCount ?? 0)
    if (typeof state.lastUnverifiedIncomingDropAt === 'number' && state.lastUnverifiedIncomingDropAt > (lastUnverifiedIncomingDropAt.value ?? 0)) {
      lastUnverifiedIncomingDropAt.value = state.lastUnverifiedIncomingDropAt
      lastUnverifiedIncomingDropFrom.value = state.lastUnverifiedIncomingDropFrom ?? ''
    }
    revokedDeviceIncomingDropCount.value += Number(state.revokedDeviceIncomingDropCount ?? 0)
    if (typeof state.lastRevokedDeviceIncomingDropAt === 'number' && state.lastRevokedDeviceIncomingDropAt > (lastRevokedDeviceIncomingDropAt.value ?? 0)) {
      lastRevokedDeviceIncomingDropAt.value = state.lastRevokedDeviceIncomingDropAt
      lastRevokedDeviceIncomingDropFrom.value = state.lastRevokedDeviceIncomingDropFrom ?? ''
    }
    if (!myDeviceCertJson.value && state.myDeviceCertJson) myDeviceCertJson.value = state.myDeviceCertJson
    if (!myDeviceBackupText.value && state.myDeviceBackupText) myDeviceBackupText.value = state.myDeviceBackupText
    if (!myDeviceId.value && state.myDeviceId) myDeviceId.value = state.myDeviceId
    if (!prekeyBundleText.value && state.prekeyBundleText) prekeyBundleText.value = state.prekeyBundleText
    if (!prekeyPrivateBundleJson.value && typeof state.prekeyPrivateBundleJson === 'string') prekeyPrivateBundleJson.value = state.prekeyPrivateBundleJson
    if (prekeySignedOneTimeRecordTexts.value.length === 0 && state.prekeySignedOneTimeRecordTexts) prekeySignedOneTimeRecordTexts.value = state.prekeySignedOneTimeRecordTexts
    const added = contactMerge.added + requestMerge.added + groupMerge.added + inviteMerge.added + senderKeyMerge.added + messageMerge.added + outboxMerge.added + ratchetMerge.added
    const skipped = contactMerge.skipped + requestMerge.skipped + groupMerge.skipped + inviteMerge.skipped + senderKeyMerge.skipped + messageMerge.skipped + outboxMerge.skipped + ratchetMerge.skipped
    persist()
    appendLog(`✅ 已合并完整数据备份：新增 ${added}，保留本机冲突 ${skipped}`)
    toast(`完整数据备份已合并：新增 ${added}`, 'success')
  } catch (e) {
    appendLog(`❌ 合并完整数据备份失败：${String(e)}`)
    showAlert('合并完整数据备份失败', userFacingError(e), 'error')
  }
}

function downloadText(value: string, filename: string) {
  run(`下载 ${filename}`, () => {
    if (!value) throw new Error('内容为空')
    const blob = new Blob([value], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    a.click()
    URL.revokeObjectURL(url)
  })
}

function exportSyncRecoveryHistory() {
  const text = JSON.stringify({
    exported_at: Date.now(),
    history: syncRecoveryHistory.value,
  }, null, 2)
  downloadText(text, `lm-talk-sync-recovery-${Date.now()}.json`)
}

function clearSyncRecoveryHistory() {
  syncRecoveryHistory.value = []
  syncRecoveryStatusText.value = '已清空恢复历史'
  appendLog('已清空跨触发器恢复历史')
  persist()
}


function warnIfFullDataBackupStale() {
  if (fullDataBackupFreshnessWarnedThisSession) return
  if (fullDataBackupFreshnessLevel.value === 'ok') return
  fullDataBackupFreshnessWarnedThisSession = true
  const message = `${fullDataBackupFreshnessText.value}。建议导出完整数据备份，避免设备丢失后无法恢复联系人信任和会话状态。`
  appendLog(`⚠️ ${message}`)
  toast(message, 'info')
}

async function afterLoginAutomation() {
  warnIfFullDataBackupStale()
  if (!nodeEnabled.value) return
  if (autoPublishPreKey.value) await ensurePreKeyInventory()
  await ensureOwnMailboxHintDhtRecord()
  await ensureOwnContactCardDhtRecord()
  await ensureOwnPublicPeerDhtRecord()
  if (autoMailboxTake.value) await takeMailboxFromNode()
}

async function syncNow() {
  if (!nodeEnabled.value) {
    appendLog('⚠️ 消息同步未开启')
    showAlert('未开启消息同步', '请先在“我 → 消息同步”开启同步。', 'warning')
    return
  }
  appendLog('🔄 开始消息同步')
  try {
    if (autoPublishPreKey.value) await ensurePreKeyInventory()
    await ensureOwnMailboxHintDhtRecord()
    await ensureOwnContactCardDhtRecord()
    await ensureOwnPublicPeerDhtRecord()
    if (autoNodeSync.value && nodeSyncPeerUrl.value.trim()) await autoPullSnapshotFromPeerNode()
    await autoRefreshStaleContactCardDht()
    await refreshOutgoingMailboxDeliveryStatusesFromNode()
    if (autoSelfMailboxSync.value) await pushSelfSyncPackageToOwnMailbox()
    await takeMailboxFromNode()
    await refreshOutgoingMailboxDeliveryStatusesFromNode()
    appendLog('✅ 消息同步完成')
  } catch (e) {
    const message = userFacingError(e)
    appendLog(`❌ 消息同步失败：${message}`)
    toast(`消息同步失败：${message}`, 'error')
    throw e
  }
}


function mailboxQuotaStatusFromResponse(body: any): { text: string; level: 'ok' | 'warning' | 'danger' } {
  const used = Number(body?.pending_bytes ?? body?.summary?.bytes ?? 0)
  const maxRaw = body?.max_bytes_per_user
  const max = maxRaw === null || maxRaw === undefined ? null : Number(maxRaw)
  if (!Number.isFinite(used) || used < 0) return { text: '', level: 'ok' }
  if (!max || !Number.isFinite(max) || max <= 0) {
    return { text: `Mailbox 容量：${formatBytes(used)} / 未设置上限`, level: 'ok' }
  }
  const ratio = Math.max(0, Math.round((used / max) * 100))
  const cappedRatio = Math.min(100, ratio)
  if (ratio >= 100) {
    return {
      text: `Mailbox 容量：${formatBytes(used)} / ${formatBytes(max)} (${cappedRatio}%)，已达上限，请同步或清理`,
      level: 'danger',
    }
  }
  if (ratio >= 80) {
    return {
      text: `Mailbox 容量：${formatBytes(used)} / ${formatBytes(max)} (${cappedRatio}%)，接近上限`,
      level: 'warning',
    }
  }
  return { text: `Mailbox 容量：${formatBytes(used)} / ${formatBytes(max)} (${cappedRatio}%)`, level: 'ok' }
}

function updateMailboxQuotaStatus(body: any) {
  const status = mailboxQuotaStatusFromResponse(body)
  if (status.text) {
    mailboxQuotaStatusText.value = status.text
    mailboxQuotaPressureLevel.value = status.level
  }
}

async function refreshOutgoingMailboxDeliveryStatusesFromNode() {
  if (!nodeEnabled.value || !identity.value) return
  const candidates = messages.value.filter((message) =>
    message.direction === 'out'
    && message.mailbox_delivery_id
    && message.peer_user_id
    && (message.status === 'mailbox' || message.status === 'delivered')
  )
  if (candidates.length === 0) return
  let updated = 0
  let failed = 0
  for (const message of candidates) {
    try {
      const body = await nodeFetchJson(`/mailbox/status?user_id=${encodeURIComponent(message.peer_user_id)}&delivery_id=${encodeURIComponent(message.mailbox_delivery_id || '')}`)
      updateMailboxQuotaStatus(body)
      const status = String(body?.delivery?.status ?? '')
      if ((status === 'delivered_unacked' || status === 'acked') && message.status !== 'read') {
        if (message.status !== 'delivered') updated += 1
        message.status = 'delivered'
        message.delivered_at = message.delivered_at || Date.now()
      } else if (status === 'pending' && message.status !== 'read') {
        message.status = 'mailbox'
      }
    } catch (e) {
      failed += 1
      appendLog(`⚠️ 查询 Mailbox 投递状态失败：${userFacingError(e)}`)
    }
  }
  if (updated > 0 || failed > 0) {
    appendLog(`Mailbox 投递状态刷新：更新 ${updated}，失败 ${failed}`)
    persist()
  }
}

type NodeEntry = { url: string; token: string }
type NodeEntrySummary = { url: string; token_configured: boolean; missing_remote_token: boolean }
class NodeRequestError extends Error {
  status?: number
  url?: string

  constructor(message: string, status?: number, url?: string) {
    super(message)
    this.name = 'NodeRequestError'
    this.status = status
    this.url = url
  }
}

function nodeEntries(): NodeEntry[] {
  return nodeControlUrl.value
    .split(/[\n,]+/)
    .map((x, index) => ({ raw: x.trim(), line: index + 1 }))
    .filter((item) => item.raw)
    .map(({ raw, line }) => {
      // 每行：<url> 或 <url>|<令牌>（令牌对应节点的 --control-token）
      const [url, token] = raw.split('|').map((s) => s.trim())
      try {
        const parsed = new URL(url)
        if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') throw new Error('protocol')
      } catch {
        throw new Error(`同步服务第 ${line} 行地址无效，请使用 http:// 或 https:// 开头的完整地址`)
      }
      return { url, token: token || '' }
    })
    .filter((e) => e.url)
}
function nodeEntryLine(e: NodeEntry): string { return e.token ? `${e.url}|${e.token}` : e.url }
function nodeUrlList(): string[] {
  return nodeEntries().map((e) => e.url)
}
function isLoopbackNodeUrl(url: string): boolean {
  try {
    const host = new URL(url).hostname.toLowerCase()
    return host === 'localhost' || host === '127.0.0.1' || host === '::1' || host === '[::1]'
  } catch {
    return false
  }
}
const nodeSettingsSummaryText = computed(() => {
  let entries: NodeEntry[]
  try {
    entries = nodeEntries()
  } catch (e) {
    return userFacingError(e)
  }
  if (entries.length === 0) return '未配置同步服务'
  const tokenCount = nodeTokenCount.value
  const missingRemoteTokens = nodeMissingRemoteTokenCount.value
  return `${entries.length} 个同步服务 · 主节点 ${entries[0].url}${tokenCount ? ` · ${tokenCount} 个已配置令牌` : ''}${missingRemoteTokens ? ` · ${missingRemoteTokens} 个远端缺令牌` : ''} · 成功节点会自动置顶`
})
const nodeEntrySummaries = computed<NodeEntrySummary[]>(() => {
  try {
    return nodeEntries().map((entry) => ({
      url: entry.url,
      token_configured: Boolean(entry.token),
      missing_remote_token: !entry.token && !isLoopbackNodeUrl(entry.url),
    }))
  } catch {
    return []
  }
})
const nodeTokenCount = computed(() => {
  try {
    return nodeEntries().filter((entry) => entry.token).length
  } catch {
    return 0
  }
})
const nodeMissingRemoteTokenCount = computed(() => {
  try {
    return nodeEntries().filter((entry) => !entry.token && !isLoopbackNodeUrl(entry.url)).length
  } catch {
    return 0
  }
})
const nodeTokenStorageText = computed(() => {
  try {
    nodeEntries()
  } catch {
    return ''
  }
  const tokenCount = nodeTokenCount.value
  return tokenCount ? `已配置 ${tokenCount} 个令牌；令牌只保存在本机浏览器，诊断报告默认不导出。` : ''
})
function nodeTokenFor(url: string): string {
  const base = url.replace(/\/$/, '')
  return nodeEntries().find((e) => e.url.replace(/\/$/, '') === base)?.token || ''
}

function primaryNodeUrl(): string {
  return nodeUrlList()[0] ?? ''
}

function saveNetworkSettings(): boolean {
  try {
    const entries = nodeEntries()
    if (entries.length > 0) nodeControlUrl.value = entries.map(nodeEntryLine).join('\n')
    persist()
    appendLog(`✅ 已保存消息同步设置：${nodeEnabled.value ? '启用' : '停用'} ${entries.length ? `${entries.length} 个节点` : '未填写节点'}`)
    return true
  } catch (e) {
    const message = userFacingError(e)
    appendLog(`❌ 保存消息同步设置: ${message}`)
    showAlert('保存消息同步设置', message, 'error')
    return false
  }
}

function toggleNodeEnabled() {
  nodeEnabled.value = !nodeEnabled.value
  if (!saveNetworkSettings()) {
    nodeEnabled.value = !nodeEnabled.value
    return
  }
  if (nodeEnabled.value) void checkNodeHealth()
}

async function autoPublishPreKeyIfEnabled() {
  if (!nodeEnabled.value || !autoPublishPreKey.value || !loggedIn.value) return
  await ensurePreKeyInventory()
}

function addDiscoveredMailboxHintToSyncServices() {
  const url = discoveredMailboxHintUrl.value.trim().replace(/\/$/, '')
  if (!url) throw new Error('没有可加入的 MailboxHint URL')
  const entries = nodeEntries()
  const alreadyConfigured = entries.some((entry) => entry.url === url)
  if (!alreadyConfigured) {
    nodeControlUrl.value = [...entries.map(nodeEntryLine), url].join('\n')
  }
  if (saveNetworkSettings()) {
    mailboxInboxStatus.value = alreadyConfigured
      ? `MailboxHint 同步服务已存在：${url}`
      : `已加入 MailboxHint 同步服务：${url}`
  }
}

function markContactDhtDiscoveryAttempt(contact: ContactItem) {
  contact.last_dht_discovery_attempt_at = Date.now()
  contact.last_dht_discovery_error = undefined
  contact.last_dht_discovery_error_kind = undefined
  contact.dht_discovery_risk_level = undefined
}

function resetContactDhtDiscoveryBackoff(contact: ContactItem) {
  contact.dht_discovery_failure_count = 0
  contact.next_dht_discovery_retry_at = undefined
  contact.last_dht_discovery_error = undefined
  contact.last_dht_discovery_error_kind = undefined
  contact.dht_discovery_risk_level = undefined
}

function markContactDhtDiscoverySuccess(contact: ContactItem, kind: 'prekey' | 'mailbox-hint' | 'contact-card') {
  contact.last_dht_discovery_success_at = Date.now()
  contact.last_dht_discovery_error = undefined
  contact.dht_discovery_failure_count = 0
  contact.next_dht_discovery_retry_at = undefined
  contact.last_dht_discovery_error_kind = undefined
  contact.dht_discovery_risk_level = undefined
  if (kind === 'prekey') contact.last_prekey_dht_found_at = contact.last_dht_discovery_success_at
  if (kind === 'mailbox-hint') contact.last_mailbox_hint_dht_found_at = contact.last_dht_discovery_success_at
  if (kind === 'contact-card') contact.last_contact_card_dht_found_at = contact.last_dht_discovery_success_at
}

function classifyDhtDiscoveryError(error: unknown): ContactItem['last_dht_discovery_error_kind'] {
  const message = userFacingError(error)
  if (/过期/.test(message)) return 'expired'
  if (/验签|签名|signature/i.test(message)) return 'signature'
  if (/未找到|无记录|not found/i.test(message)) return 'not-found'
  if (/record|格式|异常|不一致|invalid/i.test(message)) return 'invalid-record'
  if (/超时|网络|连接|不可用|timeout|failed to fetch/i.test(message)) return 'network'
  return 'unknown'
}

function dhtDiscoveryRiskLevel(kind: ContactItem['last_dht_discovery_error_kind']): ContactItem['dht_discovery_risk_level'] {
  if (kind === 'signature' || kind === 'invalid-record') return 'high'
  if (kind === 'expired') return 'medium'
  return 'low'
}

function dhtDiscoveryRetryDelayMs(kind: ContactItem['last_dht_discovery_error_kind'], failures: number): number {
  if (kind === 'signature' || kind === 'invalid-record') return 60 * 60_000
  if (kind === 'not-found' || kind === 'expired') return Math.min(60 * 60_000, 5 * 60_000 * 2 ** (failures - 1))
  if (kind === 'network') return Math.min(10 * 60_000, 15_000 * 2 ** (failures - 1))
  return Math.min(15 * 60_000, 30_000 * 2 ** (failures - 1))
}

function markContactDhtDiscoveryError(contact: ContactItem, error: unknown, kind?: ContactItem['last_dht_discovery_error_kind']) {
  const now = Date.now()
  const errorKind = kind || classifyDhtDiscoveryError(error)
  const failures = Math.min(8, (contact.dht_discovery_failure_count ?? 0) + 1)
  contact.dht_discovery_failure_count = failures
  contact.next_dht_discovery_retry_at = now + dhtDiscoveryRetryDelayMs(errorKind, failures)
  contact.last_dht_discovery_error = userFacingError(error)
  contact.last_dht_discovery_error_kind = errorKind
  contact.dht_discovery_risk_level = dhtDiscoveryRiskLevel(errorKind)
}

function contactDhtDiscoveryRetryWaitMs(contact: ContactItem): number {
  const retryAt = contact.next_dht_discovery_retry_at ?? 0
  return retryAt > Date.now() ? retryAt - Date.now() : 0
}

function shouldSkipAutoContactDhtDiscovery(contact: ContactItem): boolean {
  return contactDhtDiscoveryRetryWaitMs(contact) > 0
}

async function discoverMailboxHintForContact(contact: ContactItem): Promise<string | undefined> {
  if (contact.mailbox_hint_url?.trim()) return contact.mailbox_hint_url.trim()
  if (shouldSkipAutoContactDhtDiscovery(contact)) return undefined
  markContactDhtDiscoveryAttempt(contact)
  try {
    nodeDhtKeyKind.value = 'mailbox-hint'
    nodeDhtKeyValue.value = contact.user_id
    const body = await nodeFetchJson(`/dht/find-value?kind=mailbox-hint&value=${encodeURIComponent(contact.user_id)}&limit=8&max_peers=8&alpha=3`)
    applyDhtFindValueRecord(body)
    if (contact.mailbox_hint_url?.trim()) {
      markContactDhtDiscoverySuccess(contact, 'mailbox-hint')
      appendLog(`✅ 已通过 DHT 发现 ${contact.display_name || contact.user_id} 的 MailboxHint`)
      persist()
      return contact.mailbox_hint_url.trim()
    }
  } catch (error) {
    markContactDhtDiscoveryError(contact, error)
    appendLog(`⚠️ DHT 查找联系人 MailboxHint 失败：${userFacingError(error)}`)
    persist()
  }
  return undefined
}

async function prepareContactDhtForSend(contact: ContactItem, options: { prekey?: boolean } = {}) {
  if (!nodeEnabled.value || contact.state !== 'Friend') return
  const errors: string[] = []
  if (options.prekey && !ratchetSessionFor(contact.user_id)) {
    try {
      await ensureRatchetSessionFromNode(contact)
    } catch (error) {
      errors.push(`PreKey：${userFacingError(error)}`)
    }
  }
  try {
    await discoverMailboxHintForContact(contact)
  } catch (error) {
    errors.push(`MailboxHint：${userFacingError(error)}`)
  }
  if (errors.length) appendLog(`⚠️ 发送前 DHT 预发现未完全成功：${errors.join('；')}`)
}

async function pushMailboxPayload(to: ContactItem, kind: string, payload: string): Promise<string> {
  if (!nodeEnabled.value) throw new Error('节点未启用')
  const msg = create_mailbox_message(
    backupText.value,
    passphrase.value,
    to.user_id,
    kind,
    payload,
    BigInt(24 * 3600),
  )
  const init = {
    method: 'POST',
    body: JSON.stringify({
      message_text: msg,
      from_identity_public_key: identity.value?.identity_public_key,
    }),
  }
  const discoveredHint = await discoverMailboxHintForContact(to)
  const preferredMailboxUrl = discoveredHint?.trim().replace(/\/$/, '')
  let body: any
  if (preferredMailboxUrl && /^https?:\/\//i.test(preferredMailboxUrl)) {
    try {
      body = await fetchNodeOnce(preferredMailboxUrl, '/mailbox/push', init)
    } catch (error) {
      appendLog(`⚠️ 联系人 MailboxHint 投递失败，回退同步服务：${userFacingError(error)}`)
    }
  }
  if (!body) body = await nodeFetchJson('/mailbox/push', init)
  nodeControlStatus.value = JSON.stringify(body, null, 2)
  updateMailboxQuotaStatus(body)
  return String(body.delivery_id ?? '')
}

function createOutboxItem(contact: ContactItem, payload: string, messageId?: string, kind: OutboxItem['kind'] = 'direct-envelope'): OutboxItem {
  const now = Date.now()
  return {
    id: newId(),
    peer_user_id: contact.user_id,
    envelope_json: payload,
    message_id: messageId,
    kind,
    status: 'queued',
    created_at: now,
    retry_count: 0,
    next_retry_at: now,
    expires_at: now + 7 * 24 * 3600 * 1000,
  }
}

function queueOutboxItem(contact: ContactItem, payload: string, messageId?: string, kind: OutboxItem['kind'] = 'direct-envelope'): OutboxItem {
  const existing = outbox.value.find((item) =>
    item.peer_user_id === contact.user_id
    && item.kind === kind
    && item.envelope_json === payload
    && item.status !== 'sent'
  )
  if (existing) {
    existing.status = 'queued'
    existing.next_retry_at = Date.now()
    existing.last_error = undefined
    if (messageId) existing.message_id = messageId
    return existing
  }
  const item = createOutboxItem(contact, payload, messageId, kind)
  outbox.value.push(item)
  return item
}

function mailboxKindForOutboxKind(kind: OutboxItem['kind']): string {
  if (kind === 'group-fanout') return 'group-fanout'
  if (kind === 'delivery-receipt') return 'delivery-receipt'
  if (kind === 'read-receipt') return 'read-receipt'
  if (kind === 'contact-update') return 'contact-update'
  if (kind === 'file-package') return 'other'
  if (kind === 'other') return 'other'
  return 'direct-envelope'
}

function readReceiptsEnabledFor(contact: ContactItem): boolean {
  if (contact.read_receipts === 'enabled') return true
  if (contact.read_receipts === 'disabled') return false
  return autoReadReceipts.value
}

function setActiveContactReadReceipts(value: 'default' | 'enabled' | 'disabled') {
  if (!activeContact.value) return
  activeContact.value.read_receipts = value
  appendLog(`已更新联系人已读回执策略：${value === 'default' ? '跟随全局' : value === 'enabled' ? '始终开启' : '关闭'}`)
  persist()
}

function messageProtocolIdFromEnvelope(envelope: string): string | undefined {
  try {
    const parsed = JSON.parse(envelope) as { message_id?: string }
    return typeof parsed.message_id === 'string' ? parsed.message_id : undefined
  } catch { return undefined }
}

function conversationIdFromEnvelope(envelope: string): string | undefined {
  try {
    const parsed = JSON.parse(envelope) as { conversation_id?: string }
    return typeof parsed.conversation_id === 'string' ? parsed.conversation_id : undefined
  } catch { return undefined }
}



function normalizeContactCardUpdateFanoutRecords(records: ContactCardUpdateFanoutRecord[]): ContactCardUpdateFanoutRecord[] {
  return (records ?? [])
    .filter((record) => record?.peer_user_id && record?.update_id && record?.sent_at)
    .sort((a, b) => Number(b.sent_at ?? 0) - Number(a.sent_at ?? 0))
    .slice(0, 100)
}

function contactCardUpdateRecordIsStale(record: ContactCardUpdateFanoutRecord, now = Date.now()): boolean {
  return record.status !== 'acked' && now - Number(record.last_retry_at || record.sent_at || 0) >= CONTACT_CARD_UPDATE_ACK_STALE_MS
}

function contactCardUpdateId(cardText: string): string {
  let hash = 2166136261
  for (let i = 0; i < cardText.length; i += 1) {
    hash ^= cardText.charCodeAt(i)
    hash = Math.imul(hash, 16777619)
  }
  return `contact-card-update:${identity.value?.user_id || 'unknown'}:${(hash >>> 0).toString(16)}`
}

function rememberContactCardUpdateFanout(peerUserId: string, updateId: string, status: 'sent' | 'queued') {
  const existing = contactCardUpdateFanoutRecords.value.find((item) => item.peer_user_id === peerUserId && item.update_id === updateId)
  if (existing) {
    existing.status = existing.status === 'acked' ? 'acked' : status
    existing.sent_at = Date.now()
  } else {
    contactCardUpdateFanoutRecords.value = normalizeContactCardUpdateFanoutRecords([{ peer_user_id: peerUserId, update_id: updateId, status, sent_at: Date.now(), retry_count: 0 }, ...contactCardUpdateFanoutRecords.value])
  }
}

function markContactCardUpdateAck(updateId: string, fromUserId: string): boolean {
  const record = contactCardUpdateFanoutRecords.value.find((item) => item.peer_user_id === fromUserId && item.update_id === updateId)
  if (!record) return false
  record.status = 'acked'
  record.acked_at = record.acked_at || Date.now()
  appendLog(`✅ 联系人设备证书更新已被 ${fromUserId} 确认合并`)
  persist()
  return true
}

const contactCardUpdateFanoutAckCount = computed(() => contactCardUpdateFanoutRecords.value.filter((item) => item.status === 'acked').length)
const contactCardUpdatePendingAckCount = computed(() => contactCardUpdateFanoutRecords.value.filter((item) => item.status !== 'acked').length)
const contactCardUpdateStaleAckCount = computed(() => contactCardUpdateFanoutRecords.value.filter((item) => contactCardUpdateRecordIsStale(item)).length)

function applyDeliveryAck(messageId: string, fromUserId: string) {
  if (messageId.startsWith('contact-card-update:') && markContactCardUpdateAck(messageId, fromUserId)) return
  const msg = messages.value.find((m) => m.direction === 'out' && m.protocol_message_id === messageId && m.peer_user_id === fromUserId)
  if (msg) {
    if (msg.status !== 'read') msg.status = 'delivered'
    msg.delivered_at = msg.delivered_at || Date.now()
    appendLog(`✅ 收到送达回执：${fromUserId}`)
    persist()
  }
}

function applyMessageReceiptText(receiptText: string, sender: ContactItem): 'delivery-ack' | 'read-receipt' {
  const info = JSON.parse(inspect_message_receipt(receiptText, sender.identity_public_key)) as {
    from_user_id: string
    to_user_id: string
    target_message_id: string
    kind: 'Delivered' | 'Read'
  }
  if (identity.value && info.to_user_id !== identity.value.user_id) throw new Error('回执不是发给当前身份的')
  if (info.from_user_id !== sender.user_id) throw new Error('回执发送者与 Mailbox 发送者不一致')
  if (info.kind === 'Read') {
    const msg = messages.value.find((m) => m.direction === 'out' && m.protocol_message_id === info.target_message_id && m.peer_user_id === sender.user_id)
    if (msg) {
      msg.status = 'read'
      msg.delivered_at = msg.delivered_at || Date.now()
      msg.read_at = msg.read_at || Date.now()
      appendLog(`✅ 收到已读回执：${sender.display_name || sender.user_id}`)
      persist()
    }
  } else {
    applyDeliveryAck(info.target_message_id, sender.user_id)
  }
  return info.kind === 'Read' ? 'read-receipt' : 'delivery-ack'
}

async function sendDeliveryAck(sender: ContactItem, messageId?: string, conversationId?: string, mailboxDeliveryId?: string) {
  if (!messageId || !identity.value) return
  const ack = create_message_receipt(
    backupText.value,
    passphrase.value,
    sender.user_id,
    messageId,
    conversationId || `conv-${sender.user_id}`,
    mailboxDeliveryId || undefined,
    'delivered',
    BigInt(24 * 3600),
  )
  const result = await deliverPayloadToContact(sender, ack, '送达回执', 'delivery-receipt')
  if (result === 'queued' || result === 'failed') queueOutboxItem(sender, ack, undefined, 'delivery-receipt')
  persist()
}

async function sendReadReceipt(sender: ContactItem, messageId?: string, conversationId?: string, mailboxDeliveryId?: string) {
  if (!messageId || !identity.value || !readReceiptsEnabledFor(sender)) return
  const receipt = create_message_receipt(
    backupText.value,
    passphrase.value,
    sender.user_id,
    messageId,
    conversationId || `conv-${sender.user_id}`,
    mailboxDeliveryId || undefined,
    'read',
    BigInt(24 * 3600),
  )
  const result = await deliverPayloadToContact(sender, receipt, '已读回执', 'read-receipt')
  if (result === 'queued' || result === 'failed') queueOutboxItem(sender, receipt, undefined, 'read-receipt')
  persist()
}

function resendAckForDuplicateMailboxMessage(message: any, deliveryId?: string): boolean {
  const fromUserId = String(message?.from_user_id ?? '')
  const sender = contactByUserId(fromUserId)
  const kind = typeof message?.kind === 'string' ? message.kind.replace(/[-_]/g, '').toLowerCase() : ''
  const ciphertext = String(message?.ciphertext ?? '')
  if (!sender || kind !== 'directenvelope') return false
  void sendDeliveryAck(sender, messageProtocolIdFromEnvelope(ciphertext), conversationIdFromEnvelope(ciphertext), deliveryId)
  return true
}

function retryDelayMs(retryCount: number): number {
  return [30_000, 2 * 60_000, 10 * 60_000, 60 * 60_000, 6 * 60 * 60_000][Math.min(retryCount, 4)]
}

function classifyDeliveryError(e: unknown): string {
  if (e instanceof NodeRequestError) {
    if (e.status === 401 || e.status === 403) return '节点拒绝：鉴权失败'
    if (e.status === 413) return '节点拒绝：载荷过大'
    if (e.status === 429) return '节点拒绝：请求过于频繁'
    if (typeof e.status === 'number' && e.status >= 500) return '节点错误'
    if (typeof e.status === 'number' && e.status >= 400) return '节点拒绝'
    return '网络失败'
  }
  const message = userFacingError(e)
  if (message.includes('无法连接同步服务') || message.includes('所有同步服务都不可用')) return '网络失败'
  if (message.includes('过大')) return '载荷过大'
  if (message.includes('已过期')) return '已过期'
  return message || '投递失败'
}

function markOutboxSent(item: OutboxItem) {
  item.status = 'sent'
  const msg = messages.value.find((m) => m.id === item.message_id)
  if (msg) msg.status = item.kind === 'direct-envelope' ? 'sent' : 'mailbox'
}

async function deliverPayloadToContact(contact: ContactItem, payload: string, label: string, kind: OutboxItem['kind'] = 'direct-envelope'): Promise<'sent' | 'mailbox' | 'queued' | 'failed'> {
  lastDeliveryError = ''
  try {
    requireVerifiedContactForSend(contact)
  } catch (e) {
    lastDeliveryError = classifyDeliveryError(e)
    appendLog(`❌ ${label} 投递被安全策略阻止：${lastDeliveryError}`)
    return 'failed'
  }
  try {
    if (dc && dc.readyState === 'open' && activePeerId.value === contact.user_id && kind !== 'group-fanout') {
      sendRtcText(payload, label)
      return 'sent'
    }
    if (nodeEnabled.value) {
      await prepareContactDhtForSend(contact, { prekey: kind === 'direct-envelope' })
      await pushMailboxPayload(contact, mailboxKindForOutboxKind(kind), payload)
      return 'mailbox'
    }
    return 'queued'
  } catch (e) {
    lastDeliveryError = classifyDeliveryError(e)
    appendLog(`❌ ${label} 投递失败：${lastDeliveryError}`)
    return 'failed'
  }
}

async function tryMailboxDeliveryForMessage(contact: ContactItem, envelope: string, msg: ChatMessage) {
  try {
    await prepareContactDhtForSend(contact, { prekey: true })
    const deliveryId = await pushMailboxPayload(contact, 'direct-envelope', envelope)
    msg.status = 'mailbox'
    if (deliveryId) msg.mailbox_delivery_id = deliveryId
    appendLog(`✅ 已通过 mailbox 投递${deliveryId ? '：' + deliveryId : ''}`)
  } catch (e) {
    msg.status = 'failed'
    const item = createOutboxItem(contact, envelope, msg.id, 'direct-envelope')
    item.last_error = classifyDeliveryError(e)
    outbox.value.push(item)
    appendLog(`❌ mailbox 投递失败，已加入 outbox：${item.last_error}`)
  } finally {
    persist()
  }
}

async function ensureRatchetSessionFromNode(contact: ContactItem): Promise<boolean> {
  requireContactHasActiveDevice(contact)
  if (!nodeEnabled.value || ratchetSessionFor(contact.user_id)) return Boolean(ratchetSessionFor(contact.user_id))
  let body = await nodeFetchJson(`/prekey/get?user_id=${encodeURIComponent(contact.user_id)}&consume=true`)
  nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
  if (!body.found || !body.prekey_bundle_text) {
    try {
      if (shouldSkipAutoContactDhtDiscovery(contact)) return false
      markContactDhtDiscoveryAttempt(contact)
      nodeDhtKeyKind.value = 'prekey'
      nodeDhtKeyValue.value = contact.user_id
      const dht = await nodeFetchJson(`/dht/find-value?kind=prekey&value=${encodeURIComponent(contact.user_id)}&limit=8&max_peers=8&alpha=3`)
      if (applyDhtFindValueRecord(dht) && dht?.record?.kind === 'PreKey' && typeof dht.record.value === 'string') {
        body = { found: true, prekey_bundle_text: dht.record.value }
        markContactDhtDiscoverySuccess(contact, 'prekey')
        nodePreKeyStatusText.value = JSON.stringify({ found: true, source: 'dht', record: dht.record }, null, 2)
        appendLog(`✅ 已通过 DHT 发现 ${contact.display_name || contact.user_id} 的 PreKey`)
      }
    } catch (error) {
      markContactDhtDiscoveryError(contact, error)
      appendLog(`⚠️ DHT 查找联系人 PreKey 失败：${userFacingError(error)}`)
    }
  }
  if (!body.found || !body.prekey_bundle_text) return false
  prekeyBundleText.value = body.prekey_bundle_text
  selectedOneTimePreKeyId.value = typeof body.selected_one_time_prekey_id === 'number' ? body.selected_one_time_prekey_id : null
  selectedSignedOneTimePreKeyRecordText.value = typeof body.selected_signed_one_time_prekey_record_text === 'string' ? body.selected_signed_one_time_prekey_record_text : ''
  const init = JSON.parse(selectedSignedOneTimePreKeyRecordText.value
    ? create_x3dh_initial_message_with_one_time_prekey_record(
      backupText.value,
      passphrase.value,
      body.prekey_bundle_text,
      selectedSignedOneTimePreKeyRecordText.value,
    )
    : create_x3dh_initial_message_with_one_time_prekey_id(
      backupText.value,
      passphrase.value,
      body.prekey_bundle_text,
      selectedOneTimePreKeyId.value ?? undefined,
    )) as { initial_message_json: string; shared_secret: string }
  const pair = JSON.parse(create_ratchet_dh_keypair()) as { private_key: string; public_key: string }
  const ratchetPair = JSON.parse(create_ratchet_session_from_shared_secret(
    identity.value!.user_id,
    contact.user_id,
    init.shared_secret,
  )) as { local_state_text: string; remote_state_text: string }
  saveRatchetSession(contact.user_id, ratchetPair.local_state_text)
  contact.last_secure_session_success_at = Date.now()
  const response: SecureSessionResponse = {
    type: 'lm-secure-session-response-v1',
    version: 1,
    from_user_id: identity.value!.user_id,
    to_user_id: contact.user_id,
    initial_message_json: init.initial_message_json,
    ratchet_dh_public_key: pair.public_key,
    created_at: Date.now(),
  }
  secureSessionResponseText.value = JSON.stringify(response, null, 2)
  try {
    await pushMailboxPayload(contact, 'other', secureSessionResponseText.value)
    contact.last_secure_session_error = undefined
    contact.secure_session_failure_count = 0
  } catch (e) {
    recordSecureSessionError(contact, e, '⚠️ 安全会话 Response 发送失败')
    throw e
  }
  appendLog(`✅ 已从节点拉取 PreKey 并为 ${contact.display_name || contact.user_id} 建立会话初始化消息`)
  persist()
  return true
}



async function removeLocalIdentity(id: string) {
  const item = localIdentities.value.find((x) => x.id === id)
  if (!item) return
  const ok = await showConfirm(
    '删除本地身份',
    `删除本地身份「${item.display_name || item.user_id}」？这会删除本机保存的登录入口和该身份在本浏览器中的加密联系人、群聊、消息、待发送队列与设置。请确认你已保存身份文件和必要的数据备份。`,
    true,
  )
  if (!ok) return
  await purgeAccountTables(item.user_id)
  localIdentities.value = localIdentities.value.filter((x) => x.id !== id)
  if (selectedLocalIdentityId.value === id) selectedLocalIdentityId.value = localIdentities.value[0]?.id ?? ''
  if (lastRegisteredIdentity.value?.id === id) lastRegisteredIdentity.value = null
  saveLocalIdentityList()
  appendLog('✅ 已删除本地身份及本机加密数据')
  toast('已删除本地身份及本机数据', 'success')
}

function resetRegisterForm() {
  lastRegisteredIdentity.value = null
  displayName.value = ''
  passphrase.value = ''
  void router.push('/register')
}

function loginSelectedIdentity() {
  const selected = selectedLocalIdentity()
  if (selected) {
    backupText.value = selected.backup_text
    displayName.value = selected.display_name || displayName.value
  }
  restoreAndEnter()
}

function importIdentityOnly() {
  run('导入身份', () => {
    if (!backupText.value.trim()) throw new Error('请粘贴身份文本')
    if (!passphrase.value.trim()) throw new Error('请输入提示词')
    const out = safeJson<RestoreOutput>(restore_identity(backupText.value, passphrase.value))
    rememberLocalIdentity(out.user_id, displayName.value || 'Me', backupText.value)
    passphrase.value = ''
    appendLog('✅ 身份已导入，请在登录页登录')
    toast('身份已导入，请登录', 'success')
    void router.push('/login')
  })
}

function createIdentityAndEnter() {
  run('注册身份', () => {
    if (!passphrase.value.trim()) throw new Error('请输入提示词')
    const registerName = displayName.value.trim() || 'Me'
    displayName.value = registerName
    const out = safeJson<IdentityOutput>(create_identity(passphrase.value))
    identity.value = out
    backupText.value = out.backup_text
    exportMyCard()
    lastRegisteredIdentity.value = rememberLocalIdentity(out.user_id, registerName, out.backup_text)
    persist()
    loggedIn.value = false
    passphrase.value = ''
    appendLog('✅ 注册完成，请下载身份文件，然后返回登录')
    toast('注册成功', 'success')
  })
}

function restoreAndEnter() {
  run('登录', () => {
    if (!backupText.value.trim()) throw new Error('请粘贴身份备份包')
    if (!passphrase.value.trim()) throw new Error('请输入提示词')
    const loginBackup = backupText.value
    const loginName = displayName.value || 'Me'
    const out = safeJson<RestoreOutput>(restore_identity(loginBackup, passphrase.value))
    identity.value = out
    resetAccountScopedState()
    backupText.value = loginBackup
    displayName.value = loginName
    void loadPersistedState()
      .then(() => {
        loggedIn.value = true
        if (!myContactCardText.value) exportMyCard()
        rememberLocalIdentity(out.user_id, displayName.value, backupText.value)
        persist()
        void router.push('/chat')
        void afterLoginAutomation()
      })
      .catch((e) => {
        const message = userFacingError(e)
        appendLog(`❌ 登录失败：${message}`)
        showAlert('登录失败', message, 'error')
      })
  })
}

function exportMyCard() {
  const certById = new Map<string, DeviceCertItem>()
  for (const cert of contactCardDeviceCerts(myContactCardText.value || '')) certById.set(cert.device_id, cert)
  if (myDeviceCertJson.value) {
    const cert = safeJson<DeviceCertItem>(myDeviceCertJson.value)
    if (cert?.device_id) certById.set(cert.device_id, cert)
  }
  const certs = certById.size ? JSON.stringify([...certById.values()]) : undefined
  myContactCardText.value = export_contact_card(backupText.value, passphrase.value, displayName.value || undefined, certs)
}

function refreshMyContactCard() {
  run('更新我的 Contact Card', () => {
    if (!backupText.value || !passphrase.value) throw new Error('请先登录')
    exportMyCard()
    if (identity.value) rememberLocalIdentity(identity.value.user_id, displayName.value || 'Me', backupText.value)
    persist()
    appendLog('✅ 我的 Contact Card 已更新，可重新发布/发送给好友')
  })
}

function reencryptCurrentIdentityBackup() {
  run('重加密身份备份', () => {
    if (!identity.value) throw new Error('请先登录')
    if (!backupText.value || !passphrase.value) throw new Error('需要当前身份备份和当前提示词')
    if (!newIdentityPassphrase.value.trim()) throw new Error('请输入新提示词')
    const out = safeJson<ReencryptIdentityBackupOutput>(reencrypt_identity_backup(
      backupText.value,
      passphrase.value,
      newIdentityPassphrase.value,
    ))
    if (out.user_id !== identity.value.user_id) throw new Error('重加密后的身份不匹配')
    backupText.value = out.backup_text
    passphrase.value = newIdentityPassphrase.value
    newIdentityPassphrase.value = ''
    rememberLocalIdentity(out.user_id, displayName.value || 'Me', out.backup_text)
    persist()
    appendLog('✅ 已用新提示词重加密身份备份')
    toast('身份备份已重加密，请重新导出保存', 'success')
  })
}

function mergeContactCard(existing: ContactItem | undefined, info: ContactInfo, cardText: string): ContactItem {
  if (existing && existing.identity_public_key !== info.identity_public_key) {
    throw new Error('拒绝更新：Contact Card identity_public_key 与已有联系人不一致')
  }
  return {
    ...(existing ?? {}),
    ...info,
    contact_card_text: cardText,
    kind: 'contact',
    state: existing?.state ?? 'LocalOnly',
    pending_request_id: existing?.pending_request_id,
    revoked_device_ids: existing?.revoked_device_ids,
    device_revocations: existing?.device_revocations,
    block_reason: existing?.block_reason,
    read_receipts: existing?.read_receipts ?? 'default',
    fingerprint_verified_at: existing?.fingerprint_verified_at,
    fingerprint_verified_note: existing?.fingerprint_verified_note,
    device_certs: info.device_certs ?? contactCardDeviceCerts(cardText),
  }
}



function createMyDeviceCert() {
  run('创建设备证书', () => {
    const out = safeJson<DeviceOutput>(create_device_cert(backupText.value, passphrase.value, 'Web Browser'))
    myDeviceId.value = out.device_id
    myDeviceCertJson.value = out.device_cert_json
    myDeviceBackupText.value = out.device_backup_text ?? ''
    myContactCardText.value = export_contact_card(
      backupText.value,
      passphrase.value,
      displayName.value || undefined,
      `[${myDeviceCertJson.value}]`,
    )
    persist()
    if (friendContacts.value.length) {
      appendLog(`正在自动向 ${friendContacts.value.length} 个好友分发新的设备证书更新`)
      void fanoutMyContactCardUpdateToFriends()
    }
    if (nodeEnabled.value) void ensureOwnContactCardDhtRecord()
  })
}

function createDeviceRevokeText() {
  run('生成设备撤销事件', () => {
    if (!revokeDeviceId.value.trim()) throw new Error('请输入 device_id')
    deviceRevokeText.value = create_device_revoke(
      backupText.value,
      passphrase.value,
      revokeDeviceId.value.trim(),
      revokeReason.value || undefined,
    )
  })
}

async function fanoutDeviceRevokeToFriends() {
  await runAsync('向好友分发设备撤销事件', async () => {
    if (!deviceRevokeText.value.trim()) createDeviceRevokeText()
    if (!deviceRevokeText.value.trim()) throw new Error('请先生成设备撤销事件')
    let sent = 0
    let queued = 0
    for (const contact of friendContacts.value) {
      const result = await deliverPayloadToContact(contact, deviceRevokeText.value, '设备撤销事件', 'other')
      if (result === 'sent' || result === 'mailbox') sent += 1
      else { queued += 1; queueOutboxItem(contact, deviceRevokeText.value, undefined, 'other') }
    }
    appendLog(`设备撤销事件分发完成：已投递 ${sent}，queued ${queued}`)
    persist()
  })
}



async function sendContactCardUpdateToContact(contact: ContactItem): Promise<'sent' | 'mailbox' | 'queued' | 'failed'> {
  if (!myContactCardText.value.trim()) refreshMyContactCard()
  if (!myContactCardText.value.trim()) throw new Error('请先生成我的联系人名片')
  const updateId = contactCardUpdateId(myContactCardText.value)
  const result = await deliverPayloadToContact(contact, myContactCardText.value, '联系人设备证书更新', 'contact-update')
  const status = result === 'sent' || result === 'mailbox' ? 'sent' : 'queued'
  rememberContactCardUpdateFanout(contact.user_id, updateId, status)
  if (result === 'queued' || result === 'failed') queueOutboxItem(contact, myContactCardText.value, undefined, 'contact-update')
  return result
}

async function retryStaleContactCardUpdateAcksRaw(): Promise<{ stale: number; retried: number }> {
  const stale = contactCardUpdateFanoutRecords.value.filter((record) => contactCardUpdateRecordIsStale(record))
  if (!stale.length) return { stale: 0, retried: 0 }
  let retried = 0
  for (const record of stale) {
    const contact = contacts.value.find((item) => item.user_id === record.peer_user_id)
    if (!contact) continue
    record.retry_count = Number(record.retry_count ?? 0) + 1
    record.last_retry_at = Date.now()
    await sendContactCardUpdateToContact(contact)
    retried += 1
  }
  persist()
  return { stale: stale.length, retried }
}

async function retryStaleContactCardUpdateAcks() {
  await runAsync('重试过期设备证书更新确认', async () => {
    const result = await retryStaleContactCardUpdateAcksRaw()
    if (!result.stale) {
      appendLog('没有过期的设备证书更新确认需要重试')
      return
    }
    appendLog(`已重试过期设备证书更新确认 ${result.retried}/${result.stale} 条`)
  })
}

async function fanoutMyContactCardUpdateToFriends(options: { force?: boolean } = {}) {
  await runAsync('向好友分发联系人设备证书更新', async () => {
    if (!myContactCardText.value.trim()) refreshMyContactCard()
    if (!myContactCardText.value.trim()) throw new Error('请先生成我的联系人名片')
    const now = Date.now()
    if (!options.force && lastContactCardUpdateFanoutAt.value && now - lastContactCardUpdateFanoutAt.value < 5 * 60_000) {
      contactCardUpdateFanoutSkipCount.value += 1
      appendLog('联系人设备证书更新分发已节流，避免重复广播')
      persist()
      return
    }
    let sent = 0
    let queued = 0
    for (const contact of friendContacts.value) {
      const result = await sendContactCardUpdateToContact(contact)
      if (result === 'sent' || result === 'mailbox') sent += 1
      else queued += 1
    }
    contactCardUpdateFanoutCount.value += 1
    lastContactCardUpdateFanoutAt.value = now
    appendLog(`联系人设备证书更新分发完成：已投递 ${sent}，queued ${queued}`)
    persist()
  })
}

function applyContactCardUpdateFromMailbox(cardText: string, sender: ContactItem) {
  ensureUiTextSize('联系人设备证书更新', cardText, MAX_CONTACT_CARD_BYTES)
  const info = safeJson<ContactInfo>(inspect_contact_card(cardText))
  if (info.user_id !== sender.user_id) throw new Error('联系人更新 user_id 与发送者不匹配')
  import_contact_as_json(cardText, 'MailboxContactUpdate')
  const index = contacts.value.findIndex((c) => c.user_id === info.user_id)
  const existing = index >= 0 ? contacts.value[index] : sender
  const merged = mergeContactCard(existing, info, cardText)
  if (index >= 0) contacts.value[index] = merged
  else contacts.value.push({ ...merged, state: 'Friend' })
  appendLog(`✅ 已合并 ${merged.display_name || merged.user_id} 的联系人设备证书更新`)
  persist()
}

function applyDeviceRevokeToActiveContact() {
  run('应用设备撤销事件', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const info = safeJson<DeviceRevokeInfo>(inspect_device_revoke(
      incomingDeviceRevokeText.value,
      activeContact.value.identity_public_key,
    ))
    if (info.user_id !== activeContact.value.user_id) throw new Error('撤销事件 user_id 与当前联系人不匹配')
    const list = new Set(activeContact.value.revoked_device_ids ?? [])
    list.add(info.device_id)
    activeContact.value.revoked_device_ids = [...list]
    const existing = activeContact.value.device_revocations ?? []
    activeContact.value.device_revocations = [info, ...existing.filter((item) => item.device_id !== info.device_id)]
    if (contactAllKnownDevicesRevoked(activeContact.value)) {
      removeRatchetSession(activeContact.value.user_id)
      activeContact.value.last_secure_session_error = '所有已知设备均已撤销，已清理本地 Ratchet session'
      activeContact.value.secure_session_failure_count = (activeContact.value.secure_session_failure_count ?? 0) + 1
      appendLog(`⚠️ ${activeContact.value.display_name || activeContact.value.user_id} 所有已知设备均已撤销，已清理 Ratchet session`)
    }
    incomingDeviceRevokeText.value = ''
    persist()
  })
}

async function copyText(value: string, label: string) {
  try {
    if (!value) throw new Error('内容为空')
    await writeClipboardText(value)
    appendLog(`✅ 已复制 ${label}`)
    toast(`已复制 ${label}`, 'success')
  } catch (e) {
    appendLog(`❌ 复制失败：${String(e)}`)
    showAlert('复制失败', String(e), 'error')
  }
}



async function copyMessageEnvelope(message: ChatMessage) {
  await copyText(message.envelope_json || '', 'Envelope')
  if (message.direction === 'out' && message.status === 'queued') {
    message.status = 'copied'
    persist()
  }
}

function statusLabel(status: MessageStatus) {
  switch (status) {
    case 'queued': return '待发送'
    case 'sent': return '已发送'
    case 'mailbox': return '已投递节点，待对方收取'
    case 'delivered': return '已送达'
    case 'read': return '已读'
    case 'copied': return '待发送'
    case 'received': return '已接收'
    case 'failed': return '失败'
  }
}

function filterRank(action: FilterAction): number {
  return ['Allow', 'Warn', 'Blur', 'Hide', 'Drop'].indexOf(action)
}

function maxFilterAction(a: FilterAction, b: FilterAction): FilterAction {
  return filterRank(a) >= filterRank(b) ? a : b
}

function evaluateLocalText(text: string): FilterAction {
  const policy = safetyPolicy.value
  if (!policy.enableTextFilter || policy.textFilterLevel === 'Off') return 'Allow'
  const lower = text.toLowerCase()
  let action: FilterAction = 'Allow'
  if (policy.warnExternalLinks && (lower.includes('http://') || lower.includes('https://'))) {
    action = maxFilterAction(action, 'Warn')
  }
  if (policy.warnExecutableFiles && /\.(exe|msi|bat|cmd|scr|apk|ipa|dmg|pkg|sh)\b/.test(lower)) {
    const executableAction: FilterAction = policy.textFilterLevel === 'Relaxed'
      ? 'Warn'
      : policy.textFilterLevel === 'Strict'
        ? 'Hide'
        : 'Blur'
    action = maxFilterAction(action, executableAction)
  }
  return action
}

function applyLocalTextFilter(text: string, direction: 'in' | 'out'): { allow: boolean; text: string; action: FilterAction } {
  const action = evaluateLocalText(text)
  if (action === 'Allow') return { allow: true, text, action }
  appendLog(`⚠️ 本地过滤：${direction === 'in' ? '收到' : '发送'}文本触发 ${action}`)
  if (direction === 'out') return { allow: true, text, action }
  if (direction === 'in' && safetyPolicy.value.dropFilteredIncoming && filterRank(action) >= filterRank('Hide')) {
    return { allow: false, text: '', action: 'Drop' }
  }
  if (action === 'Hide') return { allow: true, text: '[本地策略已隐藏内容]', action }
  if (action === 'Blur') return { allow: true, text: `⚠️ [本地策略提示] ${text}`, action }
  return { allow: true, text: `⚠️ ${text}`, action }
}

async function confirmOutgoingTextIfNeeded(text: string): Promise<boolean> {
  const action = evaluateLocalText(text)
  if (action === 'Allow') return true
  const reason = text.toLowerCase().includes('http://') || text.toLowerCase().includes('https://')
    ? '消息中包含外部链接。请确认链接可信，避免钓鱼或泄露身份信息。'
    : '消息中包含可执行/脚本文件名等高风险内容。请确认这是你想发送的内容。'
  return showConfirm('发送风险内容', reason, filterRank(action) >= filterRank('Hide'))
}

function contactRevokedDeviceIds(contact: ContactItem): string[] {
  const revoked = new Set(contact.revoked_device_ids ?? [])
  for (const item of contact.device_revocations ?? []) revoked.add(item.device_id)
  const known = (contact.device_certs ?? []).map((cert) => cert.device_id).filter((deviceId) => revoked.has(deviceId))
  const knownSet = new Set(known)
  const unknown = [...revoked].filter((deviceId) => !knownSet.has(deviceId))
  return [...known, ...unknown]
}

function contactRevokedDeviceDetails(contact: ContactItem): DeviceRevokeInfo[] {
  const byId = new Map((contact.device_revocations ?? []).map((item) => [item.device_id, item]))
  return contactRevokedDeviceIds(contact).map((deviceId) => byId.get(deviceId) ?? {
    user_id: contact.user_id,
    device_id: deviceId,
    created_at: 0,
  })
}

async function unmarkActiveContactRevokedDevice(deviceId: string) {
  await runAsync('解除联系人设备撤销标记', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const contact = activeContact.value
    const ok = await showConfirm(
      '解除设备撤销标记',
      `仅在你确认该设备仍可信或撤销事件误操作时解除：${deviceId}。继续？`,
      true,
    )
    if (!ok) return
    contact.revoked_device_ids = (contact.revoked_device_ids ?? []).filter((id) => id !== deviceId)
    contact.device_revocations = (contact.device_revocations ?? []).filter((item) => item.device_id !== deviceId)
    appendLog(`已解除 ${contact.display_name || contact.user_id} 的设备撤销标记：${deviceId}`)
    persist()
  })
}

function contactActiveDeviceIds(contact: ContactItem): string[] {
  const revoked = new Set(contact.revoked_device_ids ?? [])
  return (contact.device_certs ?? [])
    .map((cert) => cert.device_id)
    .filter((deviceId) => !revoked.has(deviceId))
}

function contactRevokedDeviceCount(contact: ContactItem): number {
  return contactRevokedDeviceIds(contact).length
}

function contactKnownRevokedDeviceCount(contact: ContactItem): number {
  const revoked = new Set(contact.revoked_device_ids ?? [])
  return (contact.device_certs ?? []).filter((cert) => revoked.has(cert.device_id)).length
}

function contactAllKnownDevicesRevoked(contact: ContactItem): boolean {
  const certs = contact.device_certs ?? []
  if (certs.length === 0) return false
  const revoked = new Set(contact.revoked_device_ids ?? [])
  return certs.every((cert) => revoked.has(cert.device_id))
}

function requireContactHasActiveDevice(contact: ContactItem) {
  if (!contactAllKnownDevicesRevoked(contact)) return
  throw new Error(`联系人全部已知设备均已撤销：${contact.display_name || contact.user_id}`)
}


function requireContactSupportsSealedPerDeviceSlots(contact: ContactItem) {
  if (!safetyPolicy.value.requireSealedPerDeviceSlotsForSend) return
  const activeDeviceIds = contactActiveDeviceIds(contact)
  if (activeDeviceIds.length === 0) {
    throw new Error(`安全策略要求分设备 sealed slot，但联系人没有可投递设备证书：${contact.display_name || contact.user_id}`)
  }
  const missing = activeDeviceIds.filter((deviceId) => {
    const cert = (contact.device_certs ?? []).find((item) => item.device_id === deviceId)
    return !cert?.device_box_public_key
  })
  if (missing.length) {
    throw new Error(`安全策略要求分设备 sealed slot，但 ${missing.length} 个活跃设备缺少加密公钥`)
  }
}

function requireVerifiedContactForSend(contact: ContactItem) {
  requireContactHasActiveDevice(contact)
  requireContactSupportsSealedPerDeviceSlots(contact)
  if (!safetyPolicy.value.requireVerifiedContactsForSend) return
  if (contact.fingerprint_verified_at) return
  throw new Error(`安全策略要求先核验联系人指纹：${contact.display_name || contact.user_id}`)
}

function allowIncomingFromContact(sender: ContactItem): boolean {
  if (contactAllKnownDevicesRevoked(sender)) {
    revokedDeviceIncomingDropCount.value += 1
    lastRevokedDeviceIncomingDropAt.value = Date.now()
    lastRevokedDeviceIncomingDropFrom.value = sender.display_name || sender.user_id
    appendLog(`⚠️ 已丢弃所有已知设备均撤销的联系人消息：${lastRevokedDeviceIncomingDropFrom.value}`)
    return false
  }
  if (!safetyPolicy.value.requireVerifiedContactsForReceive) return true
  if (sender.fingerprint_verified_at) return true
  unverifiedIncomingDropCount.value += 1
  lastUnverifiedIncomingDropAt.value = Date.now()
  lastUnverifiedIncomingDropFrom.value = sender.display_name || sender.user_id
  appendLog(`⚠️ 已按安全策略丢弃未核验联系人消息：${lastUnverifiedIncomingDropFrom.value}`)
  return false
}

function clearUnverifiedIncomingDropStats() {
  unverifiedIncomingDropCount.value = 0
  lastUnverifiedIncomingDropAt.value = null
  lastUnverifiedIncomingDropFrom.value = ''
  appendLog('已清空未核验联系人入站丢弃统计')
  persist()
}

function clearRevokedDeviceIncomingDropStats() {
  revokedDeviceIncomingDropCount.value = 0
  lastRevokedDeviceIncomingDropAt.value = null
  lastRevokedDeviceIncomingDropFrom.value = ''
  appendLog('已清空撤销设备联系人入站丢弃统计')
  persist()
}

async function confirmStrictE2eeSendRiskIfNeeded(contact: ContactItem): Promise<boolean> {
  const riskText = contactStrictE2eeSendRiskText(contact)
  if (!riskText) return true
  return showConfirm(
    '发送前严格 E2EE 风险提示',
    `${riskText}

建议先启用“一键严格 E2EE”、核验指纹、刷新 ContactCard DHT，并确认所有活跃设备支持 sealed slot。仍要继续发送吗？`,
    true,
  )
}

async function confirmHighRiskDhtContactIfNeeded(contact: ContactItem): Promise<boolean> {
  if (contact.dht_discovery_risk_level !== 'high') return true
  if (contact.last_dht_discovery_error_kind !== 'signature') return true
  const reason = contact.last_dht_discovery_error_kind
    ? `该联系人最近的 DHT 发现结果存在高风险：${contact.last_dht_discovery_error_kind}。${contact.last_dht_discovery_error || ''}`
    : '该联系人最近的 DHT 发现结果存在高风险。'
  return showConfirm(
    '确认发送给 DHT 高风险联系人',
    `${reason}

建议先通过指纹/可信渠道核验联系人身份或重新发现 DHT 记录。仍要继续发送吗？`,
    true,
  )
}

function saveSafetyPolicy() {
  persist()
  appendLog('✅ 已保存本地安全策略')
}

function enableStrictE2eePolicy() {
  safetyPolicy.value = {
    ...safetyPolicy.value,
    requireVerifiedContactsForSend: true,
    requireVerifiedContactsForReceive: true,
    requireSealedPerDeviceSlotsForSend: true,
    requireSealedPerDeviceSlotsForReceive: true,
  }
  persist()
  appendLog(strictE2eeReadiness.value.ready
    ? '✅ 已启用严格 E2EE 策略：指纹核验 + sealed slot 收发'
    : `⚠️ 已启用严格 E2EE 策略，但启用前检查仍有风险：${strictE2eeReadiness.value.text}`)
}

const strictE2eePolicyEnabled = computed(() => Boolean(
  safetyPolicy.value.requireVerifiedContactsForSend
  && safetyPolicy.value.requireVerifiedContactsForReceive
  && safetyPolicy.value.requireSealedPerDeviceSlotsForSend
  && safetyPolicy.value.requireSealedPerDeviceSlotsForReceive,
))

function recordFriendRequestRate(fromUserId: string, now = Date.now()): FriendRequestRateRecord {
  const activeRecords = friendRequestRateRecords.value.filter((record) =>
    now - record.first_seen_at <= FRIEND_REQUEST_LONG_RATE_WINDOW_MS || record.from_user_id === fromUserId,
  )
  friendRequestRateRecords.value = activeRecords
  let record = friendRequestRateRecords.value.find((item) => item.from_user_id === fromUserId)
  if (!record || now - record.first_seen_at > FRIEND_REQUEST_LONG_RATE_WINDOW_MS) {
    record = { from_user_id: fromUserId, first_seen_at: now, last_seen_at: now, count: 0 }
    friendRequestRateRecords.value.push(record)
  }
  record.count += 1
  record.last_seen_at = now
  return record
}

function friendRequestQuarantineReason(info: Pick<FriendRequestItem, 'from_user_id' | 'request_id' | 'created_at'>, recordLongRate: boolean, now = Date.now()): string | undefined {
  const recentSameSourceCount = friendRequests.value.filter((req) =>
    req.from_user_id === info.from_user_id &&
    req.request_id !== info.request_id &&
    now - req.created_at <= FRIEND_REQUEST_RATE_WINDOW_MS,
  ).length
  if (recentSameSourceCount >= 1) {
    const windowMinutes = Math.round(FRIEND_REQUEST_RATE_WINDOW_MS / 60_000)
    return `同一来源 ${windowMinutes} 分钟内已有 ${recentSameSourceCount} 条未处理请求`
  }
  if (!recordLongRate) return undefined
  const rateRecord = recordFriendRequestRate(info.from_user_id, now)
  if (rateRecord.count > FRIEND_REQUEST_LONG_RATE_LIMIT) {
    const windowHours = Math.round(FRIEND_REQUEST_LONG_RATE_WINDOW_MS / 60 / 60 / 1000)
    return `同一来源 ${windowHours} 小时内已有 ${rateRecord.count} 条请求`
  }
  return undefined
}

function upsertFriendRequestWithLocalRateLimit(item: FriendRequestItem, now = Date.now()) {
  const index = friendRequests.value.findIndex((req) => req.request_id === item.request_id)
  const existing = index >= 0 ? friendRequests.value[index] : undefined
  const quarantineReason = friendRequestQuarantineReason(item, index < 0, now)
  const next: FriendRequestItem = {
    ...item,
    quarantined: existing?.quarantined || Boolean(quarantineReason),
    quarantine_reason: existing?.quarantine_reason || quarantineReason,
  }
  if (index >= 0) friendRequests.value[index] = next
  else friendRequests.value.unshift(next)
  return next
}

async function showQr(value: string, label: string) {
  try {
    if (!value) throw new Error('内容为空')
    qrTitle.value = label
    qrRawText.value = value
    qrDataUrl.value = await QRCode.toDataURL(value, {
      errorCorrectionLevel: 'M',
      margin: 2,
      width: 360,
    })
    appendLog(`✅ 已生成二维码：${label}`)
  } catch (e) {
    appendLog(`❌ 二维码生成失败：${String(e)}`)
  }
}

function closeQr() {
  qrTitle.value = ''
  qrDataUrl.value = ''
  qrRawText.value = ''
}


function addContact() {
  run('添加好友', () => {
    ensureUiTextSize('名片', addContactText.value, MAX_CONTACT_CARD_BYTES)
    const info = safeJson<ContactInfo>(inspect_contact_card(addContactText.value))
    import_contact_as_json(addContactText.value, 'LinkImported')
    const index = contacts.value.findIndex((c) => c.user_id === info.user_id)
    const existing = index >= 0 ? contacts.value[index] : undefined
    const item = mergeContactCard(existing, info, addContactText.value)
    if (index >= 0) {
      contacts.value[index] = item
      appendLog('✅ 已更新联系人资料')
    } else {
      contacts.value.push(item)
      appendLog('✅ 已添加联系人')
      toast('已添加联系人', 'success')
    }
    activePeerId.value = item.user_id
    addContactText.value = ''
    persist()
    if (nodeEnabled.value && item.state === 'LocalOnly') createFriendRequestForActive()
    void ensureRatchetSessionFromNode(item).catch((e) => appendLog(`⚠️ 自动建链失败：${String(e)}`))
  })
}


function addIncomingFriendRequest() {
  run('加入好友请求收件箱', () => {
    const info = safeJson<Omit<FriendRequestItem, 'request_text'>>(inspect_friend_request(incomingFriendRequestText.value))
    if (identity.value && info.to_user_id !== identity.value.user_id) {
      throw new Error('这个好友请求不是发给当前身份的')
    }
    const now = Date.now()
    const item = upsertFriendRequestWithLocalRateLimit({
      ...info,
      request_text: incomingFriendRequestText.value,
    }, now)
    incomingFriendRequestText.value = ''
    toast(item.quarantined ? '好友请求已隔离' : '收到新的好友请求', item.quarantined ? 'warning' : 'info')
    persist()
  })
}

function acceptInboxRequest(req: FriendRequestItem) {
  run('接受好友请求', () => {
    const response = accept_friend_request(backupText.value, passphrase.value, req.request_text)
    friendResponseText.value = response
    const info = safeJson<ContactInfo>(inspect_contact_card(req.from_contact_card_text))
    const index = contacts.value.findIndex((c) => c.user_id === info.user_id)
    const contact: ContactItem = { ...mergeContactCard(index >= 0 ? contacts.value[index] : undefined, info, req.from_contact_card_text), state: 'Friend' }
    if (index >= 0) contacts.value[index] = contact
    else contacts.value.push(contact)
    friendRequests.value = friendRequests.value.filter((r) => r.request_id !== req.request_id)
    activePeerId.value = contact.user_id
    persist()
    if (nodeEnabled.value) {
      void pushMailboxPayload(contact, 'other', response)
        .then(async () => {
          appendLog('✅ 已通过好友请求')
          toast('已添加好友', 'success')
          try {
            await sendSecureSessionOfferToContact(contact)
          } catch (e) {
            showAlert('已添加好友，但自动建链失败', userFacingError(e), 'warning')
          }
        })
        .catch((e) => {
          const message = userFacingError(e)
          appendLog(`⚠️ 好友确认发送失败：${message}`)
          showAlert('已添加好友，但好友确认发送失败', message, 'warning')
        })
    } else {
      toast('已添加好友', 'success')
    }
  })
}

function rejectInboxRequest(req: FriendRequestItem) {
  friendRequests.value = friendRequests.value.filter((r) => r.request_id !== req.request_id)
  persist()
}

function rejectAllInboxRequests() {
  run('忽略全部好友请求', () => {
    const count = visibleFriendRequests.value.length
    if (count === 0) throw new Error('没有好友请求')
    friendRequests.value = friendRequests.value.filter((req) => req.quarantined)
    appendLog(`已忽略好友请求 ${count} 条`)
    persist()
  })
}

function blockAllInboxRequests() {
  run('拉黑全部好友请求来源', () => {
    const requests = [...visibleFriendRequests.value]
    if (requests.length === 0) throw new Error('没有好友请求')
    let blocked = 0
    for (const req of requests) {
      const info = contactInfoFromCardText(req.from_contact_card_text)
      if (!info) continue
      const index = contacts.value.findIndex((c) => c.user_id === info.user_id)
      const contact = mergeContactCard(index >= 0 ? contacts.value[index] : undefined, info, req.from_contact_card_text)
      contact.state = 'Blocked'
      contact.block_reason = 'blocked from inbox'
      if (index >= 0) contacts.value[index] = contact
      else contacts.value.push(contact)
      blocked += 1
    }
    friendRequests.value = []
    appendLog(`已拉黑好友请求来源 ${blocked} 个，清空请求 ${requests.length} 条`)
    persist()
  })
}

function restoreQuarantinedFriendRequest(req: FriendRequestItem) {
  run('恢复好友请求', () => {
    const index = friendRequests.value.findIndex((item) => item.request_id === req.request_id)
    if (index < 0) throw new Error('找不到好友请求')
    friendRequests.value[index] = {
      ...friendRequests.value[index],
      quarantined: false,
      quarantine_reason: undefined,
    }
    persist()
  })
}

function restoreAllQuarantinedFriendRequests() {
  run('恢复全部垃圾请求', () => {
    const count = quarantinedFriendRequests.value.length
    if (count === 0) throw new Error('没有垃圾请求')
    friendRequests.value = friendRequests.value.map((req) => req.quarantined ? {
      ...req,
      quarantined: false,
      quarantine_reason: undefined,
    } : req)
    appendLog(`已恢复垃圾请求 ${count} 条`)
    persist()
  })
}

function clearQuarantinedFriendRequests() {
  run('清空垃圾请求', () => {
    const count = quarantinedFriendRequests.value.length
    if (count === 0) throw new Error('没有垃圾请求')
    friendRequests.value = friendRequests.value.filter((req) => !req.quarantined)
    appendLog(`已清空垃圾请求 ${count} 条`)
    persist()
  })
}

function clearFriendRequestRateRecords() {
  run('清空好友请求频率记录', () => {
    const count = friendRequestRateRecords.value.length
    if (count === 0) throw new Error('没有频率记录')
    friendRequestRateRecords.value = []
    appendLog(`已清空好友请求频率记录 ${count} 条`)
    persist()
  })
}


function selectContact(userId: string) {
  activePeerId.value = userId
  activeGroupId.value = ''
}

function selectGroup(groupId: string) {
  activeGroupId.value = groupId
  activePeerId.value = ''
}


function groupSenderKeyId(groupId: string, senderUserId: string): string {
  return `${groupId}:${senderUserId}`
}

function getGroupSenderKey(groupId: string, senderUserId: string): GroupSenderKeyItem | null {
  return groupSenderKeys.value.find((k) => k.group_id === groupId && k.sender_user_id === senderUserId) ?? null
}

function saveGroupSenderKey(item: GroupSenderKeyItem) {
  const index = groupSenderKeys.value.findIndex((k) => k.key_id === item.key_id)
  if (index >= 0) groupSenderKeys.value[index] = item
  else groupSenderKeys.value.push(item)
  const group = groups.value.find((g) => g.group_id === item.group_id)
  if (group) {
    group.last_sender_key_error = undefined
    group.last_sender_key_error_at = undefined
  }
}

function rememberGroupSenderKeyError(groupId: string | undefined, reason: string) {
  const group = groupId ? groups.value.find((item) => item.group_id === groupId) : activeGroup.value
  if (!group) return
  group.last_sender_key_error = reason
  group.last_sender_key_error_at = Date.now()
}

function createGroupSenderDistributionFanout(group: GroupItem, distributionText: string) {
  const recipients = group.member_user_ids.filter((uid) => uid !== identity.value?.user_id)
  const fanout = recipients.map((uid) => {
    const contact = contacts.value.find((c) => c.user_id === uid)
    if (!contact || contact.state !== 'Friend') throw new Error(`群成员还不是好友: ${uid}`)
    const envelope = encryptEnvelopeForContact(
      contact,
      `grp-key-${group.group_id}`,
      `${GROUP_SENDER_KEY_PAYLOAD_PREFIX}${distributionText}`,
    )
    return { to_user_id: uid, envelope }
  })
  groupSenderDistributionFanoutJson.value = JSON.stringify(fanout, null, 2)
  appendLog(`✅ 已生成 Sender Key Distribution fanout：${fanout.length} 个成员`)
  return fanout
}

async function sendGroupSenderDistributionFanout(group: GroupItem, fanout: Array<{ to_user_id: string; envelope: string }>, reason: string) {
  let sent = 0
  let queued = 0
  let failed = 0
  const errors: string[] = []
  for (const item of fanout) {
    const contact = contacts.value.find((c) => c.user_id === item.to_user_id)
    if (!contact || contact.state !== 'Friend') {
      failed += 1
      errors.push(`${item.to_user_id}: 联系人不可用`)
      continue
    }
    const result = await deliverPayloadToContact(contact, item.envelope, 'Sender Key Distribution', 'other')
    if (result === 'sent' || result === 'mailbox') sent += 1
    else {
      queued += 1
      const outboxItem = queueOutboxItem(contact, item.envelope, undefined, 'other')
      outboxItem.last_error = result === 'failed' ? lastDeliveryError || 'Sender Key Distribution 投递失败' : undefined
      if (result === 'failed') errors.push(`${contact.display_name || contact.user_id}: ${outboxItem.last_error}`)
    }
  }
  if (failed > 0 || errors.length > 0) {
    rememberGroupSenderKeyError(group.group_id, `Sender Key 分发未全部完成：已发送 ${sent}，待重试 ${queued}，失败 ${failed}；${errors.slice(0, 2).join('；')}`)
  } else {
    group.last_sender_key_error = undefined
    group.last_sender_key_error_at = undefined
  }
  appendLog(`Sender Key Distribution ${reason}：已发送 ${sent}，待重试 ${queued}，失败 ${failed}`)
  persist()
}

function scheduleGroupSenderDistributionFanout(group: GroupItem, fanout: Array<{ to_user_id: string; envelope: string }>, reason: string) {
  if (fanout.length === 0) return
  void sendGroupSenderDistributionFanout(group, fanout, reason)
}

function rotateMyGroupSenderKey(group: GroupItem, reason: string) {
  if (!identity.value) return
  const out = JSON.parse(create_group_sender_key(backupText.value, passphrase.value, group.group_id)) as {
    state_json: string
    distribution_text: string
  }
  saveGroupSenderKey({
    key_id: groupSenderKeyId(group.group_id, identity.value.user_id),
    group_id: group.group_id,
    sender_user_id: identity.value.user_id,
    state_json: out.state_json,
    distribution_text: out.distribution_text,
    updated_at: Date.now(),
  })
  groupSenderDistributionText.value = out.distribution_text
  let fanout: Array<{ to_user_id: string; envelope: string }> = []
  try {
    fanout = createGroupSenderDistributionFanout(group, out.distribution_text)
    group.last_sender_key_error = undefined
    group.last_sender_key_error_at = undefined
  } catch (e) {
    const reason = userFacingError(e)
    rememberGroupSenderKeyError(group.group_id, `轮换后分发失败：${reason}`)
    appendLog(`⚠️ Sender Key 轮换后分发失败：${reason}`)
    return
  }
  appendLog(`🔄 已轮换本群 Sender Key：${reason}；已生成新的 distribution fanout`)
  scheduleGroupSenderDistributionFanout(group, fanout, `轮换：${reason}`)
}

function createGroupSenderKeyForActiveGroup() {
  run('创建群 Sender Key', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    if (!identity.value) throw new Error('请先登录')
    const out = JSON.parse(create_group_sender_key(backupText.value, passphrase.value, activeGroup.value.group_id)) as {
      state_json: string
      distribution_text: string
    }
    const item: GroupSenderKeyItem = {
      key_id: groupSenderKeyId(activeGroup.value.group_id, identity.value.user_id),
      group_id: activeGroup.value.group_id,
      sender_user_id: identity.value.user_id,
      state_json: out.state_json,
      distribution_text: out.distribution_text,
      updated_at: Date.now(),
    }
    saveGroupSenderKey(item)
    groupSenderDistributionText.value = out.distribution_text
    const group = activeGroup.value
    const fanout = createGroupSenderDistributionFanout(group, out.distribution_text)
    appendLog('✅ 已创建我的群 Sender Key，并生成 distribution fanout')
    persist()
    scheduleGroupSenderDistributionFanout(group, fanout, '创建')
  })
}

function importGroupSenderKeyForActiveContact() {
  run('导入群 Sender Key', () => {
    if (!activeContact.value) throw new Error('请选择 Sender 联系人')
    if (!groupSenderDistributionText.value.trim()) throw new Error('请填写 Sender Key Distribution')
    const stateJson = import_group_sender_key(groupSenderDistributionText.value, activeContact.value.contact_card_text)
    const parsed = JSON.parse(stateJson) as { group_id: string; sender_user_id: string }
    const item: GroupSenderKeyItem = {
      key_id: groupSenderKeyId(parsed.group_id, parsed.sender_user_id),
      group_id: parsed.group_id,
      sender_user_id: parsed.sender_user_id,
      state_json: stateJson,
      distribution_text: groupSenderDistributionText.value,
      updated_at: Date.now(),
    }
    saveGroupSenderKey(item)
    appendLog('✅ 已导入群 Sender Key')
    persist()
  })
}

function createGroupSenderDistributionFanoutForActiveGroup() {
  run('生成 Sender Key Distribution fanout', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    if (!groupSenderDistributionText.value.trim()) throw new Error('请先创建 Sender Key Distribution')
    const group = activeGroup.value
    const fanout = createGroupSenderDistributionFanout(group, groupSenderDistributionText.value.trim())
    persist()
    scheduleGroupSenderDistributionFanout(group, fanout, '手动重发')
  })
}

function encryptGroupSenderText(group: GroupItem, text: string): string | null {
  if (!identity.value) return null
  const key = getGroupSenderKey(group.group_id, identity.value.user_id)
  if (!key) return null
  const out = JSON.parse(group_sender_encrypt_text(key.state_json, text)) as { state_json: string; envelope_json: string }
  key.state_json = out.state_json
  key.updated_at = Date.now()
  groupSenderEnvelopeText.value = JSON.stringify(JSON.parse(out.envelope_json), null, 2)
  appendLog('🔐 使用 Sender Key 加密群消息')
  return out.envelope_json
}

function tryDecryptGroupSenderEnvelope(envelopeText: string): { text: string; group_id: string; sender_user_id: string } | null {
  let parsed: any = null
  try { parsed = JSON.parse(envelopeText) } catch { return null }
  if (parsed?.type !== 'lm-group-sender-envelope-v1') return null
  const key = getGroupSenderKey(String(parsed.group_id), String(parsed.sender_user_id))
  if (!key) {
    const reason = `缺少 ${parsed.sender_user_id} 的 Sender Key Distribution`
    rememberGroupSenderKeyError(String(parsed.group_id), reason)
    throw new Error(reason)
  }
  let out: { state_json: string; plain_json: string }
  try {
    out = JSON.parse(group_sender_decrypt_text(key.state_json, envelopeText)) as { state_json: string; plain_json: string }
  } catch (e) {
    const reason = userFacingError(e)
    rememberGroupSenderKeyError(String(parsed.group_id), `解密失败：${reason}`)
    throw e
  }
  key.state_json = out.state_json
  key.updated_at = Date.now()
  const plain = JSON.parse(out.plain_json) as { text: string; group_id: string; sender_user_id: string }
  groupSenderPlainText.value = JSON.stringify(plain, null, 2)
  appendLog('🔓 已用 Sender Key 解密群消息')
  return plain
}

function groupSenderEncryptDebug() {
  run('Sender Key 加密调试', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    if (!composerText.value.trim()) throw new Error('请输入消息')
    const envelope = encryptGroupSenderText(activeGroup.value, composerText.value)
    if (!envelope) throw new Error('没有本群自己的 Sender Key，请先创建')
  })
}

function groupSenderDecryptDebug() {
  run('Sender Key 解密调试', () => {
    if (!groupSenderEnvelopeText.value.trim()) throw new Error('请粘贴 Sender Envelope')
    const plain = tryDecryptGroupSenderEnvelope(groupSenderEnvelopeText.value)
    if (!plain) throw new Error('不是 Sender Key Envelope')
    persist()
  })
}

function createGroup() {
  run('创建群组', () => {
    const riskText = createGroupStrictE2eeRiskText.value
    if (riskText) {
      if (strictE2eePolicyEnabled.value) throw new Error(`严格 E2EE 策略阻止创建风险群聊：${riskText}`)
      const ok = confirm(`
${riskText}

仍要继续创建群聊吗？`)
      if (!ok) throw new Error('已取消创建群聊')
    }
    const members = [...new Set(selectedGroupMembers.value)].filter(Boolean)
    if (!newGroupName.value.trim()) throw new Error('请输入群名')
    if (members.length === 0) throw new Error('请选择至少一个 Friend 联系人')
    const groupId = newId()
    const adminIds = identity.value ? [identity.value.user_id] : []
    const allMembers = identity.value && !members.includes(identity.value.user_id) ? [...members, identity.value.user_id] : members
    const group: GroupItem = {
      group_id: groupId,
      name: newGroupName.value.trim(),
      member_user_ids: allMembers.filter((id) => id !== identity.value?.user_id),
      admin_user_ids: adminIds,
      policy_state_json: identity.value ? create_group_policy_state(groupId, newGroupName.value.trim(), identity.value.user_id, JSON.stringify(allMembers)) : undefined,
      created_at: Date.now(),
      sequence: 0,
    }
    groups.value.push(group)
    activeGroupId.value = group.group_id
    activePeerId.value = ''
    newGroupName.value = ''
    selectedGroupMembers.value = []
    persist()
    void sendGroupInviteToMembers(group)
  })
}


function groupInviteFor(group: GroupItem): string {
  const memberIds = [...group.member_user_ids]
  if (identity.value && !memberIds.includes(identity.value.user_id)) memberIds.push(identity.value.user_id)
  return create_group_invite(
    backupText.value,
    passphrase.value,
    group.group_id,
    group.name,
    JSON.stringify(memberIds),
  )
}

function createInviteForActiveGroup() {
  run('生成群邀请', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    groupInviteText.value = groupInviteFor(activeGroup.value)
  })
}

async function sendGroupInviteToMembers(group: GroupItem) {
  if (!nodeEnabled.value) return
  const invite = groupInviteFor(group)
  groupInviteText.value = invite
  let sent = 0
  let failed = 0
  for (const userId of group.member_user_ids) {
    const contact = contacts.value.find((c) => c.user_id === userId)
    if (!contact || contact.state !== 'Friend') { failed += 1; continue }
    try {
      await pushMailboxPayload(contact, 'other', invite)
      sent += 1
    } catch (e) {
      failed += 1
      appendLog(`⚠️ 群邀请发送失败：${contact.display_name || contact.user_id}: ${String(e)}`)
    }
  }
  if (sent > 0) toast(`已邀请 ${sent} 位好友入群`, 'success')
  if (failed > 0) appendLog(`群邀请发送完成：成功 ${sent}，失败 ${failed}`)
}

function nextGroupSequence(group: GroupItem): number {
  return (group.sequence ?? 0) + 1
}

function ensureActiveGroupAdmin(actionLabel: string) {
  const group = activeGroup.value
  if (!group) throw new Error('请选择群组')
  if (group.removed_self_at) {
    const reason = `群权限拒绝：你已被移出群聊，不能${actionLabel}`
    rememberGroupEventError(group.group_id, reason)
    throw new Error(reason)
  }
  const admins = group.admin_user_ids ?? []
  if (identity.value && admins.includes(identity.value.user_id)) return
  const reason = `群权限拒绝：只有管理员可以${actionLabel}`
  rememberGroupEventError(group.group_id, reason)
  throw new Error(reason)
}

function createRenameGroupEvent() {
  run('生成群改名事件', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    ensureActiveGroupAdmin('修改群名')
    const name = groupRenameText.value.trim()
    if (!name) throw new Error('请输入新群名')
    const sequence = nextGroupSequence(activeGroup.value)
    groupEventText.value = create_group_event(
      backupText.value,
      passphrase.value,
      activeGroup.value.group_id,
      BigInt(sequence),
      JSON.stringify({ Rename: { name } }),
    )
  })
}

function createAddMemberGroupEvent(userId: string) {
  run('生成加人事件', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    ensureActiveGroupAdmin('添加成员')
    const sequence = nextGroupSequence(activeGroup.value)
    groupEventText.value = create_group_event(
      backupText.value,
      passphrase.value,
      activeGroup.value.group_id,
      BigInt(sequence),
      JSON.stringify({ AddMember: { user_id: userId } }),
    )
  })
}

function createRemoveMemberGroupEventText(userId: string) {
  if (!activeGroup.value) throw new Error('请选择群组')
  if (!identity.value) throw new Error('请先登录')
  if (userId !== identity.value.user_id) throw new Error('群成员只能生成自己的退群事件，不能移除其他成员')
  const sequence = nextGroupSequence(activeGroup.value)
  groupEventText.value = create_group_event(
    backupText.value,
    passphrase.value,
    activeGroup.value.group_id,
    BigInt(sequence),
    JSON.stringify({ RemoveMember: { user_id: userId } }),
  )
}

function createRemoveMemberGroupEvent(userId: string) {
  run('生成退群事件', () => {
    createRemoveMemberGroupEventText(userId)
  })
}


function createPromoteAdminGroupEvent(userId: string) {
  run('生成提升管理员事件', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    ensureActiveGroupAdmin('提升管理员')
    const sequence = nextGroupSequence(activeGroup.value)
    groupEventText.value = create_group_event(
      backupText.value,
      passphrase.value,
      activeGroup.value.group_id,
      BigInt(sequence),
      JSON.stringify({ PromoteAdmin: { user_id: userId } }),
    )
  })
}

function createDemoteAdminGroupEvent(userId: string) {
  run('生成取消管理员事件', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    ensureActiveGroupAdmin('取消管理员')
    const sequence = nextGroupSequence(activeGroup.value)
    groupEventText.value = create_group_event(
      backupText.value,
      passphrase.value,
      activeGroup.value.group_id,
      BigInt(sequence),
      JSON.stringify({ DemoteAdmin: { user_id: userId } }),
    )
  })
}

function summarizeGroupEventAction(action: any): string {
  if (action.Rename) return `改名为 ${action.Rename.name}`
  if (action.AddMember) return `加入成员 ${action.AddMember.user_id}`
  if (action.RemoveMember) return `成员退出 ${action.RemoveMember.user_id}`
  if (action.PromoteAdmin) return `提升管理员 ${action.PromoteAdmin.user_id}`
  if (action.DemoteAdmin) return `取消管理员 ${action.DemoteAdmin.user_id}`
  return '未知事件'
}

function rememberGroupEventError(groupId: string | undefined, reason: string) {
  const group = groupId ? groups.value.find((item) => item.group_id === groupId) : activeGroup.value
  if (!group) return
  group.last_event_error = reason
  group.last_event_error_at = Date.now()
}

function groupEventRecoveryHint(group: GroupItem, eventSequence?: number): string {
  const current = group.sequence ?? 0
  if (typeof eventSequence === 'number' && eventSequence > current + 1) {
    return `本地缺少中间群事件 ${current + 1}..${eventSequence - 1}，请先向其他成员同步缺失事件或群快照后重试。`
  }
  if (typeof eventSequence === 'number' && eventSequence <= current) {
    return `本地群事件序列已到 ${current}，该事件可视为重复或旧事件；确认无误后可清除错误。`
  }
  return '请确认事件发起者、群成员和本地群状态后重试。'
}

function groupIdFromEventText(text: string, actorId: string): string | undefined {
  const actor = actorId === identity.value?.user_id
    ? { contact_card_text: myContactCardText.value }
    : contacts.value.find((c) => c.user_id === actorId)
  if (!actor?.contact_card_text) return undefined
  try {
    const info = JSON.parse(inspect_group_event(text, actor.contact_card_text)) as { group_id?: string }
    return info.group_id
  } catch {
    return undefined
  }
}

function clearActiveGroupEventError() {
  run('清除群事件错误', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    activeGroup.value.last_event_error = undefined
    activeGroup.value.last_event_error_at = undefined
    activeGroup.value.last_event_recovery_hint = undefined
    persist()
  })
}

function applyGroupEventRaw(text: string, actorId: string): { group_id: string; summary: string } {
  if (!text) throw new Error('请粘贴群事件')
  if (!actorId) throw new Error('需要事件发起者 UserID')
  const actor = actorId === identity.value?.user_id
    ? { contact_card_text: myContactCardText.value }
    : contacts.value.find((c) => c.user_id === actorId)
  if (!actor?.contact_card_text) throw new Error('找不到发起者 Contact Card')
  const info = JSON.parse(inspect_group_event(text, actor.contact_card_text)) as {
    group_id: string
    actor_user_id: string
    sequence: number
    action: any
  }
  const group = groups.value.find((g) => g.group_id === info.group_id)
  if (!group) throw new Error(`本地没有这个群：${info.group_id}；可能尚未接受邀请或已仅本机退出`)
  if (info.sequence > (group.sequence ?? 0) + 1) {
    group.last_event_recovery_hint = groupEventRecoveryHint(group, info.sequence)
    throw new Error(`群事件 sequence 乱序：当前 ${group.sequence ?? 0}，收到 ${info.sequence}`)
  }
  if (info.sequence <= (group.sequence ?? 0)) {
    group.last_event_recovery_hint = groupEventRecoveryHint(group, info.sequence)
    throw new Error(`群事件 sequence 已过期或重复：当前 ${group.sequence ?? 0}，收到 ${info.sequence}`)
  }
  if (group.policy_state_json) {
    group.policy_state_json = apply_group_policy_event(group.policy_state_json, text, actor.contact_card_text)
    const policy = JSON.parse(group.policy_state_json) as { name: string; members: string[]; admins: string[]; sequence: number }
    group.name = policy.name
    group.member_user_ids = policy.members.filter((id) => id !== identity.value?.user_id)
    group.admin_user_ids = policy.admins
    group.sequence = policy.sequence
    if (identity.value && info.action.AddMember?.user_id === identity.value.user_id) {
      group.removed_self_at = undefined
      group.removed_self_by = undefined
    }
    if (identity.value && info.action.RemoveMember?.user_id === identity.value.user_id) {
      group.removed_self_at = Date.now()
      group.removed_self_by = info.actor_user_id
      appendLog('你已被该群事件移出群聊')
    }
  } else {
    const admins = group.admin_user_ids ?? []
    const isSelfLeave = info.action.RemoveMember?.user_id === info.actor_user_id
    if (info.action.RemoveMember && !isSelfLeave) throw new Error('群权限拒绝：成员只能自己退出，不能移除其他成员')
    if (!isSelfLeave && admins.length > 0 && !admins.includes(info.actor_user_id)) throw new Error('群权限拒绝：只有管理员可执行该事件')
    if (info.action.Rename) {
      group.name = info.action.Rename.name
    } else if (info.action.AddMember) {
      const uid = info.action.AddMember.user_id
      if (!group.member_user_ids.includes(uid)) group.member_user_ids.push(uid)
      if (uid === identity.value?.user_id) {
        group.removed_self_at = undefined
        group.removed_self_by = undefined
      }
    } else if (info.action.RemoveMember) {
      const uid = info.action.RemoveMember.user_id
      group.member_user_ids = group.member_user_ids.filter((id) => id !== uid)
      group.admin_user_ids = (group.admin_user_ids ?? []).filter((id) => id !== uid)
      if (uid === identity.value?.user_id) {
        group.removed_self_at = Date.now()
        group.removed_self_by = info.actor_user_id
        appendLog('你已被该群事件移出群聊')
      }
    } else if (info.action.PromoteAdmin) {
      const uid = info.action.PromoteAdmin.user_id
      if (!group.admin_user_ids) group.admin_user_ids = []
      if (!group.admin_user_ids.includes(uid)) group.admin_user_ids.push(uid)
    } else if (info.action.DemoteAdmin) {
      const uid = info.action.DemoteAdmin.user_id
      group.admin_user_ids = (group.admin_user_ids ?? []).filter((id) => id !== uid)
    } else {
      throw new Error('未知群事件 action')
    }
    group.sequence = info.sequence
  }
  const summary = summarizeGroupEventAction(info.action)
  group.last_event_summary = summary
  group.last_event_actor_user_id = info.actor_user_id
  group.last_event_at = Date.now()
  group.last_event_error = undefined
  group.last_event_error_at = undefined
  group.last_event_recovery_hint = undefined
  const membershipChanged = Boolean(info.action.AddMember || info.action.RemoveMember)
  if (membershipChanged) {
    groupSenderKeys.value = groupSenderKeys.value.filter((k) => k.group_id !== group.group_id || k.sender_user_id === identity.value?.user_id)
    if (group.member_user_ids.includes(identity.value?.user_id || '') || (group.admin_user_ids || []).includes(identity.value?.user_id || '')) {
      rotateMyGroupSenderKey(group, summary)
    } else if (info.action.RemoveMember?.user_id === identity.value?.user_id) {
      groupSenderKeys.value = groupSenderKeys.value.filter((k) => k.group_id !== group.group_id)
    }
  }
  activeGroupId.value = group.group_id
  activePeerId.value = ''
  return { group_id: group.group_id, summary }
}

function applyGroupEventText() {
  run('应用群事件', () => {
    const text = incomingGroupEventText.value.trim() || groupEventText.value.trim()
    const actorId = groupEventActorUserId.value.trim() || activeContact.value?.user_id || identity.value?.user_id || ''
    let result: { group_id: string; summary: string }
    try {
      result = applyGroupEventRaw(text, actorId)
    } catch (e) {
      const reason = userFacingError(e)
      rememberGroupEventError(activeGroup.value?.group_id, reason)
      persist()
      throw e
    }
    messages.value.push({
      id: newId(),
      conversation_id: `grp-${result.group_id}`,
      peer_user_id: actorId,
      group_id: result.group_id,
      direction: actorId === identity.value?.user_id ? 'out' : 'in',
      text: `[群事件] ${result.summary}`,
      envelope_json: text,
      status: 'received',
      created_at: Date.now(),
    })
    incomingGroupEventText.value = ''
    persist()
  })
}

function createGroupEventFanout() {
  run('生成群事件 fanout', () => {
    createGroupEventFanoutRaw()
  })
}

function createGroupEventFanoutRaw() {
  if (!activeGroup.value) throw new Error('请选择群组')
  if (!groupEventText.value.trim()) throw new Error('请先生成群事件')
  const fanout = activeGroup.value.member_user_ids.map((uid) => {
    const contact = contacts.value.find((c) => c.user_id === uid)
    if (!contact || contact.state !== 'Friend') throw new Error(`群成员还不是好友: ${uid}`)
    const envelope = encryptEnvelopeForContact(
      contact,
      `grp-${activeGroup.value!.group_id}`,
      `${GROUP_EVENT_PAYLOAD_PREFIX}${groupEventText.value}`,
    )
    return { to_user_id: uid, envelope }
  })
  groupEventFanoutJson.value = JSON.stringify(fanout, null, 2)
  appendLog(`✅ 已为 ${fanout.length} 个群成员生成群事件 fanout`)
}

function addIncomingGroupInvite() {
  run('加入群邀请收件箱', () => {
    if (!activeContact.value) throw new Error('请先选择邀请者联系人')
    const info = JSON.parse(inspect_group_invite(incomingGroupInviteText.value, activeContact.value.contact_card_text)) as Omit<GroupInviteItem, 'invite_text'>
    if (identity.value && !info.member_user_ids.includes(identity.value.user_id)) {
      throw new Error('该群邀请成员列表不包含当前身份')
    }
    const item: GroupInviteItem = { ...info, invite_text: incomingGroupInviteText.value }
    const index = groupInvites.value.findIndex((g) => g.invite_id === item.invite_id)
    if (index >= 0) groupInvites.value[index] = item
    else groupInvites.value.unshift(item)
    incomingGroupInviteText.value = ''
    persist()
  })
}

function acceptGroupInvite(invite: GroupInviteItem) {
  run('接受群邀请', () => {
    const riskText = groupInviteStrictE2eeRiskText(invite)
    if (riskText) {
      const ok = confirm(`
${riskText}

建议先添加/核验邀请成员并刷新 ContactCard DHT。仍要接受群邀请吗？`)
      if (!ok) throw new Error('已取消接受群邀请')
    }
    const group: GroupItem = {
      group_id: invite.group_id,
      name: invite.group_name,
      member_user_ids: invite.member_user_ids.filter((id) => id !== identity.value?.user_id),
      admin_user_ids: [invite.inviter_user_id],
      policy_state_json: create_group_policy_state(invite.group_id, invite.group_name, invite.inviter_user_id, JSON.stringify(invite.member_user_ids)),
      created_at: Date.now(),
      sequence: 0,
      last_event_summary: '已接受群邀请；历史消息不会自动同步',
      last_event_actor_user_id: invite.inviter_user_id,
      last_event_at: Date.now(),
    }
    const index = groups.value.findIndex((g) => g.group_id === group.group_id)
    if (index >= 0) groups.value[index] = group
    else groups.value.push(group)
    groupInvites.value = groupInvites.value.filter((g) => g.invite_id !== invite.invite_id)
    activeGroupId.value = group.group_id
    activePeerId.value = ''
    persist()
  })
}

function ignoreGroupInvite(invite: GroupInviteItem) {
  groupInvites.value = groupInvites.value.filter((g) => g.invite_id !== invite.invite_id)
  persist()
}

async function removeActiveGroup() {
  if (!activeGroup.value) return
  const id = activeGroup.value.group_id
  const name = activeGroup.value.name
  const ok = await showConfirm('退出群聊', `退出「${name}」并删除本地群消息和群密钥？这只影响本设备，不会通知其他成员。`, true)
  if (!ok) return
  groups.value = groups.value.filter((g) => g.group_id !== id)
  messages.value = messages.value.filter((m) => m.group_id !== id)
  groupSenderKeys.value = groupSenderKeys.value.filter((k) => k.group_id !== id)
  activeGroupId.value = groups.value[0]?.group_id ?? ''
  appendLog(`已本地退出群聊：${name}`)
  persist()
}

async function leaveActiveGroupWithNotice() {
  if (!activeGroup.value || !identity.value) return
  const group = activeGroup.value
  const ok = await showConfirm('通知退群', `生成并发送「${group.name}」退群通知，然后删除本地群消息和群密钥？`, true)
  if (!ok) return
  try {
    createRemoveMemberGroupEventText(identity.value.user_id)
    createGroupEventFanoutRaw()
  } catch (e) {
    const message = userFacingError(e)
    appendLog(`❌ 生成退群通知失败：${message}`)
    showAlert('生成退群通知失败', message, 'error')
    return
  }
  try {
    const fanout = groupEventFanoutItems.value
    let sent = 0
    let queued = 0
    let failed = 0
    for (const item of fanout) {
      const contact = contacts.value.find((c) => c.user_id === item.to_user_id)
      if (!contact) { failed += 1; continue }
      const result = await deliverPayloadToContact(contact, item.envelope, '退群通知', 'group-fanout')
      if (result === 'sent' || result === 'mailbox') sent += 1
      else {
        queued += 1
        outbox.value.push(createOutboxItem(contact, item.envelope, undefined, 'group-fanout'))
      }
    }
    appendLog(`退群通知发送完成：已发送 ${sent}，待发送 ${queued}，失败 ${failed}`)
  } finally {
    groups.value = groups.value.filter((g) => g.group_id !== group.group_id)
    messages.value = messages.value.filter((m) => m.group_id !== group.group_id)
    groupSenderKeys.value = groupSenderKeys.value.filter((k) => k.group_id !== group.group_id)
    activeGroupId.value = groups.value[0]?.group_id ?? ''
    appendLog(`已本地退出群聊：${group.name}`)
    persist()
  }
}


function createFriendRequestForActiveLocalOnly() {
  run('生成好友请求', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    if (!myContactCardText.value) exportMyCard()
    friendRequestText.value = create_friend_request(
      backupText.value,
      passphrase.value,
      myContactCardText.value,
      activeContact.value.contact_card_text,
      '你好，我想添加你',
    )
    const req = safeJson<any>(inspect_friend_request(friendRequestText.value))
    activeContact.value.state = 'RequestSent'
    activeContact.value.pending_request_id = req.request_id
    activeContact.value.last_friend_request_error = undefined
    persist()
  })
}

function createFriendRequestForActive() {
  run('发送好友请求', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    if (!myContactCardText.value) exportMyCard()
    friendRequestText.value = create_friend_request(
      backupText.value,
      passphrase.value,
      myContactCardText.value,
      activeContact.value.contact_card_text,
      '你好，我想添加你',
    )
    const req = safeJson<any>(inspect_friend_request(friendRequestText.value))
    if (!nodeEnabled.value) {
      showAlert('请先开启消息同步', '好友请求需要通过消息同步发送。请到“我 → 消息同步”开启后重试。', 'warning')
      return
    }
    const contact = activeContact.value
    contact.state = 'RequestSent'
    contact.pending_request_id = req.request_id
    contact.last_friend_request_error = undefined
    persist()
    void pushMailboxPayload(contact, 'other', friendRequestText.value)
      .then(() => {
        appendLog('✅ 好友请求已发送')
        toast('好友请求已发送', 'success')
      })
      .catch((e) => {
        contact.state = 'LocalOnly'
        contact.pending_request_id = undefined
        const message = userFacingError(e)
        contact.last_friend_request_error = message
        persist()
        appendLog(`⚠️ 好友请求发送失败：${message}`)
        showAlert('发送失败', message, 'error')
      })
  })
}

function clearActiveFriendRequestError() {
  run('清除好友请求错误', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    activeContact.value.last_friend_request_error = undefined
    persist()
  })
}

function recreateActiveRatchetSession() {
  run('重建 Ratchet 会话', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    if (activeContact.value.state !== 'Friend') throw new Error('联系人还不是 Friend')
    if (!activeContact.value.contact_card_text) throw new Error('缺少联系人名片')
    const out = JSON.parse(create_ratchet_session_pair(myContactCardText.value, activeContact.value.contact_card_text)) as {
      local_state_text: string
      remote_state_text: string
    }
    ratchetStateText.value = out.local_state_text
    ratchetPeerStateText.value = out.remote_state_text
    saveRatchetSession(activeContact.value.user_id, out.local_state_text)
    activeContact.value.last_secure_session_error = undefined
    activeContact.value.last_secure_session_success_at = Date.now()
    activeContact.value.secure_session_failure_count = 0
    secureSessionStatusText.value = '已重建本地 Ratchet Session。'
    persist()
  })
}


function ratchetSessionFor(userId: string): RatchetSessionItem | null {
  return ratchetSessions.value.find((r) => r.peer_user_id === userId) ?? null
}

function saveRatchetSession(userId: string, stateText: string) {
  const item: RatchetSessionItem = { peer_user_id: userId, state_text: stateText, updated_at: Date.now() }
  const index = ratchetSessions.value.findIndex((r) => r.peer_user_id === userId)
  if (index >= 0) ratchetSessions.value[index] = item
  else ratchetSessions.value.push(item)
}

function removeRatchetSession(userId: string) {
  ratchetSessions.value = ratchetSessions.value.filter((r) => r.peer_user_id !== userId)
}

function recordSecureSessionError(contact: ContactItem, error: unknown, logPrefix: string) {
  const message = userFacingError(error)
  contact.last_secure_session_error = message
  contact.secure_session_failure_count = (contact.secure_session_failure_count ?? 0) + 1
  persist()
  appendLog(`${logPrefix}：${message}`)
}

function encryptEnvelopeForContact(contact: ContactItem, conversationId: string, text: string): string {
  requireContactHasActiveDevice(contact)
  const session = ratchetSessionFor(contact.user_id)
  if (session) {
    const out = JSON.parse(ratchet_encrypt_text_message(session.state_text, conversationId, text)) as { state_text: string; envelope_json: string }
    saveRatchetSession(contact.user_id, out.state_text)
    appendLog(`🔐 使用 Double Ratchet 加密给 ${contact.display_name || contact.user_id}`)
    return out.envelope_json
  }
  appendLog(`⚠️ 未找到 Ratchet 会话，回退 MVP 加密：${contact.display_name || contact.user_id}`)
  return encrypt_text_message(
    backupText.value,
    passphrase.value,
    contact.contact_card_text,
    conversationId,
    text,
  )
}

function decryptEnvelopeForContact(envelopeText: string, sender: ContactItem): any {
  let parsed: any = null
  try { parsed = JSON.parse(envelopeText) } catch {}
  if (parsed?.crypto === 'x3dh-double-ratchet-v1') {
    const session = ratchetSessionFor(sender.user_id)
    if (!session) throw new Error('收到 Ratchet Envelope，但本地没有该联系人的 Ratchet Session')
    const out = JSON.parse(ratchet_decrypt_text_message(session.state_text, envelopeText)) as { state_text: string; plain_json: string }
    saveRatchetSession(sender.user_id, out.state_text)
    appendLog(`🔓 已用 Double Ratchet 解密 ${sender.display_name || sender.user_id}`)
    return safeJson<any>(out.plain_json)
  }
  return safeJson<any>(decrypt_text_message(
    backupText.value,
    passphrase.value,
    sender.contact_card_text,
    envelopeText,
  ))
}

async function sendGroupFanoutPayloads(group: GroupItem, fanout: Array<{ to_user_id: string; envelope: string }>, messageId: string) {
  let mailboxCount = 0
  let queuedCount = 0
  let failedCount = 0
  for (const item of fanout) {
    const contact = contacts.value.find((c) => c.user_id === item.to_user_id)
    if (!contact) { failedCount += 1; continue }
    const result = await deliverPayloadToContact(contact, item.envelope, '群 fanout', 'group-fanout')
    if (result === 'mailbox' || result === 'sent') mailboxCount += 1
    else {
      queuedCount += 1
      outbox.value.push(createOutboxItem(contact, item.envelope, messageId, 'group-fanout'))
    }
  }
  const msg = messages.value.find((m) => m.id === messageId)
  if (msg) msg.status = failedCount > 0 ? 'failed' : queuedCount > 0 ? 'queued' : 'mailbox'
  appendLog(`群消息发送完成：已发送 ${mailboxCount}，待发送 ${queuedCount}，失败 ${failedCount}`)
  persist()
}

async function sendMessage() {
  const pendingText = composerText.value
  if (pendingText.trim() && !(await confirmOutgoingTextIfNeeded(pendingText))) {
    appendLog('已取消发送风险文本')
    return
  }
  if (pendingText.trim() && activeContact.value && !activeGroup.value) {
    if (!(await confirmStrictE2eeSendRiskIfNeeded(activeContact.value))) {
      appendLog('已取消发送：严格 E2EE 风险未确认')
      return
    }
    try {
      requireVerifiedContactForSend(activeContact.value)
    } catch (error) {
      showAlert('发送被安全策略阻止', userFacingError(error), 'warning')
      appendLog(`已阻止发送给未核验联系人：${userFacingError(error)}`)
      return
    }
  }
  if (pendingText.trim() && activeGroup.value) {
    const riskText = activeGroupStrictE2eeRiskText.value
    if (riskText && strictE2eePolicyEnabled.value) {
      showAlert('群消息被严格 E2EE 策略阻止', riskText, 'warning')
      appendLog(`已阻止群消息发送：${riskText}`)
      return
    }
    if (riskText && !(await showConfirm('群聊严格 E2EE 风险提示', `${riskText}

建议先修复群成员指纹、ContactCard DHT 和 sealed slot 覆盖，再发送群消息。仍要继续发送吗？`, true))) {
      appendLog('已取消群消息发送：严格 E2EE 风险未确认')
      return
    }
  }
  run('发送消息', () => {
    if (!composerText.value.trim()) return
    ensureUiTextSize('消息', composerText.value, MAX_TEXT_MESSAGE_BYTES)

    const outgoingFiltered = applyLocalTextFilter(composerText.value, 'out')
    if (!outgoingFiltered.allow) throw new Error('本地策略阻止发送')

    if (activeGroup.value) {
      if (activeGroup.value.removed_self_at) throw new Error('你已被移出该群聊，不能继续发送群消息')
      let fanout: Array<{ to_user_id: string; envelope: string }> = []
      const senderEnvelope = encryptGroupSenderText(activeGroup.value, `[${activeGroup.value.name}] ${outgoingFiltered.text}`)
      if (senderEnvelope) {
        fanout = activeGroup.value.member_user_ids.map((uid) => ({ to_user_id: uid, envelope: senderEnvelope }))
        groupSenderEnvelopeText.value = senderEnvelope
      } else {
        fanout = activeGroup.value.member_user_ids.map((uid) => {
          const contact = contacts.value.find((c) => c.user_id === uid)
          if (!contact || contact.state !== 'Friend') throw new Error(`群成员还不是好友: ${uid}`)
          const envelope = encryptEnvelopeForContact(
            contact,
            `grp-${activeGroup.value!.group_id}`,
            `[${activeGroup.value!.name}] ${outgoingFiltered.text}`,
          )
          return { to_user_id: uid, envelope }
        })
      }
      groupFanoutJson.value = JSON.stringify(fanout, null, 2)
      const msg: ChatMessage = {
        id: newId(),
        conversation_id: `grp-${activeGroup.value.group_id}`,
        peer_user_id: '',
        group_id: activeGroup.value.group_id,
        direction: 'out',
        text: outgoingFiltered.text,
        envelope_json: groupFanoutJson.value,
        status: 'queued',
        created_at: Date.now(),
      }
      messages.value.push(msg)
      appendLog(`✅ 群消息已准备发送给 ${fanout.length} 个成员`)
      composerText.value = ''
      persist()
      void sendGroupFanoutPayloads(activeGroup.value, fanout, msg.id)
      return
    }

    if (!activeContact.value) throw new Error('请选择联系人')
    if (activeContact.value.state === 'Blocked') throw new Error('联系人已被拉黑')
    if (activeContact.value.state !== 'Friend') throw new Error('对方通过好友请求后才能聊天')
    const conversationId = `conv-${activeContact.value.user_id}`
    const envelope = encryptEnvelopeForContact(
      activeContact.value,
      conversationId,
      outgoingFiltered.text,
    )
    const targetDeviceIds = contactActiveDeviceIds(activeContact.value)
    const perDeviceEnvelope = createPerDeviceEnvelopeDraft(activeContact.value, conversationId, envelope)
    if (perDeviceEnvelope) {
      perDeviceEnvelopeSentCount.value += 1
      lastPerDeviceEnvelopeAt.value = Date.now()
    }
    const msg: ChatMessage = {
      id: newId(),
      conversation_id: conversationId,
      peer_user_id: activeContact.value.user_id,
      direction: 'out',
      text: outgoingFiltered.text,
      envelope_json: envelope,
      protocol_message_id: messageProtocolIdFromEnvelope(envelope),
      target_device_ids: targetDeviceIds,
      per_device_envelope_json: perDeviceEnvelope,
      per_device_envelope_version: perDeviceEnvelope ? 1 : undefined,
      status: 'queued',
      created_at: Date.now(),
    }
    const deliveryPayload = perDeviceEnvelope ?? envelope
    inboundEnvelopeText.value = deliveryPayload
    if (dc && dc.readyState === 'open') {
      sendRtcText(deliveryPayload, '消息')
      msg.status = 'sent'
      appendLog(perDeviceEnvelope ? '✅ 分设备 envelope 已发送' : '✅ 消息已发送')
    } else if (nodeEnabled.value) {
      appendLog(perDeviceEnvelope ? '正在通过消息同步发送分设备 envelope' : '正在通过消息同步发送')
      void tryMailboxDeliveryForMessage(activeContact.value, deliveryPayload, msg)
    } else {
      outbox.value.push(createOutboxItem(activeContact.value, deliveryPayload, msg.id, 'direct-envelope'))
      appendLog('未开启消息同步，消息已暂存，开启同步后会自动重发')
    }
    messages.value.push(msg)
    composerText.value = ''
    persist()
  })
}


function perDeviceEnvelopeSigningPayload(envelope: PerDeviceEnvelopeV1): string {
  return JSON.stringify({
    type: envelope.type,
    version: envelope.version,
    conversation_id: envelope.conversation_id,
    sender_user_id: envelope.sender_user_id,
    target_devices: envelope.target_devices,
    fallback_ciphertext: envelope.fallback_ciphertext,
    created_at: envelope.created_at,
  })
}

function createPerDeviceEnvelopeDraft(contact: ContactItem, conversationId: string, ciphertext: string): string | undefined {
  const targetDeviceIds = contactActiveDeviceIds(contact)
  if (targetDeviceIds.length === 0) return undefined
  const senderUserId = identity.value?.user_id || ''
  const createdAt = Date.now()
  const draft: PerDeviceEnvelopeV1 = {
    type: 'lm-per-device-envelope-v1',
    version: 1,
    conversation_id: conversationId,
    sender_user_id: senderUserId,
    target_devices: targetDeviceIds.map((deviceId) => {
      const aad = perDeviceSlotAad(conversationId, senderUserId, deviceId, createdAt)
      const cert = (contact.device_certs ?? []).find((item) => item.device_id === deviceId)
      if (cert?.device_box_public_key) {
        const sealed = safeJson<{
          crypto: 'x25519-ephemeral-hkdf-xchacha20poly1305-device-slot-v1'
          x25519_ephemeral_public_key: string
          nonce: string
          ciphertext: string
        }>(seal_device_slot(cert.device_box_public_key, aad, ciphertext))
        return {
          device_id: deviceId,
          slot_id: randomBase64Url(16),
          nonce: sealed.nonce,
          aad,
          crypto: sealed.crypto,
          x25519_ephemeral_public_key: sealed.x25519_ephemeral_public_key,
          ciphertext: sealed.ciphertext,
        }
      }
      return {
        device_id: deviceId,
        slot_id: randomBase64Url(16),
        nonce: randomBase64Url(24),
        aad,
        crypto: 'placeholder-shared-envelope-v1',
        ciphertext,
      }
    }),
    // Fallback remains for older/self devices until every contact device cert has a box key.
    fallback_ciphertext: ciphertext,
    created_at: createdAt,
  }
  draft.signature = sign_identity_text(backupText.value, passphrase.value, perDeviceEnvelopeSigningPayload(draft))
  return JSON.stringify(draft)
}

function perDeviceEnvelopeTargetCount(message: ChatMessage): number {
  if (message.per_device_envelope_json) {
    try {
      const parsed = JSON.parse(message.per_device_envelope_json) as PerDeviceEnvelopeV1
      if (Array.isArray(parsed.target_devices)) return parsed.target_devices.length
    } catch {}
  }
  return message.target_device_ids?.length ?? 0
}


function unwrapPerDeviceEnvelopeForCurrentDevice(payloadText: string, sender: ContactItem): {
  envelopeText: string
  perDeviceEnvelopeJson?: string
  targetDeviceIds?: string[]
  slotCrypto?: string
} {
  let parsed: PerDeviceEnvelopeV1 | null = null
  try { parsed = JSON.parse(payloadText) as PerDeviceEnvelopeV1 } catch {}
  if (parsed?.type !== 'lm-per-device-envelope-v1') return { envelopeText: payloadText }
  const recordDrop = (reason: string): never => {
    perDeviceEnvelopeDropCount.value += 1
    lastPerDeviceEnvelopeDropAt.value = Date.now()
    lastPerDeviceEnvelopeDropReason.value = reason
    throw new Error(reason)
  }
  if (parsed.version !== 1) recordDrop(`不支持的分设备 envelope 版本：${parsed.version}`)
  if (parsed.sender_user_id !== sender.user_id) recordDrop(`分设备 envelope 发送者不匹配：${parsed.sender_user_id || 'unknown'}`)
  const signature = parsed.signature
  if (!signature) return recordDrop('分设备 envelope 缺少身份签名')
  if (!verify_identity_text_signature(sender.identity_public_key, perDeviceEnvelopeSigningPayload(parsed), signature)) {
    recordDrop('分设备 envelope 身份签名无效')
  }
  const targets = Array.isArray(parsed.target_devices) ? parsed.target_devices : []
  const targetDeviceIds = targets.map((item) => String(item.device_id || '')).filter(Boolean)
  const slotIds = targets.map((item) => String(item.slot_id || '')).filter(Boolean)
  if (slotIds.length !== new Set(slotIds).size) recordDrop('分设备 envelope slot_id 重复')
  const currentDeviceId = myDeviceId.value
  if (currentDeviceId) {
    const matched = targets.find((item) => item.device_id === currentDeviceId)
    if (!matched) return recordDrop(`分设备 envelope 未投递给当前设备：${currentDeviceId}`)
    if (!matched.slot_id || !matched.nonce || !matched.aad) return recordDrop('分设备 envelope 缺少当前设备 slot 元数据')
    if (!matched.ciphertext) return recordDrop('分设备 envelope 缺少当前设备密文')
    let envelopeText = matched.ciphertext
    if (matched.crypto === 'x25519-ephemeral-hkdf-xchacha20poly1305-device-slot-v1') {
      if (!matched.x25519_ephemeral_public_key) return recordDrop('分设备 sealed slot 缺少临时公钥')
      if (!myDeviceBackupText.value) return recordDrop('当前设备缺少设备私钥备份，无法打开 sealed slot')
      envelopeText = open_device_slot(
        backupText.value,
        passphrase.value,
        myDeviceBackupText.value,
        matched.aad,
        JSON.stringify({
          crypto: matched.crypto,
          x25519_ephemeral_public_key: matched.x25519_ephemeral_public_key,
          nonce: matched.nonce,
          ciphertext: matched.ciphertext,
        }),
      )
    }
    perDeviceEnvelopeReceivedCount.value += 1
    lastPerDeviceEnvelopeAt.value = Date.now()
    return { envelopeText, perDeviceEnvelopeJson: payloadText, targetDeviceIds, slotCrypto: matched.crypto }
  }
  if (parsed.fallback_ciphertext) {
    appendLog('⚠️ 当前设备没有 device_id，使用分设备 envelope fallback 密文')
    perDeviceEnvelopeReceivedCount.value += 1
    lastPerDeviceEnvelopeAt.value = Date.now()
    return { envelopeText: parsed.fallback_ciphertext, perDeviceEnvelopeJson: payloadText, targetDeviceIds, slotCrypto: 'fallback-ciphertext' }
  }
  if (targets.length === 1 && targets[0]?.ciphertext) {
    appendLog('⚠️ 当前设备没有 device_id，使用唯一目标设备密文')
    perDeviceEnvelopeReceivedCount.value += 1
    lastPerDeviceEnvelopeAt.value = Date.now()
    return { envelopeText: targets[0].ciphertext, perDeviceEnvelopeJson: payloadText, targetDeviceIds, slotCrypto: targets[0].crypto }
  }
  return recordDrop('当前设备没有 device_id，无法选择分设备 envelope 密文')
}

function receiveEnvelopeWithContact(envelopeText: string, sender: ContactItem, mailboxDeliveryId?: string): boolean {
  if (sender.state === 'Blocked') throw new Error('发送者已被拉黑')
  if (!allowIncomingFromContact(sender)) { persist(); return false }
  ensureUiTextSize('Envelope', envelopeText, MAX_SIGNAL_BYTES)
  const unwrappedEnvelope = unwrapPerDeviceEnvelopeForCurrentDevice(envelopeText, sender)
  if (safetyPolicy.value.requireSealedPerDeviceSlotsForReceive && unwrappedEnvelope.slotCrypto !== 'x25519-ephemeral-hkdf-xchacha20poly1305-device-slot-v1') {
    perDeviceEnvelopeDropCount.value += 1
    lastPerDeviceEnvelopeDropAt.value = Date.now()
    lastPerDeviceEnvelopeDropReason.value = `安全策略要求 sealed slot 入站，但收到 ${unwrappedEnvelope.slotCrypto || 'direct-envelope'}`
    throw new Error(lastPerDeviceEnvelopeDropReason.value)
  }
  const innerEnvelopeText = unwrappedEnvelope.envelopeText
  ensureUiTextSize('Inner Envelope', innerEnvelopeText, MAX_SIGNAL_BYTES)
  const groupSenderPlain = tryDecryptGroupSenderEnvelope(innerEnvelopeText)
  if (groupSenderPlain) {
    const filtered = applyLocalTextFilter(groupSenderPlain.text, 'in')
    if (!filtered.allow) { persist(); return true }
    messages.value.push({
      id: newId(),
      conversation_id: `grp-${groupSenderPlain.group_id}`,
      peer_user_id: groupSenderPlain.sender_user_id,
      group_id: groupSenderPlain.group_id,
      direction: 'in',
      text: filtered.text,
      envelope_json: innerEnvelopeText,
      per_device_envelope_json: unwrappedEnvelope.perDeviceEnvelopeJson,
      per_device_envelope_version: unwrappedEnvelope.perDeviceEnvelopeJson ? 1 : undefined,
      target_device_ids: unwrappedEnvelope.targetDeviceIds,
      status: 'received',
      created_at: Date.now(),
    })
    activeGroupId.value = groupSenderPlain.group_id
    activePeerId.value = ''
    persist()
    return true
  }
  const plain = decryptEnvelopeForContact(innerEnvelopeText, sender)
  const rawText = plain.body?.Text?.text ?? JSON.stringify(plain.body)
  if (typeof rawText === 'string' && rawText.startsWith(GROUP_SENDER_KEY_PAYLOAD_PREFIX)) {
    const distribution = rawText.slice(GROUP_SENDER_KEY_PAYLOAD_PREFIX.length)
    const stateJson = import_group_sender_key(distribution, sender.contact_card_text)
    const parsed = JSON.parse(stateJson) as { group_id: string; sender_user_id: string }
    saveGroupSenderKey({
      key_id: groupSenderKeyId(parsed.group_id, parsed.sender_user_id),
      group_id: parsed.group_id,
      sender_user_id: parsed.sender_user_id,
      state_json: stateJson,
      distribution_text: distribution,
      updated_at: Date.now(),
    })
    messages.value.push({
      id: newId(),
      conversation_id: `grp-${parsed.group_id}`,
      peer_user_id: sender.user_id,
      group_id: parsed.group_id,
      direction: 'in',
      text: '[群 Sender Key] 已导入新的 Sender Key Distribution',
      envelope_json: innerEnvelopeText,
      per_device_envelope_json: unwrappedEnvelope.perDeviceEnvelopeJson,
      per_device_envelope_version: unwrappedEnvelope.perDeviceEnvelopeJson ? 1 : undefined,
      target_device_ids: unwrappedEnvelope.targetDeviceIds,
      status: 'received',
      created_at: Date.now(),
    })
    persist()
    return true
  }
  if (typeof rawText === 'string' && rawText.startsWith(GROUP_EVENT_PAYLOAD_PREFIX)) {
    const eventText = rawText.slice(GROUP_EVENT_PAYLOAD_PREFIX.length)
    let result: { group_id: string; summary: string }
    try {
      result = applyGroupEventRaw(eventText, sender.user_id)
    } catch (e) {
      const groupId = groupIdFromEventText(eventText, sender.user_id)
      rememberGroupEventError(groupId, userFacingError(e))
      throw e
    }
    messages.value.push({
      id: newId(),
      conversation_id: `grp-${result.group_id}`,
      peer_user_id: sender.user_id,
      group_id: result.group_id,
      direction: 'in',
      text: `[群事件] ${result.summary}`,
      envelope_json: innerEnvelopeText,
      per_device_envelope_json: unwrappedEnvelope.perDeviceEnvelopeJson,
      per_device_envelope_version: unwrappedEnvelope.perDeviceEnvelopeJson ? 1 : undefined,
      target_device_ids: unwrappedEnvelope.targetDeviceIds,
      status: 'received',
      created_at: Date.now(),
    })
    persist()
    return true
  }
  const filtered = applyLocalTextFilter(rawText, 'in')
  if (!filtered.allow) {
    persist()
    return true
  }
  const text = filtered.text
  const conversationId = plain.conversation_id ?? `conv-${sender.user_id}`
  const groupId = typeof conversationId === 'string' && conversationId.startsWith('grp-')
    ? conversationId.slice(4)
    : undefined
  if (groupId) {
    const group = groups.value.find((g) => g.group_id === groupId)
    if (group) {
      activeGroupId.value = group.group_id
      activePeerId.value = ''
    } else {
      appendLog(`收到未知群组消息：${groupId}，已按普通密文保存`)
    }
  }
  messages.value.push({
    id: newId(),
    conversation_id: conversationId,
    peer_user_id: sender.user_id,
    group_id: groupId,
    direction: 'in',
    text,
    envelope_json: innerEnvelopeText,
    per_device_envelope_json: unwrappedEnvelope.perDeviceEnvelopeJson,
    per_device_envelope_version: unwrappedEnvelope.perDeviceEnvelopeJson ? 1 : undefined,
    target_device_ids: unwrappedEnvelope.targetDeviceIds,
    protocol_message_id: messageProtocolIdFromEnvelope(innerEnvelopeText),
    status: 'received',
    created_at: Date.now(),
  })
  const protocolMessageId = messageProtocolIdFromEnvelope(innerEnvelopeText)
  void sendDeliveryAck(sender, protocolMessageId, conversationId, mailboxDeliveryId)
  if (!groupId && activePeerId.value === sender.user_id) {
    void sendReadReceipt(sender, protocolMessageId, conversationId, mailboxDeliveryId)
  }
  persist()
  return true
}

function receiveEnvelope() {
  run('解密收到的 Envelope', () => {
    if (!activeContact.value) throw new Error('请选择发送者联系人')
    receiveEnvelopeWithContact(inboundEnvelopeText.value, activeContact.value)
    inboundEnvelopeText.value = ''
  })
}


function applyFriendResponse() {
  run('应用好友响应', () => {
    const candidates = activeContact.value ? [activeContact.value, ...contacts.value.filter((c) => c.user_id !== activeContact.value?.user_id)] : contacts.value
    let matchedContact: ContactItem | null = null
    let info: any = null
    let lastError: unknown = null
    for (const contact of candidates) {
      try {
        const parsed = safeJson<any>(inspect_friend_response(incomingFriendResponseText.value, contact.contact_card_text))
        if (parsed.from_user_id === contact.user_id) {
          matchedContact = contact
          info = parsed
          break
        }
      } catch (e) {
        lastError = e
      }
    }
    if (!matchedContact || !info) throw lastError instanceof Error ? lastError : new Error('找不到这个好友响应对应的联系人')
    if (identity.value && info.to_user_id !== identity.value.user_id) {
      throw new Error('这个好友响应不是发给当前身份的')
    }
    if (matchedContact.pending_request_id && info.request_id !== matchedContact.pending_request_id) {
      throw new Error('响应 request_id 与待确认请求不匹配')
    }
    matchedContact.state = info.accepted ? 'Friend' : 'Rejected'
    activePeerId.value = matchedContact.user_id
    activeGroupId.value = ''
    incomingFriendResponseText.value = ''
    persist()
  })
}

function removeActiveContact() {
  if (!activeContact.value) return
  const id = activeContact.value.user_id
  contacts.value = contacts.value.filter((c) => c.user_id !== id)
  messages.value = messages.value.filter((m) => m.peer_user_id !== id)
  ratchetSessions.value = ratchetSessions.value.filter((r) => r.peer_user_id !== id)
  activePeerId.value = contacts.value[0]?.user_id ?? ''
  persist()
}

async function clearActiveConversation() {
  if (!activeContact.value && !activeGroup.value) return
  const name = activeGroup.value?.name || activeContact.value?.display_name || activeContact.value?.user_id || '当前聊天'
  const confirmed = await showConfirm('清空聊天记录', `清空「${name}」的本地聊天记录？联系人或群聊不会删除。`, true)
  if (!confirmed) return
  if (activeGroup.value) {
    const groupId = activeGroup.value.group_id
    messages.value = messages.value.filter((m) => m.group_id !== groupId)
  } else if (activeContact.value) {
    const peerId = activeContact.value.user_id
    messages.value = messages.value.filter((m) => m.peer_user_id !== peerId)
  }
  appendLog(`已清空聊天记录：${name}`)
  persist()
}

function blockActiveContact() {
  if (!activeContact.value) return
  activeContact.value.state = 'Blocked'
  activeContact.value.block_reason = blockReason.value || 'local block'
  outbox.value = outbox.value.filter((o) => o.peer_user_id !== activeContact.value?.user_id)
  blockReason.value = ''
  appendLog('已拉黑联系人：后续消息不会发送/接收')
  persist()
}

function unblockActiveContact() {
  if (!activeContact.value) return
  activeContact.value.state = 'LocalOnly'
  activeContact.value.block_reason = undefined
  appendLog('已解除拉黑；如需聊天请重新完成好友确认')
  persist()
}



async function retryOutboxItem(item: OutboxItem): Promise<boolean> {
  if (item.status === 'sent') return true
  if (item.retry_count >= MAX_OUTBOX_RETRY_COUNT) {
    item.status = 'failed'
    item.last_error = `已达到最大重试次数 ${MAX_OUTBOX_RETRY_COUNT}`
    return false
  }
  const contact = contacts.value.find((c) => c.user_id === item.peer_user_id)
  if (!contact || contact.state === 'Blocked') {
    item.status = 'failed'
    item.last_error = contact ? '联系人已拉黑' : '联系人不存在'
    return false
  }
  if (item.expires_at && Date.now() > item.expires_at) {
    item.status = 'failed'
    item.last_error = '已过期'
    return false
  }
  const result = await deliverPayloadToContact(contact, item.envelope_json, 'Outbox 重试', item.kind ?? 'direct-envelope')
  item.retry_count += 1
  if (result === 'sent' || result === 'mailbox') {
    markOutboxSent(item)
    return true
  }
  item.status = result === 'failed' ? 'failed' : 'queued'
  if (item.retry_count >= MAX_OUTBOX_RETRY_COUNT) {
    item.status = 'failed'
    item.next_retry_at = undefined
    item.last_error = `已达到最大重试次数 ${MAX_OUTBOX_RETRY_COUNT}`
  } else {
    item.next_retry_at = Date.now() + retryDelayMs(item.retry_count)
    item.last_error = result === 'failed' ? lastDeliveryError || '投递失败' : undefined
  }
  return false
}

async function retryDueOutbox() {
  if (!loggedIn.value) return
  const now = Date.now()
  let attempted = 0
  for (const item of outbox.value) {
    if (item.status !== 'sent' && (item.next_retry_at ?? item.created_at) <= now) {
      attempted += 1
      await retryOutboxItem(item)
    }
  }
  if (attempted > 0) {
    appendLog(`Outbox 自动重试 ${attempted} 条`)
    persist()
  }
}

function startOutboxRetryLoop() {
  if (outboxRetryTimer) return
  outboxRetryTimer = window.setInterval(() => { void retryDueOutbox() }, 30_000)
}

function flushOutboxForActive() {
  run('重发当前联系人待发送队列', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    let count = 0
    for (const item of outbox.value) {
      if (item.peer_user_id === activeContact.value.user_id && item.status !== 'sent') {
        item.next_retry_at = Date.now()
        count += 1
      }
    }
    void retryDueOutbox()
    appendLog(`已触发重发 ${count} 条`)
    persist()
  })
}

function retryAllOutbox() {
  run('重发全部待发送队列', () => {
    let count = 0
    for (const item of outbox.value) {
      if (item.status !== 'sent') {
        item.next_retry_at = Date.now()
        count += 1
      }
    }
    if (count === 0) throw new Error('没有待发送内容')
    void retryDueOutbox()
    appendLog(`已触发全部重发 ${count} 条`)
    persist()
  })
}

function cancelOutboxForActive() {
  run('取消当前联系人待发送队列', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const peerId = activeContact.value.user_id
    const pending = outbox.value.filter((item) => item.peer_user_id === peerId && item.status !== 'sent')
    if (pending.length === 0) throw new Error('当前联系人没有待发送内容')
    const cancelledMessageIds = new Set(pending.map((item) => item.message_id).filter(Boolean))
    outbox.value = outbox.value.filter((item) => item.peer_user_id !== peerId || item.status === 'sent')
    for (const msg of messages.value) {
      if (cancelledMessageIds.has(msg.id) && msg.direction === 'out') msg.status = 'failed'
    }
    appendLog(`已取消待发送 ${pending.length} 条`)
    persist()
  })
}

function clearSentOutbox() {
  outbox.value = outbox.value.filter((item) => item.status !== 'sent')
  persist()
}

function resetRtc() {
  try { dc?.close() } catch {}
  try { pc?.close() } catch {}
  pc = null
  dc = null
  rtcStatus.value = '未连接'
}

function setupPeerConnection() {
  resetRtc()
  pc = new RTCPeerConnection({ iceServers: [{ urls: 'stun:stun.l.google.com:19302' }] })
  pc.oniceconnectionstatechange = () => {
    rtcStatus.value = `ICE: ${pc?.iceConnectionState ?? 'closed'}`
  }
  pc.onconnectionstatechange = () => {
    rtcStatus.value = `连接: ${pc?.connectionState ?? 'closed'}`
  }
  pc.ondatachannel = (event) => setupDataChannel(event.channel)
  return pc
}

function setupDataChannel(channel: RTCDataChannel) {
  dc = channel
  dc.onopen = () => {
    rtcStatus.value = 'DataChannel 已打开'
    appendLog('✅ WebRTC DataChannel 已打开')
    flushOutboxForActive()
  }
  dc.onclose = () => {
    rtcStatus.value = 'DataChannel 已关闭'
  }
  dc.onerror = () => appendLog('❌ DataChannel error')
  dc.onmessage = (event) => {
    if (typeof event.data !== 'string') return
    handleRtcText(event.data)
  }
}

function sendRtcText(value: string, label: string) {
  if (!dc || dc.readyState !== 'open') throw new Error('DataChannel 未打开')
  ensureUiTextSize(label, value, MAX_RTC_TEXT_BYTES)
  dc.send(value)
}

function handleRtcText(value: string) {
  if (value.startsWith('lm-message-receipt-v1:')) {
    const sender = activeContact.value
    if (sender) {
      try {
        applyMessageReceiptText(value, sender)
        persist()
        return
      } catch (e) {
        appendLog(`❌ WebRTC 回执处理失败：${userFacingError(e)}`)
      }
    }
  }
  try {
    ensureUiTextSize('WebRTC 消息', value, MAX_RTC_TEXT_BYTES)
    const parsed = JSON.parse(value) as { type?: string }
    if (parsed?.type === 'lm-file-package-v1') {
      incomingFilePackageText.value = value
      appendLog('收到 WebRTC 文件包，自动解析并尝试解密')
      inspectIncomingFilePackage()
      decryptIncomingFilePackage()
      return
    }
  } catch {
    // Not JSON or too large; fall through to message decrypt path so receiveEnvelope logs the real error.
  }

  inboundEnvelopeText.value = value
  appendLog('收到 WebRTC 消息，自动尝试解密')
  receiveEnvelope()
}

async function waitIceGatheringComplete(peer: RTCPeerConnection) {
  if (peer.iceGatheringState === 'complete') return
  await new Promise<void>((resolve) => {
    const done = () => {
      if (peer.iceGatheringState === 'complete') {
        peer.removeEventListener('icegatheringstatechange', done)
        resolve()
      }
    }
    peer.addEventListener('icegatheringstatechange', done)
    setTimeout(() => resolve(), 3000)
  })
}

async function runAsync(label: string, fn: () => Promise<void>) {
  try {
    await fn()
    appendLog(`✅ ${label}`)
  } catch (e) {
    appendLog(`❌ ${label}: ${userFacingError(e)}`)
  }
}

async function createRtcOfferForActive() {
  await runAsync('创建 WebRTC Offer', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const peer = setupPeerConnection()
    setupDataChannel(peer.createDataChannel('lm-talk'))
    const offer = await peer.createOffer()
    await peer.setLocalDescription(offer)
    await waitIceGatheringComplete(peer)
    const sdp = JSON.stringify(peer.localDescription)
    ensureUiTextSize('WebRTC SDP', sdp, MAX_SIGNAL_BYTES)
    localSignalText.value = create_signal_offer(
      backupText.value,
      passphrase.value,
      activeContact.value.user_id,
      sdp,
      BigInt(600),
    )
  })
}

async function acceptRtcOfferForActive() {
  await runAsync('接受 Offer 并创建 Answer', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    ensureUiTextSize('远端 Signal', remoteSignalText.value, MAX_SIGNAL_BYTES)
    const info = JSON.parse(inspect_signal_offer(remoteSignalText.value, activeContact.value.contact_card_text)) as { sdp: string }
    const peer = setupPeerConnection()
    await peer.setRemoteDescription(JSON.parse(info.sdp))
    const answer = await peer.createAnswer()
    await peer.setLocalDescription(answer)
    await waitIceGatheringComplete(peer)
    const sdp = JSON.stringify(peer.localDescription)
    ensureUiTextSize('WebRTC SDP', sdp, MAX_SIGNAL_BYTES)
    localSignalText.value = create_signal_answer(backupText.value, passphrase.value, remoteSignalText.value, sdp, BigInt(600))
  })
}

async function applyRtcAnswerForActive() {
  await runAsync('应用 WebRTC Answer', async () => {
    if (!pc) throw new Error('请先创建 Offer')
    if (!activeContact.value) throw new Error('请选择联系人')
    ensureUiTextSize('远端 Signal', remoteSignalText.value, MAX_SIGNAL_BYTES)
    const info = JSON.parse(inspect_signal_answer(remoteSignalText.value, activeContact.value.contact_card_text)) as { sdp: string }
    await pc.setRemoteDescription(JSON.parse(info.sdp))
  })
}

function parseLines(value: string): string[] {
  return value.split(/\r?\n/).map((line) => line.trim()).filter(Boolean)
}

function defaultInspectPublicKey(): string {
  return activeContact.value?.identity_public_key || identity.value?.identity_public_key || ''
}

function createPeerAnnounceText() {
  run('生成 PeerAnnounce', () => {
    const addresses = parseLines(peerAddressesText.value)
    if (addresses.length === 0) throw new Error('请至少填写一个地址')
    peerAnnounceText.value = create_peer_announce(
      backupText.value,
      passphrase.value,
      JSON.stringify(addresses),
      peerMailboxKey.value.trim() || undefined,
      BigInt(3600),
    )
    peerAnnounceInspectPublicKey.value = identity.value?.identity_public_key || ''
    peerAnnounceInfoText.value = JSON.stringify(JSON.parse(inspect_peer_announce(
      peerAnnounceText.value,
      peerAnnounceInspectPublicKey.value,
    )), null, 2)
  })
}

function inspectPeerAnnounceText() {
  run('验签 PeerAnnounce', () => {
    const key = peerAnnounceInspectPublicKey.value.trim() || defaultInspectPublicKey()
    if (!key) throw new Error('需要发布者 identity_public_key')
    peerAnnounceInspectPublicKey.value = key
    peerAnnounceInfoText.value = JSON.stringify(JSON.parse(inspect_peer_announce(peerAnnounceText.value, key)), null, 2)
  })
}

function createPublicPeerAnnounceText() {
  run('生成 PublicPeerAnnounce', () => {
    const addresses = parseLines(publicPeerAddressesText.value)
    if (addresses.length === 0) throw new Error('请至少填写一个公网地址')
    const peerId = publicPeerId.value.trim() || `public-${identity.value?.user_id.slice(0, 12) || newId()}`
    publicPeerId.value = peerId
    publicPeerAnnounceText.value = create_public_peer_announce(
      backupText.value,
      passphrase.value,
      peerId,
      JSON.stringify(addresses),
      JSON.stringify(publicPeerCapabilities.value),
      BigInt(24 * 3600),
    )
    publicPeerAnnounceInspectPublicKey.value = identity.value?.identity_public_key || ''
    publicPeerAnnounceInfoText.value = JSON.stringify(JSON.parse(inspect_public_peer_announce(
      publicPeerAnnounceText.value,
      publicPeerAnnounceInspectPublicKey.value,
    )), null, 2)
  })
}

function inspectPublicPeerAnnounceText() {
  run('验签 PublicPeerAnnounce', () => {
    const key = publicPeerAnnounceInspectPublicKey.value.trim() || defaultInspectPublicKey()
    if (!key) throw new Error('需要发布者 identity_public_key')
    publicPeerAnnounceInspectPublicKey.value = key
    publicPeerAnnounceInfoText.value = JSON.stringify(JSON.parse(inspect_public_peer_announce(publicPeerAnnounceText.value, key)), null, 2)
  })
}




type SecureSessionOffer = {
  type: 'lm-secure-session-offer-v1'
  version: 1
  from_user_id: string
  to_user_id: string
  prekey_bundle_text: string
  signed_one_time_prekey_record_texts?: string[]
  ratchet_dh_public_key: string
  created_at: number
}

type SecureSessionResponse = {
  type: 'lm-secure-session-response-v1'
  version: 1
  from_user_id: string
  to_user_id: string
  initial_message_json: string
  ratchet_dh_public_key: string
  created_at: number
}

function buildSecureSessionOfferForContact(contact: ContactItem): string {
  if (!identity.value) throw new Error('请先登录')
  if (!prekeyBundleText.value.trim() || !prekeyPrivateBundleJson.value.trim()) {
    createMyPreKeyBundleText()
  }
  const pair = JSON.parse(create_ratchet_dh_keypair()) as { private_key: string; public_key: string }
  ratchetLocalDhKeyPairJson.value = JSON.stringify(pair, null, 2)
  ratchetRemoteDhPublicKeyForInit.value = ''
  ratchetInitRole.value = 'Responder'
  const offer: SecureSessionOffer = {
    type: 'lm-secure-session-offer-v1',
    version: 1,
    from_user_id: identity.value.user_id,
    to_user_id: contact.user_id,
    prekey_bundle_text: prekeyBundleText.value,
    signed_one_time_prekey_record_texts: prekeySignedOneTimeRecordTexts.value,
    ratchet_dh_public_key: pair.public_key,
    created_at: Date.now(),
  }
  secureSessionOfferText.value = JSON.stringify(offer, null, 2)
  secureSessionStatusText.value = '已创建 Offer：把它发给对方；对方应用后会返回 Response。Private PreKey 和 DH private_key 已留在本机。'
  persist()
  return secureSessionOfferText.value
}

async function sendSecureSessionOfferToContact(contact: ContactItem) {
  const offer = buildSecureSessionOfferForContact(contact)
  contact.last_secure_session_attempt_at = Date.now()
  try {
    await pushMailboxPayload(contact, 'other', offer)
    contact.last_secure_session_error = undefined
    contact.secure_session_failure_count = 0
  } catch (e) {
    recordSecureSessionError(contact, e, '⚠️ 安全会话 Offer 发送失败')
    queueOutboxItem(contact, offer, undefined, 'other')
    appendLog(`安全会话 Offer 已加入 outbox 自动重试：${contact.display_name || contact.user_id}`)
    throw e
  }
  appendLog(`✅ 已向 ${contact.display_name || contact.user_id} 发送安全会话 Offer`)
  persist()
}

function retrySecureSessionForActiveContact() {
  run('重试安全建链', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    if (activeContact.value.state !== 'Friend') throw new Error('联系人还不是 Friend')
    if (!nodeEnabled.value) {
      showAlert('请先开启消息同步', '安全建链重试需要通过消息同步发送。请到“我 → 消息同步”开启后重试。', 'warning')
      return
    }
    const contact = activeContact.value
    void sendSecureSessionOfferToContact(contact)
      .then(() => {
        toast('安全建链已重试', 'success')
      })
      .catch((e) => {
        showAlert('安全建链重试失败', userFacingError(e), 'error')
      })
  })
}

function createSecureSessionOfferText() {
  run('创建安全会话 Offer', () => {
    if (!activeContact.value) throw new Error('请先选择联系人')
    buildSecureSessionOfferForContact(activeContact.value)
  })
}

function clearActiveSecureSessionError() {
  run('清除安全建链错误', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    activeContact.value.last_secure_session_error = undefined
    activeContact.value.secure_session_failure_count = 0
    persist()
  })
}

function clearSecureSessionRawText() {
  run('清除安全会话原文', () => {
    secureSessionOfferText.value = ''
    secureSessionResponseText.value = ''
    incomingSecureSessionText.value = ''
    persist()
  })
}

function applySecureSessionOfferText() {
  run('应用安全会话 Offer 并生成 Response', () => {
    if (!identity.value) throw new Error('请先登录')
    const offer = JSON.parse(incomingSecureSessionText.value || secureSessionOfferText.value) as SecureSessionOffer
    if (offer.type !== 'lm-secure-session-offer-v1') throw new Error('不是 secure session offer')
    if (offer.to_user_id !== identity.value.user_id) throw new Error('Offer 不是发给当前身份')
    const contact = contacts.value.find((c) => c.user_id === offer.from_user_id)
    if (!contact || contact.state !== 'Friend') throw new Error('Offer 发送者不是 Friend 联系人')
    activePeerId.value = contact.user_id
    activeGroupId.value = ''

    prekeyBundleText.value = offer.prekey_bundle_text
    prekeySignedOneTimeRecordTexts.value = offer.signed_one_time_prekey_record_texts ?? []
    inspectPreKeyBundleText()
    const signedRecord = prekeySignedOneTimeRecordTexts.value[0] || ''
    selectedSignedOneTimePreKeyRecordText.value = signedRecord
    const init = JSON.parse(signedRecord
      ? create_x3dh_initial_message_with_one_time_prekey_record(
        backupText.value,
        passphrase.value,
        offer.prekey_bundle_text,
        signedRecord,
      )
      : create_x3dh_initial_message_with_one_time_prekey_id(
        backupText.value,
        passphrase.value,
        offer.prekey_bundle_text,
        selectedOneTimePreKeyId.value ?? undefined,
      )) as {
      initial_message_json: string
      shared_secret: string
    }
    x3dhInitialMessageJson.value = JSON.stringify(JSON.parse(init.initial_message_json), null, 2)
    x3dhSharedSecretText.value = init.shared_secret
    const pair = JSON.parse(create_ratchet_dh_keypair()) as { private_key: string; public_key: string }
    ratchetLocalDhKeyPairJson.value = JSON.stringify(pair, null, 2)
    ratchetRemoteDhPublicKeyForInit.value = offer.ratchet_dh_public_key
    ratchetInitRole.value = 'Initiator'
    const stateText = create_ratchet_session_from_shared_secret_with_keys(
      identity.value.user_id,
      offer.from_user_id,
      'Initiator',
      init.shared_secret,
      pair.private_key,
      offer.ratchet_dh_public_key,
    )
    ratchetStateText.value = stateText
    saveRatchetSession(offer.from_user_id, stateText)
    contact.last_secure_session_error = undefined
    contact.last_secure_session_success_at = Date.now()
    contact.secure_session_failure_count = 0
    const response: SecureSessionResponse = {
      type: 'lm-secure-session-response-v1',
      version: 1,
      from_user_id: identity.value.user_id,
      to_user_id: offer.from_user_id,
      initial_message_json: init.initial_message_json,
      ratchet_dh_public_key: pair.public_key,
      created_at: Date.now(),
    }
    secureSessionResponseText.value = JSON.stringify(response, null, 2)
    const responseText = secureSessionResponseText.value
    secureSessionStatusText.value = nodeEnabled.value
      ? '已建立本端 Ratchet Session，并自动回传 Response。'
      : '已建立本端 Ratchet Session，并生成 Response；开启消息同步后可自动回传。'
    incomingSecureSessionText.value = ''
    inspectRatchetStateText()
    persist()
    if (nodeEnabled.value) {
      void pushMailboxPayload(contact, 'other', responseText)
        .then(() => {
          appendLog(`✅ 已向 ${contact.display_name || contact.user_id} 自动回传安全会话 Response`)
          persist()
        })
        .catch((e) => {
          recordSecureSessionError(contact, e, '⚠️ 安全会话 Response 自动回传失败')
          queueOutboxItem(contact, responseText, undefined, 'other')
          appendLog(`安全会话 Response 已加入 outbox 自动重试：${contact.display_name || contact.user_id}`)
          persist()
        })
    }
  })
}

function applySecureSessionResponseText() {
  run('应用安全会话 Response', () => {
    if (!identity.value) throw new Error('请先登录')
    const response = JSON.parse(incomingSecureSessionText.value || secureSessionResponseText.value) as SecureSessionResponse
    if (response.type !== 'lm-secure-session-response-v1') throw new Error('不是 secure session response')
    if (response.to_user_id !== identity.value.user_id) throw new Error('Response 不是发给当前身份')
    const contact = contacts.value.find((c) => c.user_id === response.from_user_id)
    if (!contact || contact.state !== 'Friend') throw new Error('Response 发送者不是 Friend 联系人')
    if (!prekeyPrivateBundleJson.value.trim()) throw new Error('缺少创建 Offer 时保存的 Private PreKey Bundle')
    if (!ratchetLocalDhKeyPairJson.value.trim()) throw new Error('缺少创建 Offer 时保存的本端 Ratchet DH keypair')
    activePeerId.value = contact.user_id
    activeGroupId.value = ''

    const derived = JSON.parse(derive_x3dh_responder_secret(
      backupText.value,
      passphrase.value,
      prekeyPrivateBundleJson.value,
      response.initial_message_json,
    )) as { shared_secret: string }
    x3dhInitialMessageJson.value = JSON.stringify(JSON.parse(response.initial_message_json), null, 2)
    x3dhSharedSecretText.value = derived.shared_secret
    const localPair = JSON.parse(ratchetLocalDhKeyPairJson.value) as { private_key: string; public_key: string }
    ratchetRemoteDhPublicKeyForInit.value = response.ratchet_dh_public_key
    ratchetInitRole.value = 'Responder'
    const stateText = create_ratchet_session_from_shared_secret_with_keys(
      identity.value.user_id,
      response.from_user_id,
      'Responder',
      derived.shared_secret,
      localPair.private_key,
      response.ratchet_dh_public_key,
    )
    ratchetStateText.value = stateText
    saveRatchetSession(response.from_user_id, stateText)
    contact.last_secure_session_error = undefined
    contact.last_secure_session_success_at = Date.now()
    contact.secure_session_failure_count = 0
    secureSessionStatusText.value = '已应用 Response，双方现在应该都有 Ratchet Session。后续聊天会自动优先使用 Double Ratchet。'
    incomingSecureSessionText.value = ''
    inspectRatchetStateText()
    persist()
  })
}

function createMyPreKeyBundleText() {
  run('生成 PreKey Bundle', () => {
    if (!backupText.value || !passphrase.value) throw new Error('需要身份备份包和提示词')
    const out = JSON.parse(create_prekey_bundle(
      backupText.value,
      passphrase.value,
      Number(prekeySignedId.value || 1),
      Number(prekeyOneTimeCount.value || 0),
      BigInt(7 * 24 * 3600),
    )) as {
      prekey_bundle_text: string
      private_bundle_json: string
      signed_one_time_prekey_record_texts?: string[]
    }
    prekeyBundleText.value = out.prekey_bundle_text
    prekeyPrivateBundleJson.value = JSON.stringify(JSON.parse(out.private_bundle_json), null, 2)
    prekeySignedOneTimeRecordTexts.value = out.signed_one_time_prekey_record_texts ?? []
    selectedSignedOneTimePreKeyRecordText.value = ''
    selectedOneTimePreKeyId.value = null
    inspectPreKeyBundleText()
    persist()
  })
}

function inspectPreKeyBundleText() {
  run('解析 PreKey Bundle', () => {
    if (!prekeyBundleText.value.trim()) throw new Error('请先生成或粘贴 PreKey Bundle')
    const info = JSON.parse(inspect_prekey_bundle(prekeyBundleText.value))
    info.signed_one_time_prekey_records = prekeySignedOneTimeRecordTexts.value.length
    info.selected_signed_one_time_prekey_record = Boolean(selectedSignedOneTimePreKeyRecordText.value)
    prekeyInfoText.value = JSON.stringify(info, null, 2)
  })
}

function createX3dhInitialMessageText() {
  run('创建 X3DH 初始消息', () => {
    if (!backupText.value || !passphrase.value) throw new Error('需要身份备份包和提示词')
    if (!prekeyBundleText.value.trim()) throw new Error('请粘贴对方 PreKey Bundle')
    const signedRecord = selectedSignedOneTimePreKeyRecordText.value || prekeySignedOneTimeRecordTexts.value[0] || ''
    const out = JSON.parse(signedRecord
      ? create_x3dh_initial_message_with_one_time_prekey_record(
        backupText.value,
        passphrase.value,
        prekeyBundleText.value,
        signedRecord,
      )
      : create_x3dh_initial_message_with_one_time_prekey_id(
        backupText.value,
        passphrase.value,
        prekeyBundleText.value,
        selectedOneTimePreKeyId.value ?? undefined,
      )) as {
      initial_message_json: string
      shared_secret: string
    }
    x3dhInitialMessageJson.value = JSON.stringify(JSON.parse(out.initial_message_json), null, 2)
    x3dhSharedSecretText.value = out.shared_secret
  })
}

function deriveX3dhResponderSecretText() {
  run('响应方派生 X3DH 密钥', () => {
    if (!backupText.value || !passphrase.value) throw new Error('需要身份备份包和提示词')
    if (!prekeyPrivateBundleJson.value.trim()) throw new Error('需要本端 private prekey bundle')
    if (!x3dhInitialMessageJson.value.trim()) throw new Error('请粘贴 X3DH initial message JSON')
    const out = JSON.parse(derive_x3dh_responder_secret(
      backupText.value,
      passphrase.value,
      prekeyPrivateBundleJson.value,
      x3dhInitialMessageJson.value,
    )) as { shared_secret: string }
    x3dhSharedSecretText.value = out.shared_secret
  })
}



function generateRatchetDhKeyPairText() {
  run('生成 Ratchet DH keypair', () => {
    const pair = JSON.parse(create_ratchet_dh_keypair()) as { private_key: string; public_key: string }
    ratchetLocalDhKeyPairJson.value = JSON.stringify(pair, null, 2)
    appendLog('已生成本端 Ratchet DH public key，可把 public_key 发给对方')
  })
}

function createRatchetFromSharedSecretWithKeysText() {
  run('用 Shared Secret + 双方 DH 初始化 Ratchet', () => {
    if (!identity.value) throw new Error('请先登录')
    if (!activeContact.value) throw new Error('请先选择联系人')
    if (!x3dhSharedSecretText.value.trim()) throw new Error('请先得到 X3DH shared secret')
    if (!ratchetLocalDhKeyPairJson.value.trim()) throw new Error('请先生成本端 Ratchet DH keypair')
    if (!ratchetRemoteDhPublicKeyForInit.value.trim()) throw new Error('请粘贴对方 Ratchet DH public_key')
    const localPair = JSON.parse(ratchetLocalDhKeyPairJson.value) as { private_key: string; public_key: string }
    const stateText = create_ratchet_session_from_shared_secret_with_keys(
      identity.value.user_id,
      activeContact.value.user_id,
      ratchetInitRole.value,
      x3dhSharedSecretText.value.trim(),
      localPair.private_key,
      ratchetRemoteDhPublicKeyForInit.value.trim(),
    )
    ratchetStateText.value = stateText
    saveRatchetSession(activeContact.value.user_id, stateText)
    inspectRatchetStateText()
    persist()
  })
}

function createRatchetFromSharedSecretText() {
  run('用 Shared Secret 初始化 Ratchet', () => {
    if (!identity.value) throw new Error('请先登录')
    if (!activeContact.value) throw new Error('请先选择联系人')
    if (!x3dhSharedSecretText.value.trim()) throw new Error('请先得到 X3DH shared secret')
    const out = JSON.parse(create_ratchet_session_from_shared_secret(
      identity.value.user_id,
      activeContact.value.user_id,
      x3dhSharedSecretText.value.trim(),
    )) as { local_state_text: string; remote_state_text: string }
    ratchetStateText.value = out.local_state_text
    ratchetPeerStateText.value = out.remote_state_text
    saveRatchetSession(activeContact.value.user_id, out.local_state_text)
    inspectRatchetStateText()
    persist()
  })
}

function ratchetEncryptEnvelopeText() {
  run('Ratchet 加密消息', () => {
    if (!ratchetStateText.value.trim()) throw new Error('请先准备本端 Ratchet State')
    if (!composerText.value.trim()) throw new Error('请输入聊天内容')
    const conv = activeContact.value?.user_id || activeGroup.value?.group_id || 'debug'
    const out = JSON.parse(ratchet_encrypt_text_message(ratchetStateText.value, conv, composerText.value)) as {
      state_text: string
      envelope_json: string
    }
    ratchetStateText.value = out.state_text
    if (activeContact.value) saveRatchetSession(activeContact.value.user_id, out.state_text)
    ratchetEnvelopeText.value = JSON.stringify(JSON.parse(out.envelope_json), null, 2)
    inspectRatchetStateText()
  })
}

function ratchetDecryptEnvelopeText() {
  run('Ratchet 解密消息', () => {
    if (!ratchetStateText.value.trim()) throw new Error('请先准备本端 Ratchet State')
    if (!ratchetEnvelopeText.value.trim()) throw new Error('请粘贴 Ratchet Envelope JSON')
    const out = JSON.parse(ratchet_decrypt_text_message(ratchetStateText.value, ratchetEnvelopeText.value)) as {
      state_text: string
      plain_json: string
    }
    ratchetStateText.value = out.state_text
    if (activeContact.value) saveRatchetSession(activeContact.value.user_id, out.state_text)
    ratchetPlainText.value = JSON.stringify(JSON.parse(out.plain_json), null, 2)
    inspectRatchetStateText()
  })
}

function createRatchetPairForActiveContact() {
  run('创建双棘轮状态对', () => {
    if (!myContactCardText.value) throw new Error('请先生成我的 Contact Card')
    if (!activeContact.value) throw new Error('请先选择联系人')
    const out = JSON.parse(create_ratchet_session_pair(myContactCardText.value, activeContact.value.contact_card_text)) as {
      local_state_text: string
      remote_state_text: string
    }
    ratchetStateText.value = out.local_state_text
    ratchetPeerStateText.value = out.remote_state_text
    saveRatchetSession(activeContact.value.user_id, out.local_state_text)
    ratchetHeaderText.value = ''
    ratchetKeyText.value = ''
    inspectRatchetStateText()
    appendLog('已生成本端状态；对端状态仅用于本地测试，真实聊天中不会发给对方')
  })
}

function inspectRatchetStateText() {
  run('解析双棘轮状态', () => {
    if (!ratchetStateText.value.trim()) throw new Error('请先粘贴或生成 ratchet state')
    ratchetInfoText.value = JSON.stringify(JSON.parse(inspect_ratchet_state(ratchetStateText.value)), null, 2)
  })
}

function ratchetNextSendKeyText() {
  run('推进发送链', () => {
    if (!ratchetStateText.value.trim()) throw new Error('请先粘贴或生成 ratchet state')
    const out = JSON.parse(ratchet_next_sending_key(ratchetStateText.value)) as { state_text: string; key_json: string }
    ratchetStateText.value = out.state_text
    ratchetKeyText.value = JSON.stringify(JSON.parse(out.key_json), null, 2)
    ratchetHeaderText.value = JSON.stringify(JSON.parse(out.key_json).header, null, 2)
    inspectRatchetStateText()
  })
}

function ratchetNextRecvKeyText() {
  run('推进接收链', () => {
    if (!ratchetStateText.value.trim()) throw new Error('请先粘贴或生成 ratchet state')
    if (!ratchetHeaderText.value.trim()) throw new Error('请粘贴收到消息的 ratchet header')
    const out = JSON.parse(ratchet_next_receiving_key(ratchetStateText.value, ratchetHeaderText.value)) as { state_text: string; key_json: string }
    ratchetStateText.value = out.state_text
    ratchetKeyText.value = JSON.stringify(JSON.parse(out.key_json), null, 2)
    inspectRatchetStateText()
  })
}

function ratchetDhStepText() {
  run('执行 DH 棘轮步进', () => {
    if (!ratchetStateText.value.trim()) throw new Error('请先粘贴或生成 ratchet state')
    if (!ratchetRemoteDhPublicKey.value.trim()) throw new Error('请输入远端新的 DH public key')
    ratchetStateText.value = ratchet_dh_step(ratchetStateText.value, ratchetRemoteDhPublicKey.value.trim())
    inspectRatchetStateText()
  })
}

function createMailboxMessageText() {
  run('生成 MailboxMessage', () => {
    if (!activeContact.value) throw new Error('请先选择收件人联系人')
    const ciphertext = mailboxCiphertext.value.trim() || inboundEnvelopeText.value.trim() || localSignalText.value.trim()
    if (!ciphertext) throw new Error('请填写要放入 mailbox 的密文/信令文本')
    mailboxMessageText.value = create_mailbox_message(
      backupText.value,
      passphrase.value,
      activeContact.value.user_id,
      mailboxKind.value,
      ciphertext,
      BigInt(24 * 3600),
    )
    mailboxMessageInspectPublicKey.value = identity.value?.identity_public_key || ''
    mailboxMessageInfoText.value = JSON.stringify(JSON.parse(inspect_mailbox_message(
      mailboxMessageText.value,
      mailboxMessageInspectPublicKey.value,
    )), null, 2)
  })
}

function inspectMailboxMessageText() {
  run('验签 MailboxMessage', () => {
    const key = mailboxMessageInspectPublicKey.value.trim() || defaultInspectPublicKey()
    if (!key) throw new Error('需要发送者 identity_public_key')
    mailboxMessageInspectPublicKey.value = key
    mailboxMessageInfoText.value = JSON.stringify(JSON.parse(inspect_mailbox_message(mailboxMessageText.value, key)), null, 2)
  })
}

function nodeControlEndpoint(path: string, baseUrl = primaryNodeUrl()): string {
  if (!baseUrl) throw new Error('请先填写同步节点')
  return `${baseUrl.replace(/\/$/, '')}${path}`
}

async function fetchNodeOnce(baseUrl: string, path: string, init?: RequestInit): Promise<any> {
  const token = nodeTokenFor(baseUrl)
  const endpoint = nodeControlEndpoint(path, baseUrl)
  const controller = new AbortController()
  const timeout = window.setTimeout(() => controller.abort(new DOMException('同步服务请求超时', 'AbortError')), nodeFetchTimeoutMs())
  let res: Response
  try {
    res = await fetch(endpoint, {
      ...init,
      signal: init?.signal ?? controller.signal,
      headers: {
        'content-type': 'application/json',
        ...(token ? { authorization: `Bearer ${token}` } : {}),
        ...(init?.headers ?? {}),
      },
    })
  } catch (e) {
    throw new NodeRequestError(userFacingError(e), undefined, baseUrl)
  } finally {
    window.clearTimeout(timeout)
  }
  const text = await res.text()
  let body: any = text
  try { body = text ? JSON.parse(text) : {} } catch {}
  if (!res.ok) throw new NodeRequestError(typeof body === 'string' ? body : JSON.stringify(body), res.status, baseUrl)
  return body
}

async function nodeFetchJson(path: string, init?: RequestInit): Promise<any> {
  const entries = nodeEntries()
  if (entries.length === 0) throw new Error('请先填写同步节点')
  const errors: string[] = []
  for (const entry of entries) {
    try {
      const body = await fetchNodeOnce(entry.url, path, init)
      // 把可用节点移到最前，保留其令牌
      const current = nodeEntries()
      if (current[0]?.url !== entry.url) {
        nodeControlUrl.value = [entry, ...current.filter((e) => e.url !== entry.url)].map(nodeEntryLine).join('\n')
        persist()
      }
      return body
    } catch (e) {
      errors.push(`${entry.url}: ${userFacingError(e)}`)
    }
  }
  throw new Error(`所有同步服务都不可用：${errors.join('；')}`)
}

function normalizeNodePeerHealth(status: any): Array<{ url: string; consecutive_failures: number; failures: number; quarantined: boolean; last_error?: string }> {
  const peers = status?.peers && typeof status.peers === 'object' ? Object.values(status.peers) as any[] : []
  return peers.map((peer: any) => {
    const next = Number(peer?.next_attempt_at ?? 0)
    const consecutive = Number(peer?.consecutive_failures ?? 0)
    return {
      url: String(peer?.url ?? ''),
      consecutive_failures: Number.isFinite(consecutive) ? consecutive : 0,
      failures: Number(peer?.failures ?? 0) || 0,
      quarantined: consecutive >= 5 && (!Number.isFinite(next) || next * 1000 > Date.now()),
      last_error: peer?.last_error ? String(peer.last_error) : undefined,
    }
  }).filter((peer) => peer.url)
}

function nodePeerHealthSummaryFromPeers(peers: Array<{ consecutive_failures: number; quarantined: boolean }>): { text: string; level: 'ok' | 'warning' | 'danger' } {
  if (peers.length === 0) return { text: 'DHT peer：暂无同步 peer 健康记录', level: 'ok' }
  const failed = peers.filter((peer) => peer.consecutive_failures > 0)
  const quarantined = peers.filter((peer) => peer.quarantined)
  const worst = peers.reduce((max, peer) => Math.max(max, peer.consecutive_failures), 0)
  const level: 'ok' | 'warning' | 'danger' = quarantined.length > 0 ? 'danger' : failed.length > 0 ? 'warning' : 'ok'
  const suffix = quarantined.length > 0
    ? '，存在隔离 peer'
    : failed.length > 0
      ? '，存在失败 peer'
      : '，全部健康'
  return { text: `DHT peer：${peers.length} 个，失败 ${failed.length}，隔离 ${quarantined.length}，最高连续失败 ${worst}${suffix}`, level }
}

function nodeStateDbSecurityFromHealth(health: any): { text: string; level: 'ok' | 'warning' | 'danger' } {
  const stateDb = health?.state_db ?? health
  const encrypted = Boolean(stateDb?.encrypted ?? health?.state_db_encrypted)
  const hardened = Boolean(stateDb?.permissions_hardened ?? health?.state_db_permissions_hardened)
  const mode = String(stateDb?.encryption_mode || (encrypted ? 'encrypted' : 'plain'))
  if (mode === 'external') return { text: `state_db：由外部存储加密保护（external）${hardened ? '，权限已硬化' : ''}；仍非数据库级加密`, level: 'warning' }
  if (encrypted) return { text: `state_db：数据库加密已启用（${mode}）${hardened ? '，权限已硬化' : ''}`, level: 'ok' }
  if (hardened) return { text: `state_db：未加密（${mode}），仅权限硬化；生产环境建议启用数据库级加密`, level: 'warning' }
  return { text: `state_db：未加密（${mode}）且权限未硬化；不建议生产使用`, level: 'danger' }
}

function nodeStateFileSecurityFromStats(stats: any): { text: string; level: 'ok' | 'warning' | 'danger' } {
  const stateFile = stats?.state_file
  if (!stateFile) return { text: 'state_file：未配置', level: 'ok' }
  const encrypted = Boolean(stateFile.encrypted)
  const hardened = Boolean(stateFile.permissions_hardened)
  const size = Number(stateFile.file_bytes ?? 0)
  if (encrypted && hardened) return { text: `state_file：已加密，权限已硬化，${formatBytes(size)}`, level: 'ok' }
  if (encrypted) return { text: `state_file：已加密但权限未硬化，${formatBytes(size)}`, level: 'warning' }
  if (hardened) return { text: `state_file：未加密，仅权限硬化，${formatBytes(size)}`, level: 'warning' }
  return { text: `state_file：未加密且权限未硬化，${formatBytes(size)}`, level: 'danger' }
}

function nodeHealthSummaryFromResponse(health: any): string {
  const parts: string[] = []
  const peers = Number(health?.peers ?? 0)
  const prekeys = Number(health?.prekeys ?? 0)
  const mailboxDeliveries = Number(health?.mailbox_deliveries ?? 0)
  const mailboxBytes = Number(health?.mailbox_bytes ?? 0)
  const mailboxMaxBytesRaw = health?.mailbox_max_bytes
  const mailboxMaxBytes = mailboxMaxBytesRaw === null || mailboxMaxBytesRaw === undefined ? null : Number(mailboxMaxBytesRaw)
  if (Number.isFinite(peers)) parts.push(`peers ${peers}`)
  if (Number.isFinite(prekeys)) parts.push(`PreKey ${prekeys}`)
  if (Number.isFinite(mailboxDeliveries)) parts.push(`Mailbox ${mailboxDeliveries} 条`)
  if (Number.isFinite(mailboxBytes)) {
    if (mailboxMaxBytes && Number.isFinite(mailboxMaxBytes) && mailboxMaxBytes > 0) {
      parts.push(`Mailbox ${formatBytes(mailboxBytes)} / ${formatBytes(mailboxMaxBytes)}`)
    } else {
      parts.push(`Mailbox ${formatBytes(mailboxBytes)}`)
    }
  }
  const perUserBytes = health?.mailbox_max_bytes_per_user
  const perUserMessages = health?.mailbox_max_messages_per_user
  if (perUserBytes !== undefined && perUserBytes !== null) parts.push(`每用户 ${formatBytes(Number(perUserBytes))}`)
  if (perUserMessages !== undefined && perUserMessages !== null) parts.push(`每用户 ${perUserMessages} 条`)
  return parts.length ? `节点健康：${parts.join(' · ')}` : '节点健康：已连接'
}

async function checkNodeHealth() {
  await runAsync('检查 lm_node 控制面', async () => {
    const health = await nodeFetchJson('/health')
    nodeHealthSummaryText.value = nodeHealthSummaryFromResponse(health)
    const stateDb = nodeStateDbSecurityFromHealth(health)
    nodeStateDbSecurityText.value = stateDb.text
    nodeStateDbSecurityLevel.value = stateDb.level
    updateMailboxQuotaStatus({
      pending_bytes: Number(health?.mailbox_bytes ?? 0),
      max_bytes_per_user: health?.mailbox_max_bytes_per_user,
    })
    // /health 免鉴权，会掩盖令牌问题。再探测一个需要鉴权的接口，避免误报"已连接"。
    try {
      const syncStatus = await nodeFetchJson('/sync/status')
      nodePeerHealthPeers.value = normalizeNodePeerHealth(syncStatus)
      const peerHealth = nodePeerHealthSummaryFromPeers(nodePeerHealthPeers.value)
      nodePeerHealthStatusText.value = peerHealth.text
      nodePeerHealthRiskLevel.value = peerHealth.level
      nodeControlStatus.value = `已连接（鉴权通过）\n${JSON.stringify(health, null, 2)}`
    } catch (e) {
      const msg = String(e)
      if (/401|unauthorized/i.test(msg)) {
        nodeHealthSummaryText.value = `${nodeHealthSummaryText.value} · 鉴权失败`
        nodePeerHealthStatusText.value = 'DHT peer：鉴权失败，无法读取健康记录'
        nodePeerHealthRiskLevel.value = 'danger'
        nodeControlStatus.value = `节点在线，但鉴权失败（401）。\n若节点启用了 --control-token，请在地址后追加 " | 令牌"（与节点令牌一致）。\n\n${msg}`
      } else {
        nodeHealthSummaryText.value = `${nodeHealthSummaryText.value} · 控制接口异常`
        nodePeerHealthStatusText.value = 'DHT peer：控制接口异常，无法读取健康记录'
        nodePeerHealthRiskLevel.value = 'warning'
        nodeControlStatus.value = `节点在线，但控制接口异常。\n\n${msg}`
      }
    }
    try {
      const stats = await nodeFetchJson('/control/stats')
      if (stats?.state_db) {
        const stateDbFromStats = nodeStateDbSecurityFromHealth({ state_db: stats.state_db })
        nodeStateDbSecurityText.value = stateDbFromStats.text
        nodeStateDbSecurityLevel.value = stateDbFromStats.level
      }
      const stateFile = nodeStateFileSecurityFromStats(stats)
      nodeStateFileSecurityText.value = stateFile.text
      nodeStateFileSecurityLevel.value = stateFile.level
    } catch (error) {
      nodeStateFileSecurityText.value = `state_file：状态查询失败：${userFacingError(error)}`
      nodeStateFileSecurityLevel.value = 'warning'
    }
  })
}

async function resetDhtPeerHealth(url: string) {
  await runAsync('重置 DHT peer 健康', async () => {
    const body = await nodeFetchJson('/sync/peer/reset', {
      method: 'POST',
      body: JSON.stringify({ url }),
    })
    nodeControlStatus.value = JSON.stringify(body, null, 2)
    await checkNodeHealth()
  })
}

async function submitPublicPeerToNode() {
  await runAsync('提交 PublicPeerAnnounce 到 lm_node', async () => {
    if (!publicPeerAnnounceText.value.trim()) throw new Error('请先生成或粘贴 PublicPeerAnnounce')
    const key = publicPeerAnnounceInspectPublicKey.value.trim() || identity.value?.identity_public_key
    if (!key) throw new Error('需要发布者 identity_public_key')
    const body = await nodeFetchJson('/announce', {
      method: 'POST',
      body: JSON.stringify({
        announce_text: publicPeerAnnounceText.value,
        identity_public_key: key,
      }),
    })
    nodeControlStatus.value = JSON.stringify(body, null, 2)
  })
}

function recordDhtOperation(summary: string) {
  const line = `${formatDateTime(Date.now())} · ${summary}`
  nodeDhtOperationHistory.value = [line, ...nodeDhtOperationHistory.value.filter((item) => item !== line)].slice(0, DHT_OPERATION_HISTORY_MAX_RECORDS)
  persist()
}

async function clearDhtOperationHistory() {
  if (nodeDhtOperationHistory.value.length === 0) return
  const ok = await showConfirm(
    '清空 DHT 操作历史',
    '清空最近 DHT 查找、复制和路由刷新历史？这只删除本机排障记录，不影响 DHT 网络或聊天数据。',
    true,
  )
  if (!ok) return
  nodeDhtOperationHistory.value = []
  nodeDhtFindValueStatusText.value = 'DHT 查找：已清空操作历史'
  appendLog('已清空 DHT 操作历史')
  persist()
}

function exportDhtOperationHistory() {
  downloadText(dhtOperationHistoryJson(), `lm-talk-dht-history-${Date.now()}.json`)
}

function dhtOperationHistoryJson(): string {
  return JSON.stringify({
    exported_at: Date.now(),
    history: nodeDhtOperationHistory.value,
  }, null, 2)
}

async function copyDhtOperationHistory() {
  if (nodeDhtOperationHistory.value.length === 0) throw new Error('DHT 操作历史为空')
  await copyText(dhtOperationHistoryJson(), 'DHT 操作历史')
}

function normalizeDhtHistoryItems(value: unknown): string[] {
  const raw = Array.isArray(value)
    ? value
    : Array.isArray((value as any)?.history)
      ? (value as any).history
      : Array.isArray((value as any)?.dht?.operation_history)
        ? (value as any).dht.operation_history
        : []
  if (!Array.isArray(raw)) return []
  return raw
    .map((item) => String(item ?? '').trim())
    .filter((item) => item.length > 0)
    .filter((item) => !item.includes('[已脱敏]') && !item.includes('[已截断]'))
    .map((item) => item.length > DHT_OPERATION_HISTORY_ITEM_MAX_CHARS ? `${item.slice(0, DHT_OPERATION_HISTORY_ITEM_MAX_CHARS)}…` : item)
    .slice(0, DHT_OPERATION_HISTORY_IMPORT_MAX_RECORDS)
}

async function importDhtOperationHistory() {
  await runAsync('导入 DHT 操作历史', async () => {
    try {
      if (!nodeDhtOperationHistoryImportText.value.trim()) throw new Error('请粘贴 DHT 历史 JSON')
      const parsed = JSON.parse(nodeDhtOperationHistoryImportText.value)
      const incoming = normalizeDhtHistoryItems(parsed)
      if (incoming.length === 0) throw new Error('DHT 历史为空或格式不正确；请粘贴 {"history":["时间 · 操作"]} 或字符串数组')
      const merged = [...new Set([...incoming, ...nodeDhtOperationHistory.value])]
      const kept = merged.slice(0, DHT_OPERATION_HISTORY_MAX_RECORDS)
      const dropped = Math.max(0, merged.length - kept.length)
      nodeDhtOperationHistoryImportStatus.value = `DHT 历史导入：准备导入 ${incoming.length} 条，合并后保留 ${kept.length} 条${dropped ? `，将丢弃 ${dropped} 条较旧记录` : ''}`
      const ok = await showConfirm(
        '导入 DHT 操作历史',
        `将导入 ${incoming.length} 条 DHT 操作历史，单条最多保留 ${DHT_OPERATION_HISTORY_ITEM_MAX_CHARS} 字符，合并后最多保留 ${DHT_OPERATION_HISTORY_MAX_RECORDS} 条（本次保留 ${kept.length} 条）${dropped ? `，丢弃 ${dropped} 条较旧记录` : ''}。继续？`,
        false,
      )
      if (!ok) {
        nodeDhtOperationHistoryImportStatus.value = 'DHT 历史导入：已取消'
        return
      }
      nodeDhtOperationHistory.value = kept
      nodeDhtFindValueStatusText.value = `DHT 查找：已导入 ${incoming.length} 条 DHT 操作历史`
      nodeDhtOperationHistoryImportStatus.value = `DHT 历史导入：已导入 ${incoming.length} 条，当前保留 ${kept.length} 条`
      nodeDhtOperationHistoryImportText.value = ''
      persist()
    } catch (error) {
      nodeDhtOperationHistoryImportStatus.value = `DHT 历史导入失败：${userFacingError(error)}`
      throw error
    }
  })
}

function dhtKeyKindLabel(kind: string): string {
  return kind === 'public-peer' ? 'PublicPeer' : kind === 'mailbox-hint' ? 'MailboxHint' : kind === 'contact-card' ? 'ContactCard' : 'PreKey'
}

function fillDhtKeyInput(kind: 'prekey' | 'mailbox-hint' | 'public-peer' | 'contact-card', value: string) {
  nodeDhtKeyKind.value = kind
  nodeDhtKeyValue.value = value
  nodeDhtFindValueStatusText.value = `DHT key：已填入 ${dhtKeyKindLabel(kind)}，点击“派生 key”生成查询 key`
  recordDhtOperation(nodeDhtFindValueStatusText.value)
}

function fillMyPreKeyDhtKeyInput() {
  if (!identity.value?.user_id) throw new Error('需要先登录身份')
  fillDhtKeyInput('prekey', identity.value.user_id)
}

function fillMyMailboxHintDhtKeyInput() {
  if (!identity.value?.user_id) throw new Error('需要先登录身份')
  fillDhtKeyInput('mailbox-hint', identity.value.user_id)
}

function fillMyContactCardDhtKeyInput() {
  if (!identity.value?.user_id) throw new Error('需要先登录身份')
  fillDhtKeyInput('contact-card', identity.value.user_id)
}

function fingerprintProofFromInfo(info: ContactInfo): string {
  return JSON.stringify({
    type: 'lm-contact-fingerprint-v1',
    user_id: info.user_id,
    fingerprint: info.fingerprint,
    identity_public_key: info.identity_public_key,
  })
}

function contactFingerprintProof(contact: ContactItem): string {
  return fingerprintProofFromInfo(contact)
}

function myFingerprintProof(): string {
  if (!myContactCardText.value.trim()) refreshMyContactCard()
  if (!myContactCardText.value.trim()) throw new Error('请先生成我的名片')
  return fingerprintProofFromInfo(safeJson<ContactInfo>(inspect_contact_card(myContactCardText.value)))
}

async function showMyFingerprintQr() {
  await showQr(myFingerprintProof(), '我的指纹核验码')
}

async function copyMyFingerprintProof() {
  await copyText(myFingerprintProof(), '我的指纹核验码')
}

async function showActiveContactFingerprintQr() {
  if (!activeContact.value) throw new Error('请选择联系人')
  await showQr(contactFingerprintProof(activeContact.value), '联系人指纹核验码')
}

async function copyActiveContactFingerprintProof() {
  if (!activeContact.value) throw new Error('请选择联系人')
  await copyText(contactFingerprintProof(activeContact.value), '联系人指纹核验码')
}

function barcodeDetectorCtor(): any {
  return (globalThis as any).BarcodeDetector
}

async function stopFingerprintQrScan() {
  fingerprintScanStopped = true
  fingerprintScanOpen.value = false
  fingerprintScanStatus.value = ''
  if (fingerprintScanStream) {
    fingerprintScanStream.getTracks().forEach((track) => track.stop())
    fingerprintScanStream = null
  }
}

async function startFingerprintQrScan() {
  if (!activeContact.value) throw new Error('请选择联系人')
  const Detector = barcodeDetectorCtor()
  if (!Detector) {
    showAlert('当前浏览器不支持扫码', '请改用“复制/粘贴指纹核验码”。', 'warning')
    return
  }
  try {
    fingerprintScanOpen.value = true
    fingerprintScanStopped = false
    fingerprintScanStatus.value = '正在打开摄像头…'
    fingerprintScanStream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' }, audio: false })
    await nextTick()
    if (!fingerprintScanVideo.value) throw new Error('扫码视频组件未就绪')
    fingerprintScanVideo.value.srcObject = fingerprintScanStream
    await fingerprintScanVideo.value.play()
    const detector = new Detector({ formats: ['qr_code'] })
    fingerprintScanStatus.value = '请将指纹核验二维码对准摄像头'
    const scanOnce = async () => {
      if (fingerprintScanStopped || !fingerprintScanVideo.value) return
      try {
        const codes = await detector.detect(fingerprintScanVideo.value)
        const raw = String(codes?.[0]?.rawValue || '')
        if (raw) {
          activeFingerprintVerificationText.value = raw
          fingerprintScanStatus.value = '已识别二维码，正在核验…'
          await stopFingerprintQrScan()
          await verifyActiveContactFingerprintFromText()
          return
        }
      } catch (error) {
        fingerprintScanStatus.value = `扫码失败：${userFacingError(error)}`
      }
      if (!fingerprintScanStopped) window.setTimeout(scanOnce, 350)
    }
    void scanOnce()
  } catch (error) {
    await stopFingerprintQrScan()
    showAlert('扫码启动失败', userFacingError(error), 'warning')
  }
}

function parseFingerprintVerificationText(text: string): { user_id?: string; fingerprint?: string; identity_public_key?: string } {
  const trimmed = text.trim()
  if (!trimmed) throw new Error('请粘贴指纹核验码或指纹文本')
  try {
    const value = JSON.parse(trimmed)
    return {
      user_id: typeof value?.user_id === 'string' ? value.user_id : undefined,
      fingerprint: typeof value?.fingerprint === 'string' ? value.fingerprint : undefined,
      identity_public_key: typeof value?.identity_public_key === 'string' ? value.identity_public_key : undefined,
    }
  } catch {
    return { fingerprint: trimmed }
  }
}

async function verifyActiveContactFingerprintFromText() {
  await runAsync('核验联系人指纹码', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const contact = activeContact.value
    const proof = parseFingerprintVerificationText(activeFingerprintVerificationText.value)
    if (proof.user_id && proof.user_id !== contact.user_id) throw new Error('指纹码 UserID 与当前联系人不一致')
    if (proof.identity_public_key && proof.identity_public_key !== contact.identity_public_key) throw new Error('指纹码 identity_public_key 与当前联系人不一致')
    const normalizedInput = (proof.fingerprint || '').replace(/\s+/g, '').toUpperCase()
    const normalizedContact = contact.fingerprint.replace(/\s+/g, '').toUpperCase()
    if (!normalizedInput || normalizedInput !== normalizedContact) throw new Error('指纹不匹配')
    contact.fingerprint_verified_at = Date.now()
    contact.fingerprint_verified_note = 'fingerprint-code'
    resetContactDhtDiscoveryBackoff(contact)
    contact.dht_discovery_risk_level = undefined
    contact.last_dht_discovery_error_kind = undefined
    contact.last_dht_discovery_error = undefined
    activeFingerprintVerificationText.value = ''
    appendLog(`✅ 已通过指纹核验码确认 ${contact.display_name || contact.user_id}`)
    persist()
  })
}

async function verifyActiveContactFingerprint() {
  await runAsync('核验联系人指纹', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const contact = activeContact.value
    const ok = await showConfirm(
      '确认联系人指纹已核验',
      `请确认你已通过线下、语音、视频或可信二维码核对该联系人指纹：

${contact.fingerprint}

确认后会标记为已核验，并清除 DHT 高风险状态。继续？`,
      true,
    )
    if (!ok) return
    contact.fingerprint_verified_at = Date.now()
    contact.fingerprint_verified_note = 'fingerprint'
    resetContactDhtDiscoveryBackoff(contact)
    contact.dht_discovery_risk_level = undefined
    contact.last_dht_discovery_error_kind = undefined
    contact.last_dht_discovery_error = undefined
    appendLog(`✅ 已标记 ${contact.display_name || contact.user_id} 指纹核验通过`)
    persist()
  })
}

async function clearActiveContactDhtRisk() {
  await runAsync('清除联系人 DHT 风险', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const contact = activeContact.value
    const ok = await showConfirm(
      '清除 DHT 风险状态',
      `仅在你已通过指纹、二维码或其他可信渠道核验「${contact.display_name || contact.user_id}」身份后清除。继续？`,
      true,
    )
    if (!ok) return
    resetContactDhtDiscoveryBackoff(contact)
    contact.dht_discovery_risk_level = undefined
    contact.last_dht_discovery_error_kind = undefined
    contact.last_dht_discovery_error = undefined
    appendLog(`已清除 ${contact.display_name || contact.user_id} 的 DHT 风险状态`)
    persist()
  })
}

async function findActiveContactMailboxHint() {
  await runAsync('查找联系人 MailboxHint', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const contact = activeContact.value
    if (!(await confirmHighRiskDhtContactIfNeeded(contact))) {
      appendLog('已取消高风险联系人 DHT 查找')
      return
    }
    resetContactDhtDiscoveryBackoff(contact)
    markContactDhtDiscoveryAttempt(contact)
    try {
      fillDhtKeyInput('mailbox-hint', contact.user_id)
      await deriveAndFindDhtValueNow()
      persist()
    } catch (error) {
      markContactDhtDiscoveryError(contact, error)
      persist()
      throw error
    }
  })
}

async function findActiveContactContactCard() {
  await runAsync('查找联系人 ContactCard', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const contact = activeContact.value
    if (!(await confirmHighRiskDhtContactIfNeeded(contact))) {
      appendLog('已取消高风险联系人 ContactCard DHT 查找')
      return
    }
    resetContactDhtDiscoveryBackoff(contact)
    markContactDhtDiscoveryAttempt(contact)
    try {
      await refreshContactCardDhtForContact(contact)
      persist()
    } catch (error) {
      markContactDhtDiscoveryError(contact, error)
      persist()
      throw error
    }
  })
}

async function findActiveContactPreKey() {
  await runAsync('查找联系人 PreKey', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const contact = activeContact.value
    if (!(await confirmHighRiskDhtContactIfNeeded(contact))) {
      appendLog('已取消高风险联系人 DHT 查找')
      return
    }
    resetContactDhtDiscoveryBackoff(contact)
    markContactDhtDiscoveryAttempt(contact)
    try {
      fillDhtKeyInput('prekey', contact.user_id)
      await deriveAndFindDhtValueNow()
      persist()
    } catch (error) {
      markContactDhtDiscoveryError(contact, error)
      persist()
      throw error
    }
  })
}

async function discoverActiveContactDht() {
  await runAsync('发现联系人 DHT 记录', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    const contact = activeContact.value
    if (!(await confirmHighRiskDhtContactIfNeeded(contact))) {
      appendLog('已取消高风险联系人 DHT 发现')
      return
    }
    contact.last_secure_session_attempt_at = Date.now()
    resetContactDhtDiscoveryBackoff(contact)
    markContactDhtDiscoveryAttempt(contact)
    const errors: string[] = []
    try {
      fillDhtKeyInput('prekey', contact.user_id)
      await deriveAndFindDhtValueNow()
    } catch (error) {
      errors.push(`PreKey：${userFacingError(error)}`)
    }
    try {
      fillDhtKeyInput('mailbox-hint', contact.user_id)
      await deriveAndFindDhtValueNow()
    } catch (error) {
      errors.push(`MailboxHint：${userFacingError(error)}`)
    }
    try {
      fillDhtKeyInput('contact-card', contact.user_id)
      await deriveAndFindDhtValueNow()
    } catch (error) {
      errors.push(`ContactCard：${userFacingError(error)}`)
    }
    if (errors.length) {
      markContactDhtDiscoveryError(contact, new Error(errors.join('；')))
      persist()
      throw new Error(errors.join('；'))
    }
    contact.last_secure_session_error = undefined
    contact.secure_session_failure_count = 0
    contact.last_dht_discovery_success_at = Date.now()
    contact.last_dht_discovery_error = undefined
    appendLog(`✅ 已完成 ${contact.display_name || contact.user_id} 的 PreKey / MailboxHint / ContactCard DHT 发现`)
    persist()
  })
}

function fillCurrentPublicPeerDhtKeyInput() {
  fillDhtKeyInput('public-peer', publicPeerId.value.trim() || `public-${identity.value?.user_id.slice(0, 12) || 'peer'}`)
}

async function deriveDhtKeyPayload(): Promise<any> {
  const value = nodeDhtKeyValue.value.trim()
  if (!value) throw new Error('请输入 peer_id 或 UserID')
  const body = await nodeFetchJson(`/dht/key?kind=${encodeURIComponent(nodeDhtKeyKind.value)}&value=${encodeURIComponent(value)}`)
  nodeDhtFindValueKey.value = String(body.key || '')
  nodeDhtFindValueStatusText.value = `DHT key：${dhtKeyKindLabel(nodeDhtKeyKind.value)} ${value} → ${nodeDhtFindValueKey.value}`
  recordDhtOperation(nodeDhtFindValueStatusText.value)
  nodeClosestInfoText.value = JSON.stringify(body, null, 2)
  return body
}

async function deriveDhtKeyForFindValue() {
  await runAsync('派生 DHT key', async () => { await deriveDhtKeyPayload() })
}

function currentDhtTargetContact(kind: 'prekey' | 'mailbox-hint' | 'contact-card'): ContactItem | undefined {
  if (nodeDhtKeyKind.value !== kind) return undefined
  const userId = nodeDhtKeyValue.value.trim()
  return userId ? contacts.value.find((item) => item.user_id === userId) : undefined
}

function validateDhtRecordEnvelope(body: any, record: any): string | null {
  if (record?.key && body?.key && String(record.key).toLowerCase() !== String(body.key).toLowerCase()) return 'record key 与查询 key 不一致'
  const expiresAt = Number(record?.expires_at ?? 0)
  if (Number.isFinite(expiresAt) && expiresAt > 0 && expiresAt <= Math.floor(Date.now() / 1000)) return 'record 已过期'
  return null
}

function applyDhtFindValueRecord(body: any): boolean {
  const record = body?.record ?? body?.value ?? body?.found_record
  if (!record?.kind || typeof record.value !== 'string') return true
  const envelopeError = validateDhtRecordEnvelope(body, record)
  if (envelopeError) {
    nodeDhtFindValueStatusText.value = `DHT 查到 ${record.kind} record，但${envelopeError}`
    const contact = record.kind === 'PreKey' ? currentDhtTargetContact('prekey') : record.kind === 'MailboxHint' ? currentDhtTargetContact('mailbox-hint') : record.kind === 'ContactCard' ? currentDhtTargetContact('contact-card') : undefined
    if (contact) markContactDhtDiscoveryError(contact, new Error(envelopeError), envelopeError.includes('过期') ? 'expired' : 'invalid-record')
    recordDhtOperation(nodeDhtFindValueStatusText.value)
    return false
  }
  if (record.kind === 'PreKey') {
    try {
      const inspected = JSON.parse(inspect_prekey_bundle(record.value))
      prekeyBundleText.value = record.value
      const contact = currentDhtTargetContact('prekey')
      if (contact) markContactDhtDiscoverySuccess(contact, 'prekey')
      nodePreKeyStatusText.value = JSON.stringify({ found: true, source: 'dht', verified: true, record, inspected }, null, 2)
      prekeyStatusSummary.value = 'DHT 查到 PreKey record，已验签并填入本地 PreKey 文本'
    } catch (error) {
      nodePreKeyStatusText.value = JSON.stringify({ found: true, source: 'dht', verified: false, error: userFacingError(error), record }, null, 2)
      prekeyStatusSummary.value = `DHT 查到 PreKey record，但验签失败：${userFacingError(error)}`
      const contact = currentDhtTargetContact('prekey')
      if (contact) markContactDhtDiscoveryError(contact, error, 'signature')
      return false
    }
  } else if (record.kind === 'MailboxHint') {
    const hint = record.value.trim()
    if (/^(https?:\/\/|libp2p:\/\/|mailbox:\/\/)/i.test(hint)) {
      peerMailboxKey.value = hint
      if (/^https?:\/\//i.test(hint)) discoveredMailboxHintUrl.value = hint
      const contact = currentDhtTargetContact('mailbox-hint')
      if (contact) {
        contact.mailbox_hint_url = hint
        markContactDhtDiscoverySuccess(contact, 'mailbox-hint')
      }
      mailboxInboxStatus.value = contact
        ? `DHT 查到 MailboxHint：${hint}，已关联 ${contact.display_name || contact.user_id}`
        : `DHT 查到 MailboxHint：${hint}`
    } else {
      mailboxInboxStatus.value = `DHT 查到 MailboxHint，但地址格式异常：${hint.slice(0, 80)}`
      const contact = currentDhtTargetContact('mailbox-hint')
      if (contact) markContactDhtDiscoveryError(contact, new Error(mailboxInboxStatus.value), 'invalid-record')
      nodeDhtFindValueStatusText.value = mailboxInboxStatus.value
      recordDhtOperation(nodeDhtFindValueStatusText.value)
      return false
    }
  } else if (record.kind === 'ContactCard') {
    try {
      const info = safeJson<ContactInfo>(inspect_contact_card(record.value))
      const index = contacts.value.findIndex((c) => c.user_id === info.user_id)
      const existing = index >= 0 ? contacts.value[index] : undefined
      const merged = mergeContactCard(existing, info, record.value)
      if (index >= 0) contacts.value[index] = merged
      else contacts.value.push(merged)
      const contact = currentDhtTargetContact('contact-card') ?? merged
      if (contact) markContactDhtDiscoverySuccess(contact, 'contact-card')
      appendLog(`✅ DHT 查到并合并 ContactCard：${merged.display_name || merged.user_id}`)
      persist()
    } catch (error) {
      nodeDhtFindValueStatusText.value = `DHT 查到 ContactCard record，但验签失败：${userFacingError(error)}`
      const contact = currentDhtTargetContact('contact-card')
      if (contact) markContactDhtDiscoveryError(contact, error, 'signature')
      recordDhtOperation(nodeDhtFindValueStatusText.value)
      return false
    }
  } else if (record.kind === 'PublicPeer') {
    const key = publicPeerAnnounceInspectPublicKey.value.trim() || defaultInspectPublicKey()
    try {
      const inspected = key ? JSON.parse(inspect_public_peer_announce(record.value, key)) : null
      publicPeerAnnounceText.value = record.value
      publicPeerAnnounceInspectPublicKey.value = key
      publicPeerAnnounceInfoText.value = JSON.stringify({ source: 'dht', verified: Boolean(inspected), record, inspected }, null, 2)
    } catch (error) {
      publicPeerAnnounceInfoText.value = JSON.stringify({ source: 'dht', verified: false, error: userFacingError(error), record }, null, 2)
      nodeDhtFindValueStatusText.value = `DHT 查到 PublicPeer record，但验签失败：${userFacingError(error)}`
      recordDhtOperation(nodeDhtFindValueStatusText.value)
      return false
    }
  }
  return true
}

function dhtFindValueSummary(body: any): string {
  const stats = body?.stats ?? {}
  const record = body?.record ?? body?.value ?? body?.found_record
  const kind = record?.kind ? `，kind ${record.kind}` : ''
  const key = body?.key ? `，key ${String(body.key).slice(0, 12)}…` : ''
  return `DHT 查找：${body?.found ? '找到' : '未找到'}${kind}${key}，peer 尝试 ${Number(stats.attempts ?? 0)}，成功 ${Number(stats.successes ?? 0)}，失败 ${Number(stats.failures ?? 0)}，found ${Number(stats.found_records ?? 0)}，closer ${Number(stats.closer_records ?? 0)}，隔离 ${Number(stats.peers_quarantined ?? 0)}`
}

async function runDhtFindValueForKey(key: string) {
  if (!/^[0-9a-fA-F]{64}$/.test(key)) throw new Error('请输入 64 位十六进制 DHT key')
  const body = await nodeFetchJson(`/dht/find-value?key=${encodeURIComponent(key)}&limit=8&max_peers=8&alpha=3`)
  if (applyDhtFindValueRecord(body)) {
    if (!body?.found) {
      const contact = nodeDhtKeyKind.value === 'prekey'
        ? currentDhtTargetContact('prekey')
        : nodeDhtKeyKind.value === 'mailbox-hint'
          ? currentDhtTargetContact('mailbox-hint')
          : nodeDhtKeyKind.value === 'contact-card'
            ? currentDhtTargetContact('contact-card')
            : undefined
      if (contact) markContactDhtDiscoveryError(contact, new Error('DHT 未找到记录'), 'not-found')
    }
    nodeDhtFindValueStatusText.value = dhtFindValueSummary(body)
    recordDhtOperation(nodeDhtFindValueStatusText.value)
  }
  nodeClosestInfoText.value = JSON.stringify(body, null, 2)
  await checkNodeHealth()
}

async function runDhtFindValueNow() {
  await runAsync('手动查找 DHT 记录', async () => {
    await runDhtFindValueForKey(nodeDhtFindValueKey.value.trim())
  })
}

async function deriveAndFindDhtValueNow() {
  await runAsync('派生并查找 DHT 记录', async () => {
    const value = nodeDhtKeyValue.value.trim()
    if (!value) throw new Error('请输入 peer_id 或 UserID')
    const body = await nodeFetchJson(`/dht/find-value?kind=${encodeURIComponent(nodeDhtKeyKind.value)}&value=${encodeURIComponent(value)}&limit=8&max_peers=8&alpha=3`)
    nodeDhtFindValueKey.value = String(body.key || '')
    applyDhtFindValueRecord(body)
    if (!body?.found) {
      const contact = nodeDhtKeyKind.value === 'prekey'
        ? currentDhtTargetContact('prekey')
        : nodeDhtKeyKind.value === 'mailbox-hint'
          ? currentDhtTargetContact('mailbox-hint')
          : nodeDhtKeyKind.value === 'contact-card'
            ? currentDhtTargetContact('contact-card')
            : undefined
      if (contact) markContactDhtDiscoveryError(contact, new Error('DHT 未找到记录'), 'not-found')
    }
    nodeDhtFindValueStatusText.value = dhtFindValueSummary(body)
    recordDhtOperation(`DHT key：${dhtKeyKindLabel(nodeDhtKeyKind.value)} ${value} → ${nodeDhtFindValueKey.value}`)
    recordDhtOperation(nodeDhtFindValueStatusText.value)
    nodeClosestInfoText.value = JSON.stringify(body, null, 2)
    await checkNodeHealth()
  })
}

function dhtMaintenanceSummary(body: any): string {
  const replication = body?.replication ?? {}
  const refresh = body?.routing_refresh ?? {}
  return `DHT 维护：peer ${Number(body?.peers ?? 0)}，records ${Number(body?.records ?? 0)}，复制成功 ${Number(replication.successes ?? 0)}/${Number(replication.attempts ?? 0)}，刷新成功 ${Number(refresh.successes ?? 0)}/${Number(refresh.attempts ?? 0)}，合并 ${Number(refresh.nodes_merged ?? 0)}，隔离 ${Math.max(Number(replication.peers_quarantined ?? 0), Number(refresh.peers_quarantined ?? 0))}`
}

async function runDhtMaintenanceNow() {
  await runAsync('手动运行 DHT 维护', async () => {
    const body = await nodeFetchJson('/dht/maintenance?factor=3&limit=8&max_targets=8')
    nodeDhtMaintenanceStatusText.value = dhtMaintenanceSummary(body)
    recordDhtOperation(nodeDhtMaintenanceStatusText.value)
    nodeClosestInfoText.value = JSON.stringify(body, null, 2)
    await checkNodeHealth()
  })
}

function dhtReplicationSummary(body: any): string {
  const stats = body?.stats ?? {}
  return `DHT 复制：peer ${Number(body?.peers ?? 0)}，records ${Number(stats.records ?? body?.records ?? 0)}，尝试 ${Number(stats.attempts ?? 0)}，成功 ${Number(stats.successes ?? 0)}，失败 ${Number(stats.failures ?? 0)}，隔离 ${Number(stats.peers_quarantined ?? 0)}`
}

async function runDhtReplicationNow() {
  await runAsync('手动复制 DHT 记录', async () => {
    const body = await nodeFetchJson('/dht/replicate?factor=3')
    nodeDhtReplicationStatusText.value = dhtReplicationSummary(body)
    recordDhtOperation(nodeDhtReplicationStatusText.value)
    nodeClosestInfoText.value = JSON.stringify(body, null, 2)
    await checkNodeHealth()
  })
}

function dhtRoutingRefreshSummary(body: any): string {
  const stats = body?.stats ?? {}
  return `DHT 路由刷新：peer ${Number(body?.peers ?? 0)}，尝试 ${Number(stats.attempts ?? 0)}，成功 ${Number(stats.successes ?? 0)}，失败 ${Number(stats.failures ?? 0)}，返回 ${Number(stats.nodes_returned ?? 0)}，合并 ${Number(stats.nodes_merged ?? 0)}，隔离 ${Number(stats.peers_quarantined ?? 0)}`
}

async function runDhtRoutingRefreshNow() {
  await runAsync('手动刷新 DHT 路由', async () => {
    const body = await nodeFetchJson('/dht/routing-refresh?limit=8&max_targets=8')
    nodeRoutingRefreshStatusText.value = dhtRoutingRefreshSummary(body)
    recordDhtOperation(nodeRoutingRefreshStatusText.value)
    nodeClosestInfoText.value = JSON.stringify(body, null, 2)
    await checkNodeHealth()
  })
}

async function queryNodeClosestPeers() {
  await runAsync('查询 lm_node 最近节点', async () => {
    const target = encodeURIComponent(nodeClosestTarget.value.trim() || publicPeerId.value.trim() || 'public-peer-1')
    const body = await nodeFetchJson(`/peers/closest?target=${target}&limit=8`)
    nodeClosestInfoText.value = JSON.stringify(body, null, 2)
  })
}

async function pushMailboxToNode() {
  await runAsync('提交 MailboxMessage 到 lm_node', async () => {
    if (!mailboxMessageText.value.trim()) throw new Error('请先生成或粘贴 MailboxMessage')
    const key = mailboxMessageInspectPublicKey.value.trim() || identity.value?.identity_public_key
    if (!key) throw new Error('需要发送者 identity_public_key')
    const body = await nodeFetchJson('/mailbox/push', {
      method: 'POST',
      body: JSON.stringify({
        message_text: mailboxMessageText.value,
        from_identity_public_key: key,
      }),
    })
    nodeControlStatus.value = JSON.stringify(body, null, 2)
  })
}

async function ackMailboxToNode(userId: string, deliveryIds: string[]) {
  if (deliveryIds.length === 0) return
  const body = await nodeFetchJson('/mailbox/ack', {
    method: 'POST',
    body: JSON.stringify({
      user_id: userId,
      delivery_ids: deliveryIds,
    }),
  })
  nodeControlStatus.value = JSON.stringify(body, null, 2)
  updateMailboxQuotaStatus(body)
}


async function publishPreKeyToNode() {
  await runAsync('发布 PreKey Bundle 到 lm_node', async () => {
    if (!prekeyBundleText.value.trim()) createMyPreKeyBundleText()
    if (!prekeyBundleText.value.trim()) throw new Error('请先生成 PreKey Bundle')
    const body = await publishPreKeyBundlePayload()
    nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
    prekeyStatusSummary.value = summarizePreKeyStatus(body)
  })
}

async function publishAndCheckAllMyDht() {
  await runAsync('发布并检查我的 DHT 发布', async () => {
    if (!identity.value?.user_id) throw new Error('需要先登录身份')
    if (!prekeyBundleText.value.trim()) createMyPreKeyBundleText()
    if (!prekeyBundleText.value.trim()) throw new Error('请先生成 PreKey Bundle')
    const prekeyBody = await publishPreKeyBundlePayload()
    nodePreKeyStatusText.value = JSON.stringify(prekeyBody, null, 2)
    prekeyStatusSummary.value = `${summarizePreKeyStatus(prekeyBody)}，已发布，正在检查全部 DHT`
    fillDhtKeyInput('prekey', identity.value.user_id)
    let keyPayload = await deriveDhtKeyPayload()
    await runDhtFindValueForKey(String(keyPayload.key || nodeDhtFindValueKey.value).trim())

    const primaryNode = nodeEntries()[0]?.url
    if (!primaryNode) throw new Error('请先填写同步节点')
    fillDhtKeyInput('mailbox-hint', identity.value.user_id)
    keyPayload = await deriveDhtKeyPayload()
    const now = Math.floor(Date.now() / 1000)
    const mailboxRecord = {
      key: String(keyPayload.key || nodeDhtFindValueKey.value).trim(),
      kind: 'MailboxHint',
      value: primaryNode,
      created_at: now,
      expires_at: now + 24 * 3600,
      republish_at: now,
    }
    await nodeFetchJson('/dht/record', {
      method: 'POST',
      body: JSON.stringify({ record: mailboxRecord }),
    })
    await runDhtFindValueForKey(mailboxRecord.key)

    const contactCard = await publishContactCardDhtRecord({ recordHistory: false })
    await runDhtFindValueForKey(contactCard.key)

    if (!publicPeerAnnounceText.value.trim()) createPublicPeerAnnounceText()
    if (!publicPeerAnnounceText.value.trim()) throw new Error('请先生成 PublicPeerAnnounce')
    const peerId = publicPeerId.value.trim()
    if (!peerId) throw new Error('缺少 public peer id')
    fillDhtKeyInput('public-peer', peerId)
    keyPayload = await deriveDhtKeyPayload()
    const publicPeerRecord = {
      key: String(keyPayload.key || nodeDhtFindValueKey.value).trim(),
      kind: 'PublicPeer',
      value: publicPeerAnnounceText.value,
      created_at: now,
      expires_at: now + 24 * 3600,
      republish_at: now,
    }
    const store = await nodeFetchJson('/dht/record', {
      method: 'POST',
      body: JSON.stringify({ record: publicPeerRecord }),
    })
    nodeClosestInfoText.value = JSON.stringify(store, null, 2)
    await runDhtFindValueForKey(publicPeerRecord.key)

    prekeyStatusSummary.value = `${summarizePreKeyStatus(prekeyBody)}，已发布并完成全部 DHT 查找`
    mailboxInboxStatus.value = `PreKey / MailboxHint / ContactCard / PublicPeer 已发布并完成 DHT 查找`
    nodeDhtFindValueStatusText.value = `全部 DHT 发布已验证：PreKey、MailboxHint、ContactCard、PublicPeer；${nodeDhtFindValueStatusText.value}`
    recordDhtOperation('全部 DHT 发布已验证：PreKey、MailboxHint、ContactCard、PublicPeer')
  })
}

async function publishPublicPeerDhtRecord(options: { recordHistory?: boolean } = {}): Promise<{ key: string; peerId: string; store: any }> {
  if (!publicPeerAnnounceText.value.trim()) createPublicPeerAnnounceText()
  if (!publicPeerAnnounceText.value.trim()) throw new Error('请先生成 PublicPeerAnnounce')
  const peerId = publicPeerId.value.trim()
  if (!peerId) throw new Error('缺少 public peer id')
  fillDhtKeyInput('public-peer', peerId)
  const keyPayload = await deriveDhtKeyPayload()
  const now = Math.floor(Date.now() / 1000)
  const record = {
    key: String(keyPayload.key || nodeDhtFindValueKey.value).trim(),
    kind: 'PublicPeer',
    value: publicPeerAnnounceText.value,
    created_at: now,
    expires_at: now + 24 * 3600,
    republish_at: now,
  }
  const store = await nodeFetchJson('/dht/record', {
    method: 'POST',
    body: JSON.stringify({ record }),
  })
  nodeClosestInfoText.value = JSON.stringify(store, null, 2)
  if (options.recordHistory !== false) recordDhtOperation(`PublicPeer 已发布到 DHT：${peerId}`)
  return { key: record.key, peerId, store }
}

function hasConfiguredPublicPeerForAutoPublish(): boolean {
  if (publicPeerAnnounceText.value.trim()) return true
  const addresses = parseLines(publicPeerAddressesText.value)
  if (addresses.length === 0) return false
  return addresses.some((address) => address !== '/dns4/example.com/tcp/443/wss')
}

async function ensureOwnPublicPeerDhtRecord() {
  if (!identity.value || !nodeEnabled.value) return
  if (!hasConfiguredPublicPeerForAutoPublish()) return
  try {
    const { key } = await publishPublicPeerDhtRecord({ recordHistory: false })
    appendLog(`✅ PublicPeer 已自动发布到 DHT：${key.slice(0, 12)}…`)
  } catch (error) {
    appendLog(`⚠️ PublicPeer 自动发布到 DHT 失败：${userFacingError(error)}`)
  }
}

async function publishAndCheckMyPublicPeerDht() {
  await runAsync('发布并检查我的 PublicPeer DHT', async () => {
    const { key, peerId } = await publishPublicPeerDhtRecord()
    await runDhtFindValueForKey(key)
    nodeDhtFindValueStatusText.value = `PublicPeer 已发布并完成 DHT 查找：${peerId}；${nodeDhtFindValueStatusText.value}`
    recordDhtOperation(`PublicPeer 已发布并完成 DHT 查找：${peerId}`)
  })
}

async function publishMailboxHintDhtRecordFor(url: string, options: { recordHistory?: boolean } = {}): Promise<{ key: string; store: any }> {
  if (!identity.value?.user_id) throw new Error('需要先登录身份')
  const mailboxUrl = url.trim().replace(/\/$/, '')
  if (!mailboxUrl) throw new Error('请先填写同步节点')
  fillDhtKeyInput('mailbox-hint', identity.value.user_id)
  const keyPayload = await deriveDhtKeyPayload()
  const now = Math.floor(Date.now() / 1000)
  const record = {
    key: String(keyPayload.key || nodeDhtFindValueKey.value).trim(),
    kind: 'MailboxHint',
    value: mailboxUrl,
    created_at: now,
    expires_at: now + 24 * 3600,
    republish_at: now,
  }
  const store = await nodeFetchJson('/dht/record', {
    method: 'POST',
    body: JSON.stringify({ record }),
  })
  nodeClosestInfoText.value = JSON.stringify(store, null, 2)
  if (options.recordHistory !== false) recordDhtOperation(`MailboxHint 已发布到 DHT：${mailboxUrl}`)
  return { key: record.key, store }
}

async function ensureOwnMailboxHintDhtRecord() {
  const primaryNode = nodeEntries()[0]?.url
  if (!identity.value?.user_id || !primaryNode) return
  try {
    const { key } = await publishMailboxHintDhtRecordFor(primaryNode, { recordHistory: false })
    mailboxInboxStatus.value = 'MailboxHint 已自动发布到 DHT'
    appendLog(`✅ MailboxHint 已自动发布到 DHT：${key.slice(0, 12)}…`)
  } catch (e) {
    const message = userFacingError(e)
    appendLog(`⚠️ MailboxHint 自动发布到 DHT 失败：${message}`)
  }
}


async function publishContactCardDhtRecord(options: { recordHistory?: boolean } = {}): Promise<{ key: string; store: any }> {
  if (!identity.value?.user_id) throw new Error('需要先登录身份')
  if (!myContactCardText.value.trim()) refreshMyContactCard()
  if (!myContactCardText.value.trim()) throw new Error('请先生成我的联系人名片')
  fillDhtKeyInput('contact-card', identity.value.user_id)
  const keyPayload = await deriveDhtKeyPayload()
  const now = Math.floor(Date.now() / 1000)
  const record = {
    key: String(keyPayload.key || nodeDhtFindValueKey.value).trim(),
    kind: 'ContactCard',
    value: myContactCardText.value,
    created_at: now,
    expires_at: now + 24 * 3600,
    republish_at: now,
  }
  const store = await nodeFetchJson('/dht/record', {
    method: 'POST',
    body: JSON.stringify({ record }),
  })
  nodeClosestInfoText.value = JSON.stringify(store, null, 2)
  if (options.recordHistory !== false) recordDhtOperation(`ContactCard 已发布到 DHT：${record.key.slice(0, 12)}…`)
  return { key: record.key, store }
}


async function ensureOwnContactCardDhtRecord() {
  if (!identity.value?.user_id || !nodeEnabled.value) return
  try {
    const { key } = await publishContactCardDhtRecord({ recordHistory: false })
    appendLog(`✅ ContactCard 已自动发布到 DHT：${key.slice(0, 12)}…`)
  } catch (e) {
    appendLog(`⚠️ ContactCard 自动发布到 DHT 失败：${userFacingError(e)}`)
  }
}

async function publishAndCheckMyContactCardDht() {
  await runAsync('发布并检查我的 ContactCard DHT', async () => {
    const { key } = await publishContactCardDhtRecord()
    await runDhtFindValueForKey(key)
    nodeDhtFindValueStatusText.value = `ContactCard 已发布并完成 DHT 查找；${nodeDhtFindValueStatusText.value}`
    recordDhtOperation('ContactCard 已发布并完成 DHT 查找')
  })
}

async function publishAndCheckMyMailboxHintDht() {
  await runAsync('发布并检查我的 MailboxHint DHT', async () => {
    const primaryNode = nodeEntries()[0]?.url
    if (!primaryNode) throw new Error('请先填写同步节点')
    const { key } = await publishMailboxHintDhtRecordFor(primaryNode)
    await runDhtFindValueForKey(key)
    nodeDhtFindValueStatusText.value = `MailboxHint 已发布并完成 DHT 查找：${primaryNode}；${nodeDhtFindValueStatusText.value}`
    recordDhtOperation(`MailboxHint 已发布并完成 DHT 查找：${primaryNode}`)
    mailboxInboxStatus.value = `MailboxHint 已发布并完成 DHT 查找：${primaryNode}`
  })
}

async function publishAndCheckMyPreKeyDht() {
  await runAsync('发布并检查我的 PreKey DHT', async () => {
    if (!identity.value?.user_id) throw new Error('需要先登录身份')
    if (!prekeyBundleText.value.trim()) createMyPreKeyBundleText()
    if (!prekeyBundleText.value.trim()) throw new Error('请先生成 PreKey Bundle')
    const body = await publishPreKeyBundlePayload()
    nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
    prekeyStatusSummary.value = `${summarizePreKeyStatus(body)}，已发布，正在检查 DHT`
    fillDhtKeyInput('prekey', identity.value.user_id)
    const keyPayload = await deriveDhtKeyPayload()
    await runDhtFindValueForKey(String(keyPayload.key || nodeDhtFindValueKey.value).trim())
    prekeyStatusSummary.value = `${summarizePreKeyStatus(body)}，已发布并完成 DHT 查找`
  })
}

async function publishPreKeyBundlePayload(): Promise<any> {
  return nodeFetchJson('/prekey/publish', {
    method: 'POST',
    body: JSON.stringify({
      prekey_bundle_text: prekeyBundleText.value,
      signed_one_time_prekey_record_texts: prekeySignedOneTimeRecordTexts.value,
    }),
  })
}

async function fetchOwnPreKeyStatus(): Promise<any> {
  const userId = identity.value?.user_id
  if (!userId) throw new Error('需要当前身份')
  return nodeFetchJson(`/prekey/status?user_id=${encodeURIComponent(userId)}`)
}

async function ensurePreKeyInventory() {
  if (!identity.value) throw new Error('需要当前身份')
  try {
    const status = await fetchOwnPreKeyStatus()
    const missing = status?.found === false
    const low = Boolean(status?.low_one_time_prekeys || status?.replenishment_required)
    if (!missing && !low) {
      nodePreKeyStatusText.value = JSON.stringify(status, null, 2)
      prekeyStatusSummary.value = summarizePreKeyStatus(status)
      prekeyAutoStateText.value = '库存正常'
      prekeyAutoErrorText.value = ''
      return
    }
    if (missing && !prekeyBundleText.value.trim()) createMyPreKeyBundleText()
    if (low) createMyPreKeyBundleText()
    const body = await publishPreKeyBundlePayload()
    nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
    prekeyStatusSummary.value = `${summarizePreKeyStatus(body)}，${missing ? '已自动发布' : '已自动补货'}`
    prekeyAutoStateText.value = missing ? '已自动发布' : '已自动补货'
    prekeyAutoErrorText.value = ''
    appendLog(missing ? '✅ PreKey 已自动发布到节点' : '✅ PreKey one-time key 已自动补货')
  } catch (e) {
    const message = userFacingError(e)
    prekeyAutoStateText.value = '自动检查失败'
    prekeyAutoErrorText.value = message
    appendLog(`❌ PreKey 自动检查/补货失败：${message}`)
    throw e
  }
}

async function refreshPreKeyStatusFromNode() {
  await runAsync('刷新 PreKey 状态', async () => {
    const body = await fetchOwnPreKeyStatus()
    nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
    prekeyStatusSummary.value = summarizePreKeyStatus(body)
    prekeyAutoStateText.value = summarizePreKeyStatus(body)
    prekeyAutoErrorText.value = ''
  })
}

async function replenishPreKeyIfLow() {
  await runAsync('检查并补货 PreKey', async () => {
    await ensurePreKeyInventory()
  })
}

async function retryPreKeyAutoPublish() {
  await runAsync('重试 PreKey 自动发布/补货', async () => {
    await ensurePreKeyInventory()
  })
}

function clearPreKeyRawState() {
  run('清除 PreKey 原始状态', () => {
    nodePreKeyStatusText.value = ''
    prekeyBundleText.value = ''
    prekeySignedOneTimeRecordTexts.value = []
    prekeyInfoText.value = ''
    x3dhInitialMessageJson.value = ''
    selectedSignedOneTimePreKeyRecordText.value = ''
    selectedOneTimePreKeyId.value = null
    prekeyStatusSummary.value = '已清除原始 PreKey 文本，需要时会重新生成'
    persist()
  })
}

function summarizePreKeyStatus(body: any): string {
  const remaining = typeof body?.remaining_one_time_prekeys === 'number' ? body.remaining_one_time_prekeys : null
  const low = Boolean(body?.low_one_time_prekeys || body?.replenishment_required)
  const userId = String(body?.user_id ?? identity.value?.user_id ?? '')
  const found = body?.found !== false
  if (!found) return `${userId ? userId + '：' : ''}节点未找到 PreKey，需要发布`
  const keyText = remaining === null ? 'one-time key 数量未知' : `剩余 one-time key ${remaining}`
  const selected = typeof body?.selected_one_time_prekey_id === 'number' ? `，选中 key ${body.selected_one_time_prekey_id}` : ''
  const signed = body?.selected_signed_one_time_prekey_record_text ? '，signed record 可用' : ''
  const action = low ? '，需要客户端补货' : '，库存正常'
  return `${userId ? userId + '：' : ''}${keyText}${selected}${signed}${action}`
}

function applyPreKeyNodeResponse(body: any, sourceLabel: string): boolean {
  nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
  if (body.found && body.prekey_bundle_text) {
    prekeyBundleText.value = body.prekey_bundle_text
    selectedOneTimePreKeyId.value = typeof body.selected_one_time_prekey_id === 'number' ? body.selected_one_time_prekey_id : null
    selectedSignedOneTimePreKeyRecordText.value = typeof body.selected_signed_one_time_prekey_record_text === 'string' ? body.selected_signed_one_time_prekey_record_text : ''
    inspectPreKeyBundleText()
    appendLog(`✅ 已${sourceLabel} PreKey Bundle${selectedOneTimePreKeyId.value !== null ? '，选中 one-time key ' + selectedOneTimePreKeyId.value : ''}${selectedSignedOneTimePreKeyRecordText.value ? '（signed record）' : ''}`)
    return true
  }
  return false
}

async function fetchPreKeyViaDht(userId: string): Promise<boolean> {
  const dht = await nodeFetchJson(`/dht/find-value?kind=prekey&value=${encodeURIComponent(userId)}&limit=8&max_peers=8&alpha=3`)
  const ok = applyDhtFindValueRecord(dht)
  nodeDhtFindValueStatusText.value = ok ? dhtFindValueSummary(dht) : nodeDhtFindValueStatusText.value
  nodeClosestInfoText.value = JSON.stringify(dht, null, 2)
  return ok && dht?.record?.kind === 'PreKey' && typeof dht.record.value === 'string'
}

async function fetchPreKeyFromNode() {
  await runAsync('从 lm_node 拉取 PreKey Bundle', async () => {
    const userId = nodePreKeyUserId.value.trim() || activeContact.value?.user_id
    if (!userId) throw new Error('请输入 UserID 或选择联系人')
    const body = await nodeFetchJson(`/prekey/get?user_id=${encodeURIComponent(userId)}&consume=false`)
    if (!applyPreKeyNodeResponse(body, '从节点拉取')) {
      await fetchPreKeyViaDht(userId)
    }
  })
}

async function consumePreKeyFromNode() {
  await runAsync('从 lm_node 领取并消费 PreKey Bundle', async () => {
    const userId = nodePreKeyUserId.value.trim() || activeContact.value?.user_id
    if (!userId) throw new Error('请输入 UserID 或选择联系人')
    const body = await nodeFetchJson(`/prekey/get?user_id=${encodeURIComponent(userId)}&consume=true`)
    if (!applyPreKeyNodeResponse(body, '领取')) {
      await fetchPreKeyViaDht(userId)
    }
  })
}


async function exportNodeSnapshot() {
  await runAsync('导出 lm_node 同步快照', async () => {
    const body = await nodeFetchJson('/sync/snapshot')
    nodeSyncSnapshotText.value = JSON.stringify(body, null, 2)
    nodeSyncStatusText.value = `snapshot peers=${body.public_peers?.length ?? 0}, mailbox=${body.mailbox_deliveries?.length ?? 0}, prekeys=${body.prekey_bundles?.length ?? 0}, signed_otpk=${body.signed_one_time_prekey_records?.length ?? 0}`
  })
}

async function importNodeSnapshot() {
  await runAsync('导入 lm_node 同步快照', async () => {
    if (!nodeSyncSnapshotText.value.trim()) throw new Error('请先粘贴或导出 snapshot')
    const snapshot = JSON.parse(nodeSyncSnapshotText.value)
    const body = await nodeFetchJson('/sync/import', {
      method: 'POST',
      body: JSON.stringify({ snapshot }),
    })
    lastNodeSnapshotSyncAt.value = Date.now()
    nodeSyncStatusText.value = JSON.stringify(body, null, 2)
    persist()
  })
}

async function pullSnapshotFromPeerNode() {
  await runAsync('从另一个 lm_node 拉取快照并导入当前节点', async () => {
    const url = nodeSyncPeerUrl.value.replace(/\/$/, '')
    const res = await fetch(`${url}/sync/snapshot`)
    if (!res.ok) throw new Error(await res.text())
    const snapshot = await res.json()
    nodeSyncSnapshotText.value = JSON.stringify(snapshot, null, 2)
    const body = await nodeFetchJson('/sync/import', {
      method: 'POST',
      body: JSON.stringify({ snapshot }),
    })
    lastNodeSnapshotSyncAt.value = Date.now()
    nodeSyncStatusText.value = JSON.stringify(body, null, 2)
    persist()
  })
}

async function autoPullSnapshotFromPeerNode() {
  if (!loggedIn.value || !nodeEnabled.value || !autoNodeSync.value || !nodeSyncPeerUrl.value.trim()) return
  try {
    const url = nodeSyncPeerUrl.value.replace(/\/$/, '')
    const res = await fetch(`${url}/sync/snapshot`)
    if (!res.ok) throw new Error(await res.text())
    const snapshot = await res.json()
    const body = await nodeFetchJson('/sync/import', { method: 'POST', body: JSON.stringify({ snapshot }) })
    lastNodeSnapshotSyncAt.value = Date.now()
    nodeSyncStatusText.value = `auto sync ok: ${JSON.stringify(body)}`
    persist()
  } catch (e) {
    nodeSyncStatusText.value = `auto sync failed: ${String(e)}`
  }
}

function startNodeSyncLoop() {
  if (nodeSyncTimer) return
  nodeSyncTimer = window.setInterval(() => { void autoPullSnapshotFromPeerNode() }, 60_000)
}



function recordContactCardDhtAutoRefresh(item: ContactCardDhtAutoRefreshRecord) {
  contactCardDhtAutoRefreshHistory.value = [item, ...contactCardDhtAutoRefreshHistory.value].slice(0, 20)
}

async function refreshContactCardDhtForContact(contact: ContactItem, options: { preserveUi?: boolean } = {}): Promise<boolean> {
  const previousPeerId = activePeerId.value
  const previousGroupId = activeGroupId.value
  const previousKind = nodeDhtKeyKind.value
  const previousValue = nodeDhtKeyValue.value
  const previousKey = nodeDhtFindValueKey.value
  try {
    fillDhtKeyInput('contact-card', contact.user_id)
    await deriveAndFindDhtValueNow()
    return true
  } finally {
    if (options.preserveUi) {
      activePeerId.value = previousPeerId
      activeGroupId.value = previousGroupId
      nodeDhtKeyKind.value = previousKind
      nodeDhtKeyValue.value = previousValue
      nodeDhtFindValueKey.value = previousKey
    }
  }
}

async function autoRefreshStaleContactCardDht() {
  if (!loggedIn.value || !nodeEnabled.value || !autoNodeSync.value) return
  const stale = friendContacts.value
    .filter((contact) => contactCardDhtDiscoveryIsStale(contact))
    .slice(0, 3)
  if (!stale.length) return
  let refreshed = 0
  const errors: string[] = []
  for (const contact of stale) {
    try {
      await refreshContactCardDhtForContact(contact, { preserveUi: true })
      refreshed += 1
      recordContactCardDhtAutoRefresh({ user_id: contact.user_id, display_name: contact.display_name, status: 'success', refreshed_at: Date.now() })
    } catch (error) {
      const message = `${contact.display_name || contact.user_id}：${userFacingError(error)}`
      errors.push(message)
      recordContactCardDhtAutoRefresh({ user_id: contact.user_id, display_name: contact.display_name, status: 'failed', refreshed_at: Date.now(), error: userFacingError(error) })
      appendLog(`后台 ContactCard DHT 刷新失败：${message}`)
    }
  }
  if (refreshed) {
    contactCardDhtAutoRefreshCount.value += refreshed
    lastContactCardDhtAutoRefreshAt.value = Date.now()
    lastContactCardDhtAutoRefreshError.value = ''
    appendLog(`后台 ContactCard DHT 已刷新 ${refreshed}/${stale.length} 个联系人`)
    persist()
  } else if (errors.length) {
    lastContactCardDhtAutoRefreshError.value = errors.slice(0, 3).join('；')
    persist()
  }
}

function startSelfSyncLoop() {
  if (selfSyncTimer) return
  selfSyncTimer = window.setInterval(() => {
    void autoPushSelfSyncPackageToOwnMailbox()
    void autoRefreshStaleContactCardDht()
  }, 60_000)
}

function contactByUserId(userId: string): ContactItem | null {
  return contacts.value.find((c) => c.user_id === userId) ?? null
}
function contactInfoFromCardText(cardText: string): ContactInfo | null {
  try {
    return safeJson<ContactInfo>(inspect_contact_card(cardText))
  } catch {
    return null
  }
}


function unwrapMailboxDelivery(item: any): { deliveryId?: string; message: any } {
  if (item && typeof item === 'object' && item.message) {
    return { deliveryId: String(item.delivery_id ?? ''), message: item.message }
  }
  return { message: item }
}

type MailboxEventKind = 'message' | 'file' | 'friend-request' | 'friend-response' | 'group-invite' | 'delivery-ack' | 'read-receipt' | 'device-revoke' | 'contact-update' | 'secure-session' | 'data-backup' | 'self-sync' | 'other'

function handleMailboxPayload(item: any): { handled: boolean; deliveryId?: string; event?: MailboxEventKind; reason?: string } {
  const { deliveryId, message } = unwrapMailboxDelivery(item)
  const kind = typeof message.kind === 'string' ? message.kind : ''
  const normalizedKind = kind.replace(/[-_]/g, '').toLowerCase()
  const fromUserId = String(message.from_user_id ?? '')
  const ciphertext = String(message.ciphertext ?? '')
  let sender = contactByUserId(fromUserId)
  if (identity.value?.user_id && fromUserId === identity.value.user_id && normalizedKind === 'selfsyncrequest') {
    const pkg = JSON.parse(ciphertext) as SelfSyncRequestPackage
    if (pkg?.type !== 'lm-self-sync-request-v1') throw new Error('self-sync request 类型不匹配')
    void applySelfSyncRequestPackage(pkg)
    return { handled: true, deliveryId, event: 'self-sync' }
  }
  if (identity.value?.user_id && fromUserId === identity.value.user_id && normalizedKind === 'selfsync') {
    const pkg = JSON.parse(ciphertext) as SelfSyncPackage
    if (pkg?.type !== 'lm-self-sync-v1') throw new Error('self-sync package 类型不匹配')
    applySelfSyncPackage(pkg)
    return { handled: true, deliveryId, event: 'self-sync' }
  }
  if (identity.value?.user_id && fromUserId === identity.value.user_id && (normalizedKind === 'databackup' || ciphertext.startsWith('lm-data-backup-v1:'))) {
    dataBackupText.value = ciphertext
    lastSelfMailboxBackupReceivedAt.value = Date.now()
    selfMailboxBackupStatusText.value = '已从自己的 Mailbox 收到完整数据备份，可在设置页导入合并'
    appendLog(`✅ ${selfMailboxBackupStatusText.value}`)
    mailboxInboxStatus.value = selfMailboxBackupStatusText.value
    return { handled: true, deliveryId, event: 'data-backup' }
  }
  if (!sender && ciphertext.startsWith('lm-friend-request-v1:')) {
    try {
      const info = safeJson<Omit<FriendRequestItem, 'request_text'>>(inspect_friend_request(ciphertext))
      const cardInfo = contactInfoFromCardText(info.from_contact_card_text)
      const contact: ContactItem | null = cardInfo ? { ...mergeContactCard(undefined, cardInfo, info.from_contact_card_text), state: 'RequestReceived' } : null
      if (contact) {
        contacts.value.push(contact)
        sender = contact
      }
    } catch (e) {
      appendLog(`好友请求解析失败：${String(e)}`)
    }
  }
  if (!sender) {
    const reason = `未知联系人：${fromUserId || 'unknown'}`
    appendLog(`消息来自${reason}`)
    return { handled: false, deliveryId, reason }
  }
  if (sender.state === 'Blocked') {
    const reason = `已拉黑联系人：${fromUserId}`
    appendLog(`mailbox 消息来自${reason}`)
    return { handled: false, deliveryId, reason }
  }
  activePeerId.value = sender.user_id
  activeGroupId.value = ''

  if (normalizedKind === 'contactupdate' || ciphertext.startsWith('lm-contact-card-v1:')) {
    applyContactCardUpdateFromMailbox(ciphertext, sender)
    void sendDeliveryAck(sender, contactCardUpdateId(ciphertext), `contact-update-${sender.user_id}`, deliveryId)
    return { handled: true, deliveryId, event: 'contact-update' }
  }

  if (normalizedKind === 'directenvelope' || normalizedKind === 'groupfanout') {
    try {
      const accepted = receiveEnvelopeWithContact(ciphertext, sender, deliveryId)
      if (!accepted) return { handled: true, deliveryId, event: 'other' }
      appendLog(`✅ 已自动解密 mailbox ${kind}`)
      return { handled: true, deliveryId, event: 'message' }
    } catch (e) {
      const reason = `${kind || 'message'} 解密失败：${userFacingError(e)}`
      appendLog(`❌ mailbox ${reason}`)
      inboundEnvelopeText.value = ciphertext
      return { handled: false, deliveryId, reason }
    }
  }

  if (normalizedKind === 'signaloffer') {
    remoteSignalText.value = ciphertext
    appendLog('✅ 已从 mailbox 填入远端 Signal Offer')
    return { handled: true, deliveryId, event: 'secure-session' }
  }
  if (normalizedKind === 'signalanswer') {
    remoteSignalText.value = ciphertext
    appendLog('✅ 已从 mailbox 填入远端 Signal Answer')
    return { handled: true, deliveryId, event: 'secure-session' }
  }

  if (normalizedKind === 'deliveryreceipt' || normalizedKind === 'readreceipt' || ciphertext.startsWith('lm-message-receipt-v1:')) {
    const event = applyMessageReceiptText(ciphertext, sender)
    return { handled: true, deliveryId, event }
  }

  if (ciphertext.startsWith('lm-friend-request-v1:')) {
    const info = safeJson<Omit<FriendRequestItem, 'request_text'>>(inspect_friend_request(ciphertext))
    if (identity.value && info.to_user_id !== identity.value.user_id) {
      throw new Error('这个好友请求不是发给当前身份的')
    }
    const item = upsertFriendRequestWithLocalRateLimit({ ...info, request_text: ciphertext })
    toast(item.quarantined ? '好友请求已隔离' : '收到新的好友请求', item.quarantined ? 'warning' : 'info')
    return { handled: true, deliveryId, event: 'friend-request' }
  }
  if (ciphertext.startsWith('lm-friend-response-v1:')) {
    incomingFriendResponseText.value = ciphertext
    applyFriendResponse()
    return { handled: true, deliveryId, event: 'friend-response' }
  }
  if (ciphertext.startsWith('lm-group-invite-v1:')) {
    incomingGroupInviteText.value = ciphertext
    addIncomingGroupInvite()
    return { handled: true, deliveryId, event: 'group-invite' }
  }
  try {
    const parsed = JSON.parse(ciphertext) as { type?: string; message_id?: string; from_user_id?: string }
    if (parsed?.type === 'lm-delivery-ack-v1' && parsed.message_id) {
      applyDeliveryAck(parsed.message_id, fromUserId)
      return { handled: true, deliveryId, event: 'delivery-ack' }
    }
    if (parsed?.type === 'lm-device-revoke-v1') {
      incomingDeviceRevokeText.value = ciphertext
      applyDeviceRevokeToActiveContact()
      return { handled: true, deliveryId, event: 'device-revoke' }
    }
    if (parsed?.type === 'lm-file-package-v1') {
      pendingFilePackageText.value = ciphertext
      incomingFilePackageText.value = ciphertext
      inspectIncomingFilePackage()
      fileTransferPhase.value = '待解密'
      rtcFileStatus.value = '收到文件包，点击后解密'
      return { handled: true, deliveryId, event: 'file' }
    }
    if (parsed?.type === 'lm-secure-session-response-v1') {
      incomingSecureSessionText.value = ciphertext
      applySecureSessionResponseText()
      return { handled: true, deliveryId, event: 'secure-session' }
    }
    if (parsed?.type === 'lm-secure-session-offer-v1') {
      incomingSecureSessionText.value = ciphertext
      applySecureSessionOfferText()
      return { handled: true, deliveryId, event: 'secure-session' }
    }
  } catch {}

  mailboxCiphertext.value = ciphertext
  const reason = `未知类型：${kind || 'unknown'}`
  appendLog(`mailbox 消息类型 ${kind || 'unknown'} 已放入载荷输入框`)
  return { handled: false, deliveryId, reason }
}

function mailboxEventSummaryText(events: MailboxEventKind[]): string {
  const count = (kind: MailboxEventKind) => events.filter((event) => event === kind).length
  const parts = [
    ['消息', count('message')],
    ['文件', count('file')],
    ['好友请求', count('friend-request')],
    ['好友通过', count('friend-response')],
    ['群邀请', count('group-invite')],
    ['安全会话', count('secure-session')],
    ['完整备份', count('data-backup')],
    ['自同步', count('self-sync')],
    ['回执', count('delivery-ack') + count('read-receipt')],
  ].filter(([, n]) => Number(n) > 0).map(([label, n]) => `${label} ${n}`)
  return parts.length ? parts.join('，') : `已处理 ${events.length} 条`
}

function mailboxDedupeIds(deliveryId?: string, messageId?: string): string[] {
  return [deliveryId, messageId ? `message:${messageId}` : '']
    .map((id) => (id || '').trim())
    .filter(Boolean)
}

function rememberProcessedMailboxIds(ids: string[]) {
  const cleanIds = [...new Set(ids.map((id) => id.trim()).filter(Boolean))]
  if (cleanIds.length === 0) return
  const now = Date.now()
  const incoming = cleanIds.map((id) => ({ id, processed_at: now }))
  const incomingSet = new Set(cleanIds)
  processedMailboxIds.value = normalizeProcessedMailboxRecords([
    ...incoming,
    ...processedMailboxIds.value.filter((record) => !incomingSet.has(record.id)),
  ])
}

function rememberProcessedMailboxId(id: string) {
  rememberProcessedMailboxIds([id])
}

function hasProcessedMailboxIds(ids: string[]): boolean {
  const cleanIds = ids.map((id) => id.trim()).filter(Boolean)
  if (cleanIds.length === 0) return false
  return processedMailboxIds.value.some((record) => cleanIds.includes(record.id))
}

function hasProcessedMailboxId(id: string): boolean {
  return hasProcessedMailboxIds([id])
}

function mailboxFailedItemId(deliveryId: string, messageId: string): string {
  return deliveryId || messageId || `local-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
}

function rememberFailedMailboxItem(item: any, reason: string) {
  const { deliveryId, message } = unwrapMailboxDelivery(item)
  const messageId = String(message?.message_id ?? '')
  const id = mailboxFailedItemId(deliveryId || '', messageId)
  const existing = mailboxFailedItems.value.find((failed) => failed.id === id)
  const now = Date.now()
  if (existing) {
    existing.reason = reason
    existing.last_failed_at = now
    return
  }
  mailboxFailedItems.value = [{
    id,
    delivery_id: deliveryId,
    message_id: messageId || undefined,
    message,
    reason,
    first_failed_at: now,
    last_failed_at: now,
    retry_count: 0,
  }, ...mailboxFailedItems.value].slice(0, 100)
}

async function acknowledgeMailboxDeliveries(deliveryIds: string[]) {
  const userId = nodeMailboxTakeUserId.value.trim() || identity.value?.user_id
  if (!userId) throw new Error('需要 UserID')
  await ackMailboxToNode(userId, deliveryIds)
}

function clearProcessedMailboxIds() {
  processedMailboxIds.value = []
  mailboxInboxStatus.value = '已清空本地去重记录'
  mailboxInboxErrorText.value = ''
  mailboxFailureSummaryText.value = ''
  appendLog('mailbox 本地去重记录已清空')
  persist()
}

function clearFailedMailboxItems() {
  mailboxFailedItems.value = []
  mailboxInboxStatus.value = '已清空失败队列'
  mailboxInboxErrorText.value = ''
  mailboxFailureSummaryText.value = ''
  appendLog('mailbox 失败队列已清空')
  persist()
}

function summarizeMailboxFailures(reasons: string[]): string {
  if (reasons.length === 0) return ''
  const counts = new Map<string, number>()
  for (const reason of reasons) {
    const key = reason.split('：')[0] || reason
    counts.set(key, (counts.get(key) ?? 0) + 1)
  }
  return [...counts.entries()].map(([reason, count]) => `${reason} ${count}`).join('，')
}

function processMailboxMessages(messagesFromNode: any[]): string[] {
  let handled = 0
  let duplicate = 0
  let duplicateAckResent = 0
  let failed = 0
  const failureReasons: string[] = []
  const events: MailboxEventKind[] = []
  const ackIds: string[] = []
  for (const item of messagesFromNode) {
    const { deliveryId, message } = unwrapMailboxDelivery(item)
    const messageId = String(message?.message_id ?? '')
    const dedupeIds = mailboxDedupeIds(deliveryId, messageId)
    if (hasProcessedMailboxIds(dedupeIds)) {
      duplicate += 1
      if (resendAckForDuplicateMailboxMessage(message, deliveryId)) duplicateAckResent += 1
      if (deliveryId) ackIds.push(deliveryId)
      rememberProcessedMailboxIds(dedupeIds)
      continue
    }
    const result = handleMailboxPayload(item)
    if (result.handled) {
      handled += 1
      if (result.event) events.push(result.event)
      if (deliveryId) ackIds.push(deliveryId)
      rememberProcessedMailboxIds(dedupeIds)
    } else {
      failed += 1
      if (result.reason) failureReasons.push(result.reason)
      if (result.reason) rememberFailedMailboxItem(item, result.reason)
    }
  }
  mailboxInboxStatus.value = `收到 ${messagesFromNode.length}，已处理 ${handled}，重复 ${duplicate}${duplicateAckResent ? `，补发回执 ${duplicateAckResent}` : ''}，失败 ${failed}`
  mailboxInboxErrorText.value = failureReasons.slice(0, 3).join('\n')
  mailboxFailureSummaryText.value = summarizeMailboxFailures(failureReasons)
  appendLog(`mailbox 自动处理完成：${mailboxInboxStatus.value}`)
  if (events.length > 0) toast(`收到新内容：${mailboxEventSummaryText(events)}`, 'success')
  persist()
  return ackIds
}

async function retryFailedMailboxItemsNow() {
  const items = [...mailboxFailedItems.value]
  if (items.length === 0) {
    mailboxInboxStatus.value = '失败队列为空'
    return { handled: 0, failed: 0 }
  }
  let handled = 0
  let failed = 0
  const ackIds: string[] = []
  const handledItemIds: string[] = []
  const failureReasons: string[] = []
  for (const failedItem of items) {
    failedItem.retry_count += 1
    const result = handleMailboxPayload({
      delivery_id: failedItem.delivery_id,
      message: failedItem.message,
    })
    if (result.handled) {
      handled += 1
      const dedupeIds = mailboxDedupeIds(failedItem.delivery_id || result.deliveryId, failedItem.message_id)
      rememberProcessedMailboxIds(dedupeIds)
      if (failedItem.delivery_id) ackIds.push(failedItem.delivery_id)
      handledItemIds.push(failedItem.id)
    } else {
      failed += 1
      const reason = result.reason || failedItem.reason
      failedItem.reason = reason
      failedItem.last_failed_at = Date.now()
      failureReasons.push(reason)
    }
  }
  if (ackIds.length > 0) await acknowledgeMailboxDeliveries(ackIds)
  mailboxFailedItems.value = mailboxFailedItems.value.filter((item) => !handledItemIds.includes(item.id))
  mailboxInboxStatus.value = `失败队列重试：成功 ${handled}，失败 ${failed}`
  mailboxInboxErrorText.value = failureReasons.slice(0, 3).join('\n')
  mailboxFailureSummaryText.value = summarizeMailboxFailures(failureReasons)
  appendLog(`mailbox 失败队列重试完成：${mailboxInboxStatus.value}`)
  persist()
  return { handled, failed }
}

async function retryFailedMailboxItems() {
  await runAsync('重试 mailbox 失败队列', async () => { await retryFailedMailboxItemsNow() })
}

async function recoverSyncFailures() {
  await runAsync('恢复同步失败', async () => {
    const actions: string[] = []
    const results: string[] = []
    if (prekeyAutoErrorText.value) {
      await ensurePreKeyInventory()
      actions.push('PreKey')
      results.push('PreKey 已重试')
    }
    if (mailboxFailedItems.value.length > 0) {
      const result = await retryFailedMailboxItemsNow()
      actions.push('Mailbox 失败队列')
      results.push(`Mailbox 成功 ${result.handled}，失败 ${result.failed}`)
    }
    const pendingOutbox = outbox.value.filter((item) => item.status !== 'sent')
    if (pendingOutbox.length > 0) {
      for (const item of pendingOutbox) item.next_retry_at = Date.now()
      await retryDueOutbox()
      actions.push(`Outbox ${pendingOutbox.length} 条`)
      const remaining = outbox.value.filter((item) => item.status !== 'sent').length
      results.push(`Outbox 已触发 ${pendingOutbox.length}，剩余 ${remaining}`)
    }
    if (/failed|失败/i.test(nodeSyncStatusText.value) && autoNodeSync.value && nodeSyncPeerUrl.value.trim()) {
      await autoPullSnapshotFromPeerNode()
      actions.push('节点快照')
      results.push('节点快照已重试')
    }
    if (selfSyncGapCount.value > 0 && nodeEnabled.value) {
      await repairSelfSyncGapNow()
      actions.push('轻量自同步')
      results.push('轻量自同步已补发')
    }
    syncRecoveryStatusText.value = results.length ? results.join('；') : '没有需要恢复的同步失败'
    syncRecoveryHistory.value = [syncRecoveryStatusText.value, ...syncRecoveryHistory.value].slice(0, 5)
    appendLog(actions.length ? `已恢复同步失败：${actions.join('、')}` : '没有需要恢复的同步失败')
    persist()
  })
}

function processMailboxTakeInfoText() {
  run('处理 mailbox JSON', () => {
    const parsed = JSON.parse(nodeMailboxTakeInfoText.value || '{"messages":[]}') as { messages?: any[] }
    processMailboxMessages(Array.isArray(parsed.messages) ? parsed.messages : [])
  })
}

async function takeMailboxFromNode() {
  await runAsync('从 lm_node 领取 mailbox', async () => {
    const userId = nodeMailboxTakeUserId.value.trim() || identity.value?.user_id
    if (!userId) throw new Error('需要 UserID')
    const pages: any[] = []
    let totalMessages = 0
    let totalAcked = 0
    const pageLimit = 50
    const maxPages = 20
    let hasMoreAfterMaxPages = false
    for (let page = 0; page < maxPages; page += 1) {
      const body = await nodeFetchJson(`/mailbox/take?user_id=${encodeURIComponent(userId)}&limit=${pageLimit}`)
      pages.push(body)
      updateMailboxQuotaStatus(body)
      const messages = Array.isArray(body.messages) ? body.messages : []
      if (messages.length === 0) break
      totalMessages += messages.length
      const ackIds = processMailboxMessages(messages)
      if (ackIds.length > 0) {
        await ackMailboxToNode(userId, ackIds)
        totalAcked += ackIds.length
      }
      if (!body.more || ackIds.length === 0) break
      hasMoreAfterMaxPages = page === maxPages - 1 && Boolean(body.more)
    }
    const lastPage = pages[pages.length - 1]
    if (lastPage) updateMailboxQuotaStatus(lastPage)
    nodeMailboxTakeInfoText.value = JSON.stringify(pages.length === 1 ? pages[0] : { pages }, null, 2)
    if (totalMessages === 0) {
      mailboxInboxStatus.value = '没有新消息'
      appendLog('mailbox 没有新消息')
      persist()
    } else if (pages.length > 1 || hasMoreAfterMaxPages) {
      const moreText = hasMoreAfterMaxPages ? '，仍有更多，请再次同步' : ''
      mailboxInboxStatus.value = `${mailboxInboxStatus.value}，分页 ${pages.length}，已 ack ${totalAcked}${moreText}`
      appendLog(`mailbox 分页收取完成：${totalMessages} 条，分页 ${pages.length}，ack ${totalAcked}${moreText}`)
      if (hasMoreAfterMaxPages) toast('Mailbox 仍有待收取内容：本次同步已达到分页上限，请再次同步继续收取。', 'warning')
      persist()
    }
  })
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  const step = 0x8000
  for (let i = 0; i < bytes.length; i += step) {
    binary += String.fromCharCode(...bytes.subarray(i, i + step))
  }
  return btoa(binary)
}

function base64ToBytes(value: string): Uint8Array {
  const binary = atob(value)
  const out = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i += 1) out[i] = binary.charCodeAt(i)
  return out
}

function filePreviewKind(name: string, mime: string): string {
  const lower = name.toLowerCase()
  if (mime.startsWith('image/')) return '图片预览'
  if (mime.startsWith('audio/')) return '音频文件'
  if (mime.startsWith('video/')) return '视频文件'
  if (mime === 'application/pdf' || lower.endsWith('.pdf')) return 'PDF 文档'
  if (/(\.docx?|\.xlsx?|\.pptx?)$/.test(lower)) return 'Office 文档'
  if (/(\.zip|\.7z|\.rar|\.tar|\.gz)$/.test(lower)) return '压缩包'
  if (mime.startsWith('text/') || /\.(txt|md|csv|log)$/.test(lower)) return '文本文件'
  return '普通附件'
}

function onFileSelected(event: Event) {
  const input = event.target as HTMLInputElement
  selectedFile.value = input.files?.[0] ?? null
  if (selectedFile.value) {
    const warning = isDangerousFileName(selectedFile.value.name) ? '，请确认来源可信' : ''
    const transferLimit = `当前 Web 端整包发送，最大 ${formatBytes(MAX_FILE_BYTES)}，暂不支持断点续传`
    fileTransferPhase.value = '已选择'
    rtcFileStatus.value = `已选择文件：${selectedFile.value.name} (${formatBytes(selectedFile.value.size)})${warning}`
    fileProgressText.value = transferLimit
  }
}

function cancelSelectedFile() {
  selectedFile.value = null
  filePackageText.value = ''
  filePackageInfoText.value = ''
  pendingFileMeta.value = ''
  fileProgressText.value = ''
  fileTransferPhase.value = '待选择'
  rtcFileStatus.value = '未发送文件'
}

function clearSelectedFileDraft(keepProgress = false) {
  selectedFile.value = null
  filePackageText.value = ''
  filePackageInfoText.value = ''
  if (!keepProgress) fileProgressText.value = ''
}

async function readFileWithProgress(file: File): Promise<Uint8Array> {
  if (!file.stream) {
    fileProgressText.value = '读取中'
    return new Uint8Array(await file.arrayBuffer())
  }
  const reader = file.stream().getReader()
  const chunks: Uint8Array[] = []
  let loaded = 0
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    if (value) {
      chunks.push(value)
      loaded += value.byteLength
      const percent = file.size > 0 ? Math.min(100, Math.round((loaded / file.size) * 100)) : 100
      fileProgressText.value = `读取 ${formatBytes(loaded)} / ${formatBytes(file.size)} (${percent}%)`
    }
  }
  const out = new Uint8Array(loaded)
  let offset = 0
  for (const chunk of chunks) {
    out.set(chunk, offset)
    offset += chunk.byteLength
  }
  return out
}

async function createFilePackageForActive(): Promise<boolean> {
  if (selectedFile.value && safetyPolicy.value.warnExecutableFiles && isDangerousFileName(selectedFile.value.name)) {
    const confirmed = await showConfirm(
      '发送危险类型文件',
      `文件「${selectedFile.value.name}」属于可执行/安装脚本等高风险类型。LM Talk 不会自动打开附件，但接收方下载后可能触发系统风险。确认继续发送？`,
      true,
    )
    if (!confirmed) {
      fileTransferPhase.value = '已取消'
      rtcFileStatus.value = '已取消危险类型文件发送'
      appendLog('已取消危险类型文件发送')
      return false
    }
  }
  let ok = false
  await runAsync('生成文件包', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    if (activeContact.value.state !== 'Friend') throw new Error('联系人还不是 Friend')
    if (!selectedFile.value) throw new Error('请选择文件')
    if (selectedFile.value.size > MAX_FILE_BYTES) throw new Error(`文件过大：当前 Web 端最大 ${formatBytes(MAX_FILE_BYTES)}，暂不支持断点续传`)
    fileTransferPhase.value = '检查空间'
    await warnIfLowStorageForFile(selectedFile.value.size)
    fileTransferPhase.value = '读取文件'
    rtcFileStatus.value = `正在读取文件：${selectedFile.value.name}`
    const bytes = await readFileWithProgress(selectedFile.value)
    if (bytes.length === 0) throw new Error('不能发送空文件')
    fileTransferPhase.value = '加密封装'
    fileProgressText.value = `读取完成 · ${formatBytes(bytes.length)}`
    rtcFileStatus.value = `正在生成加密文件包：${selectedFile.value.name}`
    filePackageText.value = create_file_package(
      backupText.value,
      passphrase.value,
      activeContact.value.contact_card_text,
      selectedFile.value.name,
      selectedFile.value.type || 'application/octet-stream',
      bytesToBase64(bytes),
      16 * 1024,
    )
    filePackageInfoText.value = JSON.stringify(JSON.parse(inspect_file_package(filePackageText.value)), null, 2)
    fileTransferPhase.value = '待发送'
    fileProgressText.value = `封装完成 · ${formatBytes(bytes.length)}`
    rtcFileStatus.value = '文件包已生成，可复制或 WebRTC 发送'
    appendLog(`已生成文件包：${selectedFile.value.name}`)
    ok = true
  })
  if (!ok) fileTransferPhase.value = '失败'
  return ok
}

function inspectIncomingFilePackage() {
  run('解析文件包', () => {
    const text = incomingFilePackageText.value.trim() || filePackageText.value.trim()
    if (!text) throw new Error('请粘贴文件包 JSON')
    ensureUiTextSize('文件包', text, MAX_RTC_TEXT_BYTES)
    const info = JSON.parse(inspect_file_package(text)) as { manifest?: { name?: string; mime_type?: string; size?: number } }
    filePackageInfoText.value = JSON.stringify(info, null, 2)
    const manifest = info.manifest
    if (manifest) {
      const packageBytes = new TextEncoder().encode(text).byteLength
      pendingFileMeta.value = `${manifest.name || '未命名文件'} · ${manifest.mime_type || 'application/octet-stream'} · ${formatBytes(manifest.size ?? 0)} · 加密包 ${formatBytes(packageBytes)}`
      fileProgressText.value = `待解密 · 加密包 ${formatBytes(packageBytes)}`
    }
  })
}

async function decryptIncomingFilePackage() {
  await runAsync('解密文件包', async () => {
    if (!activeContact.value) throw new Error('请选择发送者联系人')
    const text = pendingFilePackageText.value.trim() || incomingFilePackageText.value.trim() || filePackageText.value.trim()
    if (!text) throw new Error('请粘贴文件包 JSON')
    ensureUiTextSize('文件包', text, MAX_RTC_TEXT_BYTES)
    let manifestName = ''
    try {
      const info = JSON.parse(inspect_file_package(text)) as { manifest?: { name?: string } }
      manifestName = info.manifest?.name || ''
    } catch {
      // decrypt_file_package below will surface the authoritative parse/decrypt error.
    }
    if (manifestName && safetyPolicy.value.warnExecutableFiles && isDangerousFileName(manifestName)) {
      const confirmed = await showConfirm(
        '解密危险类型文件',
        `文件「${manifestName}」属于可执行/安装脚本等高风险类型。确认来源可信后再解密并生成下载链接。继续？`,
        true,
      )
      if (!confirmed) {
        fileTransferPhase.value = '待解密'
        rtcFileStatus.value = '已取消危险类型文件解密'
        appendLog('已取消危险类型文件解密')
        return
      }
    }
    const out = JSON.parse(decrypt_file_package(
      backupText.value,
      passphrase.value,
      activeContact.value.contact_card_text,
      text,
    )) as { name: string; mime_type: string; size?: number; bytes_base64: string }
    if (receivedFileUrl.value) URL.revokeObjectURL(receivedFileUrl.value)
    const bytes = base64ToBytes(out.bytes_base64)
    const blob = new Blob([new Uint8Array(bytes)], { type: out.mime_type || 'application/octet-stream' })
    receivedFileUrl.value = URL.createObjectURL(blob)
    receivedFileName.value = out.name
    receivedFileMime.value = out.mime_type || 'application/octet-stream'
    receivedFileMeta.value = `${receivedFileMime.value} · ${formatBytes(out.size ?? bytes.length)}`
    receivedFilePreviewKind.value = filePreviewKind(out.name, receivedFileMime.value)
    if (activeContact.value) {
      const msg: ChatMessage = {
        id: newId(),
        conversation_id: `conv-${activeContact.value.user_id}`,
        peer_user_id: activeContact.value.user_id,
        direction: 'in',
        text: `[文件] ${out.name} (${formatBytes(out.size ?? bytes.length)})`,
        envelope_json: text,
        status: 'received',
        created_at: Date.now(),
      }
      messages.value.push(msg)
      receivedFileMessageId.value = msg.id
    }
    pendingFilePackageText.value = ''
    pendingFileMeta.value = ''
    fileTransferPhase.value = '已接收'
    rtcFileStatus.value = `已解密文件：${out.name}`
    appendLog(`已解密文件：${out.name}`)
    persist()
  })
}

function markReceivedFileDownloaded() {
  if (!receivedFileName.value) return
  const downloadedAt = Date.now()
  const msg = messages.value.find((item) => item.id === receivedFileMessageId.value)
  if (msg) msg.file_downloaded_at = downloadedAt
  fileTransferPhase.value = '已下载'
  rtcFileStatus.value = `已触发下载：${receivedFileName.value}`
  fileProgressText.value = receivedFileMeta.value ? `下载文件 · ${receivedFileMeta.value}` : '下载文件'
  appendLog(`已触发文件下载：${receivedFileName.value}`)
  persist()
}

function sendFilePackageOverRtc() {
  run('发送文件包', () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    if (!filePackageText.value.trim()) throw new Error('请先生成文件包')
    const info = JSON.parse(inspect_file_package(filePackageText.value)) as { manifest: { name: string; size: number } }
    const msg: ChatMessage = {
      id: newId(),
      conversation_id: `conv-${activeContact.value.user_id}`,
      peer_user_id: activeContact.value.user_id,
      direction: 'out',
      text: `[文件] ${info.manifest.name} (${formatBytes(info.manifest.size)})`,
      envelope_json: filePackageText.value,
      target_device_ids: contactActiveDeviceIds(activeContact.value),
      status: 'queued',
      created_at: Date.now(),
    }
    fileTransferPhase.value = '投递中'
    const packageBytes = new TextEncoder().encode(filePackageText.value).byteLength
    fileProgressText.value = `投递加密包 ${formatBytes(packageBytes)}`
    if (dc && dc.readyState === 'open') {
      fileProgressText.value = `WebRTC 投递 ${formatBytes(packageBytes)}`
      sendRtcText(filePackageText.value, '文件包')
      msg.status = 'sent'
      fileTransferPhase.value = '已发送'
      rtcFileStatus.value = `已通过 WebRTC 发送：${info.manifest.name}`
      clearSelectedFileDraft(true)
    } else if (nodeEnabled.value) {
      rtcFileStatus.value = `正在通过 Mailbox 发送：${info.manifest.name}`
      fileProgressText.value = `Mailbox 投递 ${formatBytes(packageBytes)}`
      const payload = filePackageText.value
      const contact = activeContact.value
      void deliverPayloadToContact(contact, payload, '文件包', 'file-package')
        .then((result) => {
          msg.status = result === 'mailbox' ? 'mailbox' : result === 'sent' ? 'sent' : result === 'failed' ? 'failed' : 'queued'
          if (result === 'queued' || result === 'failed') queueOutboxItem(contact, payload, msg.id, 'file-package')
          fileTransferPhase.value = result === 'failed' ? '失败' : result === 'queued' ? '已入队' : '已发送'
          rtcFileStatus.value = result === 'mailbox' ? `已通过 Mailbox 发送：${info.manifest.name}` : `文件投递状态：${result}`
          fileProgressText.value = result === 'failed'
            ? `投递失败 · ${formatBytes(packageBytes)}`
            : result === 'queued'
              ? `已入队 · ${formatBytes(packageBytes)}`
              : `投递完成 · ${formatBytes(packageBytes)}`
          if (result !== 'failed') clearSelectedFileDraft(true)
          persist()
        })
    } else {
      queueOutboxItem(activeContact.value, filePackageText.value, msg.id, 'file-package')
      fileTransferPhase.value = '已入队'
      rtcFileStatus.value = `文件已加入 outbox：${info.manifest.name}`
      fileProgressText.value = `已入队 · ${formatBytes(packageBytes)}`
      clearSelectedFileDraft(true)
    }
    messages.value.push(msg)
    persist()
  })
}

async function sendSelectedFile() {
  if (activeContact.value && !(await confirmStrictE2eeSendRiskIfNeeded(activeContact.value))) {
    appendLog('已取消发送文件：严格 E2EE 风险未确认')
    return
  }
  if (await createFilePackageForActive()) sendFilePackageOverRtc()
}

async function copySignal(value: string) {
  await copyText(value, 'Signal')
}

function formatTime(ts: number) {
  return new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false }).format(new Date(ts))
}

function formatDateTime(ts: number): string {
  return new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(new Date(ts))
}

function goChatHome() {
  activePeerId.value = ''
  activeGroupId.value = ''
  void router.push('/chat')
}

function goChatPage() {
  void router.push('/chat')
}

function goContactsPage() {
  void router.push('/contacts')
}

function goSettingsPage() {
  void router.push('/me')
}


function goDiagnosticsPage() {
  void router.push('/diagnostics')
}

function logout() {
  loggedIn.value = false
  identity.value = null
  passphrase.value = ''
  resetAccountScopedState()
  void router.push('/login')
}
const appContext = {
  goChatPage, goChatHome, goContactsPage, goSettingsPage, goDiagnosticsPage, logout, log, identity, displayName, localIdentities, selectedLocalIdentityId, lastRegisteredIdentity, loginSelectedIdentity, importIdentityOnly, refreshMyContactCard, reencryptCurrentIdentityBackup, myContactCardText, backupText, newIdentityPassphrase,
  clearBrowserCaches, refreshStorageEstimate, storageEstimateText, webVersionText,
  nodeControlUrl, nodeUrlList, nodeEntrySummaries, nodeSettingsSummaryText, nodeTokenStorageText, nodeTokenCount, nodeMissingRemoteTokenCount, syncTriggerPolicyText, syncFailureSummaryText, syncRecoveryStatusText, syncRecoveryHistory, exportSyncRecoveryHistory, clearSyncRecoveryHistory, recoverSyncFailures, syncNow, toggleNodeEnabled, nodeEnabled, saveNetworkSettings, autoPublishPreKeyIfEnabled, autoMailboxTake, autoReadReceipts,
  runtimeStatusText, inAppRuntimePolicyText, refreshRuntimeStatus,
  autoPublishPreKey, autoNodeSync, autoSelfMailboxSync, nodeControlStatus, nodeHealthSummaryText, nodeStateDbSecurityText, nodeStateDbSecurityLevel, nodeStateFileSecurityText, nodeStateFileSecurityLevel, nodePeerHealthStatusText, nodePeerHealthRiskLevel, nodePeerHealthPeers, resetDhtPeerHealth, secureSessionOfferText, secureSessionResponseText, incomingSecureSessionText,
  secureSessionStatusText, createSecureSessionOfferText, applySecureSessionOfferText, applySecureSessionResponseText, recreateActiveRatchetSession, retrySecureSessionForActiveContact, clearActiveSecureSessionError, clearSecureSessionRawText, createMyDeviceCert, fanoutMyContactCardUpdateToFriends, fanoutDeviceRevokeToFriends, myDeviceCertJson, myDeviceBackupText,
  myDeviceId, revokeDeviceId, revokeReason, createDeviceRevokeText, deviceRevokeText, dataBackupText,
  exportFullDataBackup, pushFullDataBackupToOwnMailbox, pushSelfSyncPackageToOwnMailbox, selfSyncStatusText, processedSelfSyncIds, processedSelfSyncRequestIds, selfSyncMissingRequestRecords, selfSyncRequestSentCount, selfSyncRequestHitCount, selfSyncRequestMissCount, selfSyncRecentPackages, resendLatestSelfSyncPackageToOwnMailbox, clearSelfSyncRecentPackages, lastSelfSyncPushedAt, lastSelfSyncMergedAt, lastSelfSyncSequenceSent, lastSelfSyncSequenceMerged, selfSyncGapCount, lastSelfSyncGapAt, lastSelfSyncMissingPreviousId, lastSelfSyncReceiptStatesSent, lastSelfSyncReceiptStatesMerged, totalSelfSyncReceiptStatesMerged, lastSelfSyncOutboxSummary, clearSelfSyncGapStats, repairSelfSyncGapNow, importFullDataBackup, importFullDataBackupMerge, mergeSelfMailboxBackupNow, downloadText, lastFullDataBackupAt, lastSelfMailboxBackupPushedAt, lastSelfMailboxBackupReceivedAt, lastSelfMailboxBackupMergedAt, selfMailboxBackupStatusText, selfMailboxBackupMergePending, selfMailboxBackupMergeStatusText, fullDataBackupFreshnessText, fullDataBackupFreshnessLevel, addContactText, addContact, incomingFriendRequestText,
  addIncomingFriendRequest, friendRequests, visibleFriendRequests, quarantinedFriendRequests, friendRequestRateRecords, friendRequestRateSummaryText, clearFriendRequestRateRecords, acceptInboxRequest, rejectInboxRequest, rejectAllInboxRequests, blockAllInboxRequests,
  restoreQuarantinedFriendRequest, restoreAllQuarantinedFriendRequests, clearQuarantinedFriendRequests, incomingGroupInviteText, addIncomingGroupInvite,
  groupInvites, acceptGroupInvite, ignoreGroupInvite, contacts, activePeerId, selectContact,
  newGroupName, friendContacts, selectedGroupMembers, createGroup, groups, activeGroupId,
  selectGroup, activeContact, activeGroup, activeRatchetSession, activeRatchetStatusText, activeContactSealedSlotStatusText, activeContactSealedSlotRiskLevel, activeStrictE2eeSendRiskText, activeSecureSessionOutboxCount, activeGroupMembers, activeGroupWarningText, activeGroupStrictE2eeRiskText, groupStrictE2eeRiskTextFor, createGroupStrictE2eeRiskText, groupInviteStrictE2eeRiskText, blockReason, blockActiveContact, readReceiptsEnabledFor, setActiveContactReadReceipts,
  unblockActiveContact, removeActiveContact, clearActiveConversation, createFriendRequestForActive, clearActiveFriendRequestError, createInviteForActiveGroup, groupInviteText, groupFanoutJson,
  removeActiveGroup, leaveActiveGroupWithNotice, messages, activeMessages, formatTime, formatDateTime, statusLabel, copyMessageEnvelope, perDeviceEnvelopeTargetCount, composerText,
  sendMessage, incomingDeviceRevokeText, applyDeviceRevokeToActiveContact, rtcStatus, createRtcOfferForActive, acceptRtcOfferForActive,
  applyRtcAnswerForActive, resetRtc, localSignalText, copySignal, remoteSignalText, outbox,
  flushOutboxForActive, retryAllOutbox, cancelOutboxForActive, clearSentOutbox, friendRequestText, createFriendRequestForActiveLocalOnly, incomingFriendResponseText, applyFriendResponse, inboundEnvelopeText,
  receiveEnvelope, onFileSelected, cancelSelectedFile, selectedFile, formatBytes, isDangerousFileName, createFilePackageForActive, sendFilePackageOverRtc, sendSelectedFile, filePackageText, rtcFileStatus, fileTransferPhase, fileProgressText,
  incomingFilePackageText, pendingFilePackageText, pendingFileMeta, inspectIncomingFilePackage, decryptIncomingFilePackage, markReceivedFileDownloaded, receivedFileUrl, receivedFileName, receivedFileMeta, receivedFileMime, receivedFilePreviewKind, filePackageInfoText,
  createGroupSenderKeyForActiveGroup, groupSenderDistributionText, importGroupSenderKeyForActiveContact, groupSenderEncryptDebug, groupSenderDecryptDebug, createGroupSenderDistributionFanoutForActiveGroup,
  groupSenderDistributionFanoutJson, groupSenderDistributionFanoutItems, groupSenderEnvelopeText, groupSenderPlainText, groupRenameText, createRenameGroupEvent,
  groupEventText, applyGroupEventText, createGroupEventFanout, groupEventFanoutJson, groupEventFanoutItems, incomingGroupEventText, clearActiveGroupEventError,
  groupEventActorUserId, createAddMemberGroupEvent, createRemoveMemberGroupEvent, createPromoteAdminGroupEvent, createDemoteAdminGroupEvent, fanoutItems,
  prekeySignedId, prekeyOneTimeCount, prekeyBundleText, prekeyPrivateBundleJson, prekeySignedOneTimeRecordTexts, prekeyInfoText, x3dhInitialMessageJson,
  selectedOneTimePreKeyId, selectedSignedOneTimePreKeyRecordText, x3dhSharedSecretText, ratchetStateText, ratchetPeerStateText, ratchetLocalDhKeyPairJson, ratchetRemoteDhPublicKeyForInit,
  ratchetInitRole, ratchetHeaderText, ratchetEnvelopeText, ratchetPlainText, ratchetKeyText, ratchetRemoteDhPublicKey,
  ratchetInfoText, safetyPolicy, enableStrictE2eePolicy, strictE2eePolicyEnabled, strictE2eeReadiness, strictE2eeReadinessIssues, openStrictE2eeReadinessIssue, repairStrictE2eeBlockers, contactRevokedDeviceCount, contactKnownRevokedDeviceCount, contactActiveDeviceIds, contactRevokedDeviceIds, contactRevokedDeviceDetails, unmarkActiveContactRevokedDevice, contactAllKnownDevicesRevoked, verifiedFriendContactCount, unverifiedFriendContactCount, unverifiedIncomingDropCount, clearUnverifiedIncomingDropStats, lastUnverifiedIncomingDropAt, lastUnverifiedIncomingDropFrom, revokedDeviceIncomingDropCount, clearRevokedDeviceIncomingDropStats, lastRevokedDeviceIncomingDropAt, lastRevokedDeviceIncomingDropFrom, perDeviceEnvelopeSentCount, perDeviceEnvelopeReceivedCount, perDeviceEnvelopeDropCount, lastPerDeviceEnvelopeAt, lastPerDeviceEnvelopeDropAt, lastPerDeviceEnvelopeDropReason, contactCardUpdateFanoutCount, contactCardUpdateFanoutSkipCount, lastContactCardUpdateFanoutAt, contactCardUpdateFanoutRecords, contactCardUpdateFanoutAckCount, contactCardUpdatePendingAckCount, contactCardUpdateStaleAckCount, retryStaleContactCardUpdateAcks, contactCardUpdateAckStatusFor, contactStrictE2eeStatusText, contactStrictE2eeRiskLevel, contactCardDhtDiscoveryIsStale, contactCardDhtAutoRefreshCount, lastContactCardDhtAutoRefreshAt, lastContactCardDhtAutoRefreshError, contactCardDhtAutoRefreshHistory, sealedSlotCoverageSummary, sealedSlotRiskContacts, peerAddressesText, peerMailboxKey, peerAnnounceText, peerAnnounceInspectPublicKey,
  peerAnnounceInfoText, publicPeerId, publicPeerAddressesText, publicPeerCapabilities, publicPeerAnnounceText, publicPeerAnnounceInspectPublicKey,
  publicPeerAnnounceInfoText, mailboxKind, mailboxCiphertext, mailboxMessageText, mailboxMessageInspectPublicKey, mailboxMessageInfoText,
  nodeClosestTarget, nodeDhtFindValueKey, nodeDhtKeyKind, nodeDhtKeyValue, nodeDhtFindValueStatusText, nodeDhtOperationHistory, nodeDhtOperationHistoryImportText, nodeDhtOperationHistoryImportStatus, exportDhtOperationHistory, copyDhtOperationHistory, importDhtOperationHistory, clearDhtOperationHistory, fillMyPreKeyDhtKeyInput, fillMyMailboxHintDhtKeyInput, fillMyContactCardDhtKeyInput, findActiveContactMailboxHint, findActiveContactContactCard, findActiveContactPreKey, discoverActiveContactDht, clearActiveContactDhtRisk, verifyActiveContactFingerprint, showActiveContactFingerprintQr, startFingerprintQrScan, stopFingerprintQrScan, fingerprintScanOpen, fingerprintScanStatus, copyActiveContactFingerprintProof, verifyActiveContactFingerprintFromText, activeFingerprintVerificationText, showMyFingerprintQr, copyMyFingerprintProof, fillCurrentPublicPeerDhtKeyInput, publishAndCheckMyPublicPeerDht, deriveDhtKeyForFindValue, deriveAndFindDhtValueNow, nodeClosestInfoText, nodeRoutingRefreshStatusText, nodeDhtReplicationStatusText, nodeDhtMaintenanceStatusText, runDhtFindValueNow, runDhtMaintenanceNow, runDhtRoutingRefreshNow, runDhtReplicationNow, discoveredMailboxHintUrl, addDiscoveredMailboxHintToSyncServices, nodeMailboxTakeUserId, nodeMailboxTakeInfoText, mailboxInboxStatus, mailboxQuotaStatusText, mailboxQuotaPressureLevel, mailboxInboxErrorText, mailboxFailureSummaryText, mailboxDedupeCount, mailboxFailedCount, mailboxDedupeStatusText, clearProcessedMailboxIds, retryFailedMailboxItems, clearFailedMailboxItems, nodePreKeyUserId, nodePreKeyStatusText,
  nodeSyncPeerUrl, nodeSyncSnapshotText, nodeSyncStatusText, lastNodeSnapshotSyncAt, nodeSnapshotSyncFreshnessText, nodeSnapshotSyncFreshnessLevel, prekeyStatusSummary, prekeyAutoStateText, prekeyAutoErrorText, createMyPreKeyBundleText, inspectPreKeyBundleText, retryPreKeyAutoPublish, publishAndCheckMyPreKeyDht, publishAndCheckMyMailboxHintDht, publishAndCheckMyContactCardDht, publishAndCheckAllMyDht, clearPreKeyRawState, copyText,
  showQr, createX3dhInitialMessageText, deriveX3dhResponderSecretText, createRatchetPairForActiveContact, createRatchetFromSharedSecretText, generateRatchetDhKeyPairText,
  createRatchetFromSharedSecretWithKeysText, inspectRatchetStateText, ratchetNextSendKeyText, ratchetNextRecvKeyText, ratchetEncryptEnvelopeText, ratchetDecryptEnvelopeText,
  ratchetDhStepText, saveSafetyPolicy, createPeerAnnounceText, inspectPeerAnnounceText, createPublicPeerAnnounceText, inspectPublicPeerAnnounceText,
  createMailboxMessageText, inspectMailboxMessageText, checkNodeHealth, submitPublicPeerToNode, pushMailboxToNode, queryNodeClosestPeers,
  takeMailboxFromNode, processMailboxTakeInfoText, publishPreKeyToNode, refreshPreKeyStatusFromNode, replenishPreKeyIfLow, fetchPreKeyFromNode, consumePreKeyFromNode, exportNodeSnapshot,
  importNodeSnapshot, pullSnapshotFromPeerNode,

}

</script>

<template>
  <LoginPage
    v-if="!ready || !loggedIn"
    :ready="ready"
    :logged-in="loggedIn"
    v-model:passphrase="passphrase"
    v-model:backup-text="backupText"
    v-model:display-name="displayName"
    :normalized="normalized"
    :log="log"
    :local-identities="localIdentities"
    :registered-identity="lastRegisteredIdentity"
    :mode="authMode"
    v-model:selected-identity-id="selectedLocalIdentityId"
    @create="createIdentityAndEnter"
    @login="loginSelectedIdentity"
    @import-identity="importIdentityOnly"
    @reset-register="resetRegisterForm"
    @remove-identity="removeLocalIdentity"
    @clear="clearPersisted"
  />

  <main v-else class="app-shell">
    <nav class="app-rail" aria-label="主导航">
      <button class="rail-avatar" title="我" aria-label="打开我的设置" :aria-current="currentPage === 'settings' || currentPage === 'diagnostics' ? 'page' : undefined" @click="goSettingsPage">
        {{ (displayName || identity?.user_id || '?').slice(0, 1).toUpperCase() }}
      </button>
      <button class="rail-item" :class="{ active: currentPage === 'chat' }" :aria-current="currentPage === 'chat' ? 'page' : undefined" aria-label="打开聊天" @click="goChatHome">
        <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 11.5a8.38 8.38 0 0 1-8.5 8.5 8.5 8.5 0 0 1-3.9-.9L3 21l1.9-5.6A8.5 8.5 0 0 1 12.5 3 8.38 8.38 0 0 1 21 11.5z"/></svg>
        <span>聊天</span>
      </button>
      <button class="rail-item" :class="{ active: currentPage === 'contacts' }" :aria-current="currentPage === 'contacts' ? 'page' : undefined" aria-label="打开通讯录" @click="goContactsPage">
        <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/></svg>
        <span>通讯录</span>
        <em v-if="friendRequests.length || groupInvites.length" class="rail-badge">{{ friendRequests.length + groupInvites.length }}</em>
      </button>
      <div class="rail-spacer"></div>
      <button class="rail-item" :class="{ active: currentPage === 'settings' || currentPage === 'diagnostics' }" :aria-current="currentPage === 'settings' || currentPage === 'diagnostics' ? 'page' : undefined" aria-label="打开我的设置" @click="goSettingsPage">
        <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
        <span>我</span>
      </button>
    </nav>

    <section class="app-content">
      <ChatPage v-if="currentPage === 'chat'" :ctx="appContext" />
      <ContactsPage v-else-if="currentPage === 'contacts'" :ctx="appContext" />
      <SettingsPage v-else-if="currentPage === 'settings'" :ctx="appContext" />
      <DiagnosticsPage v-else-if="currentPage === 'diagnostics'" :ctx="appContext" />
    </section>
  </main>


  <div class="toast-stack" aria-live="polite">
    <div v-for="item in toasts" :key="item.id" class="toast" :class="item.kind">{{ item.text }}</div>
  </div>

  <div v-if="alertDialog.open" class="dialog-mask" @click.self="closeAlert">
    <section class="dialog-card" :class="alertDialog.kind" role="alertdialog" aria-modal="true" aria-labelledby="alert-dialog-title">
      <h2 id="alert-dialog-title">{{ alertDialog.title }}</h2>
      <p>{{ alertDialog.message }}</p>
      <div class="row compact dialog-actions">
        <button @click="closeAlert">知道了</button>
      </div>
    </section>
  </div>

  <div v-if="confirmDialog.open" class="dialog-mask" @click.self="closeConfirm(false)">
    <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="confirm-dialog-title">
      <h2 id="confirm-dialog-title">{{ confirmDialog.title }}</h2>
      <p>{{ confirmDialog.message }}</p>
      <div class="row compact dialog-actions">
        <button class="secondary" @click="closeConfirm(false)">取消</button>
        <button :class="{ danger: confirmDialog.danger }" @click="closeConfirm(true)">确定</button>
      </div>
    </section>
  </div>

  <div v-if="fingerprintScanOpen" class="qr-mask" @click.self="stopFingerprintQrScan">
    <section class="qr-modal" role="dialog" aria-modal="true" aria-labelledby="fingerprint-scan-title">
      <header>
        <h2 id="fingerprint-scan-title">扫码核验联系人指纹</h2>
        <button class="danger" @click="stopFingerprintQrScan">关闭</button>
      </header>
      <video ref="fingerprintScanVideo" playsinline muted style="width:100%;border-radius:12px;border:1px solid var(--c-line);background:#000"></video>
      <small>{{ fingerprintScanStatus }}</small>
      <small>如果浏览器不支持摄像头扫码，请复制/粘贴 lm-contact-fingerprint-v1 核验码。</small>
    </section>
  </div>

  <div v-if="qrDataUrl" class="qr-mask" @click.self="closeQr">
    <section class="qr-modal" role="dialog" aria-modal="true" aria-labelledby="qr-dialog-title">
      <header>
        <h2 id="qr-dialog-title">{{ qrTitle }}</h2>
        <button class="danger" @click="closeQr">关闭</button>
      </header>
      <img :src="qrDataUrl" alt="二维码" />
      <small>内容长度：{{ qrRawText.length }} 字符。过长内容可能不适合二维码扫描。</small>
      <div class="row">
        <button @click="copyText(qrRawText, qrTitle)">复制原文</button>
      </div>
    </section>
  </div>

</template>
