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
        <small :class="{ 'danger-text': isQuarantined(peer) }">
          尝试 {{ peer.attempts ?? 0 }} · 成功 {{ peer.successes ?? 0 }} · 失败 {{ peer.failures ?? 0 }}
          · 连续失败 {{ peer.consecutive_failures ?? 0 }}{{ isQuarantined(peer) ? ' · 已隔离' : '' }}
        </small>
        <small v-if="peer.last_error" class="danger-text">{{ peer.last_error }}</small>
        <button class="secondary" :disabled="resetting === peer.url" @click="resetPeer(peer.url)">
          {{ resetting === peer.url ? '重置中…' : `重置 ${peer.url}` }}
        </button>
      </div>
      </div>
    </template>
  </section>
</template>
