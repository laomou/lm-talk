<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{ ctx: any }>()
const diagnosticReport = ref('')
const redactDiagnosticReport = ref(false)
const diagnosticSummaryOnly = ref(false)
const showDiagnosticReport = ref(false)

function redacted(value: string) {
  return value ? '[已脱敏]' : ''
}

async function runDiagnostics() {
  const nav = navigator as Navigator & { serviceWorker?: ServiceWorkerContainer }
  const registrations = nav.serviceWorker?.getRegistrations ? await nav.serviceWorker.getRegistrations().catch(() => []) : []
  const cacheKeys = typeof caches !== 'undefined' ? await caches.keys().catch(() => []) : []
  const report: Record<string, unknown> = {
    diagnostics_version: 1,
    report_scope: diagnosticSummaryOnly.value ? 'summary' : 'full',
    time: new Date().toISOString(),
    browser: {
      secure_context: window.isSecureContext,
      indexed_db: 'indexedDB' in window,
      webcrypto: Boolean(globalThis.crypto?.subtle),
      service_worker: 'serviceWorker' in navigator,
      service_worker_registrations: registrations.length,
      caches: cacheKeys,
    },
    sync: {
      enabled: props.ctx.nodeEnabled.value,
      services: redactDiagnosticReport.value ? props.ctx.nodeUrlList().map(() => '[已脱敏]') : props.ctx.nodeUrlList(),
      token_count: props.ctx.nodeTokenCount.value,
      missing_remote_token_count: props.ctx.nodeMissingRemoteTokenCount.value,
      status: props.ctx.nodeControlStatus.value,
    },
    local_counts: {
      contacts: props.ctx.contacts.value.length,
      groups: props.ctx.groups.value.length,
      friend_requests: props.ctx.friendRequests.value.length,
      quarantined_friend_requests: props.ctx.quarantinedFriendRequests.value.length,
      group_invites: props.ctx.groupInvites.value.length,
      mailbox_dedupe_records: props.ctx.mailboxDedupeCount.value,
      outbox: props.ctx.outbox.value.length,
      pending_outbox: props.ctx.outbox.value.filter((x: any) => x.status !== 'sent').length,
      messages: props.ctx.messages.value.length,
    },
  }
  if (!diagnosticSummaryOnly.value) {
    report.account = {
      user_id: redactDiagnosticReport.value ? redacted(props.ctx.identity.value?.user_id ?? '') : props.ctx.identity.value?.user_id ?? '',
      display_name: redactDiagnosticReport.value ? redacted(props.ctx.displayName.value ?? '') : props.ctx.displayName.value ?? '',
    }
    report.recent_logs = props.ctx.log.value.slice(0, 12)
  }
  diagnosticReport.value = JSON.stringify(report, null, 2)
}
</script>

<template>
  <section class="debug-page">
    <div class="debug-inner">
      <header class="debug-header">
        <div>
          <h1>诊断工具</h1>
          <p class="hint">用于排查登录、同步、消息收发和本地数据问题。日常聊天不需要打开。</p>
        </div>
        <button class="secondary" @click="ctx.goChatPage">返回聊天</button>
      </header>

      <section class="diagnostic-overview">
        <div class="diagnostic-card">
          <span>当前账号</span>
          <b>{{ ctx.displayName.value || '未命名' }}</b>
          <small>{{ ctx.identity.value?.user_id }}</small>
        </div>
        <div class="diagnostic-card">
          <span>消息同步</span>
          <b>{{ ctx.nodeEnabled.value ? '已开启' : '未开启' }}</b>
          <small>{{ ctx.nodeUrlList().length ? ctx.nodeUrlList().join('，') : '未配置同步服务' }}</small>
        </div>
        <div class="diagnostic-card">
          <span>待发送</span>
          <b>{{ ctx.outbox.value.filter((x: any) => x.status !== 'sent').length }}</b>
          <small>总队列 {{ ctx.outbox.value.length }}</small>
        </div>
        <div class="diagnostic-card">
          <span>新朋友</span>
          <b>{{ ctx.visibleFriendRequests.value.length }}</b>
          <small>群邀请 {{ ctx.groupInvites.value.length }}</small>
        </div>
        <div class="diagnostic-card">
          <span>垃圾请求</span>
          <b>{{ ctx.quarantinedFriendRequests.value.length }}</b>
          <small>去重记录 {{ ctx.mailboxDedupeCount.value }}</small>
        </div>
      </section>

      <section class="home-card">
        <h3>一键诊断</h3>
        <p class="hint">生成只包含状态摘要的诊断报告，不会导出提示词、身份私钥或消息明文。</p>
        <label class="check-row diagnostic-option">
          <input v-model="redactDiagnosticReport" type="checkbox" />
          脱敏账号和同步服务地址
        </label>
        <label class="check-row diagnostic-option">
          <input v-model="diagnosticSummaryOnly" type="checkbox" />
          只生成摘要报告
        </label>
        <div class="row compact">
          <button @click="runDiagnostics">生成诊断报告</button>
          <button class="secondary" :disabled="!diagnosticReport" @click="ctx.copyText(diagnosticReport, '诊断报告')">复制报告</button>
          <button class="secondary" :disabled="!diagnosticReport" @click="showDiagnosticReport = !showDiagnosticReport">{{ showDiagnosticReport ? '隐藏预览' : '显示预览' }}</button>
          <button class="secondary" @click="ctx.syncNow">立即同步</button>
        </div>
        <textarea v-if="diagnosticReport && showDiagnosticReport" v-model="diagnosticReport" class="mono" rows="12" readonly />
      </section>

      <section class="home-card">
        <h3>最近记录</h3>
        <div v-if="ctx.log.value.length" class="diagnostic-log">
          <div v-for="line in ctx.log.value.slice(0, 8)" :key="line">{{ line }}</div>
        </div>
        <div v-else class="empty">暂无记录</div>
      </section>
    </div>
  </section>
</template>
