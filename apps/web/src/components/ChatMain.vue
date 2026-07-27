<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import UiPageHeader from './UiPageHeader.vue'
import UiIcon from './UiIcon.vue'
import UiEmptyState from './UiEmptyState.vue'
import UiAvatar from './UiAvatar.vue'
import ChatAttachmentCard from './ChatAttachmentCard.vue'
import PendingFileCard from './PendingFileCard.vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ ctx: any }>()
const route = useRoute()
const router = useRouter()
const { locale, t } = useI18n()
const contactName = (userId: string) => props.ctx.contacts.value.find((c: any) => c.user_id === userId)?.display_name || userId
const messageSearch = ref('')
const messageSearchQuery = ref('')
const messageSearchPending = ref(false)
const messageSearchOpen = computed(() => route.path === '/chat/search/messages')
const MAX_MESSAGE_SEARCH_RESULTS = 50
const composerPanel = ref<'none' | 'attach' | 'emoji'>('none')
const highlightedMessageId = ref('')
const conversationMenuOpen = ref(false)
const messageMenuOpenId = ref('')
const stickToBottom = ref(true)
const showJumpToLatest = ref(false)
const keyboardInset = ref(0)
const fileInput = ref<HTMLInputElement | null>(null)
const composerTextarea = ref<HTMLTextAreaElement | null>(null)
const emojis = ['😀', '😃', '😄', '😁', '🙂', '😉', '😊', '😍', '👍', '👏', '🙏', '💪', '🎉', '❤️', '🔥', '✅']
const HISTORY_WINDOW_SIZE = 80
const visibleMessageCount = ref(HISTORY_WINDOW_SIZE)
type MessageStatusIcon = 'info' | 'check' | 'alert'

