<script setup lang="ts">
import UiPageHeader from './UiPageHeader.vue'

defineProps<{
  title: string
  backLabel: string
}>()

defineEmits<{ back: [] }>()
</script>

<template>
  <UiPageHeader :title="title" :back-label="backLabel" @back="$emit('back')">
    <template v-if="$slots.end" #end><slot name="end" /></template>
  </UiPageHeader>
</template>
