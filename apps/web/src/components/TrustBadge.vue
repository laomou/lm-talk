<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import UiStatusBadge from './UiStatusBadge.vue'
import UiIcon from './UiIcon.vue'

type TrustLevel = 'verified' | 'unverified' | 'blocked'

const props = withDefaults(defineProps<{
  level: TrustLevel
  compact?: boolean
}>(), {
  compact: false,
})

const { t } = useI18n()
const tone = computed(() => props.level === 'verified' ? 'success' : props.level === 'blocked' ? 'danger' : 'warning')
const label = computed(() => props.level === 'verified' ? t('trustStatus.verified') : props.level === 'blocked' ? t('trustStatus.blocked') : t('trustStatus.unverified'))
const icon = computed(() => props.level === 'blocked' ? 'alert' : 'lock')
</script>

<template>
  <UiStatusBadge :tone="tone" :compact="compact" :title="label" :aria-label="label">
    <UiIcon :name="icon" size="13" />
  </UiStatusBadge>
</template>
