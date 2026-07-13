<script setup lang="ts">
import { computed, ref } from 'vue'
import { avatarColor } from '../avatarColor'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')
const query = computed(() => keyword.value.trim().toLowerCase())
const contacts = computed(() => {
  const q = query.value
  const list = props.ctx.contacts.value
  if (!q) return list
  return list.filter((c: any) => `${c.display_name || ''} ${c.user_id || ''}`.toLowerCase().includes(q))
})
const groups = computed(() => {
  const q = query.value
  const list = props.ctx.groups.value
  if (!q) return list
  return list.filter((g: any) => `${g.name || ''}`.toLowerCase().includes(q))
})
</script>

<template>
  <aside class="sidebar wechat-sidebar">
    <header class="list-col-header">
      <h2>聊天</h2>
    </header>
    <div class="list-col-search">
      <input v-model="keyword" placeholder="搜索聊天" />
    </div>

    <section class="conversation-list only-conversations">
      <button
        v-for="c in contacts"
        :key="c.user_id"
        class="contact"
        :class="{ active: c.user_id === ctx.activePeerId.value }"
        @click="ctx.selectContact(c.user_id)"
      >
        <span class="avatar" :style="{ background: avatarColor(c.user_id) }">{{ (c.display_name || c.user_id || '?').slice(0, 1).toUpperCase() }}</span>
        <span class="contact-main">
          <b>{{ c.display_name || '未命名' }} <em v-if="c.state === 'RequestSent'">等待通过</em><em v-else-if="c.state === 'Blocked'">已拉黑</em></b>
          <small>私聊 · {{ c.user_id }}</small>
        </span>
      </button>

      <button
        v-for="g in groups"
        :key="g.group_id"
        class="contact"
        :class="{ active: g.group_id === ctx.activeGroupId.value }"
        @click="ctx.selectGroup(g.group_id)"
      >
        <span class="avatar group-avatar">群</span>
        <span class="contact-main">
          <b>{{ g.name }}</b>
          <small>群聊 · {{ g.member_user_ids.length }} 人</small>
        </span>
      </button>

      <div v-if="contacts.length === 0 && groups.length === 0" class="empty">
        {{ query ? '没有匹配的聊天' : '暂无聊天，去通讯录添加好友' }}
      </div>
    </section>
  </aside>
</template>
