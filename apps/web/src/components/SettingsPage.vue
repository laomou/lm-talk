<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { setLocale, type SupportedLocale } from '../i18n'
import UiPageHeader from './UiPageHeader.vue'
import UiListRow from './UiListRow.vue'
import UiStatusBadge from './UiStatusBadge.vue'
import UiCard from './UiCard.vue'
import UiField from './UiField.vue'
import UiSection from './UiSection.vue'
import UiActionGroup from './UiActionGroup.vue'
import UiListGroup from './UiListGroup.vue'

const props = defineProps<{ ctx: any }>()
type MeView = 'home' | 'profile' | 'backup' | 'security' | 'sync' | 'settings' | 'about'
const view = ref<MeView>('home')
const route = useRoute()
const router = useRouter()
const { locale, t } = useI18n()
const showSyncServiceEditor = ref(false)
const showDataBackupEditor = ref(false)
const showSyncEditor = computed(() => showSyncServiceEditor.value || props.ctx.nodeEntrySummaries.value.length === 0)
const mailboxInboxErrorLines = computed(() => props.ctx.mailboxInboxErrorText.value
  .split('\n')
  .map((line: string) => line.trim())
  .filter(Boolean))
const syncStatus = computed(() => {
  if (!props.ctx.nodeEnabled.value) return { text: t('settingsView.syncDisabled'), tone: 'neutral' as const }
  if (props.ctx.nodeMissingRemoteTokenCount.value > 0) return { text: t('settingsView.syncNeedsConfig'), tone: 'warning' as const }
  const hasOutbox = props.ctx.outbox.value.some((item: any) => item.status !== 'sent')
  const hasMailboxIssue = props.ctx.mailboxFailureSummaryText.value || props.ctx.mailboxInboxErrorText.value
  const hasSyncIssue = props.ctx.syncFailureSummaryText.value
  return hasOutbox || hasMailboxIssue || hasSyncIssue
    ? { text: t('settingsView.needsAction'), tone: 'warning' as const }
    : { text: t('settingsView.normal'), tone: 'success' as const }
})
const backupStatus = computed(() => {
  if (props.ctx.fullDataBackupFreshnessLevel.value === 'danger') return { text: t('settingsView.backupNeeded'), tone: 'warning' as const }
  if (props.ctx.fullDataBackupFreshnessLevel.value === 'warning') return { text: t('settingsView.backupSuggested'), tone: 'warning' as const }
  return { text: t('settingsView.backedUp'), tone: 'success' as const }
})
const pendingOutboxCount = computed(() => props.ctx.outbox.value.filter((item: any) => item.status !== 'sent').length)
const failedOutboxCount = computed(() => props.ctx.outbox.value.filter((item: any) => item.status === 'failed').length)
const retrySyncIssueText = computed(() => {
  if (pendingOutboxCount.value > 0) return props.ctx.nodeEnabled.value ? t('settingsView.retryPendingOutbox', { count: pendingOutboxCount.value }) : t('settingsView.requeuePendingOutbox', { count: pendingOutboxCount.value })
  if (props.ctx.mailboxFailedCount.value > 0) return t('settingsView.retryMailboxFailures', { count: props.ctx.mailboxFailedCount.value })
  if (props.ctx.prekeyAutoErrorText.value) return t('settingsView.retryPreKey')
  if (/failed|失败/i.test(props.ctx.nodeSyncStatusText.value) && props.ctx.autoNodeSync.value && props.ctx.nodeSyncPeerUrl.value.trim()) return t('settingsView.retryNodeSync')
  if (props.ctx.selfSyncGapCount.value > 0 && props.ctx.nodeEnabled.value) return t('settingsView.repairDeviceSyncGap', { count: props.ctx.selfSyncGapCount.value })
  return ''
})

watch(
  () => [route.path, route.query.section],
  ([path, section]) => {
    if (path === '/me/profile') {
      view.value = 'profile'
    } else if (path === '/me/backup') {
      view.value = 'backup'
    } else if (path === '/me/security') {
      view.value = 'security'
    } else if (path === '/me/sync') {
      view.value = 'sync'
    } else if (path === '/me/preferences') {
      view.value = 'settings'
    } else if (path === '/me/about') {
      view.value = 'about'
    } else if (section === 'sync') {
      view.value = 'sync'
    } else {
      view.value = 'home'
    }
  },
  { immediate: true },
)

function backHome() {
  void router.push('/me')
}

function saveSyncSettings() {
  if (props.ctx.saveNetworkSettings()) showSyncServiceEditor.value = false
}

function changeLocale(event: Event) {
  setLocale((event.target as HTMLSelectElement).value as SupportedLocale)
}
</script>

