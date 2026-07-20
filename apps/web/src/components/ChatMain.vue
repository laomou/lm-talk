<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'

const props = defineProps<{ ctx: any }>()
const contactName = (userId: string) => props.ctx.contacts.value.find((c: any) => c.user_id === userId)?.display_name || userId
const messageSearch = ref('')

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
function shortMessageId(value?: string) {
  if (!value) return ''
  return value.length > 12 ? `${value.slice(0, 6)}…${value.slice(-4)}` : value
}
function mailboxWaitText(message: any) {
  if (message.status !== 'mailbox' || message.direction !== 'out') return ''
  const elapsedMs = Math.max(0, Date.now() - (message.created_at ?? Date.now()))
  const minutes = Math.floor(elapsedMs / 60_000)
  if (minutes < 1) return '等待收取 <1 分钟'
  if (minutes < 60) return `等待收取 ${minutes} 分钟`
  const hours = Math.floor(minutes / 60)
  return `等待收取 ${hours} 小时`
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
    case 'mailbox': return 'Mailbox'
    case 'sent': return '已发送'
    case 'delivered': return '已送达'
    case 'read': return '已读'
    case 'failed': return '失败'
    default: return props.ctx.statusLabel(message.status)
  }
}
function messageStatusDetailText(message: any) {
  const parts = [props.ctx.statusLabel(message.status)]
  const wait = mailboxWaitText(message)
  if (wait) parts.push(wait)
  if (message.mailbox_delivery_id) parts.push(`delivery ${shortMessageId(message.mailbox_delivery_id)}`)
  if (message.read_at) parts.push(`已读 ${props.ctx.formatDateTime(message.read_at)}`)
  else if (message.delivered_at) parts.push(`送达 ${props.ctx.formatDateTime(message.delivered_at)}`)
  if (message.direction === 'out' && message.protocol_message_id) parts.push(`消息 ${shortMessageId(message.protocol_message_id)}`)
  if (message.direction === 'out' && props.ctx.perDeviceEnvelopeTargetCount(message)) parts.push(`分设备 ${props.ctx.perDeviceEnvelopeTargetCount(message)}`)
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

const activePendingOutboxCount = computed(() => {
  const peerId = props.ctx.activeContact.value?.user_id
  if (!peerId) return 0
  return props.ctx.outbox.value.filter((item: any) => item.peer_user_id === peerId && item.status !== 'sent').length
})

const activeFailedOutboxCount = computed(() => {
  const peerId = props.ctx.activeContact.value?.user_id
  if (!peerId) return 0
  return props.ctx.outbox.value.filter((item: any) => item.peer_user_id === peerId && item.status === 'failed').length
})

const activeQueuedOutboxCount = computed(() => Math.max(0, activePendingOutboxCount.value - activeFailedOutboxCount.value))

const activeOutboxError = computed(() => {
  const peerId = props.ctx.activeContact.value?.user_id
  if (!peerId) return ''
  const failed = props.ctx.outbox.value
    .filter((item: any) => item.peer_user_id === peerId && item.status === 'failed' && item.last_error)
    .sort((a: any, b: any) => (b.created_at ?? 0) - (a.created_at ?? 0))[0]
  return failed?.last_error ?? ''
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
  if (el) el.scrollTop = el.scrollHeight
}
watch(
  () => [props.ctx.activeMessages.value.length, props.ctx.activePeerId?.value, props.ctx.activeGroupId?.value, messageSearch.value],
  () => { void nextTick(scrollToBottom) },
  { immediate: true },
)

// Enter 发送，Shift+Enter 换行；输入法组词中的 Enter 不触发发送
function onComposerKeydown(e: KeyboardEvent) {
  if (e.key !== 'Enter' || e.shiftKey || e.isComposing) return
  e.preventDefault()
  props.ctx.sendMessage()
}
</script>

<template>
  <section class="chat-main clean-chat-main">
    <header class="chat-header clean-chat-header">
      <div v-if="ctx.activeContact.value" class="chat-title-block">
        <h2>{{ ctx.activeContact.value.display_name || '未命名联系人' }}</h2>
        <small v-if="ctx.activeContact.value.state === 'Friend'">好友</small>
        <small v-else-if="ctx.activeContact.value.state === 'RequestSent'">等待对方通过</small>
        <small v-else-if="ctx.activeContact.value.state === 'Blocked'">已拉黑</small>
        <small v-else>还不是好友</small>
        <small v-if="ctx.activeContact.value.state === 'Friend'">端到端会话：{{ ctx.activeRatchetStatusText.value }}</small>
        <small v-if="ctx.activeContact.value.fingerprint_verified_at">身份指纹：已核验 · {{ ctx.formatDateTime(ctx.activeContact.value.fingerprint_verified_at) }}</small>
        <small v-else-if="ctx.activeContact.value.state === 'Friend'" class="danger-text">身份指纹：未核验，请通过可信渠道核对</small>
        <small v-if="ctx.contactAllKnownDevicesRevoked(ctx.activeContact.value)" class="danger-text">所有已知设备均已撤销，已阻止发送/建链</small>
        <small v-else-if="ctx.contactRevokedDeviceCount(ctx.activeContact.value)" class="danger-text">已撤销设备：{{ ctx.contactRevokedDeviceCount(ctx.activeContact.value) }}</small>
        <small v-if="ctx.activeContact.value.mailbox_hint_url">MailboxHint：{{ ctx.activeContact.value.mailbox_hint_url }}</small>
        <small v-if="ctx.activeContact.value.last_dht_discovery_attempt_at">最近 DHT 发现尝试：{{ ctx.formatDateTime(ctx.activeContact.value.last_dht_discovery_attempt_at) }}</small>
        <small v-if="ctx.activeContact.value.last_prekey_dht_found_at">PreKey DHT 发现：{{ ctx.formatDateTime(ctx.activeContact.value.last_prekey_dht_found_at) }}</small>
        <small v-if="ctx.activeContact.value.last_mailbox_hint_dht_found_at">MailboxHint DHT 发现：{{ ctx.formatDateTime(ctx.activeContact.value.last_mailbox_hint_dht_found_at) }}</small>
        <small v-if="ctx.activeContact.value.last_contact_card_dht_found_at">ContactCard DHT 发现：{{ ctx.formatDateTime(ctx.activeContact.value.last_contact_card_dht_found_at) }}</small>
        <small v-if="ctx.activeContact.value.next_dht_discovery_retry_at">DHT 下次自动重试：{{ ctx.formatDateTime(ctx.activeContact.value.next_dht_discovery_retry_at) }}</small>
        <small v-if="ctx.activeContact.value.dht_discovery_failure_count">DHT 连续失败：{{ ctx.activeContact.value.dht_discovery_failure_count }} 次</small>
        <small v-if="ctx.activeContact.value.dht_discovery_risk_level === 'high'" class="danger-text">DHT 安全风险：记录验签或格式异常，请谨慎信任该联系人发现结果</small>
        <small v-if="ctx.activeContact.value.last_dht_discovery_error" class="danger-text">DHT 发现失败<span v-if="ctx.activeContact.value.last_dht_discovery_error_kind">({{ ctx.activeContact.value.last_dht_discovery_error_kind }})</span>：{{ ctx.activeContact.value.last_dht_discovery_error }}</small>
        <small v-if="ctx.activeContact.value.last_secure_session_attempt_at">最近建链尝试：{{ ctx.formatDateTime(ctx.activeContact.value.last_secure_session_attempt_at) }}</small>
        <small v-if="ctx.activeContact.value.last_secure_session_success_at">最近建链成功：{{ ctx.formatDateTime(ctx.activeContact.value.last_secure_session_success_at) }}</small>
        <small v-if="ctx.activeContact.value.secure_session_failure_count">连续建链失败：{{ ctx.activeContact.value.secure_session_failure_count }} 次</small>
        <small v-if="ctx.activeSecureSessionOutboxCount.value">安全建链待重试：{{ ctx.activeSecureSessionOutboxCount.value }} 条</small>
        <small v-if="ctx.activeContact.value.last_secure_session_error" class="outbox-error">安全建链失败：{{ ctx.activeContact.value.last_secure_session_error }}</small>
        <small v-if="ctx.activeContact.value.state === 'Friend' && ctx.activeContactSealedSlotStatusText.value" :class="{ 'danger-text': ctx.activeContactSealedSlotRiskLevel.value === 'high' }">分设备 sealed slot：{{ ctx.activeContactSealedSlotStatusText.value }}</small>
      </div>
      <div v-else-if="ctx.activeGroup.value" class="chat-title-block">
        <h2>{{ ctx.activeGroup.value.name }}</h2>
        <small>{{ ctx.activeGroup.value.member_user_ids.length }} 人</small>
      </div>
      <div v-else class="chat-title-block">
        <h2>选择一个聊天</h2>
      </div>
      <div v-if="ctx.activeContact.value || ctx.activeGroup.value" class="chat-header-actions">
        <input v-model="messageSearch" type="search" aria-label="搜索当前聊天" placeholder="搜索消息" />
        <small v-if="activeOutboxError" class="outbox-error">{{ activeOutboxError }}</small>
        <small v-if="activePendingOutboxCount">待发送：{{ activeQueuedOutboxCount }}，失败：{{ activeFailedOutboxCount }}</small>
        <button v-if="activePendingOutboxCount" class="secondary" @click="ctx.flushOutboxForActive">重发 {{ activePendingOutboxCount }}</button>
        <button v-if="activePendingOutboxCount" class="secondary danger" @click="ctx.cancelOutboxForActive">取消发送</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend'" class="secondary" @click="ctx.discoverActiveContactDht">发现 DHT</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend'" class="secondary" @click="ctx.findActiveContactPreKey">查找 PreKey</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend'" class="secondary" @click="ctx.findActiveContactMailboxHint">查找 MailboxHint</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend'" class="secondary" @click="ctx.findActiveContactContactCard">查找 ContactCard</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend' && !ctx.activeContact.value.fingerprint_verified_at" class="secondary" @click="ctx.verifyActiveContactFingerprint">标记指纹已核验</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend'" class="secondary" @click="ctx.showActiveContactFingerprintQr">指纹核验码</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend'" class="secondary" @click="ctx.copyActiveContactFingerprintProof">复制核验码</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend' && ctx.activeContact.value.last_secure_session_error" class="secondary" @click="ctx.retrySecureSessionForActiveContact">重试建链</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend' && ctx.activeContact.value.last_secure_session_error" class="secondary" @click="ctx.clearActiveSecureSessionError">清除建链错误</button>
        <button v-if="ctx.activeContact.value?.state === 'Friend' && !ctx.activeRatchetSession.value" class="secondary" @click="ctx.recreateActiveRatchetSession">本地建链</button>
        <label v-if="ctx.activeContact.value?.state === 'Friend'" class="identity-select compact-select">
          <span>已读回执</span>
          <select
            aria-label="当前联系人已读回执策略"
            :value="ctx.activeContact.value.read_receipts || 'default'"
            @change="ctx.setActiveContactReadReceipts(($event.target as HTMLSelectElement).value)"
          >
            <option value="default">跟随全局（{{ ctx.autoReadReceipts.value ? '开' : '关' }}）</option>
            <option value="enabled">始终开启</option>
            <option value="disabled">关闭</option>
          </select>
        </label>
        <button class="secondary danger" @click="ctx.clearActiveConversation">清空聊天</button>
      </div>
    </header>

    <section v-if="!ctx.activeContact.value && !ctx.activeGroup.value && !ctx.strictE2eePolicyEnabled.value" class="chat-notice-panel">
      <div class="notice-text">
        <b>建议开启严格 E2EE</b>
        <span>新身份建议先启用指纹核验 + 分设备 sealed slot 收发策略，再开始添加联系人和发送消息。</span>
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
        <span>建议先在设置中启用“一键严格 E2EE”、核验指纹并刷新 ContactCard DHT。</span>
      </div>
      <div class="row compact">
        <button class="secondary" @click="ctx.enableStrictE2eePolicy">一键严格 E2EE</button>
        <button class="secondary" @click="ctx.findActiveContactContactCard">刷新 ContactCard</button>
        <button v-if="!ctx.activeContact.value.fingerprint_verified_at" class="secondary" @click="ctx.showActiveContactFingerprintQr">指纹核验码</button>
      </div>
    </section>

    <section v-if="ctx.activeContact.value?.state === 'Friend' && ctx.activeContactSealedSlotRiskLevel.value === 'high'" class="chat-notice-panel">
      <div class="notice-text">
        <b>分设备加密降级风险</b>
        <span>{{ ctx.activeContactSealedSlotStatusText.value }}</span>
      </div>
    </section>

    <section v-if="ctx.activeGroup.value && ctx.activeGroupWarningText.value" class="chat-notice-panel">
      <div class="notice-text">
        <b>群聊发送检查</b>
        <span>{{ ctx.activeGroupWarningText.value }}</span>
      </div>
    </section>

    <section v-if="ctx.activeGroup.value && ctx.activeGroupStrictE2eeRiskText.value" class="chat-notice-panel">
      <div class="notice-text">
        <b>群聊严格 E2EE 风险</b>
        <span>{{ ctx.activeGroupStrictE2eeRiskText.value }}</span>
        <span>建议先修复群成员的指纹、ContactCard DHT 和 sealed slot 覆盖。</span>
      </div>
      <div class="row compact">
        <button class="secondary" @click="ctx.enableStrictE2eePolicy">一键严格 E2EE</button>
        <button class="secondary" @click="ctx.goContactsPage">修复联系人</button>
      </div>
    </section>

    <div class="messages clean-messages" ref="messagesEl" role="log" aria-label="消息列表" aria-live="polite">
      <template v-if="ctx.activeContact.value || ctx.activeGroup.value">
        <template v-for="item in thread" :key="item.id">
          <div v-if="item.kind === 'sep'" class="day-sep"><span>{{ item.label }}</span></div>
          <div v-else class="bubble" :class="item.m.direction">
            <small v-if="ctx.activeGroup.value && item.m.direction !== 'out'" class="bubble-sender">{{ contactName(item.m.peer_user_id) }}</small>
            <div class="text">{{ item.m.text }}</div>
            <small class="bubble-meta">
              <span class="status-pill" :class="messageStatusClass(item.m)" :title="messageStatusDetailText(item.m)">{{ messageStatusShortText(item.m) }}</span>
              <span>{{ hmTime(item.m.created_at) }}</span>
              <span v-if="mailboxWaitText(item.m)"> · {{ mailboxWaitText(item.m) }}</span>
              <span v-if="item.m.mailbox_delivery_id"> · delivery {{ shortMessageId(item.m.mailbox_delivery_id) }}</span>
              <span v-if="item.m.read_at"> · 已读 {{ ctx.formatDateTime(item.m.read_at) }}</span>
              <span v-else-if="item.m.delivered_at"> · 送达 {{ ctx.formatDateTime(item.m.delivered_at) }}</span>
              <span v-if="item.m.direction === 'out' && item.m.protocol_message_id"> · {{ shortMessageId(item.m.protocol_message_id) }}</span>
              <span v-if="item.m.direction === 'out' && ctx.perDeviceEnvelopeTargetCount(item.m)"> · 分设备 {{ ctx.perDeviceEnvelopeTargetCount(item.m) }}</span>
              <span v-if="item.m.file_downloaded_at"> · 已下载 {{ ctx.formatDateTime(item.m.file_downloaded_at) }}</span>
            </small>
          </div>
        </template>
        <div v-if="ctx.activeMessages.value.length === 0" class="empty center">还没有消息</div>
        <div v-else-if="thread.length === 0" class="empty center">没有匹配的消息</div>
      </template>

      <section v-else class="chat-empty-state">
        <h2>选择一个聊天</h2>
      </section>
    </div>

    <footer class="composer clean-composer" v-if="ctx.activeGroup.value || (ctx.activeContact.value && ctx.activeContact.value.state === 'Friend')">
      <textarea v-model="ctx.composerText.value" rows="3" aria-label="输入消息" placeholder="输入消息，Enter 发送 / Shift+Enter 换行" @keydown="onComposerKeydown" />
      <button @click="ctx.sendMessage">发送</button>
      <div v-if="ctx.activeContact.value" class="attachment-row">
        <input type="file" aria-label="选择附件" @change="ctx.onFileSelected" />
        <button class="secondary" :disabled="!ctx.selectedFile.value" @click="ctx.sendSelectedFile">发送文件</button>
        <button class="secondary danger" :disabled="!ctx.selectedFile.value && !ctx.filePackageText.value" @click="ctx.cancelSelectedFile">取消文件</button>
        <span class="file-transfer-phase">{{ ctx.fileTransferPhase.value }}</span>
        <small v-if="ctx.selectedFile.value">
          {{ ctx.selectedFile.value.name }} · {{ ctx.formatBytes(ctx.selectedFile.value.size) }}
          <b v-if="ctx.isDangerousFileName(ctx.selectedFile.value.name)">危险类型</b>
        </small>
        <small v-else>{{ ctx.rtcFileStatus.value }}</small>
        <small v-if="ctx.fileProgressText.value">{{ ctx.fileProgressText.value }}</small>
        <small v-if="activeFileOutboxError" class="outbox-error">文件发送失败：{{ activeFileOutboxError }}</small>
        <button v-if="activeFileOutboxError" class="secondary" @click="ctx.flushOutboxForActive">重试文件</button>
        <div v-if="ctx.pendingFilePackageText.value && !ctx.receivedFileUrl.value" class="received-file-card">
          <b>收到文件包</b>
          <small v-if="ctx.pendingFileMeta.value">{{ ctx.pendingFileMeta.value }}</small>
          <small>文件尚未解密，确认来源可信后再打开。</small>
          <button class="secondary" @click="ctx.decryptIncomingFilePackage">解密文件</button>
        </div>
        <div v-if="ctx.receivedFileUrl.value" class="received-file-card">
          <b>{{ ctx.receivedFileName.value }}</b>
          <small>{{ ctx.receivedFileMeta.value }}</small>
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
      </div>
    </footer>
  </section>
</template>
