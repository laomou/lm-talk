<script setup lang="ts">
defineProps<{
  name: 'search' | 'add' | 'back' | 'next' | 'more' | 'close' | 'send' | 'smile' | 'info' | 'alert' | 'check' | 'lock' | 'image' | 'scan'
  size?: number | string
}>()
</script>

<template>
  <svg
    class="ui-icon"
    :width="size ?? 20"
    :height="size ?? 20"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <template v-if="name === 'search'"><circle cx="11" cy="11" r="6.5" /><path d="m16 16 4 4" /></template>
    <template v-else-if="name === 'add'"><path d="M12 5v14M5 12h14" /></template>
    <template v-else-if="name === 'back'"><path d="m15 18-6-6 6-6" /></template>
    <template v-else-if="name === 'next'"><path d="m9 18 6-6-6-6" /></template>
    <template v-else-if="name === 'more'"><circle cx="5" cy="12" r="1" fill="currentColor" /><circle cx="12" cy="12" r="1" fill="currentColor" /><circle cx="19" cy="12" r="1" fill="currentColor" /></template>
    <template v-else-if="name === 'close'"><path d="m6 6 12 12M18 6 6 18" /></template>
    <template v-else-if="name === 'send'"><path d="m22 2-7 20-4-9-9-4 20-7Z" /><path d="M22 2 11 13" /></template>
    <template v-else-if="name === 'smile'"><circle cx="12" cy="12" r="8.5" /><path d="M8.5 14.5s1.2 2 3.5 2 3.5-2 3.5-2M9 9.5h.01M15 9.5h.01" /></template>
    <template v-else-if="name === 'info'"><circle cx="12" cy="12" r="8.5" /><path d="M12 11v5M12 8h.01" /></template>
    <template v-else-if="name === 'alert'"><path d="M10.3 4.1 2.2 18a2 2 0 0 0 1.7 3h16.2a2 2 0 0 0 1.7-3L13.7 4.1a2 2 0 0 0-3.4 0Z" /><path d="M12 9v4M12 17h.01" /></template>
    <template v-else-if="name === 'lock'"><rect x="5" y="10" width="14" height="10" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3" /></template>
    <template v-else-if="name === 'image'"><rect x="3.5" y="4.5" width="17" height="15" rx="2" /><circle cx="8.5" cy="9" r="1.5" /><path d="m4.5 17 5-5 3.5 3 2.5-2.5 4 4" /></template>
    <template v-else-if="name === 'scan'"><path d="M5 9V5h4M15 5h4v4M19 15v4h-4M9 19H5v-4" /><path d="M8 12h8M12 8v8" /></template>
    <template v-else><path d="m5 12 4 4L19 6" /></template>
  </svg>
</template>
