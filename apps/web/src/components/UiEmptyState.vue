<script setup lang="ts">
import UiIcon from './UiIcon.vue'

withDefaults(defineProps<{
  title: string
  description?: string
  icon?: 'search' | 'info' | 'alert'
}>(), {
  description: '',
  icon: 'info',
})
</script>

<template>
  <section class="ui-empty-state">
    <span class="ui-empty-state-icon"><UiIcon :name="icon" /></span>
    <div>
      <h3>{{ title }}</h3>
      <p v-if="description">{{ description }}</p>
    </div>
    <div v-if="$slots.actions" class="ui-empty-state-actions"><slot name="actions" /></div>
  </section>
</template>
