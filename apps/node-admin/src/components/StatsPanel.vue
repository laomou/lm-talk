<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { NodeApi } from '../nodeApi'
import { NodeError } from '../nodeApi'
import type { ControlStatsResponse } from '../types'

const props = defineProps<{ api: NodeApi; connected: boolean }>()

const data = ref<ControlStatsResponse | null>(null)
const error = ref('')
const loading = ref(false)

async function refresh() {
  loading.value = true
  error.value = ''
  try {
    data.value = await props.api.stats()
  } catch (err) {
    error.value = err instanceof NodeError ? err.message : String(err)
  } finally {
    loading.value = false
  }
}

watch(
  () => props.connected,
  (connected) => {
    if (connected) refresh()
    else data.value = null
  },
)

const endpointRows = computed(() => {
  const endpoints = data.value?.endpoints ?? {}
  return Object.entries(endpoints)
    .map(([name, stats]) => ({ name, ...stats }))
    .sort((a, b) => Number(b.requests ?? 0) - Number(a.requests ?? 0))
})

const groupedRows = computed(() => {
  const groups: Record<'mailbox' | 'dht' | 'sync' | 'other', { requests: number; responses_4xx: number; responses_5xx: number; count: number }> = {
    mailbox: { requests: 0, responses_4xx: 0, responses_5xx: 0, count: 0 },
    dht: { requests: 0, responses_4xx: 0, responses_5xx: 0, count: 0 },
    sync: { requests: 0, responses_4xx: 0, responses_5xx: 0, count: 0 },
    other: { requests: 0, responses_4xx: 0, responses_5xx: 0, count: 0 },
  }
  for (const row of endpointRows.value) {
    const name = String(row.name)
    const key = name.includes('mailbox') ? 'mailbox' : name.includes('dht') ? 'dht' : name.includes('sync') ? 'sync' : 'other'
    groups[key].requests += Number(row.requests ?? 0)
    groups[key].responses_4xx += Number(row.responses_4xx ?? 0)
    groups[key].responses_5xx += Number(row.responses_5xx ?? 0)
    groups[key].count += 1
  }
  return groups
})
</script>

<template>
  <section class="home-card">
    <div class="section-title-row">
      <h3>运行统计</h3>
      <button class="secondary" :disabled="!connected || loading" @click="refresh">
        {{ loading ? '刷新中…' : '刷新' }}
      </button>
    </div>

    <div v-if="error" class="outbox-error danger-text">{{ error }}</div>
    <div v-else-if="!data" class="empty">连接节点后显示运行统计</div>
    <template v-else>
      <div class="outbox-summary compact-summary">
        <span>请求总数 {{ data.requests_total ?? 0 }}</span>
        <span>2xx {{ data.responses_2xx ?? 0 }}</span>
        <span :class="{ 'danger-text': Number(data.responses_4xx ?? 0) > 0 }">4xx {{ data.responses_4xx ?? 0 }}</span>
        <span :class="{ 'danger-text': Number(data.responses_5xx ?? 0) > 0 }">5xx {{ data.responses_5xx ?? 0 }}</span>
      </div>
      <div class="outbox-summary">
        <span :class="{ 'danger-text': Number(data.unauthorized ?? 0) > 0 }">未授权 {{ data.unauthorized ?? 0 }}</span>
        <span :class="{ 'danger-text': Number(data.rate_limited ?? 0) > 0 }">限流命中 {{ data.rate_limited ?? 0 }}</span>
        <span :class="{ 'danger-text': Number(data.cors_rejected ?? 0) > 0 }">CORS 拒绝 {{ data.cors_rejected ?? 0 }}</span>
        <span>坏请求 {{ data.bad_requests ?? 0 }}</span>
      </div>
      <div class="outbox-summary compact-summary">
        <span>Mailbox {{ groupedRows.mailbox.requests }}</span>
        <span>DHT {{ groupedRows.dht.requests }}</span>
        <span>Sync {{ groupedRows.sync.requests }}</span>
        <span>其他 {{ groupedRows.other.requests }}</span>
      </div>
      <div v-if="endpointRows.length" class="outbox-list">
        <div v-for="row in endpointRows" :key="row.name" class="outbox-row">
          <b>{{ row.name }}</b>
          <small>
            请求 {{ row.requests ?? 0 }} · 2xx {{ row.responses_2xx ?? 0 }} · 4xx {{ row.responses_4xx ?? 0 }} · 5xx {{ row.responses_5xx ?? 0 }}
            <span v-if="row.last_status"> · 最近 {{ row.last_status }}</span>
          </small>
        </div>
      </div>
    </template>
  </section>
</template>
