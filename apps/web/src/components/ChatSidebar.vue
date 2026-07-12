<script setup lang="ts">
defineProps<{ ctx: any }>()
</script>

<template>
  <aside class="sidebar wechat-sidebar">
    <div class="me sidebar-profile compact-profile">
      <h2>{{ ctx.displayName.value }}</h2>
      <small>{{ ctx.identity.value?.user_id }}</small>
    </div>

    <section class="conversation-list only-conversations">
      <h3>聊天</h3>
      <button
        v-for="c in ctx.contacts.value"
        :key="c.user_id"
        class="contact"
        :class="{ active: c.user_id === ctx.activePeerId.value }"
        @click="ctx.selectContact(c.user_id)"
      >
        <span class="avatar">{{ (c.display_name || c.user_id || '?').slice(0, 1).toUpperCase() }}</span>
        <span class="contact-main">
          <b>{{ c.display_name || '未命名' }} <em>{{ c.state }}</em></b>
          <small>私聊 · {{ c.user_id }}</small>
        </span>
      </button>

      <button
        v-for="g in ctx.groups.value"
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

      <div v-if="ctx.contacts.value.length === 0 && ctx.groups.value.length === 0" class="empty">暂无聊天</div>
    </section>
  </aside>
</template>
