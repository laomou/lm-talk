<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import UiIcon from './UiIcon.vue'
import UiEmptyState from './UiEmptyState.vue'
import UiAvatar from './UiAvatar.vue'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')
const route = useRoute()
const router = useRouter()
const { locale, t } = useI18n()
const searchOpen = computed(() => route.path === '/chat/search')

type DeliveryTone = 'failed' | 'pending'
type ConversationItem = {
  type: 'contact'
  id: string
  data: any
  active: boolean
  name: string
  preview: string
  time: string
  ts: number
  lastId: string
  unread: number
  unreadText: string
  pending: number
  failed: number
  deliveryText: string
  deliveryTone: DeliveryTone | ''
  trustIcon: 'alert' | 'lock'
  trustTitle: string
  trustRevoked: boolean
}

const conversationStats = computed(() => {
  const lastByUser = new Map<string, any>()
  const unreadByUser = new Map<string, number>()
  const pendingByUser = new Map<string, number>()
  const failedByUser = new Map<string, number>()
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
  for (const item of props.ctx.outbox.value) {
    if (item.status === 'sent') continue
    const target = item.status === 'failed' ? failedByUser : pendingByUser
    target.set(item.peer_user_id, (target.get(item.peer_user_id) ?? 0) + 1)
  }
  return { lastByUser, unreadByUser, pendingByUser, failedByUser }
})

function formatConversationTime(ts: number) {
  if (!ts) return ''
  const d = new Date(ts)
  const now = new Date()
  const sameDay = d.toDateString() === now.toDateString()
  if (sameDay) return new Intl.DateTimeFormat(locale.value, { hour: '2-digit', minute: '2-digit', hour12: false }).format(d)
  const yesterday = new Date(now)
  yesterday.setDate(now.getDate() - 1)
  if (d.toDateString() === yesterday.toDateString()) return t('chatView.yesterday')
  if (d.getFullYear() === now.getFullYear()) return new Intl.DateTimeFormat(locale.value, { month: 'numeric', day: 'numeric' }).format(d)
  return new Intl.DateTimeFormat(locale.value, { year: 'numeric', month: 'numeric', day: 'numeric' }).format(d)
}

function conversationPreview(contact: any, last: any) {
  if (last) return last.text
  if (contact.state === 'RequestSent') return t('chatView.waitingApproval')
  if (contact.state === 'Blocked') return t('chatView.blocked')
  if (contact.state !== 'Friend') return t('chatView.notFriend')
  return t('chatView.noPreview')
}

const conversations = computed<ConversationItem[]>(() => {
  const stats = conversationStats.value
  const activePeerId = props.ctx.activePeerId.value
  const items: ConversationItem[] = []
  for (const contact of props.ctx.contacts.value) {
    const last = stats.lastByUser.get(contact.user_id)
    const unread = stats.unreadByUser.get(contact.user_id) ?? 0
    const pending = stats.pendingByUser.get(contact.user_id) ?? 0
    const failed = stats.failedByUser.get(contact.user_id) ?? 0
    const active = contact.user_id === activePeerId
    if (!last && !active && !pending && !failed) continue
    const trustRevoked = contact.state === 'Friend' ? props.ctx.contactAllKnownDevicesRevoked(contact) : false
    const deliveryTone: DeliveryTone | '' = failed ? 'failed' : pending ? 'pending' : ''
    items.push({
      type: 'contact',
      id: contact.user_id,
      data: contact,
      active,
      name: contact.display_name || t('chatView.unnamed'),
      preview: conversationPreview(contact, last),
      time: formatConversationTime(last?.created_at ?? 0),
      ts: last?.created_at ?? 0,
      lastId: last?.id ?? '',
      unread,
      unreadText: unread ? props.ctx.badgeCountText(unread) : '',
      pending,
      failed,
      deliveryText: failed
        ? t('chatView.conversationFailed', { count: failed })
        : pending
          ? t('chatView.conversationPending', { count: pending })
          : '',
      deliveryTone,
      trustIcon: trustRevoked ? 'alert' : 'lock',
      trustTitle: contact.state === 'Friend' ? (trustRevoked ? t('securityStatus.abnormal') : t('securityStatus.normal')) : '',
      trustRevoked,
    })
  }
  return items.sort((a, b) => b.ts - a.ts)
})

