<script setup lang="ts">
defineProps<{
  icon: string
  name: string
  label: string
  hint?: string
  dangerHint?: boolean
  downloadUrl?: string
  downloadName?: string
  previewUrl?: string
  previewAlt?: string
  actionLabel?: string
  downloadLabel?: string
}>()

defineEmits<{ action: [] }>()
</script>

<template>
  <div class="message-attachment-card">
    <div class="file-icon">{{ icon }}</div>
    <div class="file-card-main">
      <b>{{ name }}</b>
      <small>{{ label }}</small>
      <small v-if="hint" :class="{ 'attachment-error': dangerHint }">{{ hint }}</small>
    </div>
    <button v-if="actionLabel" class="secondary" @click="$emit('action')">{{ actionLabel }}</button>
    <a v-if="downloadUrl && downloadName" :href="downloadUrl" :download="downloadName">{{ downloadLabel || '下载' }}</a>
    <img v-if="previewUrl" class="message-attachment-preview" :src="previewUrl" :alt="previewAlt || name" />
  </div>
</template>
