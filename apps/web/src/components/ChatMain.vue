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
        <span>发送好友请求，等待对方通过后开始聊天。</span>
      </div>
      <div class="row compact">
        <button v-if="ctx.activeContact.value.state !== 'RequestSent' && ctx.activeContact.value.state !== 'Blocked'" @click="ctx.createFriendRequestForActive">发送好友请求</button>
        <button v-if="ctx.activeContact.value.state === 'Blocked'" @click="ctx.unblockActiveContact">解除拉黑</button>
      </div>
      <details class="inline-details">
        <summary>离线添加</summary>
        <p class="hint">没有开启消息同步时，可复制请求发给对方，再粘贴对方返回的响应。</p>
        <button v-if="ctx.friendRequestText.value" @click="ctx.copyText(ctx.friendRequestText.value, '好友请求')">复制请求</button>
        <textarea v-model="ctx.incomingFriendResponseText.value" rows="3" placeholder="粘贴好友响应" />
        <button @click="ctx.applyFriendResponse">应用响应</button>
      </details>
    </section>

    <div class="messages clean-messages">
      <template v-if="ctx.activeContact.value || ctx.activeGroup.value">
        <div v-for="m in ctx.activeMessages.value" :key="m.id" class="bubble" :class="m.direction">
          <div class="text">{{ m.text }}</div>
          <small>{{ m.direction === 'out' ? '我' : contactName(m.peer_user_id) }} · {{ ctx.formatTime(m.created_at) }} · {{ ctx.statusLabel(m.status) }}</small>
          <div class="bubble-actions" v-if="m.envelope_json">
            <button @click="ctx.copyMessageEnvelope(m)">复制密文</button>
            <button @click="ctx.showQr(m.envelope_json, 'Envelope')">二维码</button>
          </div>
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
      <details v-if="ctx.activeContact.value" class="receive-inline">
        <summary>手动接收密文</summary>
        <textarea v-model="ctx.inboundEnvelopeText.value" rows="4" placeholder="粘贴对方发来的密文" />
        <button @click="ctx.receiveEnvelope">解密并加入聊天</button>
      </details>
    </footer>
  </section>
</template>
