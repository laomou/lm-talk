<script setup lang="ts">
withDefaults(defineProps<{
  variant?: 'default' | 'compact' | 'inset'
}>(), {
  variant: 'default',
})
</script>

<template>
  <section class="ui-card" :class="variant">
    <slot />
  </section>
</template>