const filtered = computed(() => {
  const q = keyword.value.trim().toLowerCase()
  if (!q) return conversations.value
  return conversations.value.filter((it) => `${it.name} ${it.id}`.toLowerCase().includes(q))
})

function select(it: ConversationItem) {
  props.ctx.selectContact(it.id)
  void router.push(`/chat/${encodeURIComponent(it.id)}`)
}
function selectOnKeydown(event: KeyboardEvent, it: ConversationItem) {
  if (event.key !== 'Enter' && event.key !== ' ') return
  event.preventDefault()
  select(it)
}
function retryConversation(event: Event, it: ConversationItem) {
  event.stopPropagation()
  props.ctx.retryOutboxForPeer(it.id)
}
</script>

<template>
  <aside class="sidebar wechat-sidebar" :class="{ 'chat-search-page': searchOpen }">
    <header v-if="!searchOpen" class="list-col-header product-chat-list-header">
      <span></span>
      <h2>{{ t('chatView.title') }}</h2>
      <button class="icon-btn" :aria-label="t('chatView.searchChat')" :title="t('chatView.searchChat')" @click="router.push('/chat/search')"><UiIcon name="search" /></button>
    </header>
    <header v-else class="list-col-header product-chat-search-header">
      <button class="back-btn" :aria-label="t('chatView.backToChat')" @click="router.push('/chat')"><UiIcon name="back" /></button>
      <input v-model="keyword" type="search" :aria-label="t('chatView.searchChat')" :placeholder="t('chatView.searchChat')" autofocus />
    </header>

    <section class="conversation-list only-conversations">
      <div
        v-for="it in filtered"
        :key="it.type + ':' + it.id"
        v-memo="[it.active, it.lastId, it.unread, it.pending, it.failed, it.name, it.data.avatar_data_url, it.data.state, it.trustRevoked, it.time]"
        class="contact"
        :class="{ active: it.active }"
        role="button"
        tabindex="0"
        :aria-current="it.active ? 'true' : undefined"
        @click="select(it)"
        @keydown="selectOnKeydown($event, it)"
      >
        <span class="conversation-avatar-wrap">
          <UiAvatar :src="it.data.avatar_data_url" :name="it.name" :seed="it.id" />
          <em v-if="it.unread" class="conversation-avatar-badge">{{ it.unreadText }}</em>
        </span>
        <span class="contact-main">
          <b>
            <span class="conv-name">
              {{ it.name }}
              <em v-if="it.data.state === 'RequestSent'">{{ t('chatView.waitingApprovalBadge') }}</em>
              <em v-else-if="it.data.state === 'Blocked'">{{ t('chatView.blocked') }}</em>
              <em
                v-else-if="it.data.state === 'Friend'"
                class="strict-badge"
                :class="{ danger: it.trustRevoked }"
                :title="it.trustTitle"
              ><UiIcon :name="it.trustIcon" size="12" /></em>
            </span>
            <span v-if="it.time" class="conv-time">{{ it.time }}</span>
          </b>
          <small class="conv-preview">
            <span>{{ it.preview }}</span>
            <button
              v-if="it.deliveryTone === 'failed'"
              class="conversation-retry"
              type="button"
              :title="it.deliveryText"
              :aria-label="t('chatView.retryConversation')"
              @click="retryConversation($event, it)"
            >{{ t('chatView.retry') }}</button>
            <em v-else-if="it.deliveryTone" class="conversation-delivery" :class="`is-${it.deliveryTone}`">{{ it.deliveryText }}</em>
            <em v-if="it.unread" class="conversation-badge">{{ it.unreadText }}</em>
          </small>
        </span>
      </div>

      <UiEmptyState
        v-if="filtered.length === 0"
        :icon="searchOpen && keyword ? 'search' : undefined"
        :title="searchOpen && keyword ? t('chatView.noChatMatchesTitle') : searchOpen ? t('chatView.searchChat') : t('chatView.noChatsTitle')"
        :description="searchOpen && keyword ? t('chatView.noChatMatchesDescription') : searchOpen ? t('chatView.searchChatDescription') : t('chatView.noChatsDescription')"
      />
    </section>
  </aside>
</template>
