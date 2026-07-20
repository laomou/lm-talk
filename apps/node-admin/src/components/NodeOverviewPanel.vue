<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { NodeApi } from '../nodeApi'
import { NodeError } from '../nodeApi'
import type { ControlStatsResponse, HealthResponse, SyncStatusResponse } from '../types'

const props = defineProps<{ api: NodeApi; connected: boolean }>()

const health = ref<HealthResponse | null>(null)
const stats = ref<ControlStatsResponse | null>(null)
const sync = ref<SyncStatusResponse | null>(null)
const error = ref('')
const loading = ref(false)
const reportText = ref('')

function isQuarantined(nextAttemptAt?: number | null) {
  return typeof nextAttemptAt === 'number' && nextAttemptAt > Date.now() / 1000
}

async function refresh() {
  loading.value = true
  error.value = ''
  try {
    const [healthRes, statsRes, syncRes] = await Promise.all([
      props.api.health(),
      props.api.stats(),
      props.api.syncStatus(),
    ])
    health.value = healthRes
    stats.value = statsRes
    sync.value = syncRes
    reportText.value = JSON.stringify(buildReport(), null, 2)
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
    else {
      health.value = null
      stats.value = null
      sync.value = null
      reportText.value = ''
    }
  },
)

const endpointRows = computed(() => {
  const endpoints = stats.value?.endpoints ?? {}
  return Object.entries(endpoints)
    .map(([name, row]) => ({ name, ...row }))
    .sort((a, b) => Number(b.requests ?? 0) - Number(a.requests ?? 0))
    .slice(0, 5)
})

const peerSummary = computed(() => {
  const peers = Object.values(sync.value?.peers ?? {})
  const total = peers.length
  const quarantined = peers.filter((peer) => isQuarantined(peer.next_attempt_at ?? null)).length
  const failing = peers.filter((peer) => Number(peer.consecutive_failures ?? 0) > 0 || Boolean(peer.last_error)).length
  const healthy = Math.max(0, total - failing - quarantined)
  return { total, healthy, failing, quarantined }
})

const mailboxPressure = computed(() => {
  const used = Number(health.value?.mailbox_bytes ?? 0)
  const max = Number(health.value?.mailbox_max_bytes ?? 0)
  const pct = max > 0 ? Math.round((used / max) * 100) : 0
  return { used, max, pct }
})

const controlSummary = computed(() => {
  const requests = Number(stats.value?.requests_total ?? 0)
  return {
    requests,
    unauthorized: Number(stats.value?.unauthorized ?? 0),
    bad_requests: Number(stats.value?.bad_requests ?? 0),
    rate_limited: Number(stats.value?.rate_limited ?? 0),
    cors_rejected: Number(stats.value?.cors_rejected ?? 0),
    responses_5xx: Number(stats.value?.responses_5xx ?? 0),
  }
})

const nodeHealthText = computed(() => {
  const status = health.value?.status ?? 'unknown'
  if (status !== 'ok') return '异常'
  if (controlSummary.value.responses_5xx > 0 || peerSummary.value.failing > 0) return '需关注'
  return '正常'
})

function buildReport() {
  return {
    exported_at: new Date().toISOString(),
    node: {
      status: health.value?.status ?? null,
      peer_id: health.value?.peer_id ?? null,
      node_id: health.value?.node_id ?? null,
      state_db_encrypted: health.value?.state_db_encrypted ?? null,
      state_db_permissions_hardened: health.value?.state_db_permissions_hardened ?? null,
      mailbox: {
        deliveries: health.value?.mailbox_deliveries ?? 0,
        bytes: health.value?.mailbox_bytes ?? 0,
        max_bytes: health.value?.mailbox_max_bytes ?? 0,
        max_bytes_per_user: health.value?.mailbox_max_bytes_per_user ?? null,
        max_messages_per_user: health.value?.mailbox_max_messages_per_user ?? null,
        pressure_percent: mailboxPressure.value.pct,
      },
    },
    control: {
      started_at: stats.value?.started_at ?? null,
      requests_total: controlSummary.value.requests,
      responses_2xx: Number(stats.value?.responses_2xx ?? 0),
      responses_4xx: Number(stats.value?.responses_4xx ?? 0),
      responses_5xx: controlSummary.value.responses_5xx,
      bad_requests: controlSummary.value.bad_requests,
      unauthorized: controlSummary.value.unauthorized,
      cors_rejected: controlSummary.value.cors_rejected,
      rate_limited: controlSummary.value.rate_limited,
      top_endpoints: endpointRows.value.map((row) => ({
        name: row.name,
        requests: Number(row.requests ?? 0),
        responses_2xx: Number(row.responses_2xx ?? 0),
        responses_4xx: Number(row.responses_4xx ?? 0),
        responses_5xx: Number(row.responses_5xx ?? 0),
        last_status: row.last_status ?? null,
      })),
    },
    sync: {
      total: peerSummary.value.total,
      healthy: peerSummary.value.healthy,
      failing: peerSummary.value.failing,
      quarantined: peerSummary.value.quarantined,
      peers: Object.values(sync.value?.peers ?? {}).map((peer) => ({
        url: peer.url,
        attempts: peer.attempts ?? 0,
        successes: peer.successes ?? 0,
        failures: peer.failures ?? 0,
        consecutive_failures: peer.consecutive_failures ?? 0,
        last_attempt_at: peer.last_attempt_at ?? null,
        last_success_at: peer.last_success_at ?? null,
        last_error_at: peer.last_error_at ?? null,
        next_attempt_at: peer.next_attempt_at ?? null,
        last_error: peer.last_error ?? null,
      })),
    },
  }
}

