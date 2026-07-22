<script setup lang="ts">
import ChatSidebar from './ChatSidebar.vue'
import ChatMain from './ChatMain.vue'

defineProps<{ ctx: any }>()
</script>

<template>
  <div class="chat-shell single-column-chat">
    <ChatSidebar v-if="!ctx.activePeerId.value" :ctx="ctx" />
    <ChatMain v-else :ctx="ctx" />
  </div>
</template>
