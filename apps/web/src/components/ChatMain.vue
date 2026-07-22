<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'

const props = defineProps<{ ctx: any }>()
const contactName = (userId: string) => props.ctx.contacts.value.find((c: any) => c.user_id === userId)?.display_name || userId
const messageSearch = ref('')
const messageSearchOpen = ref(false)
const composerPanel = ref<'none' | 'attach' | 'emoji'>('none')
const fileInput = ref<HTMLInputElement | null>(null)
const composerTextarea = ref<HTMLTextAreaElement | null>(null)
const emojis = ['😀', '😃', '😄', '😁', '🙂', '😉', '😊', '😍', '👍', '👏', '🙏', '💪', '🎉', '❤️', '🔥', '✅']

function hmTime(ts: number) {
  return new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false }).format(new Date(ts))
}
function dayLabel(ts: number) {
  const d = new Date(ts)
  const now = new Date()
  if (d.toDateString() === now.toDateString()) return '今天'
  const yesterday = new Date(now)
  yesterday.setDate(now.getDate() - 1)
  if (d.toDateString() === yesterday.toDateString()) return '昨天'
  if (d.getFullYear() === now.getFullYear()) return `${d.getMonth() + 1}月${d.getDate()}日`
  return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日`
}
function messageStatusClass(message: any) {
  if (message.status === 'failed') return 'danger'
  if (message.status === 'queued' || message.status === 'copied') return 'warning'
  if (message.status === 'mailbox' || message.status === 'delivered') return 'info'
  if (message.status === 'read' || message.status === 'sent' || message.status === 'received') return 'ok'
  return 'info'
}
function messageStatusShortText(message: any) {
  if (message.direction === 'in') return message.status === 'received' ? '已接收' : props.ctx.statusLabel(message.status)
  switch (message.status) {
    case 'queued': return '待发送'
    case 'copied': return '待发送'
    case 'mailbox': return '待收取'
    case 'sent': return '已发送'
    case 'delivered': return '已送达'
    case 'read': return '已读'
    case 'failed': return '失败'
    default: return props.ctx.statusLabel(message.status)
  }
}
function filePreviewLabel(name?: string, mime?: string) {
  const value = `${name || ''} ${mime || ''}`.toLowerCase()
  if (/image\//.test(value)) return '图片'
  if (/pdf/.test(value)) return 'PDF'
  if (/zip|tar|gzip|7z|rar/.test(value)) return '压缩包'
  if (/text|markdown|json|csv|log/.test(value)) return '文本'
  if (/audio\//.test(value)) return '音频'
  if (/video\//.test(value)) return '视频'
  return '附件'
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
  const parts = [messageStatusShortText(message)]
  if (message.read_at) parts.push(`已读 ${props.ctx.formatDateTime(message.read_at)}`)
  else if (message.delivered_at) parts.push(`送达 ${props.ctx.formatDateTime(message.delivered_at)}`)
  if (message.file_downloaded_at) parts.push(`已下载 ${props.ctx.formatDateTime(message.file_downloaded_at)}`)
  return parts.filter(Boolean).join(' · ')
}

// 把消息序列展开成「日期分割线 + 气泡」的渲染项
const thread = computed(() => {
  const out: any[] = []
  let lastDay = ''
  const q = messageSearch.value.trim().toLowerCase()
  const messages = q
    ? props.ctx.activeMessages.value.filter((m: any) => `${m.text || ''} ${contactName(m.peer_user_id)}`.toLowerCase().includes(q))
    : props.ctx.activeMessages.value
  for (const m of messages) {
    const day = new Date(m.created_at).toDateString()
    if (day !== lastDay) {
      out.push({ kind: 'sep', id: `sep-${day}-${m.id}`, label: dayLabel(m.created_at) })
      lastDay = day
    }
    out.push({ kind: 'msg', id: m.id, m })
  }
  return out
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
  () => [props.ctx.activeMessages.value.length, props.ctx.activePeerId?.value, messageSearch.value],
  () => { void nextTick(scrollToBottom) },
  { immediate: true },
)

// Enter 发送，Shift+Enter 换行；输入法组词中的 Enter 不触发发送
function onComposerKeydown(e: KeyboardEvent) {
  if (e.key !== 'Enter' || e.shiftKey || e.isComposing) return
  e.preventDefault()
  props.ctx.sendMessage()
}
function trustText(contact: any) {
  return contact?.fingerprint_verified_at ? '✓ 已核验' : '⚠️ 未核验'
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
function sendAndClose() {
  props.ctx.sendMessage()
  composerPanel.value = 'none'
}
</script>

<template>
  <section class="chat-main clean-chat-main">
    <header v-if="ctx.activeContact.value" class="chat-header clean-chat-header product-chat-header">
      <button class="back-btn chat-back-btn" aria-label="返回聊天列表" @click="ctx.goChatHome">‹</button>
      <div class="chat-title-block product-chat-title">
        <h2>{{ ctx.activeContact.value.display_name || '未命名联系人' }}</h2>
        <span class="trust-inline" :class="{ danger: !ctx.activeContact.value.fingerprint_verified_at }">{{ trustText(ctx.activeContact.value) }}</span>
      </div>
      <div class="chat-header-actions product-chat-actions">
        <button
          class="icon-btn"
          :aria-label="messageSearchOpen ? '关闭会话内搜索' : '搜索消息'"
          :title="messageSearchOpen ? '关闭搜索' : '搜索消息'"
          @click="messageSearchOpen = !messageSearchOpen"
        >🔍</button>
        <button class="icon-btn" aria-label="更多" title="更多">⋯</button>
      </div>
    </header>
    <div v-if="ctx.activeContact.value && messageSearchOpen" class="message-search-bar">
      <input v-model="messageSearch" type="search" aria-label="搜索当前会话消息" placeholder="搜索当前会话消息" autofocus />
      <button v-if="messageSearch" class="secondary" @click="messageSearch = ''">清除</button>
    </div>

    <section v-if="!ctx.activeContact.value && !ctx.strictE2eePolicyEnabled.value" class="chat-notice-panel">
      <div class="notice-text">
        <b>建议开启严格 E2EE</b>
        <span>新身份建议先启用指纹核验和安全收发策略，再开始添加联系人和发送消息。</span>
        <span>{{ ctx.strictE2eeReadiness.value.text }}</span>
      </div>
      <div class="row compact">
        <button class="secondary" @click="ctx.enableStrictE2eePolicy">一键严格 E2EE</button>
        <button class="secondary" @click="ctx.goSettingsPage">查看安全策略</button>
      </div>
    </section>

    <section v-if="ctx.activeContact.value && ctx.activeContact.value.state !== 'Friend'" class="chat-notice-panel">
      <div v-if="ctx.activeContact.value.state === 'RequestSent'" class="notice-text">
        <b>好友请求已发送</b>
        <span>等待对方通过后即可聊天。</span>
      </div>
      <div v-else-if="ctx.activeContact.value.state === 'Blocked'" class="notice-text">
        <b>联系人已拉黑</b>
        <span>解除拉黑后才能继续操作。</span>
      </div>
      <div v-else class="notice-text">
        <b>你们还不是好友</b>
        <span v-if="ctx.activeContact.value.last_friend_request_error">上次发送失败：{{ ctx.activeContact.value.last_friend_request_error }}</span>
        <span v-else>发送好友请求，对方通过后即可开始聊天。</span>
      </div>
      <div class="row compact">
        <button v-if="ctx.activeContact.value.state === 'RequestSent'" class="secondary" @click="ctx.createFriendRequestForActive">重新发送</button>
        <button v-if="ctx.activeContact.value.state !== 'RequestSent' && ctx.activeContact.value.state !== 'Blocked'" @click="ctx.createFriendRequestForActive">发送好友请求</button>
        <button v-if="ctx.activeContact.value.last_friend_request_error" class="secondary" @click="ctx.clearActiveFriendRequestError">清除请求错误</button>
        <button v-if="ctx.activeContact.value.state === 'Blocked'" @click="ctx.unblockActiveContact">解除拉黑</button>
      </div>

    </section>

    <section v-if="ctx.activeContact.value?.state === 'Friend' && ctx.activeStrictE2eeSendRiskText.value" class="chat-notice-panel">
      <div class="notice-text">
        <b>发送前严格 E2EE 风险</b>
        <span v-if="ctx.activeStrictE2eeSendBlockingText.value" class="danger-text">{{ ctx.activeStrictE2eeSendBlockingText.value }}</span>
        <span v-else>非阻塞提醒：核心条件已满足，但仍建议修复下列新鲜度/确认状态。</span>
        <span>{{ ctx.activeStrictE2eeSendRiskText.value }}</span>
        <span>建议先核验指纹并刷新联系人安全信息。</span>
      </div>
      <div class="row compact">
        <button class="secondary" @click="ctx.enableStrictE2eePolicy">一键严格 E2EE</button>
        <button class="secondary" @click="ctx.findActiveContactContactCard">刷新 ContactCard</button>
        <button v-if="!ctx.activeContact.value.fingerprint_verified_at" class="secondary" @click="ctx.showActiveContactFingerprintQr">指纹核验码</button>
      </div>
    </section>



    <div class="messages clean-messages" ref="messagesEl" role="log" aria-label="消息列表" aria-live="polite">
      <template v-if="ctx.activeContact.value">
        <template v-for="item in thread" :key="item.id">
          <div v-if="item.kind === 'sep'" class="day-sep"><span>{{ item.label }}</span></div>
          <div v-else class="bubble" :class="item.m.direction">
            <div class="text">{{ item.m.text }}</div>
            <small class="bubble-meta">
              <span class="status-pill" :class="messageStatusClass(item.m)" :title="messageStatusDetailText(item.m)">{{ messageStatusShortText(item.m) }}</span>
              <span>{{ hmTime(item.m.created_at) }}</span>
              <span v-if="item.m.read_at"> · 已读 {{ ctx.formatDateTime(item.m.read_at) }}</span>
              <span v-else-if="item.m.delivered_at"> · 送达 {{ ctx.formatDateTime(item.m.delivered_at) }}</span>
              <span v-if="item.m.file_downloaded_at"> · 已下载 {{ ctx.formatDateTime(item.m.file_downloaded_at) }}</span>
            </small>
            <div v-if="canManageMessageOutbox(item.m)" class="bubble-actions">
              <small v-if="messageOutboxError(item.m)" class="outbox-error">{{ messageOutboxError(item.m) }}</small>
              <span>{{ messageOutboxCount(item.m) }} 个投递项待处理</span>
              <button class="secondary" @click="ctx.retryOutboxForMessage(item.m.id)">重试</button>
              <button class="secondary danger" @click="ctx.cancelOutboxForMessage(item.m.id)">取消</button>
            </div>
          </div>
        </template>
        <div v-if="ctx.activeMessages.value.length === 0" class="empty center">还没有消息</div>
        <div v-else-if="thread.length === 0" class="empty center">没有匹配的消息</div>
      </template>

      <section v-else class="chat-empty-state">
        <h2>暂无聊天</h2>
        <p>去通讯录添加好友后开始聊天</p>
        <button class="secondary" @click="ctx.goContactsPage">去通讯录</button>
      </section>
    </div>

    <footer class="composer clean-composer product-composer" v-if="ctx.activeContact.value && ctx.activeContact.value.state === 'Friend'">
      <input ref="fileInput" class="hidden-file-input" type="file" aria-label="选择附件" @change="onHiddenFileChange" />
      <div v-if="ctx.selectedFile.value" class="selected-file-card pending-file-card">
        <div class="file-icon">{{ filePreviewLabel(ctx.selectedFile.value.name, ctx.selectedFile.value.type).slice(0, 1) }}</div>
        <div class="file-card-main">
          <b>{{ ctx.selectedFile.value.name }}</b>
          <small>{{ selectedFileLabel(ctx.selectedFile.value) }}</small>
          <small v-if="ctx.isDangerousFileName(ctx.selectedFile.value.name)" class="danger-text">危险类型：接收方下载后请谨慎打开</small>
        </div>
        <button class="secondary danger" @click="ctx.cancelSelectedFile">删除</button>
        <button @click="ctx.sendSelectedFile">发送文件</button>
      </div>
      <div v-if="ctx.pendingFilePackageText.value && !ctx.receivedFileUrl.value" class="received-file-card pending">
        <div class="file-icon">密</div>
        <div class="file-card-main">
          <b>收到加密文件包</b>
          <small v-if="ctx.pendingFileMeta.value">{{ ctx.pendingFileMeta.value }}</small>
          <small>文件尚未解密，确认来源可信后再打开。</small>
        </div>
        <button class="secondary" @click="ctx.decryptIncomingFilePackage">解密文件</button>
      </div>
      <div v-if="ctx.receivedFileUrl.value" class="received-file-card ready">
        <div class="file-icon">{{ filePreviewLabel(ctx.receivedFileName.value, ctx.receivedFileMime.value).slice(0, 1) }}</div>
        <div class="file-card-main">
          <b>{{ ctx.receivedFileName.value }}</b>
          <small>{{ filePreviewLabel(ctx.receivedFileName.value, ctx.receivedFileMime.value) }} · {{ ctx.receivedFileMeta.value }}</small>
          <small>已解密，下载动作只在你点击后发生。</small>
        </div>
        <a :href="ctx.receivedFileUrl.value" :download="ctx.receivedFileName.value" @click="ctx.markReceivedFileDownloaded">下载</a>
        <img
          v-if="ctx.receivedFileMime.value.startsWith('image/')"
          class="received-file-preview"
          :src="ctx.receivedFileUrl.value"
          :alt="ctx.receivedFileName.value"
        />
        <div v-else class="received-file-placeholder">
          <b>{{ ctx.receivedFilePreviewKind.value }}</b>
          <small>非图片附件不会内联预览，请下载后用本机应用打开。</small>
        </div>
      </div>
      <div class="composer-bar">
        <button class="composer-icon" aria-label="添加附件" @click="togglePanel('attach')">＋</button>
        <textarea ref="composerTextarea" v-model="ctx.composerText.value" rows="1" aria-label="输入消息" placeholder="输入消息…" @keydown="onComposerKeydown" />
        <button class="composer-icon" aria-label="选择 Emoji" @click="togglePanel('emoji')">😊</button>
        <button class="send-icon" :disabled="!ctx.composerText.value.trim()" aria-label="发送" @click="sendAndClose">↑</button>
      </div>
      <div v-if="composerPanel === 'attach'" class="composer-panel attachment-panel">
        <button class="panel-choice" @click="chooseFile('image')">图片</button>
        <button class="panel-choice" @click="chooseFile('file')">文件</button>
      </div>
      <div v-else-if="composerPanel === 'emoji'" class="composer-panel emoji-panel">
        <button v-for="emoji in emojis" :key="emoji" class="emoji-choice" @click="appendEmoji(emoji)">{{ emoji }}</button>
      </div>
      <small v-if="ctx.fileProgressText.value" class="file-progress-line">{{ ctx.fileProgressText.value }}</small>
      <small v-if="activeFileOutboxError" class="outbox-error">文件发送失败：{{ activeFileOutboxError }}</small>
    </footer>
  </section>
</template>
