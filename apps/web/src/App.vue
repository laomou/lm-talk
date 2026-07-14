<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
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

type DeviceOutput = {
  device_id: string
  device_public_key: string
  device_cert_json: string
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
  revoked_device_ids?: string[]
  device_certs?: DeviceCertItem[]
  block_reason?: string
}

type FilterLevel = 'Off' | 'Relaxed' | 'Standard' | 'Strict'
type FilterAction = 'Allow' | 'Warn' | 'Blur' | 'Hide' | 'Drop'
type SafetyPolicy = {
  enableTextFilter: boolean
  textFilterLevel: FilterLevel
  warnExternalLinks: boolean
  warnExecutableFiles: boolean
  dropFilteredIncoming: boolean
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
}

type GroupItem = {
  group_id: string
  name: string
  member_user_ids: string[]
  admin_user_ids?: string[]
  policy_state_json?: string
  created_at: number
  sequence?: number
}

type GroupSenderKeyItem = {
  key_id: string
  group_id: string
  sender_user_id: string
  state_json: string
  distribution_text?: string
  updated_at: number
}

type MessageStatus = 'queued' | 'sent' | 'mailbox' | 'delivered' | 'copied' | 'received' | 'failed'

type ChatMessage = {
  id: string
  conversation_id: string
  peer_user_id: string
  group_id?: string
  direction: 'out' | 'in'
  text: string
  envelope_json?: string
  protocol_message_id?: string
  status: MessageStatus
  created_at: number
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
  kind?: 'direct-envelope' | 'group-fanout' | 'file-package' | 'other'
  status: 'queued' | 'sent' | 'failed'
  created_at: number
  retry_count: number
  next_retry_at?: number
  expires_at?: number
  last_error?: string
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
  myDeviceId?: string
  prekeyBundleText?: string
  prekeyPrivateBundleJson?: string | EncryptedStringV1
  prekeySignedOneTimeRecordTexts?: string[]
  safetyPolicy?: SafetyPolicy
  nodeControlUrl?: string
  nodeEnabled?: boolean
  autoMailboxTake?: boolean
  autoPublishPreKey?: boolean
  autoNodeSync?: boolean
  processedMailboxIds?: string[]
}

type PersistedMeta = {
  backupText: string
  myContactCardText: string
  myDeviceCertJson?: string
  myDeviceId?: string
  prekeyBundleText?: string
  prekeyPrivateBundleJson?: string | EncryptedStringV1
  prekeySignedOneTimeRecordTexts?: string[]
  safetyPolicy?: SafetyPolicy
  nodeControlUrl?: string
  nodeEnabled?: boolean
  autoMailboxTake?: boolean
  autoPublishPreKey?: boolean
  autoNodeSync?: boolean
  processedMailboxIds?: string[]
  schemaVersion: number
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
const MAX_SIGNAL_BYTES = 256 * 1024
const MAX_FILE_BYTES = 16 * 1024 * 1024
const MAX_RTC_TEXT_BYTES = MAX_FILE_BYTES * 3
const MAX_OUTBOX_RETRY_COUNT = 5
const GROUP_EVENT_PAYLOAD_PREFIX = 'lm-group-event-message-v1:'
const GROUP_SENDER_KEY_PAYLOAD_PREFIX = 'lm-group-sender-key-message-v1:'

const passphrase = ref('')
const backupText = ref('')
const identity = ref<(IdentityOutput | RestoreOutput) | null>(null)
const displayName = ref('Me')
const localIdentities = ref<LocalIdentityRecord[]>([])
const selectedLocalIdentityId = ref('')
const lastRegisteredIdentity = ref<LocalIdentityRecord | null>(null)
const myContactCardText = ref('')
const myDeviceCertJson = ref('')
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
})

const contacts = ref<ContactItem[]>([])
const friendRequests = ref<FriendRequestItem[]>([])
const groups = ref<GroupItem[]>([])
const groupInvites = ref<GroupInviteItem[]>([])
const groupSenderKeys = ref<GroupSenderKeyItem[]>([])
const messages = ref<ChatMessage[]>([])
const outbox = ref<OutboxItem[]>([])
const ratchetSessions = ref<RatchetSessionItem[]>([])
const processedMailboxIds = ref<string[]>([])
let outboxRetryTimer: number | undefined
let lastDeliveryError = ''
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
const autoPublishPreKey = ref(true)
const autoNodeSync = ref(false)
let nodeSyncTimer: number | undefined
const nodeControlStatus = ref('未连接')
const nodeClosestTarget = ref('')
const nodeClosestInfoText = ref('')
const nodeMailboxTakeUserId = ref('')
const nodeMailboxTakeInfoText = ref('')
const mailboxInboxStatus = ref('尚未同步')
const nodePreKeyUserId = ref('')
const nodePreKeyStatusText = ref('')
const nodeSyncPeerUrl = ref('http://127.0.0.1:8788')
const nodeSyncSnapshotText = ref('')
const nodeSyncStatusText = ref('')
const selectedFile = ref<File | null>(null)
const filePackageText = ref('')
const incomingFilePackageText = ref('')
const filePackageInfoText = ref('')
const receivedFileName = ref('')
const receivedFileUrl = ref('')
const rtcFileStatus = ref('未发送文件')
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
const activeMessages = computed(() => activeGroup.value
  ? messages.value.filter((m) => m.group_id === activeGroup.value?.group_id)
  : messages.value.filter((m) => m.peer_user_id === activePeerId.value)
)
const friendContacts = computed(() => contacts.value.filter((c) => c.state === 'Friend'))
const activeGroupMembers = computed(() => activeGroup.value
  ? activeGroup.value.member_user_ids.map((id) => contacts.value.find((c) => c.user_id === id)).filter(Boolean) as ContactItem[]
  : []
)
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
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible' && loggedIn.value && nodeEnabled.value && autoMailboxTake.value) {
        void takeMailboxFromNode()
      }
    })
    if ('serviceWorker' in navigator) {
      const swUrl = new URL('sw.js', import.meta.env.BASE_URL ? new URL(import.meta.env.BASE_URL, window.location.origin) : window.location.origin)
      void navigator.serviceWorker.register(swUrl.pathname).catch((e) => appendLog(`PWA Service Worker 注册失败：${String(e)}`))
    }
    ready.value = true
  } catch (e) {
    appendLog(`WASM 初始化失败：${String(e)}`)
  }
})

