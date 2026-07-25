<script setup lang="ts">
import { computed } from 'vue'
import { avatarColor, avatarInitial } from '../avatar'

const props = defineProps<{
  src?: string
  name?: string
  seed?: string
  size?: 'normal' | 'large'
}>()

const label = computed(() => avatarInitial(props.name, props.seed))
const fallbackStyle = computed(() => ({ background: avatarColor(props.seed || props.name) }))
</script>

<template>
  <span class="avatar" :class="{ large: props.size === 'large', 'has-image': Boolean(props.src) }" :style="props.src ? undefined : fallbackStyle">
    <img v-if="props.src" :src="props.src" alt="" loading="lazy" decoding="async" />
    <span v-else>{{ label }}</span>
  </span>
</template>
