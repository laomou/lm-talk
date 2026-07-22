<script setup lang="ts">
import UiIcon from './UiIcon.vue'

withDefaults(defineProps<{
  tone?: 'info' | 'warning' | 'danger' | 'success'
  compact?: boolean
}>(), {
  tone: 'warning',
  compact: false,
})
</script>

<template>
  <section class="ui-notice" :class="[tone, { compact }]">
    <div class="ui-notice-icon" aria-hidden="true">
      <UiIcon :name="tone === 'success' ? 'check' : tone === 'info' ? 'info' : 'alert'" size="16" />
    </div>
    <div class="ui-notice-content">
      <slot />
    </div>
    <div v-if="$slots.actions" class="ui-notice-actions">
      <slot name="actions" />
    </div>
  </section>
</template>
