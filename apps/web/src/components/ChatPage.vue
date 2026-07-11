<script setup lang="ts">
import ChatSidebar from './ChatSidebar.vue'
import ChatMain from './ChatMain.vue'

defineProps<{ ctx: any }>()
</script>

<template>
  <div class="chat-shell">
    <ChatSidebar :ctx="ctx" />
    <ChatMain :ctx="ctx" />
  </div>
</template>
