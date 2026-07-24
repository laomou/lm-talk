<script setup lang="ts">
defineProps<{
  label?: string
  forId?: string
  hint?: string
  error?: string
}>()
</script>

<template>
  <div class="ui-field">
    <label v-if="label" :for="forId">{{ label }}</label>
    <slot />
    <small v-if="error" class="ui-field-error">{{ error }}</small>
    <small v-else-if="hint" class="ui-field-hint">{{ hint }}</small>
  </div>
</template>
