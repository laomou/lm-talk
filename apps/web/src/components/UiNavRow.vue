<script setup lang="ts">
defineProps<{
  icon?: string
  badge?: number | string
}>()

defineEmits<{ click: [] }>()
</script>

<template>
  <button class="ui-nav-row" @click="$emit('click')">
    <span v-if="icon" class="ui-nav-row-icon">{{ icon }}</span>
    <span class="ui-nav-row-main"><slot /></span>
    <em v-if="badge" class="ui-nav-row-badge">{{ badge }}</em>
    <span class="chevron">›</span>
  </button>
</template>
