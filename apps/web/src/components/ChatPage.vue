<script setup lang="ts">
import ChatSidebar from './ChatSidebar.vue'
import ChatMain from './ChatMain.vue'
import { computed } from 'vue'
import { useRoute } from 'vue-router'

const props = defineProps<{ ctx: any }>()
const route = useRoute()
const isConversationSearch = computed(() => route.path === '/chat/search/messages')
const showChatList = computed(() => route.path === '/chat/search' || (!isConversationSearch.value && !props.ctx.activePeerId.value))
</script>

<template>
  <div class="chat-shell single-column-chat">
    <ChatSidebar v-if="showChatList" :ctx="ctx" />
    <ChatMain v-else :ctx="ctx" />
  </div>
</template>
