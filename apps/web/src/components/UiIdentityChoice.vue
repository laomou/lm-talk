<script setup lang="ts">
import UiIcon from './UiIcon.vue'

defineProps<{
  id: string
  name: string
  userId: string
  selected: boolean
}>()

const emit = defineEmits<{
  select: [id: string]
  remove: [id: string]
}>()
</script>

<template>
  <label class="ui-identity-choice" :class="{ selected }">
    <input :checked="selected" type="radio" name="local-identity" @change="emit('select', id)" />
    <span class="ui-identity-avatar">{{ (name || userId || '?').slice(0, 1).toUpperCase() }}</span>
    <span class="ui-identity-copy">
      <b>{{ name || '未命名' }}</b>
      <small>{{ userId }}</small>
    </span>
    <button class="icon-btn ui-identity-delete" type="button" title="删除本地身份" aria-label="删除本地身份" @click.prevent="emit('remove', id)">
      <UiIcon name="close" size="18" />
    </button>
  </label>
</template>
