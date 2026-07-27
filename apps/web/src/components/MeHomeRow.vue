<script setup lang="ts">
import UiListRow from './UiListRow.vue'
import UiStatusBadge from './UiStatusBadge.vue'

defineProps<{
  label: string
  statusText?: string
  statusTone?: 'neutral' | 'success' | 'warning' | 'danger'
  danger?: boolean
  ariaLabel?: string
}>()

defineEmits<{ click: [] }>()
</script>

<template>
  <UiListRow :danger="danger" :aria-label="ariaLabel" @click="$emit('click')">
    {{ label }}
    <template v-if="statusText" #end>
      <UiStatusBadge compact :tone="statusTone || 'neutral'">{{ statusText }}</UiStatusBadge>
      <span class="chevron">›</span>
    </template>
  </UiListRow>
</template>
