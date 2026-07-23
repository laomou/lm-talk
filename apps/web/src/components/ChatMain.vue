<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import UiPageHeader from './UiPageHeader.vue'
import UiIcon from './UiIcon.vue'
import UiEmptyState from './UiEmptyState.vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ ctx: any }>()
const route = useRoute()
const router = useRouter()
const { locale, t } = useI18n()
const contactName = (userId: string) => props.ctx.contacts.value.find((c: any) => c.user_id === userId)?.display_name || userId
const messageSearch = ref('')
const messageSearchOpen = computed(() => route.path === '/chat/search/messages')
const composerPanel = ref<'none' | 'attach' | 'emoji'>('none')
const highlightedMessageId = ref('')
const conversationMenuOpen = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)
const composerTextarea = ref<HTMLTextAreaElement | null>(null)
const emojis = ['😀', '😃', '😄', '😁', '🙂', '😉', '😊', '😍', '👍', '👏', '🙏', '💪', '🎉', '❤️', '🔥', '✅']
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

function messageStatusDetailText(message: any) {
  if (message.direction === 'in') return ''
  const parts = [outgoingMessageStatus(message).text]
  if (message.read_at) parts.push(t('chatView.readAt', { time: props.ctx.formatDateTime(message.read_at) }))
  else if (message.delivered_at) parts.push(t('chatView.deliveredAt', { time: props.ctx.formatDateTime(message.delivered_at) }))
  if (message.file_downloaded_at) parts.push(t('chatView.downloadedAt', { time: props.ctx.formatDateTime(message.file_downloaded_at) }))
  return parts.filter(Boolean).join(' · ')
}

