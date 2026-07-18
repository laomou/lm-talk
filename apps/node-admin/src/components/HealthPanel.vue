<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import type { NodeApi } from '../nodeApi'
import { NodeError } from '../nodeApi'
import type { HealthResponse } from '../types'

const props = defineProps<{ api: NodeApi; connected: boolean }>()

const data = ref<HealthResponse | null>(null)
const error = ref('')
const loading = ref(false)
const autoRefresh = ref(false)
const intervalSeconds = ref(5)
let timer: ReturnType<typeof setInterval> | null = null

async function refresh() {
  loading.value = true
  error.value = ''
  try {
    data.value = await props.api.health()
  } catch (err) {
    error.value = err instanceof NodeError ? err.message : String(err)
  } finally {
    loading.value = false
  }
}

function stopTimer() {
  if (timer) {
    clearInterval(timer)
    timer = null
  }
}

function startTimer() {
  stopTimer()
  const ms = Math.max(1, intervalSeconds.value) * 1000
  timer = setInterval(refresh, ms)
}

watch([autoRefresh, intervalSeconds], () => {
  if (autoRefresh.value && props.connected) startTimer()
  else stopTimer()
})

watch(
  () => props.connected,
  (connected) => {
    if (connected) refresh()
    else {
      stopTimer()
      data.value = null
    }
  },
)

onMounted(() => {
  if (props.connected) refresh()
})
onUnmounted(stopTimer)

const bar = (used?: number, max?: number) => {
  if (!max || !used) return 0
  return Math.min(100, Math.round((used / max) * 100))
}
</script>

<template>
  <section class="home-card">
    <div class="section-title-row">
      <h3>节点健康</h3>
      <div class="row compact">
        <label class="identity-select">
          <input v-model="autoRefresh" type="checkbox" :disabled="!connected" />
          <span>自动刷新</span>
        </label>
        <input
          v-model.number="intervalSeconds"
          type="number"
          min="1"
          aria-label="刷新间隔秒数"
          style="width: 72px"
        />
        <button class="secondary" :disabled="!connected || loading" @click="refresh">
          {{ loading ? '刷新中…' : '刷新' }}
        </button>
      </div>
    </div>

    <div v-if="error" class="outbox-error danger-text">{{ error }}</div>
    <div v-else-if="!data" class="empty">连接节点后显示健康状态</div>
    <template v-else>
      <div class="outbox-summary">
        <span>
          状态
          <b :class="{ 'danger-text': data.status !== 'ok' }">{{ data.status ?? '未知' }}</b>
        </span>
        <span>peers {{ data.peers ?? 0 }}</span>
        <span>prekeys {{ data.prekeys ?? 0 }}</span>
        <span>mailbox {{ data.mailbox_deliveries ?? 0 }}</span>
      </div>
      <div class="sync-status">
        <small v-if="data.peer_id">peer_id：{{ data.peer_id }}</small>
        <small v-if="data.node_id">node_id：{{ data.node_id }}</small>
        <small>
          Mailbox 用量：{{ data.mailbox_bytes ?? 0 }} / {{ data.mailbox_max_bytes ?? 0 }} bytes
          ({{ bar(data.mailbox_bytes, data.mailbox_max_bytes) }}%)
        </small>
        <small :class="{ 'danger-text': data.state_db_encrypted === false }">
          state_db 加密：{{ data.state_db_encrypted ? '是' : '否' }}
        </small>
        <small v-if="data.state_db_permissions_hardened !== undefined">
          state_db 权限硬化：{{ data.state_db_permissions_hardened ? '是' : '否' }}
        </small>
      </div>
    </template>
  </section>
</template>
