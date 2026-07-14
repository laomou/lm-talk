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

const activeOutboxError = computed(() => {
  const peerId = props.ctx.activeContact.value?.user_id
  if (!peerId) return ''
  const failed = props.ctx.outbox.value
    .filter((item: any) => item.peer_user_id === peerId && item.status === 'failed' && item.last_error)
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
        <button v-if="activePendingOutboxCount" class="secondary" @click="ctx.flushOutboxForActive">重发 {{ activePendingOutboxCount }}</button>
        <button v-if="activePendingOutboxCount" class="secondary danger" @click="ctx.cancelOutboxForActive">取消发送</button>
        <button class="secondary danger" @click="ctx.clearActiveConversation">清空聊天</button>
      </div>
    </header>

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
        <span>发送好友请求，对方通过后即可开始聊天。</span>
      </div>
      <div class="row compact">
        <button v-if="ctx.activeContact.value.state === 'RequestSent'" class="secondary" @click="ctx.createFriendRequestForActive">重新发送</button>
        <button v-if="ctx.activeContact.value.state !== 'RequestSent' && ctx.activeContact.value.state !== 'Blocked'" @click="ctx.createFriendRequestForActive">发送好友请求</button>
        <button v-if="ctx.activeContact.value.state === 'Blocked'" @click="ctx.unblockActiveContact">解除拉黑</button>
      </div>

    </section>

    <section v-if="ctx.activeGroup.value && ctx.activeGroupWarningText.value" class="chat-notice-panel">
      <div class="notice-text">
        <b>群聊发送检查</b>
        <span>{{ ctx.activeGroupWarningText.value }}</span>
      </div>
    </section>

    <div class="messages clean-messages" ref="messagesEl" role="log" aria-label="消息列表" aria-live="polite">
      <template v-if="ctx.activeContact.value || ctx.activeGroup.value">
        <template v-for="item in thread" :key="item.id">
          <div v-if="item.kind === 'sep'" class="day-sep"><span>{{ item.label }}</span></div>
          <div v-else class="bubble" :class="item.m.direction">
            <small v-if="ctx.activeGroup.value && item.m.direction !== 'out'" class="bubble-sender">{{ contactName(item.m.peer_user_id) }}</small>
            <div class="text">{{ item.m.text }}</div>
            <small class="bubble-meta">{{ hmTime(item.m.created_at) }} · {{ ctx.statusLabel(item.m.status) }}</small>
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
        <div v-if="ctx.pendingFilePackageText.value && !ctx.receivedFileUrl.value" class="received-file-card">
          <b>收到文件包</b>
          <small>文件尚未解密，确认来源可信后再打开。</small>
          <button class="secondary" @click="ctx.decryptIncomingFilePackage">解密文件</button>
        </div>
        <div v-if="ctx.receivedFileUrl.value" class="received-file-card">
          <b>{{ ctx.receivedFileName.value }}</b>
          <small>{{ ctx.receivedFileMeta.value }}</small>
          <a :href="ctx.receivedFileUrl.value" :download="ctx.receivedFileName.value">下载</a>
          <img
            v-if="ctx.receivedFileMime.value.startsWith('image/')"
            class="received-file-preview"
            :src="ctx.receivedFileUrl.value"
            :alt="ctx.receivedFileName.value"
          />
        </div>
      </div>
    </footer>
  </section>
</template>
