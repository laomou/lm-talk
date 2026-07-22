<script setup lang="ts">
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
      <slot name="end"><span class="chevron">›</span></slot>
    </span>
  </button>
</template>
