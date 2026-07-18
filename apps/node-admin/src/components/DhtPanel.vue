<script setup lang="ts">
import { ref } from 'vue'
import type { NodeApi } from '../nodeApi'
import { NodeError } from '../nodeApi'

const props = defineProps<{ api: NodeApi; connected: boolean }>()

const result = ref('')
const error = ref('')
const busy = ref('')
const findKey = ref('')

async function run(label: string, fn: () => Promise<any>) {
  busy.value = label
  error.value = ''
  try {
    const res = await fn()
    result.value = JSON.stringify(res, null, 2)
  } catch (err) {
    error.value = err instanceof NodeError ? err.message : String(err)
  } finally {
    busy.value = ''
  }
}

function findValue() {
  const key = findKey.value.trim()
  if (!/^[0-9a-fA-F]{64}$/.test(key)) {
    error.value = 'DHT record key 必须是 64 位十六进制'
    return
  }
  run('find', () => props.api.dhtFindValue(key))
}
</script>

<template>
  <section class="home-card">
    <div class="section-title-row">
      <h3>DHT 运维</h3>
    </div>
    <div class="row compact">
      <button class="secondary" :disabled="!connected || !!busy" @click="run('maintenance', api.dhtMaintenance)">
        {{ busy === 'maintenance' ? '运行中…' : '运行 DHT 维护' }}
      </button>
      <button class="secondary" :disabled="!connected || !!busy" @click="run('replicate', api.dhtReplicate)">
        {{ busy === 'replicate' ? '运行中…' : '复制 DHT 记录' }}
      </button>
      <button class="secondary" :disabled="!connected || !!busy" @click="run('refresh', api.dhtRoutingRefresh)">
        {{ busy === 'refresh' ? '运行中…' : '刷新 DHT 路由' }}
      </button>
    </div>
    <label for="dht-find-key">DHT record key</label>
    <div class="inline-field">
      <input id="dht-find-key" v-model="findKey" aria-label="DHT record key" placeholder="64 位十六进制 key" />
      <button class="secondary" :disabled="!connected || !!busy" @click="findValue">
        {{ busy === 'find' ? '查找中…' : '查找 DHT 记录' }}
      </button>
    </div>
    <div v-if="error" class="outbox-error danger-text">{{ error }}</div>
    <pre v-if="result" class="mono" aria-label="DHT 运维结果">{{ result }}</pre>
  </section>
</template>
