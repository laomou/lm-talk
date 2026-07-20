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
  const fromApi = data.value?.endpoint_groups
  if (fromApi) {
    return {
      mailbox: normalizeGroup(fromApi.mailbox),
      dht: normalizeGroup(fromApi.dht),
      sync: normalizeGroup(fromApi.sync),
      other: normalizeGroup(fromApi.other),
    }
  }
  const groups: Record<'mailbox' | 'dht' | 'sync' | 'other', ReturnType<typeof normalizeGroup>> = {
    mailbox: normalizeGroup(),
    dht: normalizeGroup(),
    sync: normalizeGroup(),
    other: normalizeGroup(),
  }
  for (const row of endpointRows.value) {
    const name = String(row.name)
    const key = name.includes('mailbox') ? 'mailbox' : name.includes('dht') ? 'dht' : name.includes('sync') ? 'sync' : 'other'
    groups[key].requests += Number(row.requests ?? 0)
    groups[key].responses_2xx += Number(row.responses_2xx ?? 0)
    groups[key].responses_4xx += Number(row.responses_4xx ?? 0)
    groups[key].responses_5xx += Number(row.responses_5xx ?? 0)
    groups[key].endpoints += 1
    const slowest = Number(row.max_duration_micros ?? 0)
    if (slowest > groups[key].max_duration_micros) groups[key].max_duration_micros = slowest
  }
  return groups
})

function normalizeGroup(group?: any) {
  return {
    endpoints: Number(group?.endpoints ?? 0),
    requests: Number(group?.requests ?? 0),
    responses_2xx: Number(group?.responses_2xx ?? 0),
    responses_4xx: Number(group?.responses_4xx ?? 0),
    responses_5xx: Number(group?.responses_5xx ?? 0),
    max_duration_micros: Number(group?.max_duration_micros ?? 0),
  }
}
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
      <div class="stats-overview-grid">
        <div class="stats-overview-card">
          <span>请求总数</span>
          <b>{{ data.requests_total ?? 0 }}</b>
          <small>2xx {{ data.responses_2xx ?? 0 }} · 4xx {{ data.responses_4xx ?? 0 }} · 5xx {{ data.responses_5xx ?? 0 }}</small>
        </div>
        <div class="stats-overview-card">
          <span>安全事件</span>
          <b :class="{ 'danger-text': Number(data.unauthorized ?? 0) > 0 || Number(data.rate_limited ?? 0) > 0 }">{{ Number(data.unauthorized ?? 0) + Number(data.rate_limited ?? 0) + Number(data.cors_rejected ?? 0) + Number(data.bad_requests ?? 0) }}</b>
          <small>401 {{ data.unauthorized ?? 0 }} · 429 {{ data.rate_limited ?? 0 }} · CORS {{ data.cors_rejected ?? 0 }} · bad {{ data.bad_requests ?? 0 }}</small>
        </div>
        <div class="stats-overview-card">
          <span>Mailbox</span>
          <b>{{ groupedRows.mailbox.requests }}</b>
          <small>{{ groupedRows.mailbox.endpoints }} 个端点 · 2xx {{ groupedRows.mailbox.responses_2xx }} · 5xx {{ groupedRows.mailbox.responses_5xx }}</small>
        </div>
        <div class="stats-overview-card">
          <span>DHT</span>
          <b>{{ groupedRows.dht.requests }}</b>
          <small>{{ groupedRows.dht.endpoints }} 个端点 · 2xx {{ groupedRows.dht.responses_2xx }} · 5xx {{ groupedRows.dht.responses_5xx }}</small>
        </div>
      </div>
      <div class="outbox-summary compact-summary">
        <span>Sync {{ groupedRows.sync.requests }}</span>
        <span>其他 {{ groupedRows.other.requests }}</span>
        <span>Mailbox 慢 {{ groupedRows.mailbox.max_duration_micros }}</span>
        <span>DHT 慢 {{ groupedRows.dht.max_duration_micros }}</span>
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