function appendLog(line: string) {
  log.value = [`${new Date().toLocaleTimeString()} ${line}`, ...log.value].slice(0, 50)
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
  if (size > max) throw new Error(`${label} 过大：${size} bytes > ${max} bytes`)
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
  groups.value = await Promise.all(groups.value.map((g: any) => decryptGroupFromStore(g, key)))
  messages.value = await Promise.all(messages.value.map((m: any) => decryptMessageFromStore(m, key)))
  outbox.value = await Promise.all(outbox.value.map((o: any) => decryptOutboxFromStore(o, key)))
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
    myDeviceId: myDeviceId.value,
    prekeyBundleText: prekeyBundleText.value,
    prekeyPrivateBundleJson: prekeyPrivateBundleJson.value,
    prekeySignedOneTimeRecordTexts: prekeySignedOneTimeRecordTexts.value,
    safetyPolicy: safetyPolicy.value,
    nodeControlUrl: nodeControlUrl.value,
    nodeEnabled: nodeEnabled.value,
    autoMailboxTake: autoMailboxTake.value,
    autoPublishPreKey: autoPublishPreKey.value,
    autoNodeSync: autoNodeSync.value,
    processedMailboxIds: processedMailboxIds.value,
  }
}

function persistedMeta(): PersistedMeta {
  return {
    backupText: backupText.value,
    myContactCardText: myContactCardText.value,
    myDeviceCertJson: myDeviceCertJson.value,
    myDeviceId: myDeviceId.value,
    prekeyBundleText: prekeyBundleText.value,
    prekeyPrivateBundleJson: prekeyPrivateBundleJson.value,
    prekeySignedOneTimeRecordTexts: prekeySignedOneTimeRecordTexts.value,
    safetyPolicy: safetyPolicy.value,
    nodeControlUrl: nodeControlUrl.value,
    nodeEnabled: nodeEnabled.value,
    autoMailboxTake: autoMailboxTake.value,
    autoPublishPreKey: autoPublishPreKey.value,
    autoNodeSync: autoNodeSync.value,
    processedMailboxIds: processedMailboxIds.value,
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

async function persistStateTables() {
  if (!identity.value) return
  const key = await localStorageCryptoKey()
  const storedContacts = await Promise.all(contacts.value.map((c) => encryptContactForStore(c, key)))
  const storedGroups = await Promise.all(groups.value.map((g) => encryptGroupForStore(g, key)))
  const storedMessages = await Promise.all(messages.value.map((m) => encryptMessageForStore(m, key)))
  const storedOutbox = await Promise.all(outbox.value.map((o) => encryptOutboxForStore(o, key)))
  const storedRatchets = await Promise.all(ratchetSessions.value.map((r) => encryptRatchetForStore(r, key)))
  const meta = persistedMeta()
  meta.prekeyPrivateBundleJson = await encryptLocalString(prekeyPrivateBundleJson.value, key)
  const prefix = ownerPrefix()
  await idbTableReplaceByPrefix(TABLES.meta, prefix, [[ownerKey('main'), meta]])
  await idbTableReplaceByPrefix(TABLES.contacts, prefix, storedContacts.map((c) => [ownerKey(c.user_id), c]))
  await idbTableReplaceByPrefix(TABLES.friendRequests, prefix, friendRequests.value.map((r) => [ownerKey(r.request_id), r]))
  await idbTableReplaceByPrefix(TABLES.groups, prefix, storedGroups.map((g) => [ownerKey(g.group_id), g]))
  await idbTableReplaceByPrefix(TABLES.groupInvites, prefix, groupInvites.value.map((g) => [ownerKey(g.invite_id), g]))
  await idbTableReplaceByPrefix(TABLES.groupSenderKeys, prefix, groupSenderKeys.value.map((k) => [ownerKey(k.key_id), k]))
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
}

async function writeStateToTables(state: PersistedState) {
  backupText.value = state.backupText ?? ''
  const key = await localStorageCryptoKey()
  contacts.value = await Promise.all((state.contacts ?? []).map((c: any) => decryptContactFromStore(c, key)))
  friendRequests.value = state.friendRequests ?? []
  groups.value = await Promise.all((state.groups ?? []).map((g: any) => decryptGroupFromStore(g, key)))
  groupInvites.value = state.groupInvites ?? []
  groupSenderKeys.value = state.groupSenderKeys ?? []
  messages.value = await Promise.all((state.messages ?? []).map((m: any) => decryptMessageFromStore(m, key)))
  outbox.value = await Promise.all((state.outbox ?? []).map((o: any) => decryptOutboxFromStore(o, key)))
  ratchetSessions.value = await Promise.all((state.ratchetSessions ?? []).map((r: any) => decryptRatchetFromStore(r, key)))
  myContactCardText.value = state.myContactCardText ?? ''
  myDeviceCertJson.value = state.myDeviceCertJson ?? ''
  myDeviceId.value = state.myDeviceId ?? ''
  prekeyBundleText.value = state.prekeyBundleText ?? ''
  prekeyPrivateBundleJson.value = state.prekeyPrivateBundleJson ? await decryptLocalString(state.prekeyPrivateBundleJson, key) : ''
  prekeySignedOneTimeRecordTexts.value = state.prekeySignedOneTimeRecordTexts ?? []
  safetyPolicy.value = { ...safetyPolicy.value, ...(state.safetyPolicy ?? {}) }
  nodeControlUrl.value = state.nodeControlUrl ?? nodeControlUrl.value
  nodeEnabled.value = state.nodeEnabled ?? false
  autoMailboxTake.value = state.autoMailboxTake ?? true
  autoPublishPreKey.value = state.autoPublishPreKey ?? true
  autoNodeSync.value = state.autoNodeSync ?? false
  processedMailboxIds.value = state.processedMailboxIds ?? []
  await persistStateTables()
}

async function loadStateFromTables(): Promise<boolean> {
  if (!identity.value) return false
  const meta = await idbTableGet<PersistedMeta>(TABLES.meta, ownerKey('main'))
  if (!meta) return false
  backupText.value = meta.backupText ?? ''
  myContactCardText.value = meta.myContactCardText ?? ''
  myDeviceCertJson.value = meta.myDeviceCertJson ?? ''
  myDeviceId.value = meta.myDeviceId ?? ''
  const key = await localStorageCryptoKey()
  prekeyBundleText.value = meta.prekeyBundleText ?? ''
  prekeyPrivateBundleJson.value = meta.prekeyPrivateBundleJson ? await decryptLocalString(meta.prekeyPrivateBundleJson, key) : ''
  prekeySignedOneTimeRecordTexts.value = meta.prekeySignedOneTimeRecordTexts ?? []
  safetyPolicy.value = { ...safetyPolicy.value, ...(meta.safetyPolicy ?? {}) }
  nodeControlUrl.value = meta.nodeControlUrl ?? nodeControlUrl.value
  nodeEnabled.value = meta.nodeEnabled ?? false
  autoMailboxTake.value = meta.autoMailboxTake ?? true
  autoPublishPreKey.value = meta.autoPublishPreKey ?? true
  autoNodeSync.value = meta.autoNodeSync ?? false
  processedMailboxIds.value = meta.processedMailboxIds ?? []
  const prefix = ownerPrefix()
  contacts.value = await Promise.all((await idbTableGetAllByPrefix<any>(TABLES.contacts, prefix)).map((c: any) => decryptContactFromStore(c, key)))
  friendRequests.value = await idbTableGetAllByPrefix<FriendRequestItem>(TABLES.friendRequests, prefix)
  groups.value = await Promise.all((await idbTableGetAllByPrefix<any>(TABLES.groups, prefix)).map((g: any) => decryptGroupFromStore(g, key)))
  groupInvites.value = await idbTableGetAllByPrefix<GroupInviteItem>(TABLES.groupInvites, prefix)
  groupSenderKeys.value = await idbTableGetAllByPrefix<GroupSenderKeyItem>(TABLES.groupSenderKeys, prefix)
  messages.value = await Promise.all((await idbTableGetAllByPrefix<any>(TABLES.messages, prefix)).map((m: any) => decryptMessageFromStore(m, key)))
  outbox.value = await Promise.all((await idbTableGetAllByPrefix<any>(TABLES.outbox, prefix)).map((o: any) => decryptOutboxFromStore(o, key)))
  ratchetSessions.value = await Promise.all((await idbTableGetAllByPrefix<any>(TABLES.ratchetSessions, prefix)).map((r: any) => decryptRatchetFromStore(r, key)))
  if (backupText.value && myContactCardText.value) {
    try {
      const info = safeJson<ContactInfo>(inspect_contact_card(myContactCardText.value))
      rememberLocalIdentity(info.user_id, info.display_name || displayName.value, backupText.value)
    } catch { /* ignore old/incomplete local identity */ }
  }
  return true
}

async function loadPersistedState() {
  try {
    if (await loadStateFromTables()) return

    let state = await idbGet<PersistedState>('chat-state-v1')

    // One-time migration from the old localStorage demo state.
    if (!state) {
      const raw = localStorage.getItem('lm-talk-chat-state-v1')
      if (raw) {
        state = JSON.parse(raw) as PersistedState
        localStorage.removeItem('lm-talk-chat-state-v1')
        appendLog('✅ 已从 localStorage 迁移到 IndexedDB 分表')
      }
    } else {
      appendLog('✅ 已从旧 IndexedDB 单对象迁移到分表')
    }

    if (!state) return
    await writeStateToTables(state)
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
  activePeerId.value = ''
  activeGroupId.value = ''
  myContactCardText.value = ''
  myDeviceCertJson.value = ''
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
  }
  nodeEnabled.value = false
  autoMailboxTake.value = true
  autoPublishPreKey.value = true
  autoNodeSync.value = false
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
  myContactCardText.value = ''
  myDeviceCertJson.value = ''
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
  }
  nodeEnabled.value = false
  autoMailboxTake.value = true
  autoPublishPreKey.value = true
  autoNodeSync.value = false
  nodeControlStatus.value = '未连接'
  loggedIn.value = false
  appendLog('已清空本地状态')
}


function exportFullDataBackup() {
  run('导出完整数据备份', () => {
    if (!backupText.value || !passphrase.value) throw new Error('需要当前身份备份包和提示词')
    dataBackupText.value = export_data_backup(
      backupText.value,
      passphrase.value,
      JSON.stringify(currentPersistedState()),
    )
  })
}

async function importFullDataBackup() {
  try {
    if (!backupText.value || !passphrase.value) throw new Error('需要当前身份备份包和提示词')
    if (!dataBackupText.value.trim()) throw new Error('请粘贴完整数据备份')
    const json = import_data_backup(backupText.value, passphrase.value, dataBackupText.value)
    const state = JSON.parse(json) as PersistedState
    await writeStateToTables(state)
    appendLog('✅ 已导入完整数据备份')
  } catch (e) {
    appendLog(`❌ 导入完整数据备份失败：${String(e)}`)
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


async function afterLoginAutomation() {
  if (!nodeEnabled.value) return
  if (autoPublishPreKey.value) await publishPreKeyToNode()
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
    if (autoPublishPreKey.value) await publishPreKeyToNode()
    await takeMailboxFromNode()
    appendLog('✅ 消息同步完成')
  } catch (e) {
    appendLog(`❌ 消息同步失败：${userFacingError(e)}`)
    throw e
  }
}

type NodeEntry = { url: string; token: string }
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
    .map((x) => x.trim())
    .filter(Boolean)
    .map((raw) => {
      // 每行：<url> 或 <url>|<令牌>（令牌对应节点的 --control-token）
      const [url, token] = raw.split('|').map((s) => s.trim())
      return { url, token: token || '' }
    })
    .filter((e) => e.url)
}
function nodeEntryLine(e: NodeEntry): string { return e.token ? `${e.url}|${e.token}` : e.url }
function nodeUrlList(): string[] {
  return nodeEntries().map((e) => e.url)
}
function nodeTokenFor(url: string): string {
  const base = url.replace(/\/$/, '')
  return nodeEntries().find((e) => e.url.replace(/\/$/, '') === base)?.token || ''
}

function primaryNodeUrl(): string {
  return nodeUrlList()[0] ?? ''
}

function saveNetworkSettings() {
  const entries = nodeEntries()
  if (entries.length > 0) nodeControlUrl.value = entries.map(nodeEntryLine).join('\n')
  persist()
  appendLog(`✅ 已保存消息同步设置：${nodeEnabled.value ? '启用' : '停用'} ${entries.length ? `${entries.length} 个节点` : '未填写节点'}`)
}

function toggleNodeEnabled() {
  nodeEnabled.value = !nodeEnabled.value
  saveNetworkSettings()
  if (nodeEnabled.value) void checkNodeHealth()
}

async function autoPublishPreKeyIfEnabled() {
  if (!nodeEnabled.value || !autoPublishPreKey.value || !loggedIn.value) return
  await publishPreKeyToNode()
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
  const body = await nodeFetchJson('/mailbox/push', {
    method: 'POST',
    body: JSON.stringify({
      message_text: msg,
      from_identity_public_key: identity.value?.identity_public_key,
    }),
  })
  nodeControlStatus.value = JSON.stringify(body, null, 2)
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

function mailboxKindForOutboxKind(kind: OutboxItem['kind']): string {
  if (kind === 'group-fanout') return 'group-fanout'
  if (kind === 'file-package') return 'other'
  if (kind === 'other') return 'other'
  return 'direct-envelope'
}

function messageProtocolIdFromEnvelope(envelope: string): string | undefined {
  try {
    const parsed = JSON.parse(envelope) as { message_id?: string }
    return typeof parsed.message_id === 'string' ? parsed.message_id : undefined
  } catch { return undefined }
}

function createDeliveryAckPayload(messageId: string): string {
  return JSON.stringify({
    type: 'lm-delivery-ack-v1',
    version: 1,
    message_id: messageId,
    from_user_id: identity.value?.user_id,
    created_at: Date.now(),
  })
}

function applyDeliveryAck(messageId: string, fromUserId: string) {
  const msg = messages.value.find((m) => m.direction === 'out' && m.protocol_message_id === messageId && m.peer_user_id === fromUserId)
  if (msg) {
    msg.status = 'delivered'
    appendLog(`✅ 收到送达回执：${fromUserId}`)
    persist()
  }
}

async function sendDeliveryAck(sender: ContactItem, messageId?: string) {
  if (!messageId || !identity.value) return
  const ack = createDeliveryAckPayload(messageId)
  const result = await deliverPayloadToContact(sender, ack, '送达回执', 'other')
  if (result === 'queued' || result === 'failed') outbox.value.push(createOutboxItem(sender, ack, undefined, 'other'))
  persist()
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
    if (dc && dc.readyState === 'open' && activePeerId.value === contact.user_id && kind !== 'group-fanout') {
      sendRtcText(payload, label)
      return 'sent'
    }
    if (nodeEnabled.value) {
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
    const deliveryId = await pushMailboxPayload(contact, 'direct-envelope', envelope)
    msg.status = 'mailbox'
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
  if (!nodeEnabled.value || ratchetSessionFor(contact.user_id)) return Boolean(ratchetSessionFor(contact.user_id))
  const body = await nodeFetchJson(`/prekey/get?user_id=${encodeURIComponent(contact.user_id)}&consume=true`)
  nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
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
  await pushMailboxPayload(contact, 'other', secureSessionResponseText.value)
  appendLog(`✅ 已从节点拉取 PreKey 并为 ${contact.display_name || contact.user_id} 建立会话初始化消息`)
  persist()
  return true
}



async function removeLocalIdentity(id: string) {
  const item = localIdentities.value.find((x) => x.id === id)
  if (!item) return
  const ok = await showConfirm(
    '删除本地身份',
    `删除本地身份「${item.display_name || item.user_id}」？这只会删除本机保存的登录入口，请确认你已保存身份文件。`,
    true,
  )
  if (!ok) return
  localIdentities.value = localIdentities.value.filter((x) => x.id !== id)
  if (selectedLocalIdentityId.value === id) selectedLocalIdentityId.value = localIdentities.value[0]?.id ?? ''
  if (lastRegisteredIdentity.value?.id === id) lastRegisteredIdentity.value = null
  saveLocalIdentityList()
  appendLog('✅ 已删除本地身份')
  toast('已删除本地身份', 'success')
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
  const certs = myDeviceCertJson.value ? `[${myDeviceCertJson.value}]` : undefined
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
    block_reason: existing?.block_reason,
    device_certs: info.device_certs ?? contactCardDeviceCerts(cardText),
  }
}



function createMyDeviceCert() {
  run('创建设备证书', () => {
    const out = safeJson<DeviceOutput>(create_device_cert(backupText.value, passphrase.value, 'Web Browser'))
    myDeviceId.value = out.device_id
    myDeviceCertJson.value = out.device_cert_json
    myContactCardText.value = export_contact_card(
      backupText.value,
      passphrase.value,
      displayName.value || undefined,
      `[${myDeviceCertJson.value}]`,
    )
    persist()
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
      else { queued += 1; outbox.value.push(createOutboxItem(contact, deviceRevokeText.value, undefined, 'other')) }
    }
    appendLog(`设备撤销事件分发完成：已投递 ${sent}，queued ${queued}`)
    persist()
  })
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
    case 'mailbox': return '已发送'
    case 'delivered': return '已送达'
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
  if (direction === 'in' && safetyPolicy.value.dropFilteredIncoming && filterRank(action) >= filterRank('Hide')) {
    return { allow: false, text: '', action: 'Drop' }
  }
  if (action === 'Hide') return { allow: true, text: '[本地策略已隐藏内容]', action }
  if (action === 'Blur') return { allow: true, text: `⚠️ [本地策略提示] ${text}`, action }
  return { allow: true, text: `⚠️ ${text}`, action }
}

function saveSafetyPolicy() {
  persist()
  appendLog('✅ 已保存本地安全策略')
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
    const item: FriendRequestItem = {
      ...info,
      request_text: incomingFriendRequestText.value,
    }
    const index = friendRequests.value.findIndex((r) => r.request_id === item.request_id)
    if (index >= 0) friendRequests.value[index] = item
    else friendRequests.value.unshift(item)
    incomingFriendRequestText.value = ''
    toast('收到新的好友请求', 'info')
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
        .then(() => {
          appendLog('✅ 已通过好友请求')
          toast('已添加好友', 'success')
        })
        .catch((e) => {
          const message = userFacingError(e)
          appendLog(`⚠️ 好友确认发送失败：${message}`)
          showAlert('已添加好友，但通知对方失败', message, 'warning')
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
  createGroupSenderDistributionFanout(group, out.distribution_text)
  appendLog(`🔄 已轮换本群 Sender Key：${reason}；已生成新的 distribution fanout`)
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
    createGroupSenderDistributionFanout(activeGroup.value, out.distribution_text)
    appendLog('✅ 已创建我的群 Sender Key，并生成 distribution fanout')
    persist()
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
    createGroupSenderDistributionFanout(activeGroup.value, groupSenderDistributionText.value.trim())
    persist()
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
  if (!key) throw new Error('收到 Sender Key 群消息，但本地没有该 sender key')
  const out = JSON.parse(group_sender_decrypt_text(key.state_json, envelopeText)) as { state_json: string; plain_json: string }
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

function createRenameGroupEvent() {
  run('生成群改名事件', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
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

function createRemoveMemberGroupEvent(userId: string) {
  run('生成移除成员事件', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
    const sequence = nextGroupSequence(activeGroup.value)
    groupEventText.value = create_group_event(
      backupText.value,
      passphrase.value,
      activeGroup.value.group_id,
      BigInt(sequence),
      JSON.stringify({ RemoveMember: { user_id: userId } }),
    )
  })
}


function createPromoteAdminGroupEvent(userId: string) {
  run('生成提升管理员事件', () => {
    if (!activeGroup.value) throw new Error('请选择群组')
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
  if (action.RemoveMember) return `移除成员 ${action.RemoveMember.user_id}`
  if (action.PromoteAdmin) return `提升管理员 ${action.PromoteAdmin.user_id}`
  if (action.DemoteAdmin) return `取消管理员 ${action.DemoteAdmin.user_id}`
  return '未知事件'
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
  if (!group) throw new Error('本地没有这个群')
  if (info.sequence <= (group.sequence ?? 0)) throw new Error('群事件 sequence 已过期或重复')
  if (group.policy_state_json) {
    group.policy_state_json = apply_group_policy_event(group.policy_state_json, text, actor.contact_card_text)
    const policy = JSON.parse(group.policy_state_json) as { name: string; members: string[]; admins: string[]; sequence: number }
    group.name = policy.name
    group.member_user_ids = policy.members.filter((id) => id !== identity.value?.user_id)
    group.admin_user_ids = policy.admins
    group.sequence = policy.sequence
  } else {
    const admins = group.admin_user_ids ?? []
    const isSelfLeave = info.action.RemoveMember?.user_id === info.actor_user_id
    if (!isSelfLeave && admins.length > 0 && !admins.includes(info.actor_user_id)) throw new Error('群权限拒绝：只有管理员可执行该事件')
    if (info.action.Rename) {
      group.name = info.action.Rename.name
    } else if (info.action.AddMember) {
      const uid = info.action.AddMember.user_id
      if (!group.member_user_ids.includes(uid)) group.member_user_ids.push(uid)
    } else if (info.action.RemoveMember) {
      const uid = info.action.RemoveMember.user_id
      group.member_user_ids = group.member_user_ids.filter((id) => id !== uid)
      group.admin_user_ids = (group.admin_user_ids ?? []).filter((id) => id !== uid)
      if (uid === identity.value?.user_id) appendLog('你已被该群事件移出群聊')
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
    const result = applyGroupEventRaw(text, actorId)
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
  })
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
    const group: GroupItem = {
      group_id: invite.group_id,
      name: invite.group_name,
      member_user_ids: invite.member_user_ids.filter((id) => id !== identity.value?.user_id),
      admin_user_ids: [invite.inviter_user_id],
      policy_state_json: create_group_policy_state(invite.group_id, invite.group_name, invite.inviter_user_id, JSON.stringify(invite.member_user_ids)),
      created_at: Date.now(),
      sequence: 0,
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

function removeActiveGroup() {
  if (!activeGroup.value) return
  const id = activeGroup.value.group_id
  groups.value = groups.value.filter((g) => g.group_id !== id)
  messages.value = messages.value.filter((m) => m.group_id !== id)
  groupSenderKeys.value = groupSenderKeys.value.filter((k) => k.group_id !== id)
  activeGroupId.value = groups.value[0]?.group_id ?? ''
  persist()
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
    persist()
    void pushMailboxPayload(contact, 'other', friendRequestText.value)
      .then(() => {
        appendLog('✅ 好友请求已发送')
        toast('好友请求已发送', 'success')
      })
      .catch((e) => {
        contact.state = 'LocalOnly'
        contact.pending_request_id = undefined
        persist()
        const message = userFacingError(e)
        appendLog(`⚠️ 好友请求发送失败：${message}`)
        showAlert('发送失败', message, 'error')
      })
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

function encryptEnvelopeForContact(contact: ContactItem, conversationId: string, text: string): string {
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

function sendMessage() {
  run('发送消息', () => {
    if (!composerText.value.trim()) return
    ensureUiTextSize('消息', composerText.value, MAX_TEXT_MESSAGE_BYTES)

    const outgoingFiltered = applyLocalTextFilter(composerText.value, 'out')
    if (!outgoingFiltered.allow) throw new Error('本地策略阻止发送')

    if (activeGroup.value) {
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
    const envelope = encryptEnvelopeForContact(
      activeContact.value,
      `conv-${activeContact.value.user_id}`,
      outgoingFiltered.text,
    )
    const msg: ChatMessage = {
      id: newId(),
      conversation_id: `conv-${activeContact.value.user_id}`,
      peer_user_id: activeContact.value.user_id,
      direction: 'out',
      text: outgoingFiltered.text,
      envelope_json: envelope,
      protocol_message_id: messageProtocolIdFromEnvelope(envelope),
      status: 'queued',
      created_at: Date.now(),
    }
    inboundEnvelopeText.value = envelope
    if (dc && dc.readyState === 'open') {
      sendRtcText(envelope, '消息')
      msg.status = 'sent'
      appendLog('✅ 消息已发送')
    } else if (nodeEnabled.value) {
      appendLog('正在通过消息同步发送')
      void tryMailboxDeliveryForMessage(activeContact.value, envelope, msg)
    } else {
      outbox.value.push(createOutboxItem(activeContact.value, envelope, msg.id, 'direct-envelope'))
      appendLog('未开启消息同步，消息已暂存，开启同步后会自动重发')
    }
    messages.value.push(msg)
    composerText.value = ''
    persist()
  })
}

function receiveEnvelopeWithContact(envelopeText: string, sender: ContactItem) {
  if (sender.state === 'Blocked') throw new Error('发送者已被拉黑')
  ensureUiTextSize('Envelope', envelopeText, MAX_SIGNAL_BYTES)
  const groupSenderPlain = tryDecryptGroupSenderEnvelope(envelopeText)
  if (groupSenderPlain) {
    const filtered = applyLocalTextFilter(groupSenderPlain.text, 'in')
    if (!filtered.allow) { persist(); return }
    messages.value.push({
      id: newId(),
      conversation_id: `grp-${groupSenderPlain.group_id}`,
      peer_user_id: groupSenderPlain.sender_user_id,
      group_id: groupSenderPlain.group_id,
      direction: 'in',
      text: filtered.text,
      envelope_json: envelopeText,
      status: 'received',
      created_at: Date.now(),
    })
    activeGroupId.value = groupSenderPlain.group_id
    activePeerId.value = ''
    persist()
    return
  }
  const plain = decryptEnvelopeForContact(envelopeText, sender)
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
      envelope_json: envelopeText,
      status: 'received',
      created_at: Date.now(),
    })
    persist()
    return
  }
  if (typeof rawText === 'string' && rawText.startsWith(GROUP_EVENT_PAYLOAD_PREFIX)) {
    const eventText = rawText.slice(GROUP_EVENT_PAYLOAD_PREFIX.length)
    const result = applyGroupEventRaw(eventText, sender.user_id)
    messages.value.push({
      id: newId(),
      conversation_id: `grp-${result.group_id}`,
      peer_user_id: sender.user_id,
      group_id: result.group_id,
      direction: 'in',
      text: `[群事件] ${result.summary}`,
      envelope_json: envelopeText,
      status: 'received',
      created_at: Date.now(),
    })
    persist()
    return
  }
  const filtered = applyLocalTextFilter(rawText, 'in')
  if (!filtered.allow) {
    persist()
    return
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
    envelope_json: envelopeText,
    protocol_message_id: messageProtocolIdFromEnvelope(envelopeText),
    status: 'received',
    created_at: Date.now(),
  })
  void sendDeliveryAck(sender, messageProtocolIdFromEnvelope(envelopeText))
  persist()
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

function createSecureSessionOfferText() {
  run('创建安全会话 Offer', () => {
    if (!identity.value) throw new Error('请先登录')
    if (!activeContact.value) throw new Error('请先选择联系人')
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
      to_user_id: activeContact.value.user_id,
      prekey_bundle_text: prekeyBundleText.value,
      signed_one_time_prekey_record_texts: prekeySignedOneTimeRecordTexts.value,
      ratchet_dh_public_key: pair.public_key,
      created_at: Date.now(),
    }
    secureSessionOfferText.value = JSON.stringify(offer, null, 2)
    secureSessionStatusText.value = '已创建 Offer：把它发给对方；对方应用后会返回 Response。Private PreKey 和 DH private_key 已留在本机。'
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
    secureSessionStatusText.value = '已建立本端 Ratchet Session，并生成 Response；把 Response 发回给 Offer 创建者。'
    incomingSecureSessionText.value = ''
    inspectRatchetStateText()
    persist()
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
  let res: Response
  try {
    res = await fetch(endpoint, {
      ...init,
      headers: {
        'content-type': 'application/json',
        ...(token ? { authorization: `Bearer ${token}` } : {}),
        ...(init?.headers ?? {}),
      },
    })
  } catch (e) {
    throw new NodeRequestError(userFacingError(e), undefined, baseUrl)
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

async function checkNodeHealth() {
  await runAsync('检查 lm_node 控制面', async () => {
    const health = await nodeFetchJson('/health')
    // /health 免鉴权，会掩盖令牌问题。再探测一个需要鉴权的接口，避免误报"已连接"。
    try {
      await nodeFetchJson('/sync/status')
      nodeControlStatus.value = `已连接（鉴权通过）\n${JSON.stringify(health, null, 2)}`
    } catch (e) {
      const msg = String(e)
      if (/401|unauthorized/i.test(msg)) {
        nodeControlStatus.value = `节点在线，但鉴权失败（401）。\n若节点启用了 --control-token，请在地址后追加 " | 令牌"（与节点令牌一致）。\n\n${msg}`
      } else {
        nodeControlStatus.value = `节点在线，但控制接口异常。\n\n${msg}`
      }
    }
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
}


async function publishPreKeyToNode() {
  await runAsync('发布 PreKey Bundle 到 lm_node', async () => {
    if (!prekeyBundleText.value.trim()) createMyPreKeyBundleText()
    if (!prekeyBundleText.value.trim()) throw new Error('请先生成 PreKey Bundle')
    const body = await nodeFetchJson('/prekey/publish', {
      method: 'POST',
      body: JSON.stringify({
        prekey_bundle_text: prekeyBundleText.value,
        signed_one_time_prekey_record_texts: prekeySignedOneTimeRecordTexts.value,
      }),
    })
    nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
  })
}

async function fetchPreKeyFromNode() {
  await runAsync('从 lm_node 拉取 PreKey Bundle', async () => {
    const userId = nodePreKeyUserId.value.trim() || activeContact.value?.user_id
    if (!userId) throw new Error('请输入 UserID 或选择联系人')
    const body = await nodeFetchJson(`/prekey/get?user_id=${encodeURIComponent(userId)}&consume=false`)
    nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
    if (body.found && body.prekey_bundle_text) {
      prekeyBundleText.value = body.prekey_bundle_text
      selectedOneTimePreKeyId.value = typeof body.selected_one_time_prekey_id === 'number' ? body.selected_one_time_prekey_id : null
      selectedSignedOneTimePreKeyRecordText.value = typeof body.selected_signed_one_time_prekey_record_text === 'string' ? body.selected_signed_one_time_prekey_record_text : ''
      inspectPreKeyBundleText()
      appendLog(`✅ 已拉取 PreKey Bundle${selectedOneTimePreKeyId.value !== null ? '，选中 one-time key ' + selectedOneTimePreKeyId.value : ''}${selectedSignedOneTimePreKeyRecordText.value ? '（signed record）' : ''}`)
    }
  })
}

async function consumePreKeyFromNode() {
  await runAsync('从 lm_node 领取并消费 PreKey Bundle', async () => {
    const userId = nodePreKeyUserId.value.trim() || activeContact.value?.user_id
    if (!userId) throw new Error('请输入 UserID 或选择联系人')
    const body = await nodeFetchJson(`/prekey/get?user_id=${encodeURIComponent(userId)}&consume=true`)
    nodePreKeyStatusText.value = JSON.stringify(body, null, 2)
    if (body.found && body.prekey_bundle_text) {
      prekeyBundleText.value = body.prekey_bundle_text
      selectedOneTimePreKeyId.value = typeof body.selected_one_time_prekey_id === 'number' ? body.selected_one_time_prekey_id : null
      selectedSignedOneTimePreKeyRecordText.value = typeof body.selected_signed_one_time_prekey_record_text === 'string' ? body.selected_signed_one_time_prekey_record_text : ''
      inspectPreKeyBundleText()
      appendLog(`✅ 已领取 PreKey Bundle；节点端已标记 one-time key ${selectedOneTimePreKeyId.value ?? 'none'} 已消费${selectedSignedOneTimePreKeyRecordText.value ? '（signed record）' : ''}`)
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
    nodeSyncStatusText.value = JSON.stringify(body, null, 2)
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
    nodeSyncStatusText.value = JSON.stringify(body, null, 2)
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
    nodeSyncStatusText.value = `auto sync ok: ${JSON.stringify(body)}`
  } catch (e) {
    nodeSyncStatusText.value = `auto sync failed: ${String(e)}`
  }
}

function startNodeSyncLoop() {
  if (nodeSyncTimer) return
  nodeSyncTimer = window.setInterval(() => { void autoPullSnapshotFromPeerNode() }, 60_000)
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

function handleMailboxPayload(item: any): { handled: boolean; deliveryId?: string } {
  const { deliveryId, message } = unwrapMailboxDelivery(item)
  const kind = typeof message.kind === 'string' ? message.kind : ''
  const normalizedKind = kind.replace(/[-_]/g, '').toLowerCase()
  const fromUserId = String(message.from_user_id ?? '')
  const ciphertext = String(message.ciphertext ?? '')
  let sender = contactByUserId(fromUserId)
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
    appendLog(`消息来自未知联系人：${fromUserId}`)
    return { handled: false, deliveryId }
  }
  if (sender.state === 'Blocked') {
    appendLog(`mailbox 消息来自已拉黑联系人：${fromUserId}`)
    return { handled: false, deliveryId }
  }
  activePeerId.value = sender.user_id
  activeGroupId.value = ''

  if (normalizedKind === 'directenvelope' || normalizedKind === 'groupfanout') {
    try {
      receiveEnvelopeWithContact(ciphertext, sender)
      appendLog(`✅ 已自动解密 mailbox ${kind}`)
      return { handled: true, deliveryId }
    } catch (e) {
      appendLog(`❌ mailbox ${kind} 自动解密失败：${String(e)}`)
      inboundEnvelopeText.value = ciphertext
      return { handled: false, deliveryId }
    }
  }

  if (normalizedKind === 'signaloffer') {
    remoteSignalText.value = ciphertext
    appendLog('✅ 已从 mailbox 填入远端 Signal Offer')
    return { handled: true, deliveryId }
  }
  if (normalizedKind === 'signalanswer') {
    remoteSignalText.value = ciphertext
    appendLog('✅ 已从 mailbox 填入远端 Signal Answer')
    return { handled: true, deliveryId }
  }

  if (ciphertext.startsWith('lm-friend-request-v1:')) {
    incomingFriendRequestText.value = ciphertext
    addIncomingFriendRequest()
    return { handled: true, deliveryId }
  }
  if (ciphertext.startsWith('lm-friend-response-v1:')) {
    incomingFriendResponseText.value = ciphertext
    applyFriendResponse()
    return { handled: true, deliveryId }
  }
  if (ciphertext.startsWith('lm-group-invite-v1:')) {
    incomingGroupInviteText.value = ciphertext
    addIncomingGroupInvite()
    return { handled: true, deliveryId }
  }
  try {
    const parsed = JSON.parse(ciphertext) as { type?: string; message_id?: string; from_user_id?: string }
    if (parsed?.type === 'lm-delivery-ack-v1' && parsed.message_id) {
      applyDeliveryAck(parsed.message_id, fromUserId)
      return { handled: true, deliveryId }
    }
    if (parsed?.type === 'lm-device-revoke-v1') {
      incomingDeviceRevokeText.value = ciphertext
      applyDeviceRevokeToActiveContact()
      return { handled: true, deliveryId }
    }
    if (parsed?.type === 'lm-file-package-v1') {
      incomingFilePackageText.value = ciphertext
      inspectIncomingFilePackage()
      decryptIncomingFilePackage()
      return { handled: true, deliveryId }
    }
    if (parsed?.type === 'lm-secure-session-response-v1') {
      incomingSecureSessionText.value = ciphertext
      applySecureSessionResponseText()
      return { handled: true, deliveryId }
    }
    if (parsed?.type === 'lm-secure-session-offer-v1') {
      incomingSecureSessionText.value = ciphertext
      applySecureSessionOfferText()
      return { handled: true, deliveryId }
    }
  } catch {}

  mailboxCiphertext.value = ciphertext
  appendLog(`mailbox 消息类型 ${kind || 'unknown'} 已放入载荷输入框`)
  return { handled: false, deliveryId }
}

function rememberProcessedMailboxId(id: string) {
  if (!id) return
  processedMailboxIds.value = [id, ...processedMailboxIds.value.filter((x) => x !== id)].slice(0, 1000)
}

function processMailboxMessages(messagesFromNode: any[]): string[] {
  let handled = 0
  let duplicate = 0
  let failed = 0
  const ackIds: string[] = []
  for (const item of messagesFromNode) {
    const { deliveryId, message } = unwrapMailboxDelivery(item)
    const messageId = String(message?.message_id ?? '')
    const dedupeId = deliveryId || messageId
    if (dedupeId && processedMailboxIds.value.includes(dedupeId)) {
      duplicate += 1
      if (deliveryId) ackIds.push(deliveryId)
      continue
    }
    const result = handleMailboxPayload(item)
    if (result.handled) {
      handled += 1
      if (deliveryId) ackIds.push(deliveryId)
      if (dedupeId) rememberProcessedMailboxId(dedupeId)
    } else failed += 1
  }
  mailboxInboxStatus.value = `收到 ${messagesFromNode.length}，已处理 ${handled}，重复 ${duplicate}，失败 ${failed}`
  appendLog(`mailbox 自动处理完成：${mailboxInboxStatus.value}`)
  persist()
  return ackIds
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
    const body = await nodeFetchJson(`/mailbox/take?user_id=${encodeURIComponent(userId)}`)
    nodeMailboxTakeInfoText.value = JSON.stringify(body, null, 2)
    const messages = Array.isArray(body.messages) ? body.messages : []
    if (messages.length > 0) {
      const ackIds = processMailboxMessages(messages)
      if (ackIds.length > 0) await ackMailboxToNode(userId, ackIds)
    } else {
      mailboxInboxStatus.value = '没有新消息'
      appendLog('mailbox 没有新消息')
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

function onFileSelected(event: Event) {
  const input = event.target as HTMLInputElement
  selectedFile.value = input.files?.[0] ?? null
}

async function createFilePackageForActive() {
  await runAsync('生成文件包', async () => {
    if (!activeContact.value) throw new Error('请选择联系人')
    if (activeContact.value.state !== 'Friend') throw new Error('联系人还不是 Friend')
    if (!selectedFile.value) throw new Error('请选择文件')
    if (selectedFile.value.size > MAX_FILE_BYTES) throw new Error(`文件过大：最大 ${MAX_FILE_BYTES} bytes`)
    const bytes = new Uint8Array(await selectedFile.value.arrayBuffer())
    if (bytes.length === 0) throw new Error('不能发送空文件')
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
    rtcFileStatus.value = '文件包已生成，可复制或 WebRTC 发送'
    appendLog(`已生成文件包：${selectedFile.value.name}`)
  })
}

function inspectIncomingFilePackage() {
  run('解析文件包', () => {
    const text = incomingFilePackageText.value.trim() || filePackageText.value.trim()
    if (!text) throw new Error('请粘贴文件包 JSON')
    ensureUiTextSize('文件包', text, MAX_RTC_TEXT_BYTES)
    filePackageInfoText.value = JSON.stringify(JSON.parse(inspect_file_package(text)), null, 2)
  })
}

function decryptIncomingFilePackage() {
  run('解密文件包', () => {
    if (!activeContact.value) throw new Error('请选择发送者联系人')
    const text = incomingFilePackageText.value.trim() || filePackageText.value.trim()
    if (!text) throw new Error('请粘贴文件包 JSON')
    ensureUiTextSize('文件包', text, MAX_RTC_TEXT_BYTES)
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
    if (activeContact.value) {
      messages.value.push({
        id: newId(),
        conversation_id: `conv-${activeContact.value.user_id}`,
        peer_user_id: activeContact.value.user_id,
        direction: 'in',
        text: `[文件] ${out.name} (${out.size ?? bytes.length} bytes)`,
        envelope_json: text,
        status: 'received',
        created_at: Date.now(),
      })
    }
    rtcFileStatus.value = `已解密文件：${out.name}`
    appendLog(`已解密文件：${out.name}`)
    persist()
  })
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
      text: `[文件] ${info.manifest.name} (${info.manifest.size} bytes)`,
      envelope_json: filePackageText.value,
      status: 'queued',
      created_at: Date.now(),
    }
    if (dc && dc.readyState === 'open') {
      sendRtcText(filePackageText.value, '文件包')
      msg.status = 'sent'
      rtcFileStatus.value = `已通过 WebRTC 发送：${info.manifest.name}`
    } else if (nodeEnabled.value) {
      rtcFileStatus.value = `正在通过 Mailbox 发送：${info.manifest.name}`
      const payload = filePackageText.value
      const contact = activeContact.value
      void deliverPayloadToContact(contact, payload, '文件包', 'file-package')
        .then((result) => {
          msg.status = result === 'mailbox' ? 'mailbox' : result === 'sent' ? 'sent' : result === 'failed' ? 'failed' : 'queued'
          if (result === 'queued' || result === 'failed') outbox.value.push(createOutboxItem(contact, payload, msg.id, 'file-package'))
          rtcFileStatus.value = result === 'mailbox' ? `已通过 Mailbox 发送：${info.manifest.name}` : `文件投递状态：${result}`
          persist()
        })
    } else {
      outbox.value.push(createOutboxItem(activeContact.value, filePackageText.value, msg.id, 'file-package'))
      rtcFileStatus.value = `文件已加入 outbox：${info.manifest.name}`
    }
    messages.value.push(msg)
    persist()
  })
}

async function copySignal(value: string) {
  await copyText(value, 'Signal')
}

function formatTime(ts: number) {
  return new Date(ts).toLocaleTimeString()
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
  goChatPage, goChatHome, goContactsPage, goSettingsPage, goDiagnosticsPage, logout, log, identity, displayName, localIdentities, selectedLocalIdentityId, lastRegisteredIdentity, loginSelectedIdentity, importIdentityOnly, refreshMyContactCard, myContactCardText, backupText,
  nodeControlUrl, nodeUrlList, syncNow, toggleNodeEnabled, nodeEnabled, saveNetworkSettings, autoPublishPreKeyIfEnabled, autoMailboxTake,
  autoPublishPreKey, autoNodeSync, nodeControlStatus, secureSessionOfferText, secureSessionResponseText, incomingSecureSessionText,
  secureSessionStatusText, createSecureSessionOfferText, applySecureSessionOfferText, applySecureSessionResponseText, createMyDeviceCert, myDeviceCertJson,
  myDeviceId, revokeDeviceId, revokeReason, createDeviceRevokeText, deviceRevokeText, dataBackupText,
  exportFullDataBackup, importFullDataBackup, downloadText, addContactText, addContact, incomingFriendRequestText,
  addIncomingFriendRequest, friendRequests, acceptInboxRequest, rejectInboxRequest, incomingGroupInviteText, addIncomingGroupInvite,
  groupInvites, acceptGroupInvite, ignoreGroupInvite, contacts, activePeerId, selectContact,
  newGroupName, friendContacts, selectedGroupMembers, createGroup, groups, activeGroupId,
  selectGroup, activeContact, activeGroup, activeGroupMembers, blockReason, blockActiveContact,
  unblockActiveContact, removeActiveContact, createFriendRequestForActive, createInviteForActiveGroup, groupInviteText, groupFanoutJson,
  removeActiveGroup, messages, activeMessages, formatTime, statusLabel, copyMessageEnvelope, composerText,
  sendMessage, incomingDeviceRevokeText, applyDeviceRevokeToActiveContact, rtcStatus, createRtcOfferForActive, acceptRtcOfferForActive,
  applyRtcAnswerForActive, resetRtc, localSignalText, copySignal, remoteSignalText, outbox,
  flushOutboxForActive, cancelOutboxForActive, clearSentOutbox, friendRequestText, createFriendRequestForActiveLocalOnly, incomingFriendResponseText, applyFriendResponse, inboundEnvelopeText,
  receiveEnvelope, onFileSelected, createFilePackageForActive, sendFilePackageOverRtc, filePackageText, rtcFileStatus,
  incomingFilePackageText, inspectIncomingFilePackage, decryptIncomingFilePackage, receivedFileUrl, receivedFileName, filePackageInfoText,
  createGroupSenderKeyForActiveGroup, groupSenderDistributionText, importGroupSenderKeyForActiveContact, groupSenderEncryptDebug, groupSenderDecryptDebug, createGroupSenderDistributionFanoutForActiveGroup,
  groupSenderDistributionFanoutJson, groupSenderDistributionFanoutItems, groupSenderEnvelopeText, groupSenderPlainText, groupRenameText, createRenameGroupEvent,
  groupEventText, applyGroupEventText, createGroupEventFanout, groupEventFanoutJson, groupEventFanoutItems, incomingGroupEventText,
  groupEventActorUserId, createAddMemberGroupEvent, createRemoveMemberGroupEvent, createPromoteAdminGroupEvent, createDemoteAdminGroupEvent, fanoutItems,
  prekeySignedId, prekeyOneTimeCount, prekeyBundleText, prekeyPrivateBundleJson, prekeySignedOneTimeRecordTexts, prekeyInfoText, x3dhInitialMessageJson,
  selectedOneTimePreKeyId, selectedSignedOneTimePreKeyRecordText, x3dhSharedSecretText, ratchetStateText, ratchetPeerStateText, ratchetLocalDhKeyPairJson, ratchetRemoteDhPublicKeyForInit,
  ratchetInitRole, ratchetHeaderText, ratchetEnvelopeText, ratchetPlainText, ratchetKeyText, ratchetRemoteDhPublicKey,
  ratchetInfoText, safetyPolicy, peerAddressesText, peerMailboxKey, peerAnnounceText, peerAnnounceInspectPublicKey,
  peerAnnounceInfoText, publicPeerId, publicPeerAddressesText, publicPeerCapabilities, publicPeerAnnounceText, publicPeerAnnounceInspectPublicKey,
  publicPeerAnnounceInfoText, mailboxKind, mailboxCiphertext, mailboxMessageText, mailboxMessageInspectPublicKey, mailboxMessageInfoText,
  nodeClosestTarget, nodeClosestInfoText, nodeMailboxTakeUserId, nodeMailboxTakeInfoText, mailboxInboxStatus, nodePreKeyUserId, nodePreKeyStatusText,
  nodeSyncPeerUrl, nodeSyncSnapshotText, nodeSyncStatusText, createMyPreKeyBundleText, inspectPreKeyBundleText, copyText,
  showQr, createX3dhInitialMessageText, deriveX3dhResponderSecretText, createRatchetPairForActiveContact, createRatchetFromSharedSecretText, generateRatchetDhKeyPairText,
  createRatchetFromSharedSecretWithKeysText, inspectRatchetStateText, ratchetNextSendKeyText, ratchetNextRecvKeyText, ratchetEncryptEnvelopeText, ratchetDecryptEnvelopeText,
  ratchetDhStepText, saveSafetyPolicy, createPeerAnnounceText, inspectPeerAnnounceText, createPublicPeerAnnounceText, inspectPublicPeerAnnounceText,
  createMailboxMessageText, inspectMailboxMessageText, checkNodeHealth, submitPublicPeerToNode, pushMailboxToNode, queryNodeClosestPeers,
  takeMailboxFromNode, processMailboxTakeInfoText, publishPreKeyToNode, fetchPreKeyFromNode, consumePreKeyFromNode, exportNodeSnapshot,
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
