<script setup lang="ts">
const props = defineProps<{ ctx: any }>()
const contactName = (userId: string) => props.ctx.contacts.value.find((c: any) => c.user_id === userId)?.display_name || userId
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
        <button v-if="ctx.activeContact.value.state !== 'RequestSent' && ctx.activeContact.value.state !== 'Blocked'" @click="ctx.createFriendRequestForActive">发送好友请求</button>
        <button v-if="ctx.activeContact.value.state === 'Blocked'" @click="ctx.unblockActiveContact">解除拉黑</button>
      </div>

    </section>

    <div class="messages clean-messages">
      <template v-if="ctx.activeContact.value || ctx.activeGroup.value">
        <div v-for="m in ctx.activeMessages.value" :key="m.id" class="bubble" :class="m.direction">
          <div class="text">{{ m.text }}</div>
          <small>{{ m.direction === 'out' ? '我' : contactName(m.peer_user_id) }} · {{ ctx.formatTime(m.created_at) }} · {{ ctx.statusLabel(m.status) }}</small>

        </div>
        <div v-if="ctx.activeMessages.value.length === 0" class="empty center">还没有消息</div>
      </template>

      <section v-else class="chat-empty-state">
        <h2>选择一个聊天</h2>
      </section>
    </div>

    <footer class="composer clean-composer" v-if="ctx.activeGroup.value || (ctx.activeContact.value && ctx.activeContact.value.state === 'Friend')">
      <textarea v-model="ctx.composerText.value" rows="3" placeholder="输入消息" />
      <button @click="ctx.sendMessage">发送</button>

    </footer>
  </section>
</template>
