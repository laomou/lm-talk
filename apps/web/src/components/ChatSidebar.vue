<script setup lang="ts">
import { computed, ref } from 'vue'
import { avatarColor } from '../avatarColor'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')
const searchOpen = ref(false)

function lastMessageFor(pred: (m: any) => boolean) {
  let last: any = null
  for (const m of props.ctx.messages.value) {
    if (!pred(m)) continue
    if (!last || m.created_at > last.created_at) last = m
  }
  return last
}

const conversations = computed(() => {
  const items: any[] = []
  for (const c of props.ctx.contacts.value) {
    const last = lastMessageFor((m) => !m.group_id && m.peer_user_id === c.user_id)
    items.push({ type: 'contact', id: c.user_id, data: c, last, ts: last?.created_at ?? 0 })
  }
  return items.sort((a, b) => b.ts - a.ts)
})

const filtered = computed(() => {
  const q = keyword.value.trim().toLowerCase()
  if (!q) return conversations.value
  return conversations.value.filter((it) => `${it.data.display_name || ''} ${it.id}`.toLowerCase().includes(q))
})

function convName(it: any) {
  return it.data.display_name || '未命名'
}
function trustBadgeText(it: any) {
  if (it.type !== 'contact' || it.data.state !== 'Friend') return ''
  return it.data.fingerprint_verified_at ? '✓' : '⚠️'
}
function trustBadgeTitle(it: any) {
  if (it.type !== 'contact' || it.data.state !== 'Friend') return ''
  return it.data.fingerprint_verified_at ? '指纹已核验' : '指纹未核验'
}
function convPreview(it: any) {
  if (it.last) {
    return it.last.text
  }
  if (it.data.state === 'RequestSent') return '等待对方通过'
  if (it.data.state === 'Blocked') return '已拉黑'
  if (it.data.state !== 'Friend') return '还不是好友'
  return '暂无消息'
}
function convTime(ts: number) {
  if (!ts) return ''
  const d = new Date(ts)
  const now = new Date()
  const sameDay = d.toDateString() === now.toDateString()
  if (sameDay) return new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false }).format(d)
  const yesterday = new Date(now)
  yesterday.setDate(now.getDate() - 1)
  if (d.toDateString() === yesterday.toDateString()) return '昨天'
  if (d.getFullYear() === now.getFullYear()) return `${d.getMonth() + 1}/${d.getDate()}`
  return `${d.getFullYear()}/${d.getMonth() + 1}/${d.getDate()}`
}
function isActive(it: any) {
  return it.id === props.ctx.activePeerId.value
}
function select(it: any) {
  props.ctx.selectContact(it.id)
}
</script>

<template>
  <aside class="sidebar wechat-sidebar">
    <header v-if="!searchOpen" class="list-col-header product-chat-list-header">
      <span></span>
      <h2>聊天</h2>
      <button class="icon-btn" aria-label="搜索聊天" title="搜索聊天" @click="searchOpen = true">🔍</button>
    </header>
    <header v-else class="list-col-header product-chat-search-header">
      <button class="back-btn" aria-label="返回聊天" @click="searchOpen = false">‹</button>
      <input v-model="keyword" type="search" aria-label="搜索聊天" placeholder="搜索聊天" autofocus />
    </header>

    <section class="conversation-list only-conversations">
      <button
        v-for="it in filtered"
        :key="it.type + ':' + it.id"
        class="contact"
        :class="{ active: isActive(it) }"
        :aria-current="isActive(it) ? 'true' : undefined"
        @click="select(it)"
      >
        <span class="avatar" :style="{ background: avatarColor(it.id) }">{{ (convName(it) || '?').slice(0, 1).toUpperCase() }}</span>
        <span class="contact-main">
          <b>
            <span class="conv-name">
              {{ convName(it) }}
              <em v-if="it.data.state === 'RequestSent'">等待通过</em>
              <em v-else-if="it.data.state === 'Blocked'">已拉黑</em>
              <em
                v-else-if="trustBadgeText(it)"
                class="strict-badge"
                :class="{ danger: !it.data.fingerprint_verified_at }"
                :title="trustBadgeTitle(it)"
              >{{ trustBadgeText(it) }}</em>
            </span>
            <span v-if="it.ts" class="conv-time">{{ convTime(it.ts) }}</span>
          </b>
          <small class="conv-preview">{{ convPreview(it) }}</small>
        </span>
      </button>

      <div v-if="filtered.length === 0" class="empty">
        {{ keyword ? '没有匹配的聊天' : '暂无聊天，去通讯录添加好友' }}
      </div>
    </section>
  </aside>
</template>
