<script setup lang="ts">
import UiCard from './UiCard.vue'

defineProps<{
  title?: string
  description?: string
}>()
</script>

<template>
  <UiCard class="ui-section">
    <header v-if="title || $slots.title || $slots.actions" class="ui-section-header">
      <div class="ui-section-heading">
        <slot name="title"><h3 v-if="title">{{ title }}</h3></slot>
        <small v-if="description">{{ description }}</small>
      </div>
      <div v-if="$slots.actions" class="ui-section-actions"><slot name="actions" /></div>
    </header>
    <slot />
  </UiCard>
</template>
