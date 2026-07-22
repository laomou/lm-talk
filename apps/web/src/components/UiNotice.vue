<script setup lang="ts">
withDefaults(defineProps<{
  tone?: 'info' | 'warning' | 'danger' | 'success'
}>(), {
  tone: 'warning',
})
</script>

<template>
  <section class="ui-notice" :class="tone">
    <div class="ui-notice-icon" aria-hidden="true">
      {{ tone === 'success' ? '✓' : tone === 'danger' ? '!' : tone === 'info' ? 'i' : '!' }}
    </div>
    <div class="ui-notice-content">
      <slot />
    </div>
    <div v-if="$slots.actions" class="ui-notice-actions">
      <slot name="actions" />
    </div>
  </section>
</template>
