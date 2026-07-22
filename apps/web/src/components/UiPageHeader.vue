<script setup lang="ts">
import UiIcon from './UiIcon.vue'

defineProps<{
  title?: string
  backLabel?: string
}>()

const emit = defineEmits<{ back: [] }>()
</script>

<template>
  <header class="ui-page-header">
    <div class="ui-page-header-start">
      <button v-if="backLabel" class="back-btn" :aria-label="backLabel" @click="emit('back')"><UiIcon name="back" /></button>
      <slot v-else name="start" />
    </div>
    <div class="ui-page-header-title">
      <slot name="title"><h2>{{ title }}</h2></slot>
    </div>
    <div class="ui-page-header-end">
      <slot name="end" />
    </div>
  </header>
</template>
