<script setup lang="ts">
defineProps<{
  icon: string
  name: string
  label: string
  dangerous?: boolean
  transferLabel: string
  sendLabel: string
  deleteLabel: string
}>()

defineEmits<{ send: []; delete: [] }>()
</script>

<template>
  <div class="selected-file-card pending-file-card">
    <div class="file-icon">{{ icon }}</div>
    <div class="file-card-main">
      <b>{{ name }}</b>
      <small>{{ label }}</small>
      <small v-if="dangerous" class="danger-text">{{ transferLabel }}</small>
    </div>
    <button class="secondary danger" @click="$emit('delete')">{{ deleteLabel }}</button>
    <button @click="$emit('send')">{{ sendLabel }}</button>
  </div>
</template>
