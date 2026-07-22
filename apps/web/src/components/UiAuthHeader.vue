<script setup lang="ts">
defineProps<{
  title: string
  description: string
}>()
</script>

<template>
  <header class="ui-auth-header">
    <span class="ui-auth-mark" aria-hidden="true">LM</span>
    <div>
      <p class="ui-auth-product">LM Talk</p>
      <h1>{{ title }}</h1>
      <p>{{ description }}</p>
    </div>
  </header>
</template>
