<script setup lang="ts">
defineProps<{
  title: string
  description: string
}>()
</script>

<template>
  <div class="diagnostic-row">
    <span>
      <b>{{ title }}</b>
      <small>{{ description }}</small>
    </span>
    <slot name="value" />
  </div>
</template>
