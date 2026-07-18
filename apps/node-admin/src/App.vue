<script setup lang="ts">
import { computed, ref } from 'vue'
import { createNodeApi } from './nodeApi'
import type { NodeConfig } from './types'
import HealthPanel from './components/HealthPanel.vue'
import StatsPanel from './components/StatsPanel.vue'
import SyncPeersPanel from './components/SyncPeersPanel.vue'
import DhtPanel from './components/DhtPanel.vue'
import SnapshotPanel from './components/SnapshotPanel.vue'

const STORAGE_KEY = 'lm-node-admin'

function loadConfig(): NodeConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw)
      return { url: String(parsed.url ?? ''), token: String(parsed.token ?? '') }
    }
  } catch {
    // ignore malformed storage
  }
  return { url: 'http://127.0.0.1:8787', token: '' }
}

const draft = ref<NodeConfig>(loadConfig())
const config = ref<NodeConfig | null>(null)
const connected = computed(() => config.value !== null)

const api = createNodeApi(() => config.value ?? { url: '', token: '' })

function connect() {
  const url = draft.value.url.trim()
  if (!url) return
  const next: NodeConfig = { url, token: draft.value.token.trim() }
  config.value = next
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
  } catch {
    // ignore storage failures
  }
}

function disconnect() {
  config.value = null
}
</script>

<template>
  <div class="me-page">
    <div class="me-inner">
      <header class="me-hero">
        <span class="avatar large">N</span>
        <div class="me-hero-text">
          <h2>LM Node Admin</h2>
          <small>lm_node 运维面板 · 纯控制面 API，不加载任何身份</small>
        </div>
      </header>

      <section class="home-card">
        <div class="section-title-row">
          <h3>节点连接</h3>
          <span class="sync-pill" :class="{ on: connected }">{{ connected ? '已连接' : '未连接' }}</span>
        </div>
        <label for="node-url">节点地址</label>
        <input id="node-url" v-model="draft.url" aria-label="节点地址" placeholder="http://127.0.0.1:8787" />
        <label for="node-token">控制面令牌</label>
        <input
          id="node-token"
          v-model="draft.token"
          type="password"
          aria-label="控制面令牌"
          autocomplete="off"
          placeholder="节点 --control-token（跨域访问必填）"
        />
        <small>
          远程访问要求节点以 <code>--control-token</code> 且 <code>--cors-allow-origin</code> 启动；仅本机可不填令牌。
          令牌仅保存在本机浏览器 localStorage。
        </small>
        <div class="row compact">
          <button :disabled="!draft.url.trim()" @click="connect">{{ connected ? '重新连接' : '连接' }}</button>
          <button v-if="connected" class="secondary" @click="disconnect">断开</button>
        </div>
      </section>

      <HealthPanel :api="api" :connected="connected" />
      <StatsPanel :api="api" :connected="connected" />
      <SyncPeersPanel :api="api" :connected="connected" />
      <DhtPanel :api="api" :connected="connected" />
      <SnapshotPanel :api="api" :connected="connected" />
    </div>
  </div>
</template>
