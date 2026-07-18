<script setup lang="ts">
import { ref } from 'vue'
import type { NodeApi } from '../nodeApi'
import { NodeError } from '../nodeApi'

const props = defineProps<{ api: NodeApi; connected: boolean }>()

const text = ref('')
const status = ref('')
const error = ref('')
const busy = ref(false)

async function exportSnapshot() {
  busy.value = true
  error.value = ''
  status.value = ''
  try {
    const snapshot = await props.api.snapshotExport()
    text.value = JSON.stringify(snapshot, null, 2)
    status.value = '已导出节点快照'
  } catch (err) {
    error.value = err instanceof NodeError ? err.message : String(err)
  } finally {
    busy.value = false
  }
}

async function importSnapshot() {
  error.value = ''
  status.value = ''
  let snapshot: unknown
  try {
    snapshot = JSON.parse(text.value)
  } catch {
    error.value = '快照文本不是合法 JSON'
    return
  }
  busy.value = true
  try {
    await props.api.snapshotImport(snapshot)
    status.value = '已导入到本节点'
  } catch (err) {
    error.value = err instanceof NodeError ? err.message : String(err)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="home-card">
    <div class="section-title-row">
      <h3>节点快照</h3>
      <div class="row compact">
        <button class="secondary" :disabled="!connected || busy" @click="exportSnapshot">导出快照</button>
        <button class="secondary" :disabled="!connected || busy || !text.trim()" @click="importSnapshot">导入到本节点</button>
      </div>
    </div>
    <small>快照包含 mailbox、PreKey、DHT records 和 routing peers，不含身份私钥。</small>
    <div v-if="error" class="outbox-error danger-text">{{ error }}</div>
    <small v-else-if="status">{{ status }}</small>
    <textarea
      v-model="text"
      class="mono"
      rows="6"
      aria-label="节点快照 JSON"
      placeholder="点击导出快照，或粘贴 snapshot JSON 后导入"
    />
  </section>
</template>
