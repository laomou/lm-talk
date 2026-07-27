<script setup lang="ts">
import UiAvatar from './UiAvatar.vue'

defineProps<{
  avatarSrc?: string
  displayName?: string
  userId?: string
  unnamedLabel: string
}>()
</script>

<template>
  <header class="me-hero">
    <UiAvatar class="me-avatar" size="large" :src="avatarSrc" :name="displayName" :seed="userId" />
    <div class="me-hero-text">
      <h2>{{ displayName || unnamedLabel }}</h2>
      <small>{{ userId }}</small>
    </div>
  </header>
</template>
