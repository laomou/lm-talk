<script setup lang="ts">
import UiIcon from './UiIcon.vue'

withDefaults(defineProps<{
  danger?: boolean
  ariaLabel?: string
}>(), {
  danger: false,
  ariaLabel: undefined,
})

const emit = defineEmits<{ click: [] }>()
</script>

<template>
  <button class="ui-list-row" :class="{ 'is-danger': danger }" :aria-label="ariaLabel" @click="emit('click')">
    <span class="ui-list-row-main"><slot /></span>
    <span class="ui-list-row-end">
      <slot name="end"><UiIcon name="next" /></slot>
    </span>
  </button>
</template>
