<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import UiPageHeader from './UiPageHeader.vue'
import UiSection from './UiSection.vue'
import UiActionGroup from './UiActionGroup.vue'
import UiStatusBadge from './UiStatusBadge.vue'

const props = defineProps<{ ctx: any }>()
const { t } = useI18n()
const diagnosticReport = ref('')
const redactDiagnosticReport = ref(false)
const diagnosticSummaryOnly = ref(false)
const showDiagnosticReport = ref(false)

function redacted(value: string) {
  return value ? t('diagnosticsView.redacted') : ''
}

function sanitizeDiagnosticText(value: string) {
  return value
    .replace(/Bearer\s+[A-Za-z0-9._~+\/=:-]+/gi, `Bearer ${t('diagnosticsView.redacted')}`)
    .replace(/(https?:\/\/[^\s|]+)\|[^\s，；,]+/gi, `$1|${t('diagnosticsView.redacted')}`)
    .replace(/\b(lm-[a-z0-9-]+-v\d+):[^\s\"'，；,]+/gi, `$1:${t('diagnosticsView.redacted')}`)
    .replace(/"(backupText|backup_text|private_key|identity_seed|seed|ciphertext|envelope_json|message_text|contact_card_text)"\s*:\s*"[^"]{16,}"/gi, `"$1":"${t('diagnosticsView.redacted')}"`)
}

function diagnosticLogLine(value: string) {
  const sanitized = sanitizeDiagnosticText(value)
  return sanitized.length > 160 ? `${sanitized.slice(0, 160)}…${t('diagnosticsView.truncated')}` : sanitized
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
      services: redactDiagnosticReport.value ? props.ctx.nodeUrlList().map(() => t('diagnosticsView.redacted')) : props.ctx.nodeUrlList(),
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
      confirmed_friends: props.ctx.contacts.value.filter((x: any) => x.state === 'Friend' && !props.ctx.contactAllKnownDevicesRevoked(x)).length,
      contacts_with_revoked_devices: props.ctx.contacts.value.filter((x: any) => (x.revoked_device_ids || []).length > 0).length,
      fully_revoked_contacts: props.ctx.contacts.value.filter((x: any) => props.ctx.contactAllKnownDevicesRevoked(x)).length,
      revoked_devices: props.ctx.contacts.value.reduce((sum: number, x: any) => sum + (x.revoked_device_ids || []).length, 0),
      legacy_unverified_incoming_drops: props.ctx.unverifiedIncomingDropCount.value,
      revoked_device_incoming_drops: props.ctx.revokedDeviceIncomingDropCount.value,
      sealed_slot_coverage: props.ctx.sealedSlotCoverageSummary.value,
      strict_e2ee_group_risk_count: props.ctx.groups.value.filter((g: any) => props.ctx.groupStrictE2eeRiskTextFor(g)).length,
      strict_e2ee_group_ready_count: props.ctx.groups.value.filter((g: any) => !props.ctx.groupStrictE2eeRiskTextFor(g)).length,
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
      last_self_sync_receipt_states_sent: props.ctx.lastSelfSyncReceiptStatesSent.value,
      outbound_messages_with_mailbox_delivery_id: props.ctx.messages.value.filter((m: any) => m.direction === 'out' && m.mailbox_delivery_id).length,
      last_self_sync_receipt_states_merged: props.ctx.lastSelfSyncReceiptStatesMerged.value,
      total_self_sync_receipt_states_merged: props.ctx.totalSelfSyncReceiptStatesMerged.value,
      last_self_sync_outbox_summary: props.ctx.lastSelfSyncOutboxSummary.value,
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
      strict_e2ee_group_risks: props.ctx.groups.value
        .filter((g: any) => props.ctx.groupStrictE2eeRiskTextFor(g))
        .slice(0, 8)
        .map((g: any) => ({
          group_id: redactDiagnosticReport.value ? redacted(g.group_id || '') : g.group_id,
          name: redactDiagnosticReport.value ? redacted(g.name || '') : sanitizeDiagnosticText(g.name || ''),
          member_count: (g.member_user_ids || []).length,
          risk: sanitizeDiagnosticText(props.ctx.groupStrictE2eeRiskTextFor(g)),
        })),
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
      <UiPageHeader :title="t('diagnosticsView.title')" :back-label="t('diagnosticsView.back')" @back="ctx.goDiagnosticsBack" />
      <p class="hint diagnostic-page-hint">{{ t('diagnosticsView.hint') }}</p>

      <UiSection :title="t('diagnosticsView.runtimeStatus')">
        <div class="diagnostic-list">
          <div class="diagnostic-row">
            <span><b>{{ t('diagnosticsView.currentAccount') }}</b><small>{{ ctx.identity.value?.user_id }}</small></span>
            <strong>{{ ctx.displayName.value || t('diagnosticsView.unnamed') }}</strong>
          </div>
          <div class="diagnostic-row">
            <span><b>{{ t('diagnosticsView.messageSync') }}</b><small>{{ ctx.nodeUrlList().length ? ctx.nodeUrlList().join('，') : t('diagnosticsView.syncNotConfigured') }}</small></span>
            <UiStatusBadge :tone="ctx.nodeEnabled.value ? 'success' : 'neutral'">{{ ctx.nodeEnabled.value ? t('diagnosticsView.enabled') : t('diagnosticsView.disabled') }}</UiStatusBadge>
          </div>
          <div class="diagnostic-row">
            <span><b>{{ t('diagnosticsView.outbox') }}</b><small>{{ t('diagnosticsView.totalQueue', { count: ctx.outbox.value.length }) }}</small></span>
            <strong>{{ ctx.outbox.value.filter((x: any) => x.status !== 'sent').length }}</strong>
          </div>
          <div class="diagnostic-row">
            <span><b>{{ t('diagnosticsView.newFriends') }}</b><small>{{ t('diagnosticsView.groupInvites', { count: ctx.groupInvites.value.length }) }}</small></span>
            <strong>{{ ctx.visibleFriendRequests.value.length }}</strong>
          </div>
          <div class="diagnostic-row">
            <span><b>{{ t('diagnosticsView.junkRequests') }}</b><small>{{ t('diagnosticsView.dedupeRecords', { count: ctx.mailboxDedupeCount.value }) }}</small></span>
            <strong>{{ ctx.quarantinedFriendRequests.value.length }}</strong>
          </div>
          <div class="diagnostic-row">
            <span><b>{{ t('diagnosticsView.dhtStatus') }}</b><small>{{ ctx.nodePeerHealthStatusText.value }}</small></span>
            <UiStatusBadge :tone="ctx.nodePeerHealthRiskLevel.value === 'ok' ? 'success' : ctx.nodePeerHealthRiskLevel.value === 'warning' ? 'warning' : 'danger'">{{ ctx.nodePeerHealthRiskLevel.value === 'ok' ? t('diagnosticsView.normal') : ctx.nodePeerHealthRiskLevel.value === 'warning' ? t('diagnosticsView.warning') : t('diagnosticsView.abnormal') }}</UiStatusBadge>
          </div>
          <div class="diagnostic-row">
            <span><b>{{ t('diagnosticsView.groupStrictE2ee') }}</b><small>{{ t('diagnosticsView.groupRiskSummary', { count: ctx.groups.value.length }) }}</small></span>
            <strong>{{ ctx.groups.value.filter((g: any) => ctx.groupStrictE2eeRiskTextFor(g)).length }}</strong>
          </div>
        </div>
      </UiSection>

      <UiSection :title="t('diagnosticsView.reportTitle')" :description="t('diagnosticsView.reportDescription')">
        <label class="check-row diagnostic-option">
          <input v-model="redactDiagnosticReport" type="checkbox" />
          {{ t('diagnosticsView.redactAccountAndServices') }}
        </label>
        <label class="check-row diagnostic-option">
          <input v-model="diagnosticSummaryOnly" type="checkbox" />
          {{ t('diagnosticsView.summaryOnly') }}
        </label>
        <UiActionGroup>
          <button @click="runDiagnostics">{{ t('diagnosticsView.generateReport') }}</button>
          <button class="secondary" :disabled="!diagnosticReport" @click="ctx.copyText(diagnosticReport, t('diagnosticsView.reportTitle'))">{{ t('diagnosticsView.copyReport') }}</button>
          <button class="secondary" :disabled="!diagnosticReport" @click="showDiagnosticReport = !showDiagnosticReport">{{ showDiagnosticReport ? t('diagnosticsView.hidePreview') : t('diagnosticsView.showPreview') }}</button>
          <button class="secondary" @click="ctx.syncNow">{{ t('diagnosticsView.syncNow') }}</button>
        </UiActionGroup>
        <textarea v-if="diagnosticReport && showDiagnosticReport" v-model="diagnosticReport" class="mono" rows="12" readonly />
      </UiSection>

      <UiSection :title="t('diagnosticsView.recentRecords')">
        <div v-if="ctx.log.value.length" class="diagnostic-log">
          <div v-for="(line, index) in ctx.log.value.slice(0, 8).map(diagnosticLogLine)" :key="`${index}-${line}`">{{ line }}</div>
        </div>
        <div v-else class="empty">{{ t('diagnosticsView.noRecords') }}</div>
      </UiSection>
    </div>
  </section>
</template>
