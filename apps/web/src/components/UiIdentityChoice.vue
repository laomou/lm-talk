<script setup lang="ts">
import UiIcon from './UiIcon.vue'

const props = defineProps<{
  id: string
  name: string
  userId: string
  selected: boolean
}>()

const emit = defineEmits<{
  select: [id: string]
  requestDelete: [id: string]
}>()

function requestDelete(event: MouseEvent) {
  event.preventDefault()
  event.stopPropagation()
  emit('requestDelete', props.id)
}
</script>

<template>
  <div class="ui-identity-choice" :class="{ selected }">
    <label class="ui-identity-select">
      <input :checked="selected" type="radio" name="local-identity" @change="emit('select', id)" />
      <span class="ui-identity-avatar">{{ (name || userId || '?').slice(0, 1).toUpperCase() }}</span>
      <span class="ui-identity-copy">
        <b>{{ name || '未命名' }}</b>
        <small>{{ userId }}</small>
      </span>
    </label>
    <button
      class="icon-btn ui-identity-delete"
      type="button"
      title="删除本地身份"
      aria-label="删除本地身份"
      @pointerdown.stop
      @click="requestDelete"
    >
      <UiIcon name="close" size="18" />
    </button>
  </div>
</template>
