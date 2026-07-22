<script setup lang="ts">
withDefaults(defineProps<{
  align?: 'start' | 'end' | 'center'
  fullWidth?: boolean
}>(), {
  align: 'start',
  fullWidth: false,
})
</script>

<template>
  <div class="ui-action-group" :class="[align, { 'full-width': fullWidth }]"><slot /></div>
</template>
