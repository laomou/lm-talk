<script setup lang="ts">
import UiIcon from './UiIcon.vue'

withDefaults(defineProps<{
  title: string
  kind?: 'info' | 'success' | 'warning' | 'error'
  alert?: boolean
}>(), {
  kind: 'info',
  alert: false,
})

const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <div class="dialog-mask" @click.self="emit('close')">
    <section
      class="dialog-card"
      :class="kind"
      :role="alert ? 'alertdialog' : 'dialog'"
      aria-modal="true"
      :aria-label="title"
    >
      <header class="dialog-card-header">
        <span class="dialog-kind-icon" :class="kind"><UiIcon :name="kind === 'success' ? 'check' : kind === 'info' ? 'info' : 'alert'" /></span>
        <h2>{{ title }}</h2>
        <button class="icon-btn" aria-label="关闭对话框" title="关闭" @click="emit('close')"><UiIcon name="close" /></button>
      </header>
      <div class="dialog-card-body">
        <slot />
      </div>
      <footer v-if="$slots.actions" class="dialog-actions">
        <slot name="actions" />
      </footer>
    </section>
  </div>
</template>
