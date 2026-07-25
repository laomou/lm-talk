<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { avatarColor, avatarInitial } from '../avatar'

const props = defineProps<{
  src?: string
  name?: string
  seed?: string
  size?: 'normal' | 'large'
}>()

const label = computed(() => avatarInitial(props.name, props.seed))
const fallbackStyle = computed(() => ({ background: avatarColor(props.seed || props.name) }))
const imageFailed = ref(false)
const showImage = computed(() => Boolean(props.src) && !imageFailed.value)

watch(() => props.src, () => {
  imageFailed.value = false
})
</script>

<template>
  <span class="avatar" :class="{ large: props.size === 'large', 'has-image': showImage }" :style="showImage ? undefined : fallbackStyle">
    <img v-if="showImage" :src="props.src" alt="" loading="lazy" decoding="async" @error="imageFailed = true" />
    <span v-else>{{ label }}</span>
  </span>
</template>
