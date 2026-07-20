<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { NodeApi } from '../nodeApi'
import { NodeError } from '../nodeApi'
import type { SyncPeerStatus, SyncStatusResponse } from '../types'

const props = defineProps<{ api: NodeApi; connected: boolean }>()

const data = ref<SyncStatusResponse | null>(null)
const error = ref('')
const loading = ref(false)
const resetting = ref('')

async function refresh() {
  loading.value = true
  error.value = ''
  try {
    data.value = await props.api.syncStatus()
  } catch (err) {
    error.value = err instanceof NodeError ? err.message : String(err)
  } finally {
    loading.value = false
  }
}

async function resetPeer(url: string) {
  resetting.value = url
  error.value = ''
  try {
    await props.api.resetPeer(url)
    await refresh()
  } catch (err) {
    error.value = err instanceof NodeError ? err.message : String(err)
  } finally {
    resetting.value = ''
  }
}

watch(
  () => props.connected,
  (connected) => {
    if (connected) refresh()
    else data.value = null
  },
)

const now = () => Math.floor(Date.now() / 1000)
const isQuarantined = (peer: SyncPeerStatus) =>
  typeof peer.next_attempt_at === 'number' && peer.next_attempt_at > now()

function formatUnixTime(ts?: number | null) {
  if (!ts) return '无'
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(new Date(ts * 1000))
}

function relativeUnixTime(ts?: number | null) {
  if (!ts) return '无'
  const delta = ts - now()
  const abs = Math.abs(delta)
  const unit = abs < 60 ? `${abs} 秒` : abs < 3600 ? `${Math.round(abs / 60)} 分钟` : `${Math.round(abs / 3600)} 小时`
  return delta >= 0 ? `${unit} 后` : `${unit} 前`
}

function peerStateText(peer: SyncPeerStatus) {
  if (isQuarantined(peer)) return '隔离中'
  if (Number(peer.consecutive_failures ?? 0) > 0 || peer.last_error) return '异常'
  return '健康'
}

const peers = computed(() => Object.values(data.value?.peers ?? {}))
const federationSummary = computed(() => {
  const items = peers.value
  const total = items.length
  const quarantined = items.filter(isQuarantined).length
  const failing = items.filter((peer) => Number(peer.consecutive_failures ?? 0) > 0 || Boolean(peer.last_error)).length
  const healthy = Math.max(0, total - failing - quarantined)
  const attempts = items.reduce((sum, peer) => sum + Number(peer.attempts ?? 0), 0)
  const successes = items.reduce((sum, peer) => sum + Number(peer.successes ?? 0), 0)
  const successRate = attempts > 0 ? Math.round((successes / attempts) * 100) : 0
  return { total, healthy, failing, quarantined, attempts, successes, successRate }
})
</script>

<template>
  <section class="home-card">
    <div class="section-title-row">
      <h3>同步 Peer</h3>
      <button class="secondary" :disabled="!connected || loading" @click="refresh">
        {{ loading ? '刷新中…' : '刷新' }}
      </button>
    </div>

    <div v-if="error" class="outbox-error danger-text">{{ error }}</div>
    <div v-else-if="!data" class="empty">连接节点后显示 sync peer 状态</div>
    <template v-else>
      <div class="outbox-summary federation-summary">
        <span>联邦 peer <b>{{ federationSummary.total }}</b></span>
        <span>健康 <b>{{ federationSummary.healthy }}</b></span>
        <span :class="{ 'danger-text': federationSummary.failing > 0 }">异常 <b>{{ federationSummary.failing }}</b></span>
        <span :class="{ 'danger-text': federationSummary.quarantined > 0 }">隔离 <b>{{ federationSummary.quarantined }}</b></span>
        <span>成功率 <b>{{ federationSummary.successRate }}%</b></span>
      </div>
      <div v-if="!peers.length" class="empty">未配置 sync peer</div>
      <div v-else class="outbox-list">
      <div v-for="peer in peers" :key="peer.url" class="outbox-row">
        <b>{{ peer.url }}</b>
        <small :class="{ 'danger-text': peerStateText(peer) !== '健康' }">
          状态 {{ peerStateText(peer) }} · 尝试 {{ peer.attempts ?? 0 }} · 成功 {{ peer.successes ?? 0 }} · 失败 {{ peer.failures ?? 0 }}
          · 连续失败 {{ peer.consecutive_failures ?? 0 }}
        </small>
        <small>最近尝试 {{ formatUnixTime(peer.last_attempt_at) }}（{{ relativeUnixTime(peer.last_attempt_at) }}） · 最近成功 {{ formatUnixTime(peer.last_success_at) }}（{{ relativeUnixTime(peer.last_success_at) }}）</small>
        <small v-if="isQuarantined(peer)" class="danger-text">下次重试 {{ formatUnixTime(peer.next_attempt_at) }}（{{ relativeUnixTime(peer.next_attempt_at) }}）</small>
        <small v-if="peer.last_error" class="danger-text">最近错误 {{ formatUnixTime(peer.last_error_at) }}：{{ peer.last_error }}</small>
        <button class="secondary" :disabled="resetting === peer.url" @click="resetPeer(peer.url)">
          {{ resetting === peer.url ? '重置中…' : `重置 ${peer.url}` }}
        </button>
      </div>
      </div>
    </template>
  </section>
</template>