function hmTime(ts: number) {
  return new Intl.DateTimeFormat(locale.value, { hour: '2-digit', minute: '2-digit', hour12: false }).format(new Date(ts))
}
function dayLabel(ts: number) {
  const d = new Date(ts)
  const now = new Date()
  if (d.toDateString() === now.toDateString()) return t('chatView.today')
  const yesterday = new Date(now)
  yesterday.setDate(now.getDate() - 1)
  if (d.toDateString() === yesterday.toDateString()) return t('chatView.yesterday')
  if (d.getFullYear() === now.getFullYear()) return new Intl.DateTimeFormat(locale.value, { month: 'long', day: 'numeric' }).format(d)
  return new Intl.DateTimeFormat(locale.value, { year: 'numeric', month: 'long', day: 'numeric' }).format(d)
}
function outgoingMessageStatus(message: any) {
  switch (message.status) {
    case 'queued':
    case 'copied':
      return { text: t('chatView.outgoingQueued'), icon: 'info', tone: 'pending' }
    case 'mailbox':
      return { text: t('chatView.outgoingMailbox'), icon: 'check', tone: 'pending' }
    case 'sent':
      return { text: t('chatView.outgoingSent'), icon: 'check', tone: 'sent' }
    case 'delivered':
      return { text: t('chatView.outgoingDelivered'), icon: 'check', tone: 'delivered' }
    case 'read':
      return { text: t('chatView.outgoingRead'), icon: 'check', tone: 'read' }
    case 'failed':
      return { text: t('chatView.outgoingFailed'), icon: 'alert', tone: 'failed' }
    default:
      return { text: props.ctx.statusLabel(message.status), icon: 'info', tone: 'pending' }
  }
}
function outgoingMessageStatusIcon(message: any): MessageStatusIcon {
  return outgoingMessageStatus(message).icon as MessageStatusIcon
}
function filePreviewLabel(name?: string, mime?: string) {
  const value = `${name || ''} ${mime || ''}`.toLowerCase()
  if (/image\//.test(value)) return t('chatView.image')
  if (/pdf/.test(value)) return t('chatView.pdf')
  if (/zip|tar|gzip|7z|rar/.test(value)) return t('chatView.archive')
  if (/text|markdown|json|csv|log/.test(value)) return t('chatView.textFile')
  if (/audio\//.test(value)) return t('chatView.audio')
  if (/video\//.test(value)) return t('chatView.video')
  return t('chatView.attachment')
}
function selectedFileLabel(file: File) {
  return `${filePreviewLabel(file.name, file.type)} · ${file.type || 'application/octet-stream'} · ${props.ctx.formatBytes(file.size)}`
}
function isAttachmentMessage(message: any) {
  return Boolean(message.attachment_name || message.attachment_mime || /^\[文件\]\s/.test(message.text || ''))
}
function attachmentInfo(message: any) {
  const name = message.attachment_name || String(message.text || '').replace(/^\[文件\]\s*/, '').replace(/\s+\([^)]*\)$/, '') || t('chatView.attachment')
  const mime = message.attachment_mime || 'application/octet-stream'
  const size = Number(message.attachment_size ?? 0)
  return { name, mime, size, label: `${filePreviewLabel(name, mime)} · ${props.ctx.formatBytes(size)}` }
}
function attachmentDownload(message: any) {
  return props.ctx.attachmentDownloads.value[message.id]
}
function attachmentHint(message: any) {
  if (message.direction !== 'in') return ''
  if (message.attachment_error) return message.attachment_error
  if (attachmentDownload(message)) return t('chatView.decryptedFileHint')
  if (message.attachment_decrypted_at) return t('chatView.attachmentNeedsRedecryptHint')
  return t('chatView.encryptedFileHint')
}
function attachmentActionLabel(message: any) {
  return message.attachment_error || message.attachment_decrypted_at
    ? t('chatView.retryDecryptFile')
    : t('chatView.decryptFile')
}


function messageOutboxItems(message: any) {
  if (!message?.id || message.direction !== 'out') return []
  return props.ctx.outbox.value.filter((item: any) => item.message_id === message.id && item.status !== 'sent')
}
function messageOutboxCount(message: any) {
  return messageOutboxItems(message).length
}
function messageOutboxError(message: any) {
  const failed = messageOutboxItems(message)
    .filter((item: any) => item.status === 'failed' && item.last_error)
    .sort((a: any, b: any) => (b.created_at ?? 0) - (a.created_at ?? 0))[0]
  return failed?.last_error ?? ''
}
function canManageMessageOutbox(message: any) {
  return messageOutboxCount(message) > 0
}

function toggleMessageMenu(messageId: string) {
  messageMenuOpenId.value = messageMenuOpenId.value === messageId ? '' : messageId
}

function closeMessageMenu() {
  messageMenuOpenId.value = ''
}

async function copyMessageText(message: any) {
  await props.ctx.copyText(message.text || '', t('chatView.messageText'))
  closeMessageMenu()
}

async function copyMessageEnvelope(message: any) {
  await props.ctx.copyMessageEnvelope(message)
  closeMessageMenu()
}

async function deleteMessage(messageId: string) {
  await props.ctx.deleteChatMessage(messageId)
  closeMessageMenu()
}

function retryMessageOutbox(messageId: string) {
  props.ctx.retryOutboxForMessage(messageId)
  closeMessageMenu()
}

function cancelMessageOutbox(messageId: string) {
  props.ctx.cancelOutboxForMessage(messageId)
  closeMessageMenu()
}

function messageStatusDetailText(message: any) {
  if (message.direction === 'in') return ''
  const parts = [outgoingMessageStatus(message).text]
  if (message.read_at) parts.push(t('chatView.readAt', { time: props.ctx.formatDateTime(message.read_at) }))
  else if (message.delivered_at) parts.push(t('chatView.deliveredAt', { time: props.ctx.formatDateTime(message.delivered_at) }))
  if (message.file_downloaded_at) parts.push(t('chatView.downloadedAt', { time: props.ctx.formatDateTime(message.file_downloaded_at) }))
  return parts.filter(Boolean).join(' · ')
}

const visibleMessages = computed(() => {
  const messages = props.ctx.activeMessages.value
  return messages.slice(Math.max(0, messages.length - visibleMessageCount.value))
})
const hiddenMessageCount = computed(() => Math.max(0, props.ctx.activeMessages.value.length - visibleMessages.value.length))

// 把消息序列展开成「日期分割线 + 气泡」的渲染项
const thread = computed(() => {
  const out: any[] = []
  let lastDay = ''
  for (const m of visibleMessages.value) {
    const day = new Date(m.created_at).toDateString()
    if (day !== lastDay) {
      out.push({ kind: 'sep', id: `sep-${day}-${m.id}`, label: dayLabel(m.created_at) })
      lastDay = day
    }
    out.push({ kind: 'msg', id: m.id, m })
  }
  return out
})
watch(messageSearch, (value, _, onCleanup) => {
  const q = value.trim()
  if (!q) {
    messageSearchQuery.value = ''
    messageSearchPending.value = false
    return
  }
  messageSearchPending.value = true
  const timer = window.setTimeout(() => {
    messageSearchQuery.value = q.toLowerCase()
    messageSearchPending.value = false
  }, 180)
  onCleanup(() => window.clearTimeout(timer))
})

const messageSearchResults = computed(() => {
  const q = messageSearchQuery.value
  if (!q) return []
  return props.ctx.activeMessages.value
    .filter((m: any) => `${m.text || ''}`.toLowerCase().includes(q))
    .slice(0, MAX_MESSAGE_SEARCH_RESULTS)
})
const messageSearchTotalMatches = computed(() => {
  const q = messageSearchQuery.value
  if (!q) return 0
  return props.ctx.activeMessages.value.filter((m: any) => `${m.text || ''}`.toLowerCase().includes(q)).length
})
const messageSearchHasMore = computed(() => messageSearchTotalMatches.value > messageSearchResults.value.length)
const messageSearchStatus = computed(() => {
  if (!messageSearch.value.trim()) return ''
  if (messageSearchPending.value) return t('chatView.searchingMessages')
  if (messageSearchHasMore.value) return t('chatView.searchResultsLimited', { count: MAX_MESSAGE_SEARCH_RESULTS })
  return t('chatView.searchResultsCount', { count: messageSearchTotalMatches.value })
})

const activeFileOutboxError = computed(() => {
  const peerId = props.ctx.activeContact.value?.user_id
  if (!peerId) return ''
  const failed = props.ctx.outbox.value
    .filter((item: any) => item.peer_user_id === peerId && item.kind === 'file-package' && item.status === 'failed' && item.last_error)
    .sort((a: any, b: any) => (b.created_at ?? 0) - (a.created_at ?? 0))[0]
  return failed?.last_error ?? ''
})


const messagesEl = ref<HTMLElement | null>(null)
let keyboardViewportCleanup = () => {}

function updateKeyboardInset() {
  const viewport = window.visualViewport
  if (!viewport) {
    keyboardInset.value = 0
    return
  }
  keyboardInset.value = Math.max(0, Math.round(window.innerHeight - viewport.height - viewport.offsetTop))
}

function isMessagesNearBottom() {
  const el = messagesEl.value
  if (!el) return true
  return el.scrollHeight - (el.scrollTop + el.clientHeight) < 24
}

function syncMessagesScrollState() {
  stickToBottom.value = isMessagesNearBottom()
  if (stickToBottom.value) showJumpToLatest.value = false
}

function scrollToBottom() {
  const el = messagesEl.value
  if (!el) return
  // Returning to the chat list reuses the same scroll container. Reset the
  // previous conversation's scroll position so the empty state starts at top.
  if (!props.ctx.activePeerId?.value) {
    el.scrollTop = 0
    stickToBottom.value = false
    return
  }
  el.scrollTop = el.scrollHeight
  stickToBottom.value = true
  showJumpToLatest.value = false
}

function loadEarlierMessages() {
  const el = messagesEl.value
  if (!el || hiddenMessageCount.value === 0) return
  const previousHeight = el.scrollHeight
  const previousTop = el.scrollTop
  visibleMessageCount.value += HISTORY_WINDOW_SIZE
  void nextTick(() => {
    el.scrollTop = previousTop + (el.scrollHeight - previousHeight)
    syncMessagesScrollState()
  })
}

function jumpToLatest() {
  scrollToBottom()
}

function onMessagesScroll() {
  syncMessagesScrollState()
}

function onComposerFocus() {
  showJumpToLatest.value = false
  void nextTick(scrollToBottom)
}

onMounted(() => {
  updateKeyboardInset()
  const viewport = window.visualViewport
  if (!viewport) return
  const onViewportChange = () => updateKeyboardInset()
  viewport.addEventListener('resize', onViewportChange)
  viewport.addEventListener('scroll', onViewportChange)
  keyboardViewportCleanup = () => {
    viewport.removeEventListener('resize', onViewportChange)
    viewport.removeEventListener('scroll', onViewportChange)
  }
})

onUnmounted(() => {
  keyboardViewportCleanup()
})

watch(
  () => [props.ctx.activePeerId?.value, props.ctx.activeMessages.value.at(-1)?.id],
  ([peerId], previous = []) => {
    const previousPeerId = previous[0]
    if (peerId !== previousPeerId) {
      visibleMessageCount.value = HISTORY_WINDOW_SIZE
      stickToBottom.value = true
      showJumpToLatest.value = false
      if (!highlightedMessageId.value) void nextTick(scrollToBottom)
      return
    }
    const lastMessage = props.ctx.activeMessages.value.at(-1)
    if (!lastMessage || highlightedMessageId.value) return
    if (stickToBottom.value || lastMessage.direction === 'out') {
      showJumpToLatest.value = false
      void nextTick(scrollToBottom)
      return
    }
    showJumpToLatest.value = true
  },
  { immediate: true },
)
watch(() => props.ctx.activePeerId?.value, () => { conversationMenuOpen.value = false; messageMenuOpenId.value = '' })

// Enter 发送，Shift+Enter 换行；输入法组词中的 Enter 不触发发送
function onComposerKeydown(e: KeyboardEvent) {
  if (e.key !== 'Enter' || e.shiftKey || e.isComposing) return
  e.preventDefault()
  props.ctx.sendMessage()
}
function trustLevel(contact: any): 'verified' | 'unverified' | 'blocked' {
  const status = props.ctx.contactStrictE2eeStatus(contact)
  if (status.level === 'blocking') return 'blocked'
  return contact.fingerprint_verified_at ? 'verified' : 'unverified'
}
function trustIconName(contact: any) {
  return trustLevel(contact) === 'blocked' ? 'alert' : 'lock'
}
function trustTitle(contact: any) {
  const level = trustLevel(contact)
  return level === 'verified'
    ? t('trustStatus.verified')
    : level === 'blocked'
      ? t('trustStatus.blocked')
      : t('trustStatus.unverified')
}
function trustBadgeTone(contact: any): 'success' | 'warning' | 'danger' {
  const level = trustLevel(contact)
  return level === 'verified' ? 'success' : level === 'blocked' ? 'danger' : 'warning'
}
const contactSecurityStatus = computed(() => {
  const contact = props.ctx.activeContact.value
  return contact ? props.ctx.contactStrictE2eeStatus(contact) : null
})
function contactSecurityTone(status: any) {
  if (status?.level === 'blocking') return 'danger'
  if (status?.level === 'advisory') return 'warning'
  return 'success'
}
function contactSecurityIcon(status: any): 'alert' | 'check' {
  return status?.level === 'ok' ? 'check' : 'alert'
}
function appendEmoji(emoji: string) {
  const el = composerTextarea.value
  const text = props.ctx.composerText.value || ''
  const start = el?.selectionStart ?? text.length
  const end = el?.selectionEnd ?? start
  props.ctx.composerText.value = `${text.slice(0, start)}${emoji}${text.slice(end)}`
  void nextTick(() => {
    composerTextarea.value?.focus()
    composerTextarea.value?.setSelectionRange(start + emoji.length, start + emoji.length)
  })
}
function togglePanel(panel: 'attach' | 'emoji') {
  composerPanel.value = composerPanel.value === panel ? 'none' : panel
}
function chooseFile(kind: 'image' | 'file') {
  if (!fileInput.value) return
  fileInput.value.accept = kind === 'image' ? 'image/*' : ''
  fileInput.value.click()
}
function onHiddenFileChange(event: Event) {
  props.ctx.onFileSelected(event)
  composerPanel.value = 'none'
}

function scrollToMessage(messageId: string) {
  const el = messagesEl.value
  if (!el) return false
  const target = Array.from(el.querySelectorAll<HTMLElement>('[data-message-id]'))
    .find((item) => item.dataset.messageId === messageId)
  if (!target) return false
  target.scrollIntoView({ block: 'center', behavior: 'smooth' })
  return true
}
function locateMessage(messageId: string) {
  highlightedMessageId.value = messageId
  const targetIndex = props.ctx.activeMessages.value.findIndex((message: any) => message.id === messageId)
  if (targetIndex >= 0) visibleMessageCount.value = props.ctx.activeMessages.value.length - targetIndex
  const peerId = props.ctx.activePeerId.value
  const showTarget = () => nextTick(() => {
    if (!scrollToMessage(messageId)) scrollToBottom()
    window.setTimeout(() => {
      if (highlightedMessageId.value === messageId) highlightedMessageId.value = ''
    }, 1800)
  })
  if (peerId) void router.push(`/chat/${encodeURIComponent(peerId)}`).then(showTarget)
  else void showTarget()
}

function messageSearchPreview(message: any) {
  const text = String(message.text || '')
  const q = messageSearchQuery.value
  if (!q) return text.slice(0, 160)
  const lower = text.toLowerCase()
  const index = lower.indexOf(q)
  if (index < 0) return text.slice(0, 160)
  const start = Math.max(0, index - 20)
  const end = Math.min(text.length, index + q.length + 40)
  return `${start > 0 ? '…' : ''}${text.slice(start, end)}${end < text.length ? '…' : ''}`
}

function sendAndClose() {
  props.ctx.sendMessage()
  composerPanel.value = 'none'
  stickToBottom.value = true
  showJumpToLatest.value = false
}
function deleteActiveConversation() {
  conversationMenuOpen.value = false
  void props.ctx.clearActiveConversation()
}
</script>

<template>
  <section class="chat-main clean-chat-main">
    <header v-if="ctx.activeContact.value && !messageSearchOpen" class="chat-header clean-chat-header product-chat-header">
      <button class="back-btn chat-back-btn" :aria-label="t('chatView.backToChatList')" @click="ctx.goChatHome"><UiIcon name="back" /></button>
      <div class="chat-title-block product-chat-title">
        <UiAvatar :src="ctx.activeContact.value.avatar_data_url" :name="ctx.activeContact.value.display_name" :seed="ctx.activeContact.value.user_id" />
        <h2>{{ ctx.activeContact.value.display_name || t('chatView.unnamedContact') }}</h2>
        <UiStatusBadge :tone="trustBadgeTone(ctx.activeContact.value)" compact :title="trustTitle(ctx.activeContact.value)" :aria-label="trustTitle(ctx.activeContact.value)"><UiIcon :name="trustIconName(ctx.activeContact.value)" size="13" /></UiStatusBadge>
      </div>
      <div class="chat-header-actions product-chat-actions">
        <button
          class="icon-btn"
          :aria-label="messageSearchOpen ? t('chatView.closeMessageSearch') : t('chatView.searchMessages')"
          :title="messageSearchOpen ? t('chatView.closeSearch') : t('chatView.searchMessages')"
          @click="router.push('/chat/search/messages')"
        ><UiIcon name="search" /></button>
        <div class="chat-more-menu">
          <button class="icon-btn" :aria-label="t('chatView.more')" :title="t('chatView.more')" :aria-expanded="conversationMenuOpen ? 'true' : 'false'" @click="conversationMenuOpen = !conversationMenuOpen"><UiIcon name="more" /></button>
          <div v-if="conversationMenuOpen" class="chat-action-menu" role="menu">
            <button class="danger" role="menuitem" @click="deleteActiveConversation">{{ t('chatView.deleteConversation') }}</button>
          </div>
        </div>
      </div>
    </header>
    <section v-if="ctx.activeContact.value && messageSearchOpen" class="chat-message-search-page">
      <UiPageHeader :back-label="t('chatView.backToChat')" @back="ctx.goChatPage">
        <template #title>
          <input v-model="messageSearch" class="subbar-search" type="search" :aria-label="t('chatView.searchMessages')" :placeholder="t('chatView.searchMessages')" autofocus />
        </template>
      </UiPageHeader>
      <div class="chat-message-search-results">
        <UiEmptyState v-if="!messageSearch" :title="t('chatView.searchMessages')" :description="t('chatView.messageSearchDescription')" />
        <template v-else>
          <small v-if="messageSearchStatus" class="search-message-status">{{ messageSearchStatus }}</small>
          <UiEmptyState
            v-if="!messageSearchPending && messageSearchResults.length === 0"
            icon="search"
            :title="t('chatView.noMessageMatchesTitle')"
            :description="t('chatView.noMessageMatchesDescription')"
          />
          <button v-for="message in messageSearchResults" :key="message.id" class="search-message-result" @click="locateMessage(message.id)">
            <span>{{ message.direction === 'out' ? t('chatView.me') : contactName(message.peer_user_id) }} · {{ hmTime(message.created_at) }}</span>
            <b>{{ messageSearchPreview(message) }}</b>
          </button>
        </template>
      </div>
    </section>

    <UiNotice v-if="!messageSearchOpen && ctx.activeContact.value && ctx.activeContact.value.state !== 'Friend'">
      <div v-if="ctx.activeContact.value.state === 'RequestSent'" class="notice-text">
        <b>{{ t('chatView.friendRequestSentTitle') }}</b>
        <span>{{ t('chatView.friendRequestSentDescription') }}</span>
      </div>
      <div v-else-if="ctx.activeContact.value.state === 'Blocked'" class="notice-text">
        <b>{{ t('chatView.contactBlockedTitle') }}</b>
        <span>{{ t('chatView.contactBlockedDescription') }}</span>
      </div>
      <div v-else class="notice-text">
        <b>{{ t('chatView.notFriendTitle') }}</b>
        <span v-if="ctx.activeContact.value.last_friend_request_error">{{ t('chatView.lastRequestError') }}：{{ ctx.activeContact.value.last_friend_request_error }}</span>
        <span v-else>{{ t('chatView.sendFriendRequestDescription') }}</span>
      </div>
      <template #actions>
        <button v-if="ctx.activeContact.value.state === 'RequestSent'" class="secondary" @click="ctx.createFriendRequestForActive">{{ t('chatView.resend') }}</button>
        <button v-if="ctx.activeContact.value.state !== 'RequestSent' && ctx.activeContact.value.state !== 'Blocked'" @click="ctx.createFriendRequestForActive">{{ t('chatView.sendFriendRequest') }}</button>
        <button v-if="ctx.activeContact.value.last_friend_request_error" class="secondary" @click="ctx.clearActiveFriendRequestError">{{ t('chatView.clearRequestError') }}</button>
        <button v-if="ctx.activeContact.value.state === 'Blocked'" @click="ctx.unblockActiveContact">{{ t('chatView.unblock') }}</button>
      </template>
    </UiNotice>

    <UiNotice
      v-else-if="!messageSearchOpen && contactSecurityStatus && contactSecurityStatus.level === 'blocking'"
      compact
      :tone="contactSecurityTone(contactSecurityStatus)"
    >
      <div class="notice-text">
        <b>{{ contactSecurityStatus.label }}</b>
        <span>{{ contactSecurityStatus.detail }}</span>
      </div>
      <template v-if="contactSecurityStatus.level === 'advisory'" #actions>
        <button class="secondary" @click="ctx.refreshContactSecurityInfoForActiveContact">{{ t('settingsView.refreshSecurityInfo') }}</button>
      </template>
      <template v-else #actions>
        <button @click="ctx.repairStrictE2eeForActiveContact">修复后重试</button>
      </template>
    </UiNotice>

    <UiNotice
      v-else-if="!messageSearchOpen && ctx.activeMailboxFailedItems.value.length"
      compact
      tone="warning"
    >
      <div class="notice-text">
        <b>{{ t('chatView.incomingRecoveryTitle', { count: ctx.activeMailboxFailedItems.value.length }) }}</b>
        <span v-if="ctx.activeSessionRecoveryState.value === 'waiting'">{{ t('chatView.sessionRecoveryWaiting') }}</span>
        <span v-else-if="ctx.activeSessionRecoveryState.value === 'recovered'">{{ t('chatView.sessionRecoveryComplete') }}</span>
        <span v-else>{{ ctx.activeMailboxFailedItems.value[0].reason }}。{{ ctx.activeMailboxFailedItems.value[0].hint }}</span>
      </div>
      <template #actions>
        <button
          v-if="ctx.activeSessionRecoveryState.value === 'needed'"
          class="secondary"
          @click="ctx.retrySecureSessionForActiveContact"
        >{{ t('chatView.recoverSession') }}</button>
        <button
          v-if="ctx.activeSessionRecoveryState.value === 'recovered'"
          class="secondary danger"
          @click="ctx.discardUnrecoverableSessionFailuresForActiveContact"
        >{{ t('chatView.ignoreOldMessages') }}</button>
        <button
          v-else-if="ctx.activeSessionRecoveryState.value === 'none'"
          class="secondary"
          @click="ctx.retryMailboxFailuresForActiveContact"
        >{{ t('chatView.retryIncoming') }}</button>
      </template>
    </UiNotice>




    <div v-if="!messageSearchOpen" class="messages clean-messages" ref="messagesEl" role="log" :aria-label="t('chatView.messageList')" aria-live="polite" @scroll.passive="onMessagesScroll">
      <template v-if="ctx.activeContact.value">
        <button
          v-if="hiddenMessageCount > 0"
          class="load-earlier-messages"
          type="button"
          @click="loadEarlierMessages"
        >{{ t('chatView.loadEarlierMessages', { count: hiddenMessageCount }) }}</button>
        <template v-for="item in thread" :key="item.id">
          <div v-if="item.kind === 'sep'" class="day-sep"><span>{{ item.label }}</span></div>
          <div v-else class="bubble" :class="[item.m.direction, { highlighted: highlightedMessageId === item.m.id }]" :data-message-id="item.m.id">
            <template v-if="isAttachmentMessage(item.m)">
              <ChatAttachmentCard
                :icon="filePreviewLabel(attachmentInfo(item.m).name, attachmentInfo(item.m).mime).slice(0, 1)"
                :name="attachmentInfo(item.m).name"
                :label="attachmentInfo(item.m).label"
                :hint="item.m.direction === 'in' ? attachmentHint(item.m) : ''"
                :danger-hint="Boolean(item.m.attachment_error)"
                :download-url="item.m.direction === 'in' && attachmentDownload(item.m) ? attachmentDownload(item.m).url : undefined"
                :download-name="item.m.direction === 'in' && attachmentDownload(item.m) ? attachmentDownload(item.m).name : undefined"
                :preview-url="item.m.direction === 'in' && attachmentDownload(item.m)?.mime.startsWith('image/') ? attachmentDownload(item.m).url : undefined"
                :preview-alt="item.m.direction === 'in' && attachmentDownload(item.m) ? attachmentDownload(item.m).name : undefined"
                :action-label="item.m.direction === 'in' && !attachmentDownload(item.m) ? attachmentActionLabel(item.m) : ''"
                :download-label="t('chatView.download')"
                @action="ctx.decryptAttachmentMessage(item.m.id)"
              />
            </template>
            <div v-else class="text">{{ item.m.text }}</div>
            <small class="bubble-meta">
              <span>{{ hmTime(item.m.created_at) }}</span>
              <span
                v-if="item.m.direction === 'out'"
                class="message-status"
                :class="`is-${outgoingMessageStatus(item.m).tone}`"
                :title="messageStatusDetailText(item.m)"
              >
                <UiIcon :name="outgoingMessageStatusIcon(item.m)" size="12" />
                {{ outgoingMessageStatus(item.m).text }}
              </span>
              <span v-if="item.m.file_downloaded_at"> · {{ t('chatView.downloadedAt', { time: ctx.formatDateTime(item.m.file_downloaded_at) }) }}</span>
            </small>
            <div class="bubble-actions">
              <small v-if="messageOutboxError(item.m)" class="outbox-error">{{ messageOutboxError(item.m) }}</small>
              <button class="icon-btn message-more-btn" :aria-label="t('chatView.messageActions')" :title="t('chatView.messageActions')" :aria-expanded="messageMenuOpenId === item.m.id ? 'true' : 'false'" @click="toggleMessageMenu(item.m.id)"><UiIcon name="more" /></button>
              <div v-if="messageMenuOpenId === item.m.id" class="message-action-menu" role="menu">
                <button role="menuitem" @click="copyMessageText(item.m)">{{ t('chatView.copyMessageText') }}</button>
                <button v-if="item.m.envelope_json" role="menuitem" @click="copyMessageEnvelope(item.m)">{{ t('chatView.copyMessageEnvelope') }}</button>
                <button role="menuitem" class="danger" @click="deleteMessage(item.m.id)">{{ t('chatView.deleteMessage') }}</button>
                <button v-if="canManageMessageOutbox(item.m)" role="menuitem" @click="retryMessageOutbox(item.m.id)">{{ t('chatView.retrySend') }}</button>
                <button v-if="canManageMessageOutbox(item.m)" role="menuitem" class="danger" @click="cancelMessageOutbox(item.m.id)">{{ t('chatView.cancelSend') }}</button>
              </div>
            </div>
          </div>
        </template>
        <UiEmptyState v-if="ctx.activeMessages.value.length === 0" class="chat-thread-empty" :title="t('chatView.noMessagesTitle')" :description="t('chatView.noMessagesDescription')" />
        <button v-if="showJumpToLatest" class="jump-to-latest" type="button" @click="jumpToLatest">{{ t('chatView.jumpToLatest') }}</button>
      </template>

      <section v-else class="chat-empty-state">
        <h2>{{ t('chatView.noChatsTitle') }}</h2>
        <p>{{ t('chatView.noChatsDescription') }}</p>
        <button class="secondary" @click="ctx.goContactsPage">{{ t('chatView.goContacts') }}</button>
      </section>
    </div>

    <footer class="composer clean-composer product-composer" v-if="!messageSearchOpen && ctx.activeContact.value && ctx.activeContact.value.state === 'Friend'" :style="{ '--chat-keyboard-inset': `${keyboardInset}px` }">
      <input ref="fileInput" class="hidden-file-input" type="file" :aria-label="t('chatView.selectAttachment')" @change="onHiddenFileChange" />
      <PendingFileCard
        v-if="ctx.selectedFile.value"
        :icon="filePreviewLabel(ctx.selectedFile.value.name, ctx.selectedFile.value.type).slice(0, 1)"
        :name="ctx.selectedFile.value.name"
        :label="selectedFileLabel(ctx.selectedFile.value)"
        :dangerous="ctx.isDangerousFileName(ctx.selectedFile.value.name)"
        :transfer-label="t('chatView.dangerousFileWarning')"
        :send-label="ctx.fileTransferPhase.value === '失败' ? t('chatView.retrySendFile') : t('chatView.sendFile')"
        :delete-label="t('chatView.delete')"
        @delete="ctx.cancelSelectedFile"
        @send="ctx.fileTransferPhase.value === '失败' ? ctx.retrySelectedFileSend() : ctx.sendSelectedFile()"
      />
      <div class="composer-bar">
        <button class="composer-icon" :aria-label="t('chatView.chooseAttachment')" @click="togglePanel('attach')"><UiIcon name="add" /></button>
        <textarea ref="composerTextarea" v-model="ctx.composerText.value" rows="1" :aria-label="t('chatView.inputMessage')" :placeholder="t('chatView.inputMessage') + '…'" @keydown="onComposerKeydown" @focus="onComposerFocus" />
        <button class="composer-icon" :aria-label="t('chatView.chooseEmoji')" @click="togglePanel('emoji')"><UiIcon name="smile" /></button>
        <button class="send-icon" :disabled="!ctx.composerText.value.trim()" :aria-label="t('chatView.send')" @click="sendAndClose"><UiIcon name="send" /></button>
      </div>
      <div v-if="composerPanel === 'attach'" class="composer-panel attachment-panel">
        <button class="panel-choice" @click="chooseFile('image')">{{ t('chatView.image') }}</button>
        <button class="panel-choice" @click="chooseFile('file')">{{ t('chatView.attachment') }}</button>
      </div>
      <div v-else-if="composerPanel === 'emoji'" class="composer-panel emoji-panel">
        <button v-for="emoji in emojis" :key="emoji" class="emoji-choice" @click="appendEmoji(emoji)">{{ emoji }}</button>
      </div>
      <small v-if="ctx.fileTransferPhase.value !== '待选择'" class="file-progress-line">{{ ctx.fileTransferPhase.value }} · {{ ctx.rtcFileStatus.value }}</small>
      <small v-else-if="ctx.fileProgressText.value" class="file-progress-line">{{ ctx.fileProgressText.value }}</small>
      <small v-if="activeFileOutboxError" class="outbox-error">{{ t('chatView.fileSendFailed') }}：{{ activeFileOutboxError }}</small>
    </footer>
  </section>
</template>