// 把消息序列展开成「日期分割线 + 气泡」的渲染项
const thread = computed(() => {
  const out: any[] = []
  let lastDay = ''
  for (const m of props.ctx.activeMessages.value) {
    const day = new Date(m.created_at).toDateString()
    if (day !== lastDay) {
      out.push({ kind: 'sep', id: `sep-${day}-${m.id}`, label: dayLabel(m.created_at) })
      lastDay = day
    }
    out.push({ kind: 'msg', id: m.id, m })
  }
  return out
})
const messageSearchResults = computed(() => {
  const q = messageSearch.value.trim().toLowerCase()
  if (!q) return []
  return props.ctx.activeMessages.value.filter((m: any) => `${m.text || ''}`.toLowerCase().includes(q))
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
function scrollToBottom() {
  const el = messagesEl.value
  if (!el) return
  // Returning to the chat list reuses the same scroll container. Reset the
  // previous conversation's scroll position so the empty state starts at top.
  if (!props.ctx.activePeerId?.value) {
    el.scrollTop = 0
    return
  }
  el.scrollTop = el.scrollHeight
}
watch(
  () => [props.ctx.activeMessages.value.length, props.ctx.activePeerId?.value],
  () => { if (!highlightedMessageId.value) void nextTick(scrollToBottom) },
  { immediate: true },
)
watch(() => props.ctx.activePeerId?.value, () => { conversationMenuOpen.value = false })

// Enter 发送，Shift+Enter 换行；输入法组词中的 Enter 不触发发送
function onComposerKeydown(e: KeyboardEvent) {
  if (e.key !== 'Enter' || e.shiftKey || e.isComposing) return
  e.preventDefault()
  props.ctx.sendMessage()
}
function trustIconName(contact: any) {
  return props.ctx.contactAllKnownDevicesRevoked(contact) ? 'alert' : 'lock'
}
function trustTitle(contact: any) {
  return props.ctx.contactAllKnownDevicesRevoked(contact) ? t('securityStatus.abnormal') : t('securityStatus.normal')
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
  void router.push('/chat')
  void nextTick(() => {
    if (!scrollToMessage(messageId)) scrollToBottom()
    window.setTimeout(() => {
      if (highlightedMessageId.value === messageId) highlightedMessageId.value = ''
    }, 1800)
  })
}

function sendAndClose() {
  props.ctx.sendMessage()
  composerPanel.value = 'none'
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
        <h2>{{ ctx.activeContact.value.display_name || t('chatView.unnamedContact') }}</h2>
        <UiStatusBadge :tone="ctx.contactAllKnownDevicesRevoked(ctx.activeContact.value) ? 'warning' : 'success'" compact :title="trustTitle(ctx.activeContact.value)" :aria-label="trustTitle(ctx.activeContact.value)"><UiIcon :name="trustIconName(ctx.activeContact.value)" size="13" /></UiStatusBadge>
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
      <UiPageHeader :back-label="t('chatView.backToChat')" @back="router.push('/chat')">
        <template #title>
          <input v-model="messageSearch" class="subbar-search" type="search" :aria-label="t('chatView.searchMessages')" :placeholder="t('chatView.searchMessages')" autofocus />
        </template>
      </UiPageHeader>
      <div class="chat-message-search-results">
        <UiEmptyState v-if="!messageSearch" :title="t('chatView.searchMessages')" :description="t('chatView.messageSearchDescription')" />
        <template v-else>
          <button v-for="message in messageSearchResults" :key="message.id" class="search-message-result" @click="locateMessage(message.id)">
            <span>{{ message.direction === 'out' ? t('chatView.me') : contactName(message.peer_user_id) }} · {{ hmTime(message.created_at) }}</span>
            <b>{{ message.text }}</b>
          </button>
          <UiEmptyState v-if="messageSearchResults.length === 0" icon="search" :title="t('chatView.noMessageMatchesTitle')" :description="t('chatView.noMessageMatchesDescription')" />
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




    <div v-if="!messageSearchOpen" class="messages clean-messages" ref="messagesEl" role="log" :aria-label="t('chatView.messageList')" aria-live="polite">
      <template v-if="ctx.activeContact.value">
        <template v-for="item in thread" :key="item.id">
          <div v-if="item.kind === 'sep'" class="day-sep"><span>{{ item.label }}</span></div>
          <div v-else class="bubble" :class="[item.m.direction, { highlighted: highlightedMessageId === item.m.id }]" :data-message-id="item.m.id">
            <div class="text">{{ item.m.text }}</div>
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
            <div v-if="canManageMessageOutbox(item.m)" class="bubble-actions">
              <small v-if="messageOutboxError(item.m)" class="outbox-error">{{ messageOutboxError(item.m) }}</small>
              <span>{{ t('chatView.outboxItemsPending', { count: messageOutboxCount(item.m) }) }}</span>
              <button class="secondary" @click="ctx.retryOutboxForMessage(item.m.id)">{{ t('chatView.retry') }}</button>
              <button class="secondary danger" @click="ctx.cancelOutboxForMessage(item.m.id)">{{ t('chatView.cancel') }}</button>
            </div>
          </div>
        </template>
        <UiEmptyState v-if="ctx.activeMessages.value.length === 0" class="chat-thread-empty" :title="t('chatView.noMessagesTitle')" :description="t('chatView.noMessagesDescription')" />
      </template>

      <section v-else class="chat-empty-state">
        <h2>{{ t('chatView.noChatsTitle') }}</h2>
        <p>{{ t('chatView.noChatsDescription') }}</p>
        <button class="secondary" @click="ctx.goContactsPage">{{ t('chatView.goContacts') }}</button>
      </section>
    </div>

    <footer class="composer clean-composer product-composer" v-if="!messageSearchOpen && ctx.activeContact.value && ctx.activeContact.value.state === 'Friend'">
      <input ref="fileInput" class="hidden-file-input" type="file" :aria-label="t('chatView.selectAttachment')" @change="onHiddenFileChange" />
      <div v-if="ctx.selectedFile.value" class="selected-file-card pending-file-card">
        <div class="file-icon">{{ filePreviewLabel(ctx.selectedFile.value.name, ctx.selectedFile.value.type).slice(0, 1) }}</div>
        <div class="file-card-main">
          <b>{{ ctx.selectedFile.value.name }}</b>
          <small>{{ selectedFileLabel(ctx.selectedFile.value) }}</small>
          <small v-if="ctx.isDangerousFileName(ctx.selectedFile.value.name)" class="danger-text">{{ t('chatView.dangerousFileWarning') }}</small>
        </div>
        <button class="secondary danger" @click="ctx.cancelSelectedFile">{{ t('chatView.delete') }}</button>
        <button @click="ctx.sendSelectedFile">{{ t('chatView.sendFile') }}</button>
      </div>
      <div v-if="ctx.pendingFilePackageText.value && !ctx.receivedFileUrl.value" class="received-file-card pending">
        <div class="file-icon">{{ t('chatView.encryptedFileIcon') }}</div>
        <div class="file-card-main">
          <b>{{ t('chatView.encryptedFilePackage') }}</b>
          <small v-if="ctx.pendingFileMeta.value">{{ ctx.pendingFileMeta.value }}</small>
          <small>{{ t('chatView.encryptedFileHint') }}</small>
        </div>
        <button class="secondary" @click="ctx.decryptIncomingFilePackage">{{ t('chatView.decryptFile') }}</button>
      </div>
      <div v-if="ctx.receivedFileUrl.value" class="received-file-card ready">
        <div class="file-icon">{{ filePreviewLabel(ctx.receivedFileName.value, ctx.receivedFileMime.value).slice(0, 1) }}</div>
        <div class="file-card-main">
          <b>{{ ctx.receivedFileName.value }}</b>
          <small>{{ filePreviewLabel(ctx.receivedFileName.value, ctx.receivedFileMime.value) }} · {{ ctx.receivedFileMeta.value }}</small>
          <small>{{ t('chatView.decryptedFileHint') }}</small>
        </div>
        <a :href="ctx.receivedFileUrl.value" :download="ctx.receivedFileName.value" @click="ctx.markReceivedFileDownloaded">{{ t('chatView.download') }}</a>
        <img
          v-if="ctx.receivedFileMime.value.startsWith('image/')"
          class="received-file-preview"
          :src="ctx.receivedFileUrl.value"
          :alt="ctx.receivedFileName.value"
        />
        <div v-else class="received-file-placeholder">
          <b>{{ ctx.receivedFilePreviewKind.value }}</b>
          <small>{{ t('chatView.nonImagePreviewHint') }}</small>
        </div>
      </div>
      <div class="composer-bar">
        <button class="composer-icon" :aria-label="t('chatView.chooseAttachment')" @click="togglePanel('attach')"><UiIcon name="add" /></button>
        <textarea ref="composerTextarea" v-model="ctx.composerText.value" rows="1" :aria-label="t('chatView.inputMessage')" :placeholder="t('chatView.inputMessage') + '…'" @keydown="onComposerKeydown" />
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
      <small v-if="ctx.fileProgressText.value" class="file-progress-line">{{ ctx.fileProgressText.value }}</small>
      <small v-if="activeFileOutboxError" class="outbox-error">{{ t('chatView.fileSendFailed') }}：{{ activeFileOutboxError }}</small>
    </footer>
  </section>
</template>