<template>
  <div class="me-page">
    <div class="me-inner">
      <template v-if="view === 'home'">
        <header class="me-hero">
          <span class="avatar large">{{ (ctx.displayName.value || ctx.identity.value?.user_id || '?').slice(0, 1).toUpperCase() }}</span>
          <div class="me-hero-text">
            <h2>{{ ctx.displayName.value || t('settingsView.unnamed') }}</h2>
            <small>{{ ctx.identity.value?.user_id }}</small>
          </div>
        </header>

        <UiCard>
          <UiListGroup class="product-me-rows">
            <UiListRow @click="router.push('/me/profile')">{{ t('me.profile') }}</UiListRow>
            <UiListRow @click="router.push('/me/backup')">
              {{ t('me.backup') }}
              <template #end><UiStatusBadge compact :tone="backupStatus.tone">{{ backupStatus.text }}</UiStatusBadge><span class="chevron">›</span></template>
            </UiListRow>
            <UiListRow @click="router.push('/me/security')">{{ t('me.security') }}</UiListRow>
            <UiListRow @click="router.push('/me/sync')">
              {{ t('me.sync') }}
              <template #end><UiStatusBadge compact :tone="syncStatus.tone">{{ syncStatus.text }}</UiStatusBadge><span class="chevron">›</span></template>
            </UiListRow>
            <UiListRow @click="router.push('/me/preferences')">{{ t('me.settings') }}</UiListRow>
            <UiListRow @click="router.push('/me/about')">{{ t('me.about') }}</UiListRow>
          </UiListGroup>
        </UiCard>

        <UiCard>
          <UiListRow danger :aria-label="t('me.logout')" @click="ctx.logout">{{ t('me.logout') }}</UiListRow>
        </UiCard>
      </template>

      <template v-else-if="view === 'profile'">
        <UiPageHeader :title="t('settingsView.profileTitle')" :back-label="t('settingsView.backToMe')" @back="backHome" />
        <UiSection :title="t('settingsView.myProfile')">
          <UiField :label="t('settingsView.displayName')" for-id="display-name-input">
          <div class="inline-field">
            <input id="display-name-input" v-model="ctx.displayName.value" :aria-label="t('settingsView.displayName')" />
            <button @click="ctx.saveMyProfile">{{ t('settingsView.save') }}</button>
          </div>
          </UiField>
          <UiActionGroup>
            <button class="secondary" @click="ctx.showQr(ctx.myContactCardText.value, t('settingsView.myCard'))">{{ t('settingsView.myCard') }}</button>
          </UiActionGroup>
        </UiSection>
      </template>

      <template v-else-if="view === 'backup'">
        <UiPageHeader :title="t('settingsView.identityBackup')" :back-label="t('settingsView.backToMe')" @back="backHome" />
        <UiSection :title="t('settingsView.identityBackup')" :description="t('settingsView.identityBackupDescription')">
          <template #actions><button class="secondary" @click="ctx.showQr(ctx.backupText.value, t('settingsView.exportIdentity'))">{{ t('settingsView.exportIdentity') }}</button></template>
        </UiSection>
        <UiSection :title="t('settingsView.fullDataBackup')" :description="t('settingsView.fullDataBackupDescription')">
          <template #actions><button class="secondary" @click="showDataBackupEditor = !showDataBackupEditor">{{ showDataBackupEditor ? t('settingsView.hideBackup') : t('settingsView.showBackup') }}</button></template>
          <UiActionGroup>
            <button class="secondary" @click="ctx.exportFullDataBackup">{{ t('settingsView.generateBackup') }}</button>
            <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.downloadText(ctx.dataBackupText.value, 'lm-talk-data-backup.txt')">{{ t('settingsView.downloadBackup') }}</button>
            <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackupMerge">{{ t('settingsView.importMerge') }}</button>
            <button class="secondary danger" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackup">{{ t('settingsView.importOverwrite') }}</button>
          </UiActionGroup>
          <UiField v-if="showDataBackupEditor" :label="t('settingsView.fullDataBackupText')">
            <textarea v-model="ctx.dataBackupText.value" class="mono" rows="5" :aria-label="t('settingsView.fullDataBackupText')" :placeholder="t('settingsView.fullDataBackupPlaceholder')" />
          </UiField>
        </UiSection>
      </template>

      <template v-else-if="view === 'security'">
        <UiPageHeader :title="t('me.security')" :back-label="t('settingsView.backToMe')" @back="backHome" />
        <UiSection class="sync-card" :title="t('settingsView.securityStatus')" :description="t('settingsView.securityStatusDescription')">
          <template #actions><UiStatusBadge tone="success">{{ t('settingsView.autoProtect') }}</UiStatusBadge></template>
        </UiSection>
        <UiSection class="sync-card" :title="t('settingsView.device')">
          <small>{{ t('settingsView.currentDevice', { device: ctx.myDeviceId.value || t('settingsView.notGenerated') }) }}</small>
          <small>{{ ctx.sealedSlotCoverageSummary.value }}</small>
        </UiSection>
      </template>

      <template v-else-if="view === 'sync'">
        <UiPageHeader :title="t('me.sync')" :back-label="t('settingsView.backToMe')" @back="backHome">
          <template #end><UiStatusBadge compact :tone="syncStatus.tone">{{ syncStatus.text }}</UiStatusBadge></template>
        </UiPageHeader>
        <UiSection class="sync-card" :title="t('settingsView.messageSync')">
          <template #actions><UiStatusBadge :tone="ctx.nodeEnabled.value ? 'success' : 'neutral'">{{ ctx.nodeEnabled.value ? t('settingsView.enabled') : t('settingsView.syncDisabled') }}</UiStatusBadge></template>
          <div v-if="ctx.nodeEntrySummaries.value.length && !showSyncEditor" class="outbox-list">
            <div v-for="entry in ctx.nodeEntrySummaries.value" :key="entry.url" class="outbox-row">
              <b>{{ entry.url }}</b>
              <small>{{ entry.token_configured ? t('settingsView.tokenConfigured') : entry.missing_remote_token ? t('settingsView.remoteTokenMissing') : t('settingsView.localTokenNotRequired') }}</small>
            </div>
          </div>
          <UiField v-if="showSyncEditor" :label="t('settingsView.syncServiceAddress')" for-id="sync-service-input">
            <textarea id="sync-service-input" v-model="ctx.nodeControlUrl.value" rows="3" :aria-label="t('settingsView.syncServiceAddress')" :placeholder="t('settingsView.syncServicePlaceholder')" />
          </UiField>
          <small>{{ t('settingsView.syncDescription') }}</small>
          <UiActionGroup>
            <button class="secondary" @click="showSyncServiceEditor = !showSyncServiceEditor">{{ showSyncEditor ? t('settingsView.hideEditor') : t('settingsView.editAddress') }}</button>
            <button @click="ctx.toggleNodeEnabled">{{ ctx.nodeEnabled.value ? t('settingsView.disableSync') : t('settingsView.enableSync') }}</button>
            <button v-if="showSyncEditor" class="secondary" @click="saveSyncSettings">{{ t('settingsView.save') }}</button>
            <button class="secondary" @click="ctx.syncNow">{{ t('settingsView.syncNow') }}</button>
          </UiActionGroup>
        </UiSection>
        <UiSection class="sync-card" :title="t('settingsView.outbox')">
          <template #actions><UiStatusBadge :tone="pendingOutboxCount ? 'warning' : 'neutral'">{{ pendingOutboxCount }}</UiStatusBadge></template>
          <small>{{ t('settingsView.failed', { count: failedOutboxCount }) }}</small>
          <small v-if="ctx.syncFailureSummaryText.value" class="danger-text">{{ ctx.syncFailureSummaryText.value }}</small>
          <UiActionGroup>
            <button v-if="retrySyncIssueText" class="secondary" @click="ctx.recoverSyncFailures">{{ retrySyncIssueText }}</button>
          </UiActionGroup>
          <small v-if="ctx.syncRecoveryStatusText.value && ctx.syncRecoveryStatusText.value !== '尚未恢复'">{{ ctx.syncRecoveryStatusText.value }}</small>
        </UiSection>
        <UiSection class="sync-card" :title="t('settingsView.diagnostics')">
          <template #actions><button class="secondary" @click="ctx.goDiagnosticsPage('me-sync')">{{ t('settingsView.openDiagnostics') }}</button></template>
          <small v-if="ctx.mailboxFailureSummaryText.value" class="danger-text">{{ ctx.mailboxFailureSummaryText.value }}</small>
          <div v-if="mailboxInboxErrorLines.length" class="mailbox-error-lines">
            <small v-for="(line, index) in mailboxInboxErrorLines" :key="`${index}-${line}`" class="danger-text">{{ line }}</small>
          </div>
          <small>{{ ctx.mailboxInboxStatus.value }}</small>
          <small>{{ ctx.mailboxQuotaStatusText.value }}</small>
        </UiSection>
      </template>

      <template v-else-if="view === 'settings'">
        <UiPageHeader :title="t('me.settings')" :back-label="t('common.backToMe')" @back="backHome" />
        <UiSection class="sync-card" :title="t('me.language')" :description="t('me.languageDescription')">
          <UiField :label="t('me.language')" for-id="locale-select">
            <select id="locale-select" :value="locale" aria-label="Language" @change="changeLocale">
              <option value="zh-CN">{{ t('language.zhCN') }}</option>
              <option value="en-US">{{ t('language.enUS') }}</option>
            </select>
          </UiField>
        </UiSection>
        <UiSection class="sync-card" :title="t('settingsView.pwaApp')">
          <template #actions><button class="secondary" @click="ctx.refreshPwaStatus">{{ t('settingsView.refreshStatus') }}</button></template>
          <small>{{ ctx.pwaStatusText.value }}</small>
          <small>{{ t('settingsView.pwaSecurityBoundary') }}</small>
        </UiSection>
        <UiCard><UiListRow :aria-label="t('settingsView.clearBrowserCaches')" @click="ctx.clearBrowserCaches">{{ t('settingsView.clearBrowserCaches') }}</UiListRow></UiCard>
      </template>

      <template v-else-if="view === 'about'">
        <UiPageHeader :title="t('settingsView.about')" :back-label="t('settingsView.backToMe')" @back="backHome" />
        <UiCard class="about-card">
          <h2>LM Talk Web</h2>
          <p>{{ ctx.webVersionText }}</p>
        </UiCard>
      </template>
    </div>
  </div>
</template>