async function exportReport() {
  if (!health.value || !stats.value || !sync.value) return
  reportText.value = JSON.stringify(buildReport(), null, 2)
  const blob = new Blob([`${reportText.value}\n`], { type: 'application/json;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `lm-node-admin-report-${new Date().toISOString().replace(/[:.]/g, '-')}.json`
  a.click()
  URL.revokeObjectURL(url)
}
</script>

<template>
  <section class="home-card">
    <div class="section-title-row">
      <h3>节点总览</h3>
      <div class="row compact">
        <button class="secondary" :disabled="!connected || loading" @click="refresh">
          {{ loading ? '刷新中…' : '刷新摘要' }}
        </button>
        <button class="secondary" :disabled="!connected || !health || !stats || !sync" @click="exportReport">导出诊断报告</button>
      </div>
    </div>

    <div v-if="error" class="outbox-error danger-text">{{ error }}</div>
    <div v-else-if="!health || !stats || !sync" class="empty">连接节点后显示总览</div>
    <template v-else>
      <div class="node-overview-grid">
        <div class="overview-card">
          <span>节点状态</span>
          <b :class="{ 'danger-text': nodeHealthText !== '正常' }">{{ nodeHealthText }}</b>
          <small>{{ health.status ?? 'unknown' }} · peer {{ health.peer_id ?? 'n/a' }}</small>
        </div>
        <div class="overview-card">
          <span>Mailbox 压力</span>
          <b :class="{ 'danger-text': mailboxPressure.pct >= 80 }">{{ mailboxPressure.pct }}%</b>
          <small>{{ mailboxPressure.used }} / {{ mailboxPressure.max }} bytes</small>
        </div>
        <div class="overview-card">
          <span>控制面风险</span>
          <b :class="{ 'danger-text': controlSummary.responses_5xx > 0 || controlSummary.unauthorized > 0 }">{{ controlSummary.responses_5xx > 0 || controlSummary.unauthorized > 0 ? '需关注' : '正常' }}</b>
          <small>5xx {{ controlSummary.responses_5xx }} · 未授权 {{ controlSummary.unauthorized }} · 限流 {{ controlSummary.rate_limited }}</small>
        </div>
        <div class="overview-card">
          <span>联邦同步</span>
          <b :class="{ 'danger-text': peerSummary.failing > 0 || peerSummary.quarantined > 0 }">{{ peerSummary.total }} peers</b>
          <small>健康 {{ peerSummary.healthy }} · 异常 {{ peerSummary.failing }} · 隔离 {{ peerSummary.quarantined }}</small>
        </div>
      </div>
      <div class="outbox-summary compact-summary">
        <span>Mailbox {{ health.mailbox_deliveries ?? 0 }}</span>
        <span>请求 {{ controlSummary.requests }}</span>
        <span>2xx {{ stats.responses_2xx ?? 0 }}</span>
        <span>4xx {{ stats.responses_4xx ?? 0 }}</span>
        <span>5xx {{ stats.responses_5xx ?? 0 }}</span>
      </div>
      <div class="sync-status">
        <small v-if="health.state_db_encrypted !== undefined" :class="{ 'danger-text': health.state_db_encrypted === false }">
          state_db 加密：{{ health.state_db_encrypted ? '是' : '否' }}
        </small>
        <small v-if="health.state_db_permissions_hardened !== undefined">
          state_db 权限硬化：{{ health.state_db_permissions_hardened ? '是' : '否' }}
        </small>
        <small v-if="peerSummary.quarantined > 0" class="danger-text">部分 sync peer 已隔离，请查看 Sync Peer 面板</small>
      </div>
      <div v-if="endpointRows.length" class="outbox-list">
        <div class="outbox-row">
          <b>控制面 Top endpoints</b>
          <small>已按请求量排序，便于快速定位异常路径</small>
        </div>
        <div v-for="row in endpointRows" :key="row.name" class="outbox-row">
          <b>{{ row.name }}</b>
          <small>
            请求 {{ row.requests ?? 0 }} · 2xx {{ row.responses_2xx ?? 0 }} · 4xx {{ row.responses_4xx ?? 0 }} · 5xx {{ row.responses_5xx ?? 0 }}
            <span v-if="row.last_status"> · 最近 {{ row.last_status }}</span>
          </small>
        </div>
      </div>
      <textarea v-model="reportText" class="mono" rows="8" aria-label="诊断报告 JSON" placeholder="点击导出诊断报告后可直接复制/保存" />
    </template>
  </section>
</template>
