<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
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
const showSyncServiceEditor = ref(false)
const showDataBackupEditor = ref(false)
const showSyncEditor = computed(() => showSyncServiceEditor.value || props.ctx.nodeEntrySummaries.value.length === 0)
const syncStatus = computed(() => {
  if (!props.ctx.nodeEnabled.value) return { text: '未开启', tone: 'neutral' as const }
  if (props.ctx.nodeMissingRemoteTokenCount.value > 0) return { text: '需配置', tone: 'warning' as const }
  const hasOutbox = props.ctx.outbox.value.some((item: any) => item.status !== 'sent')
  const hasMailboxIssue = props.ctx.mailboxFailureSummaryText.value || props.ctx.mailboxInboxErrorText.value
  const hasSyncIssue = props.ctx.syncFailureSummaryText.value
  return hasOutbox || hasMailboxIssue || hasSyncIssue
    ? { text: '需处理', tone: 'warning' as const }
    : { text: '正常', tone: 'success' as const }
})
const pendingOutboxCount = computed(() => props.ctx.outbox.value.filter((item: any) => item.status !== 'sent').length)
const failedOutboxCount = computed(() => props.ctx.outbox.value.filter((item: any) => item.status === 'failed').length)
const retrySyncIssueText = computed(() => {
  if (pendingOutboxCount.value > 0) return props.ctx.nodeEnabled.value ? `重试待发送（${pendingOutboxCount.value}）` : `重新排队待发送（${pendingOutboxCount.value}）`
  if (props.ctx.mailboxFailedCount.value > 0) return `重新处理收取失败（${props.ctx.mailboxFailedCount.value}）`
  if (props.ctx.prekeyAutoErrorText.value) return '重试 PreKey 发布'
  if (/failed|失败/i.test(props.ctx.nodeSyncStatusText.value) && props.ctx.autoNodeSync.value && props.ctx.nodeSyncPeerUrl.value.trim()) return '重试节点同步'
  if (props.ctx.selfSyncGapCount.value > 0 && props.ctx.nodeEnabled.value) return `修复设备同步缺口（${props.ctx.selfSyncGapCount.value}）`
  return ''
})

watch(
  () => route.query.section,
  (section) => {
    if (section === 'sync') view.value = 'sync'
  },
  { immediate: true },
)

function backHome() {
  view.value = 'home'
  if (route.query.section) void router.replace('/me')
}

function saveSyncSettings() {
  if (props.ctx.saveNetworkSettings()) showSyncServiceEditor.value = false
}
</script>

