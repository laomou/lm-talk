<script setup lang="ts">
withDefaults(defineProps<{
  tone?: 'neutral' | 'success' | 'warning' | 'danger' | 'info' | 'ok'
  compact?: boolean
}>(), {
  tone: 'neutral',
  compact: false,
})
</script>

<template>
  <span class="ui-status-badge" :class="[tone, { compact }]"><slot /></span>
</template>
