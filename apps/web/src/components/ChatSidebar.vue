<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { avatarColor } from '../avatarColor'
import { useI18n } from 'vue-i18n'
import UiIcon from './UiIcon.vue'
import UiEmptyState from './UiEmptyState.vue'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')
const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const searchOpen = computed(() => route.path === '/chat/search')

const conversationStats = computed(() => {
  const lastByUser = new Map<string, any>()
  const unreadByUser = new Map<string, number>()
  for (const message of props.ctx.messages.value) {
    if (message.group_id) continue
    const prev = lastByUser.get(message.peer_user_id)
    if (!prev || message.created_at > prev.created_at || (message.created_at === prev.created_at && String(message.id) > String(prev.id))) {
      lastByUser.set(message.peer_user_id, message)
    }
    if (message.direction === 'in' && !message.read_at) {
      unreadByUser.set(message.peer_user_id, (unreadByUser.get(message.peer_user_id) ?? 0) + 1)
    }
  }
  return { lastByUser, unreadByUser }
})

const conversations = computed(() => {
  const items: any[] = []
  for (const c of props.ctx.contacts.value) {
    const last = conversationStats.value.lastByUser.get(c.user_id)
    const unread = conversationStats.value.unreadByUser.get(c.user_id) ?? 0
    const isActive = c.user_id === props.ctx.activePeerId.value
    if (!last && !isActive) continue
    items.push({ type: 'contact', id: c.user_id, data: c, last, unread, ts: last?.created_at ?? 0 })
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
function trustBadgeIcon(it: any): 'alert' | 'lock' {
  return props.ctx.contactAllKnownDevicesRevoked(it.data) ? 'alert' : 'lock'
}
function trustBadgeTitle(it: any) {
  if (it.type !== 'contact' || it.data.state !== 'Friend') return ''
  return props.ctx.contactAllKnownDevicesRevoked(it.data) ? t('securityStatus.abnormal') : t('securityStatus.normal')
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
function unreadCount(it: any) {
  return it.type === 'contact' ? Number(it.unread ?? 0) : 0
}
function select(it: any) {
  props.ctx.selectContact(it.id)
}
</script>

<template>
  <aside v-if="!searchOpen" class="sidebar wechat-sidebar">
    <header class="list-col-header product-chat-list-header">
      <span></span>
      <h2>聊天</h2>
      <button class="icon-btn" aria-label="搜索聊天" title="搜索聊天" @click="router.push('/chat/search')"><UiIcon name="search" /></button>
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
        <span class="conversation-avatar-wrap">
          <span class="avatar" :style="{ background: avatarColor(it.id) }">{{ (convName(it) || '?').slice(0, 1).toUpperCase() }}</span>
          <em v-if="unreadCount(it)" class="conversation-avatar-badge">{{ props.ctx.badgeCountText(unreadCount(it)) }}</em>
        </span>
        <span class="contact-main">
          <b>
            <span class="conv-name">
              {{ convName(it) }}
              <em v-if="it.data.state === 'RequestSent'">等待通过</em>
              <em v-else-if="it.data.state === 'Blocked'">已拉黑</em>
              <em
                v-else-if="it.type === 'contact' && it.data.state === 'Friend'"
                class="strict-badge"
                :class="{ danger: props.ctx.contactAllKnownDevicesRevoked(it.data) }"
                :title="trustBadgeTitle(it)"
              ><UiIcon :name="trustBadgeIcon(it)" size="12" /></em>
            </span>
            <span v-if="it.ts" class="conv-time">{{ convTime(it.ts) }}</span>
          </b>
          <small class="conv-preview">
            <span>{{ convPreview(it) }}</span>
            <em v-if="unreadCount(it)" class="conversation-badge">{{ props.ctx.badgeCountText(unreadCount(it)) }}</em>
          </small>
        </span>
      </button>

      <UiEmptyState v-if="filtered.length === 0" title="暂无聊天" description="去通讯录添加好友后开始聊天。" />
    </section>
  </aside>

  <aside v-else class="sidebar wechat-sidebar chat-search-page">
    <header class="list-col-header product-chat-search-header">
      <button class="back-btn" aria-label="返回聊天" @click="router.push('/chat')"><UiIcon name="back" /></button>
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
        <span class="conversation-avatar-wrap">
          <span class="avatar" :style="{ background: avatarColor(it.id) }">{{ (convName(it) || '?').slice(0, 1).toUpperCase() }}</span>
          <em v-if="unreadCount(it)" class="conversation-avatar-badge">{{ props.ctx.badgeCountText(unreadCount(it)) }}</em>
        </span>
        <span class="contact-main">
          <b>
            <span class="conv-name">
              {{ convName(it) }}
              <em v-if="it.data.state === 'RequestSent'">等待通过</em>
              <em v-else-if="it.data.state === 'Blocked'">已拉黑</em>
              <em
                v-else-if="it.type === 'contact' && it.data.state === 'Friend'"
                class="strict-badge"
                :class="{ danger: props.ctx.contactAllKnownDevicesRevoked(it.data) }"
                :title="trustBadgeTitle(it)"
              ><UiIcon :name="trustBadgeIcon(it)" size="12" /></em>
            </span>
            <span v-if="it.ts" class="conv-time">{{ convTime(it.ts) }}</span>
          </b>
          <small class="conv-preview">
            <span>{{ convPreview(it) }}</span>
            <em v-if="unreadCount(it)" class="conversation-badge">{{ props.ctx.badgeCountText(unreadCount(it)) }}</em>
          </small>
        </span>
      </button>

      <UiEmptyState v-if="filtered.length === 0" :icon="keyword ? 'search' : 'info'" :title="keyword ? '没有匹配的聊天' : '搜索聊天'" :description="keyword ? '换个名称或关键词试试。' : '输入名称搜索聊天。'" />
    </section>
  </aside>
</template>