<template>
  <div class="me-page">
    <div class="me-inner">
      <template v-if="view === 'home'">
        <header class="me-hero">
          <span class="avatar large">{{ (ctx.displayName.value || ctx.identity.value?.user_id || '?').slice(0, 1).toUpperCase() }}</span>
          <div class="me-hero-text">
            <h2>{{ ctx.displayName.value || '未命名' }}</h2>
            <small>{{ ctx.identity.value?.user_id }}</small>
          </div>
        </header>

        <UiCard>
          <UiListGroup class="product-me-rows">
            <UiListRow @click="view = 'profile'">个人资料</UiListRow>
            <UiListRow @click="view = 'backup'">身份备份</UiListRow>
            <UiListRow @click="view = 'security'">安全与设备</UiListRow>
            <UiListRow @click="view = 'sync'">
              同步与安全
              <template #end><UiStatusBadge compact :tone="syncStatus.tone">{{ syncStatus.text }}</UiStatusBadge><span class="chevron">›</span></template>
            </UiListRow>
            <UiListRow @click="view = 'settings'">设置</UiListRow>
            <UiListRow @click="view = 'about'">关于</UiListRow>
          </UiListGroup>
        </UiCard>

        <UiCard>
          <UiListRow danger aria-label="退出登录" @click="ctx.logout">退出登录</UiListRow>
        </UiCard>
      </template>

      <template v-else-if="view === 'profile'">
        <UiPageHeader title="个人资料" back-label="返回我" @back="backHome" />
        <UiSection title="我的资料">
          <UiField label="显示名" for-id="display-name-input">
          <div class="inline-field">
            <input id="display-name-input" v-model="ctx.displayName.value" aria-label="显示名" @change="ctx.refreshMyContactCard" />
            <button @click="ctx.refreshMyContactCard">保存</button>
          </div>
          </UiField>
          <UiActionGroup>
            <button class="secondary" @click="ctx.showQr(ctx.myContactCardText.value, '我的名片')">我的名片</button>
          </UiActionGroup>
        </UiSection>
      </template>

      <template v-else-if="view === 'backup'">
        <UiPageHeader title="身份备份" back-label="返回我" @back="backHome" />
        <UiSection title="身份备份" description="身份文件和提示词缺一不可；任意一项丢失都无法恢复这个身份。">
          <template #actions><button class="secondary" @click="ctx.showQr(ctx.backupText.value, '导出身份')">导出身份</button></template>
        </UiSection>
        <UiSection title="完整数据备份" description="加密导出本机联系人、消息和设置；可用于换设备恢复。">
          <template #actions><button class="secondary" @click="showDataBackupEditor = !showDataBackupEditor">{{ showDataBackupEditor ? '隐藏' : '显示备份' }}</button></template>
          <UiActionGroup>
            <button class="secondary" @click="ctx.exportFullDataBackup">生成备份</button>
            <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.downloadText(ctx.dataBackupText.value, 'lm-talk-data-backup.txt')">下载备份</button>
            <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackupMerge">导入合并</button>
            <button class="secondary danger" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackup">导入覆盖</button>
          </UiActionGroup>
          <UiField v-if="showDataBackupEditor" label="完整数据备份文本">
            <textarea v-model="ctx.dataBackupText.value" class="mono" rows="5" aria-label="完整数据备份文本" placeholder="点击生成备份，或粘贴 lm-data-backup-v1 文本后导入" />
          </UiField>
        </UiSection>
      </template>

      <template v-else-if="view === 'security'">
        <UiPageHeader title="安全与设备" back-label="返回我" @back="backHome" />
        <UiSection class="sync-card" title="严格 E2EE" :description="ctx.strictE2eeReadiness.value.text">
          <template #actions><UiStatusBadge :tone="ctx.strictE2eePolicyEnabled.value ? 'success' : 'neutral'">{{ ctx.strictE2eePolicyEnabled.value ? '已开启' : '未开启' }}</UiStatusBadge></template>
          <UiActionGroup>
            <button class="secondary" @click="ctx.enableStrictE2eePolicy">一键严格 E2EE</button>
            <button class="secondary" @click="ctx.showMyFingerprintQr">我的指纹核验码</button>
            <button class="secondary" @click="ctx.copyMyFingerprintProof">复制我的核验码</button>
          </UiActionGroup>
        </UiSection>
        <UiSection class="sync-card" title="设备">
          <small>当前设备：{{ ctx.myDeviceId.value || '未生成' }}</small>
          <small>{{ ctx.sealedSlotCoverageSummary.value }}</small>
        </UiSection>
      </template>

      <template v-else-if="view === 'sync'">
        <UiPageHeader title="同步与安全" back-label="返回我" @back="backHome">
          <template #end><UiStatusBadge compact :tone="syncStatus.tone">{{ syncStatus.text }}</UiStatusBadge></template>
        </UiPageHeader>
        <UiSection class="sync-card" title="消息同步">
          <template #actions><UiStatusBadge :tone="ctx.nodeEnabled.value ? 'success' : 'neutral'">{{ ctx.nodeEnabled.value ? '已开启' : '未开启' }}</UiStatusBadge></template>
          <div v-if="ctx.nodeEntrySummaries.value.length && !showSyncEditor" class="outbox-list">
            <div v-for="entry in ctx.nodeEntrySummaries.value" :key="entry.url" class="outbox-row">
              <b>{{ entry.url }}</b>
              <small>{{ entry.token_configured ? '令牌已配置' : entry.missing_remote_token ? '远端缺令牌' : '本机无需令牌' }}</small>
            </div>
          </div>
          <UiField v-if="showSyncEditor" label="同步服务地址" for-id="sync-service-input">
            <textarea id="sync-service-input" v-model="ctx.nodeControlUrl.value" rows="3" aria-label="同步服务地址列表" placeholder="每行一个同步服务地址，例如：&#10;http://127.0.0.1:8787&#10;http://192.168.1.23:8787|令牌" />
          </UiField>
          <small>开启后可自动收发好友请求和离线消息。跨设备访问节点需在地址后用 <code>|令牌</code> 附上。</small>
          <UiActionGroup>
            <button class="secondary" @click="showSyncServiceEditor = !showSyncServiceEditor">{{ showSyncEditor ? '隐藏编辑' : '编辑地址' }}</button>
            <button @click="ctx.toggleNodeEnabled">{{ ctx.nodeEnabled.value ? '关闭同步' : '开启同步' }}</button>
            <button v-if="showSyncEditor" class="secondary" @click="saveSyncSettings">保存</button>
            <button class="secondary" @click="ctx.syncNow">立即同步</button>
          </UiActionGroup>
        </UiSection>
        <UiSection class="sync-card" title="待发送">
          <template #actions><UiStatusBadge :tone="pendingOutboxCount ? 'warning' : 'neutral'">{{ pendingOutboxCount }}</UiStatusBadge></template>
          <small>失败：{{ failedOutboxCount }}</small>
          <small v-if="ctx.syncFailureSummaryText.value" class="danger-text">{{ ctx.syncFailureSummaryText.value }}</small>
          <UiActionGroup>
            <button v-if="retrySyncIssueText" class="secondary" @click="ctx.recoverSyncFailures">{{ retrySyncIssueText }}</button>
          </UiActionGroup>
          <small v-if="ctx.syncRecoveryStatusText.value && ctx.syncRecoveryStatusText.value !== '尚未恢复'">{{ ctx.syncRecoveryStatusText.value }}</small>
        </UiSection>
        <UiSection class="sync-card" title="诊断">
          <template #actions><button class="secondary" @click="ctx.goDiagnosticsPage('me-sync')">打开诊断工具</button></template>
          <small v-if="ctx.mailboxFailureSummaryText.value" class="danger-text">{{ ctx.mailboxFailureSummaryText.value }}</small>
          <small v-if="ctx.mailboxInboxErrorText.value" class="danger-text">{{ ctx.mailboxInboxErrorText.value }}</small>
          <small>{{ ctx.mailboxInboxStatus.value }}</small>
          <small>{{ ctx.mailboxQuotaStatusText.value }}</small>
        </UiSection>
      </template>

      <template v-else-if="view === 'settings'">
        <UiPageHeader title="设置" back-label="返回我" @back="backHome" />
        <UiSection class="sync-card" title="PWA 应用">
          <template #actions><button class="secondary" @click="ctx.refreshPwaStatus">刷新状态</button></template>
          <small>{{ ctx.pwaStatusText.value }}</small>
          <small>安全边界：PWA 只缓存静态应用壳；不会在 Service Worker 中保存身份密钥、解密消息、后台同步或发送队列。</small>
        </UiSection>
        <UiCard><UiListRow aria-label="清理浏览器缓存" @click="ctx.clearBrowserCaches">清理浏览器缓存</UiListRow></UiCard>
      </template>

      <template v-else-if="view === 'about'">
        <UiPageHeader title="关于" back-label="返回我" @back="backHome" />
        <UiCard class="about-card">
          <h2>LM Talk Web</h2>
          <p>{{ ctx.webVersionText }}</p>
        </UiCard>
      </template>
    </div>
  </div>
</template>
