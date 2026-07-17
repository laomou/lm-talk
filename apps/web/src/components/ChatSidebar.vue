<script setup lang="ts">
import { computed, ref } from 'vue'
import { avatarColor } from '../avatarColor'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')

const contactName = (userId: string) =>
  props.ctx.contacts.value.find((c: any) => c.user_id === userId)?.display_name || userId

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
  for (const g of props.ctx.groups.value) {
    const last = lastMessageFor((m) => m.group_id === g.group_id)
    items.push({ type: 'group', id: g.group_id, data: g, last, ts: last?.created_at ?? 0 })
  }
  return items.sort((a, b) => b.ts - a.ts)
})

const filtered = computed(() => {
  const q = keyword.value.trim().toLowerCase()
  if (!q) return conversations.value
  return conversations.value.filter((it) => {
    const name = it.type === 'contact' ? it.data.display_name || it.data.user_id : it.data.name
    return `${name || ''} ${it.id}`.toLowerCase().includes(q)
  })
})

function convName(it: any) {
  return it.type === 'contact' ? it.data.display_name || '未命名' : it.data.name
}
function strictBadgeText(it: any) {
  if (it.type !== 'contact' || it.data.state !== 'Friend') return ''
  return props.ctx.contactStrictE2eeRiskLevel(it.data) === 'high' ? '严格风险' : '严格就绪'
}
function strictBadgeTitle(it: any) {
  if (it.type !== 'contact' || it.data.state !== 'Friend') return ''
  return props.ctx.contactStrictE2eeStatusText(it.data)
}
function convPreview(it: any) {
  if (it.last) {
    if (it.type === 'group' && it.last.direction !== 'out') return `${contactName(it.last.peer_user_id)}：${it.last.text}`
    return it.last.text
  }
  if (it.type === 'contact') {
    if (it.data.state === 'RequestSent') return '等待对方通过'
    if (it.data.state === 'Blocked') return '已拉黑'
    if (it.data.state !== 'Friend') return '还不是好友'
    return '暂无消息'
  }
  return `${it.data.member_user_ids.length} 人的群聊`
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
  return it.type === 'contact' ? it.id === props.ctx.activePeerId.value : it.id === props.ctx.activeGroupId.value
}
function select(it: any) {
  if (it.type === 'contact') props.ctx.selectContact(it.id)
  else props.ctx.selectGroup(it.id)
}
</script>

<template>
  <aside class="sidebar wechat-sidebar">
    <header class="list-col-header">
      <h2>聊天</h2>
    </header>
    <div class="list-col-search">
      <input v-model="keyword" type="search" aria-label="搜索聊天" placeholder="搜索聊天" />
    </div>

    <section class="conversation-list only-conversations">
      <button
        v-for="it in filtered"
        :key="it.type + ':' + it.id"
        class="contact"
        :class="{ active: isActive(it) }"
        :aria-current="isActive(it) ? 'true' : undefined"
        @click="select(it)"
      >
        <span v-if="it.type === 'group'" class="avatar group-avatar">群</span>
        <span v-else class="avatar" :style="{ background: avatarColor(it.id) }">{{ (convName(it) || '?').slice(0, 1).toUpperCase() }}</span>
        <span class="contact-main">
          <b>
            <span class="conv-name">
              {{ convName(it) }}
              <em v-if="it.type === 'contact' && it.data.state === 'RequestSent'">等待通过</em>
              <em v-else-if="it.type === 'contact' && it.data.state === 'Blocked'">已拉黑</em>
              <em
                v-else-if="strictBadgeText(it)"
                class="strict-badge"
                :class="{ danger: props.ctx.contactStrictE2eeRiskLevel(it.data) === 'high' }"
                :title="strictBadgeTitle(it)"
              >{{ strictBadgeText(it) }}</em>
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
