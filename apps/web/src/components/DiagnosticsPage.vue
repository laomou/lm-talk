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

function sanitizeDiagnosticText(value: string) {
  return value
    .replace(/Bearer\s+[A-Za-z0-9._~+\/=:-]+/gi, 'Bearer [已脱敏]')
    .replace(/(https?:\/\/[^\s|]+)\|[^\s，；,]+/gi, '$1|[已脱敏]')
    .replace(/\b(lm-[a-z0-9-]+-v\d+):[^\s\"'，；,]+/gi, '$1:[已脱敏]')
    .replace(/"(backupText|backup_text|private_key|identity_seed|seed|ciphertext|envelope_json|message_text|contact_card_text)"\s*:\s*"[^"]{16,}"/gi, '"$1":"[已脱敏]"')
}

function diagnosticLogLine(value: string) {
  const sanitized = sanitizeDiagnosticText(value)
  return sanitized.length > 160 ? `${sanitized.slice(0, 160)}…[已截断]` : sanitized
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
    pwa: {
      status: sanitizeDiagnosticText(props.ctx.pwaStatusText.value),
      background_capability: sanitizeDiagnosticText(props.ctx.pwaBackgroundCapabilityText.value),
      background_event_count: props.ctx.pwaBackgroundEventHistory.value.length,
      last_background_event: sanitizeDiagnosticText(props.ctx.pwaLastBackgroundEventText.value),
    },
    sync: {
      enabled: props.ctx.nodeEnabled.value,
      services: redactDiagnosticReport.value ? props.ctx.nodeUrlList().map(() => '[已脱敏]') : props.ctx.nodeUrlList(),
      token_count: props.ctx.nodeTokenCount.value,
      missing_remote_token_count: props.ctx.nodeMissingRemoteTokenCount.value,
      status: sanitizeDiagnosticText(props.ctx.nodeControlStatus.value),
      node_health_summary: sanitizeDiagnosticText(props.ctx.nodeHealthSummaryText.value),
      state_db_security: sanitizeDiagnosticText(props.ctx.nodeStateDbSecurityText.value),
      state_db_security_level: props.ctx.nodeStateDbSecurityLevel.value,
      state_file_security: sanitizeDiagnosticText(props.ctx.nodeStateFileSecurityText.value),
      state_file_security_level: props.ctx.nodeStateFileSecurityLevel.value,
      dht_peer_health_summary: sanitizeDiagnosticText(props.ctx.nodePeerHealthStatusText.value),
      dht_peer_health_risk: props.ctx.nodePeerHealthRiskLevel.value,
      node_snapshot_sync_enabled: props.ctx.autoNodeSync.value,
      node_snapshot_peer: redactDiagnosticReport.value ? redacted(props.ctx.nodeSyncPeerUrl.value) : sanitizeDiagnosticText(props.ctx.nodeSyncPeerUrl.value),
      node_snapshot_status: sanitizeDiagnosticText(props.ctx.nodeSyncStatusText.value),
      last_node_snapshot_sync_at: props.ctx.lastNodeSnapshotSyncAt.value,
      node_snapshot_sync_freshness: props.ctx.nodeSnapshotSyncFreshnessText.value,
      node_snapshot_sync_freshness_level: props.ctx.nodeSnapshotSyncFreshnessLevel.value,
    },
    dht: {
      find_value_status: sanitizeDiagnosticText(props.ctx.nodeDhtFindValueStatusText.value),
      replication_status: sanitizeDiagnosticText(props.ctx.nodeDhtReplicationStatusText.value),
      routing_refresh_status: sanitizeDiagnosticText(props.ctx.nodeRoutingRefreshStatusText.value),
      operation_history: props.ctx.nodeDhtOperationHistory.value.slice(0, 8).map((line: string) => sanitizeDiagnosticText(line)),
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
      verified_contacts: props.ctx.contacts.value.filter((x: any) => x.fingerprint_verified_at).length,
      contacts_with_revoked_devices: props.ctx.contacts.value.filter((x: any) => (x.revoked_device_ids || []).length > 0).length,
      fully_revoked_contacts: props.ctx.contacts.value.filter((x: any) => props.ctx.contactAllKnownDevicesRevoked(x)).length,
      revoked_devices: props.ctx.contacts.value.reduce((sum: number, x: any) => sum + (x.revoked_device_ids || []).length, 0),
      unverified_incoming_drops: props.ctx.unverifiedIncomingDropCount.value,
      revoked_device_incoming_drops: props.ctx.revokedDeviceIncomingDropCount.value,
      sealed_slot_coverage: props.ctx.sealedSlotCoverageSummary.value,
      per_device_envelope_sent_count: props.ctx.perDeviceEnvelopeSentCount.value,
      per_device_envelope_received_count: props.ctx.perDeviceEnvelopeReceivedCount.value,
      per_device_envelope_drop_count: props.ctx.perDeviceEnvelopeDropCount.value,
      last_per_device_envelope_at: props.ctx.lastPerDeviceEnvelopeAt.value,
      last_per_device_envelope_drop_at: props.ctx.lastPerDeviceEnvelopeDropAt.value,
      last_per_device_envelope_drop_reason: sanitizeDiagnosticText(props.ctx.lastPerDeviceEnvelopeDropReason.value),
      contact_card_update_fanout_count: props.ctx.contactCardUpdateFanoutCount.value,
      contact_card_update_fanout_skip_count: props.ctx.contactCardUpdateFanoutSkipCount.value,
      contact_card_update_fanout_ack_count: props.ctx.contactCardUpdateFanoutAckCount.value,
      contact_card_update_pending_ack_count: props.ctx.contactCardUpdatePendingAckCount.value,
      contact_card_update_stale_ack_count: props.ctx.contactCardUpdateStaleAckCount.value,
      contact_card_dht_auto_refresh_count: props.ctx.contactCardDhtAutoRefreshCount.value,
      last_contact_card_dht_auto_refresh_at: props.ctx.lastContactCardDhtAutoRefreshAt.value,
      last_contact_card_dht_auto_refresh_error: sanitizeDiagnosticText(props.ctx.lastContactCardDhtAutoRefreshError.value),
      contact_card_dht_auto_refresh_history: props.ctx.contactCardDhtAutoRefreshHistory.value.slice(0, 8).map((item: any) => ({ ...item, display_name: redactDiagnosticReport.value ? redacted(item.display_name) : sanitizeDiagnosticText(item.display_name || ''), error: sanitizeDiagnosticText(item.error || '') })),
      last_contact_card_update_fanout_at: props.ctx.lastContactCardUpdateFanoutAt.value,
      last_full_data_backup_at: props.ctx.lastFullDataBackupAt.value,
      last_self_mailbox_backup_pushed_at: props.ctx.lastSelfMailboxBackupPushedAt.value,
      last_self_mailbox_backup_received_at: props.ctx.lastSelfMailboxBackupReceivedAt.value,
      last_self_mailbox_backup_merged_at: props.ctx.lastSelfMailboxBackupMergedAt.value,
      self_mailbox_backup_merge_pending: props.ctx.selfMailboxBackupMergePending.value,
      processed_self_sync_ids: props.ctx.processedSelfSyncIds.value.length,
      processed_self_sync_request_ids: props.ctx.processedSelfSyncRequestIds.value.length,
      pending_self_sync_missing_requests: props.ctx.selfSyncMissingRequestRecords.value.length,
      self_sync_request_sent_count: props.ctx.selfSyncRequestSentCount.value,
      self_sync_request_hit_count: props.ctx.selfSyncRequestHitCount.value,
      self_sync_request_miss_count: props.ctx.selfSyncRequestMissCount.value,
      cached_self_sync_packages: props.ctx.selfSyncRecentPackages.value.length,
      latest_cached_self_sync_expires_at: props.ctx.selfSyncRecentPackages.value[0]?.expires_at ?? null,
      last_self_sync_pushed_at: props.ctx.lastSelfSyncPushedAt.value,
      last_self_sync_merged_at: props.ctx.lastSelfSyncMergedAt.value,
      last_self_sync_sequence_sent: props.ctx.lastSelfSyncSequenceSent.value,
      last_self_sync_sequence_merged: props.ctx.lastSelfSyncSequenceMerged.value,
      self_sync_gap_count: props.ctx.selfSyncGapCount.value,
      last_self_sync_gap_at: props.ctx.lastSelfSyncGapAt.value,
      last_self_sync_missing_previous_id: props.ctx.lastSelfSyncMissingPreviousId.value,
      full_data_backup_freshness: props.ctx.fullDataBackupFreshnessText.value,
      full_data_backup_freshness_level: props.ctx.fullDataBackupFreshnessLevel.value,
    },
    security: {
      require_verified_contacts_for_send: Boolean(props.ctx.safetyPolicy.value.requireVerifiedContactsForSend),
      require_verified_contacts_for_receive: Boolean(props.ctx.safetyPolicy.value.requireVerifiedContactsForReceive),
      require_sealed_per_device_slots_for_send: Boolean(props.ctx.safetyPolicy.value.requireSealedPerDeviceSlotsForSend),
      require_sealed_per_device_slots_for_receive: Boolean(props.ctx.safetyPolicy.value.requireSealedPerDeviceSlotsForReceive),
      strict_e2ee_policy_enabled: Boolean(props.ctx.strictE2eePolicyEnabled.value),
      strict_e2ee_readiness: props.ctx.strictE2eeReadiness.value,
      last_unverified_incoming_drop_at: props.ctx.lastUnverifiedIncomingDropAt.value,
      last_unverified_incoming_drop_from: redactDiagnosticReport.value ? redacted(props.ctx.lastUnverifiedIncomingDropFrom.value) : sanitizeDiagnosticText(props.ctx.lastUnverifiedIncomingDropFrom.value),
      last_revoked_device_incoming_drop_at: props.ctx.lastRevokedDeviceIncomingDropAt.value,
      last_revoked_device_incoming_drop_from: redactDiagnosticReport.value ? redacted(props.ctx.lastRevokedDeviceIncomingDropFrom.value) : sanitizeDiagnosticText(props.ctx.lastRevokedDeviceIncomingDropFrom.value),
      revoked_device_contacts: props.ctx.contacts.value
        .filter((x: any) => (x.revoked_device_ids || []).length > 0)
        .slice(0, 8)
        .map((x: any) => ({
          user_id: redactDiagnosticReport.value ? redacted(x.user_id || '') : x.user_id,
          display_name: redactDiagnosticReport.value ? redacted(x.display_name || '') : sanitizeDiagnosticText(x.display_name || ''),
          revoked_devices: (x.revoked_device_ids || []).length,
          fully_revoked: props.ctx.contactAllKnownDevicesRevoked(x),
        })),
    },
  }
  if (!diagnosticSummaryOnly.value) {
    report.account = {
      user_id: redactDiagnosticReport.value ? redacted(props.ctx.identity.value?.user_id ?? '') : props.ctx.identity.value?.user_id ?? '',
      display_name: redactDiagnosticReport.value ? redacted(props.ctx.displayName.value ?? '') : props.ctx.displayName.value ?? '',
    }
    report.recent_logs = props.ctx.log.value.slice(0, 12).map(diagnosticLogLine)
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
        <div class="diagnostic-card">
          <span>DHT 状态</span>
          <b>{{ ctx.nodePeerHealthRiskLevel.value === 'ok' ? '正常' : ctx.nodePeerHealthRiskLevel.value === 'warning' ? '警告' : '异常' }}</b>
          <small>{{ ctx.nodePeerHealthStatusText.value }}</small>
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
          <div v-for="line in ctx.log.value.slice(0, 8).map(diagnosticLogLine)" :key="line">{{ line }}</div>
        </div>
        <div v-else class="empty">暂无记录</div>
      </section>
    </div>
  </section>
</template>
