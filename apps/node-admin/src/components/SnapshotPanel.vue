<script setup lang="ts">
import { ref } from 'vue'
import type { NodeApi } from '../nodeApi'
import { NodeError } from '../nodeApi'

const props = defineProps<{ api: NodeApi; connected: boolean }>()

const text = ref('')
const status = ref('')
const error = ref('')
const busy = ref(false)

function snapshotFileName() {
  return `lm-node-snapshot-${new Date().toISOString().replace(/[:.]/g, '-')}.json`
}

function downloadSnapshot() {
  if (!text.value.trim()) return
  const blob = new Blob([`${text.value}\n`], { type: 'application/json;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = snapshotFileName()
  a.click()
  URL.revokeObjectURL(url)
  status.value = '已下载节点快照 JSON'
}

function fallbackCopyText(value: string) {
  const textarea = document.createElement('textarea')
  textarea.value = value
  textarea.setAttribute('readonly', 'true')
  textarea.style.position = 'fixed'
  textarea.style.left = '-9999px'
  document.body.appendChild(textarea)
  textarea.select()
  document.execCommand('copy')
  document.body.removeChild(textarea)
}

async function copySnapshot() {
  if (!text.value.trim()) return
  if (navigator.clipboard?.writeText) await navigator.clipboard.writeText(text.value)
  else fallbackCopyText(text.value)
  status.value = '已复制节点快照 JSON'
}

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
        <button class="secondary" :disabled="!text.trim()" @click="copySnapshot">复制 JSON</button>
        <button class="secondary" :disabled="!text.trim()" @click="downloadSnapshot">下载 JSON</button>
        <button class="secondary" :disabled="!connected || busy || !text.trim()" @click="importSnapshot">导入到本节点</button>
      </div>
    </div>
    <small>快照包含 mailbox、PreKey、DHT records 和 routing peers，不含身份私钥；导入前请确认来源可信。</small>
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
