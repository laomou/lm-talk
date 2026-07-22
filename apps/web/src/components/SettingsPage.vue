<script setup lang="ts">
import { computed, ref } from 'vue'

const props = defineProps<{ ctx: any }>()
type MeView = 'home' | 'profile' | 'backup' | 'security' | 'sync' | 'settings' | 'about'
const view = ref<MeView>('home')
const showSyncServiceEditor = ref(false)
const showDataBackupEditor = ref(false)
const showSyncEditor = computed(() => showSyncServiceEditor.value || props.ctx.nodeEntrySummaries.value.length === 0)
const syncStatus = computed(() => {
  const hasOutbox = props.ctx.outbox.value.some((item: any) => item.status !== 'sent')
  const hasMailboxIssue = props.ctx.mailboxFailureSummaryText.value || props.ctx.mailboxInboxErrorText.value
  const hasSyncIssue = props.ctx.syncFailureSummaryText.value
  return hasOutbox || hasMailboxIssue || hasSyncIssue ? '需处理' : '正常'
})
const syncStatusClass = computed(() => syncStatus.value === '正常' ? 'on' : 'warning')
const pendingOutboxCount = computed(() => props.ctx.outbox.value.filter((item: any) => item.status !== 'sent').length)
const failedOutboxCount = computed(() => props.ctx.outbox.value.filter((item: any) => item.status === 'failed').length)

function backHome() {
  view.value = 'home'
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

        <section class="home-card">
          <div class="settings-rows product-me-rows">
            <button class="settings-row" @click="view = 'profile'"><span>个人资料</span><span class="chevron">›</span></button>
            <button class="settings-row" @click="view = 'backup'"><span>身份备份</span><span class="chevron">›</span></button>
            <button class="settings-row" @click="view = 'security'"><span>安全与设备</span><span class="chevron">›</span></button>
            <button class="settings-row" @click="view = 'sync'"><span>同步与安全</span><span><span class="sync-pill" :class="syncStatusClass">{{ syncStatus }}</span><span class="chevron">›</span></span></button>
            <button class="settings-row" @click="view = 'settings'"><span>设置</span><span class="chevron">›</span></button>
            <button class="settings-row" @click="view = 'about'"><span>关于</span><span class="chevron">›</span></button>
          </div>
        </section>

        <section class="home-card">
          <button class="settings-row danger-row" aria-label="退出登录" @click="ctx.logout">
            <span>退出登录</span><span class="chevron">›</span>
          </button>
        </section>
      </template>

      <template v-else-if="view === 'profile'">
        <header class="detail-bar product-subbar"><button class="back-btn" @click="backHome">‹</button><h2>个人资料</h2><span></span></header>
        <section class="home-card">
          <h3>我的资料</h3>
          <label for="display-name-input">显示名</label>
          <div class="inline-field">
            <input id="display-name-input" v-model="ctx.displayName.value" aria-label="显示名" @change="ctx.refreshMyContactCard" />
            <button @click="ctx.refreshMyContactCard">保存</button>
          </div>
          <div class="row compact">
            <button class="secondary" @click="ctx.showQr(ctx.myContactCardText.value, '我的名片')">我的名片</button>
          </div>
        </section>
      </template>

      <template v-else-if="view === 'backup'">
        <header class="detail-bar product-subbar"><button class="back-btn" @click="backHome">‹</button><h2>身份备份</h2><span></span></header>
        <section class="home-card">
          <div class="section-title-row">
            <h3>身份备份</h3>
            <button class="secondary" @click="ctx.showQr(ctx.backupText.value, '导出身份')">导出身份</button>
          </div>
          <small>身份文件和提示词缺一不可；任意一项丢失都无法恢复这个身份。</small>
        </section>
        <section class="home-card">
          <div class="section-title-row">
            <h3>完整数据备份</h3>
            <button class="secondary" @click="showDataBackupEditor = !showDataBackupEditor">{{ showDataBackupEditor ? '隐藏' : '显示备份' }}</button>
          </div>
          <small>加密导出本机联系人、消息和设置；可用于换设备恢复。</small>
          <div class="row compact">
            <button class="secondary" @click="ctx.exportFullDataBackup">生成备份</button>
            <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.downloadText(ctx.dataBackupText.value, 'lm-talk-data-backup.txt')">下载备份</button>
            <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackupMerge">导入合并</button>
            <button class="secondary danger" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackup">导入覆盖</button>
          </div>
          <textarea v-if="showDataBackupEditor" v-model="ctx.dataBackupText.value" class="mono" rows="5" aria-label="完整数据备份文本" placeholder="点击生成备份，或粘贴 lm-data-backup-v1 文本后导入" />
        </section>
      </template>

      <template v-else-if="view === 'security'">
        <header class="detail-bar product-subbar"><button class="back-btn" @click="backHome">‹</button><h2>安全与设备</h2><span></span></header>
        <section class="home-card sync-card">
          <div class="section-title-row">
            <h3>严格 E2EE</h3>
            <span class="sync-pill" :class="{ on: ctx.strictE2eePolicyEnabled.value }">{{ ctx.strictE2eePolicyEnabled.value ? '已开启' : '未开启' }}</span>
          </div>
          <small>{{ ctx.strictE2eeReadiness.value.text }}</small>
          <div class="row compact">
            <button class="secondary" @click="ctx.enableStrictE2eePolicy">一键严格 E2EE</button>
            <button class="secondary" @click="ctx.showMyFingerprintQr">我的指纹核验码</button>
            <button class="secondary" @click="ctx.copyMyFingerprintProof">复制我的核验码</button>
          </div>
        </section>
        <section class="home-card sync-card">
          <h3>设备</h3>
          <small>当前设备：{{ ctx.myDeviceId.value || '未生成' }}</small>
          <small>{{ ctx.sealedSlotCoverageSummary.value }}</small>
        </section>
      </template>

      <template v-else-if="view === 'sync'">
        <header class="detail-bar product-subbar"><button class="back-btn" @click="backHome">‹</button><h2>同步与安全</h2><span class="sync-pill" :class="syncStatusClass">{{ syncStatus }}</span></header>
        <section class="home-card sync-card">
          <div class="section-title-row">
            <h3>消息同步</h3>
            <span class="sync-pill" :class="{ on: ctx.nodeEnabled.value }">{{ ctx.nodeEnabled.value ? '已开启' : '未开启' }}</span>
          </div>
          <div v-if="ctx.nodeEntrySummaries.value.length && !showSyncEditor" class="outbox-list">
            <div v-for="entry in ctx.nodeEntrySummaries.value" :key="entry.url" class="outbox-row">
              <b>{{ entry.url }}</b>
              <small>{{ entry.token_configured ? '令牌已配置' : entry.missing_remote_token ? '远端缺令牌' : '本机无需令牌' }}</small>
            </div>
          </div>
          <textarea v-if="showSyncEditor" id="sync-service-input" v-model="ctx.nodeControlUrl.value" rows="3" aria-label="同步服务地址列表" placeholder="每行一个同步服务地址，例如：&#10;http://127.0.0.1:8787&#10;http://192.168.1.23:8787|令牌" />
          <small>开启后可自动收发好友请求和离线消息。跨设备访问节点需在地址后用 <code>|令牌</code> 附上。</small>
          <div class="row compact">
            <button class="secondary" @click="showSyncServiceEditor = !showSyncServiceEditor">{{ showSyncEditor ? '隐藏编辑' : '编辑地址' }}</button>
            <button @click="ctx.toggleNodeEnabled">{{ ctx.nodeEnabled.value ? '关闭同步' : '开启同步' }}</button>
            <button v-if="showSyncEditor" class="secondary" @click="ctx.saveNetworkSettings">保存</button>
            <button class="secondary" @click="ctx.syncNow">立即同步</button>
          </div>
        </section>
        <section class="home-card sync-card">
          <div class="section-title-row"><h3>待发送</h3><span class="sync-pill" :class="{ warning: pendingOutboxCount }">{{ pendingOutboxCount }}</span></div>
          <small>失败：{{ failedOutboxCount }}</small>
          <small v-if="ctx.syncFailureSummaryText.value" class="danger-text">{{ ctx.syncFailureSummaryText.value }}</small>
          <div class="row compact">
            <button class="secondary" :disabled="!pendingOutboxCount" @click="ctx.retryAllOutbox">全部重试</button>
            <button class="secondary" @click="ctx.recoverSyncFailures">修复同步失败</button>
          </div>
        </section>
        <section class="home-card sync-card">
          <div class="section-title-row"><h3>诊断</h3><button class="secondary" @click="ctx.goDiagnosticsPage">打开诊断工具</button></div>
          <small v-if="ctx.mailboxFailureSummaryText.value" class="danger-text">{{ ctx.mailboxFailureSummaryText.value }}</small>
          <small v-if="ctx.mailboxInboxErrorText.value" class="danger-text">{{ ctx.mailboxInboxErrorText.value }}</small>
          <small>{{ ctx.mailboxInboxStatus.value }}</small>
          <small>{{ ctx.mailboxQuotaStatusText.value }}</small>
        </section>
      </template>

      <template v-else-if="view === 'settings'">
        <header class="detail-bar product-subbar"><button class="back-btn" @click="backHome">‹</button><h2>设置</h2><span></span></header>
        <section class="home-card sync-card">
          <div class="section-title-row"><h3>PWA 应用</h3><button class="secondary" @click="ctx.refreshPwaStatus">刷新状态</button></div>
          <small>{{ ctx.pwaStatusText.value }}</small>
          <small>安全边界：PWA 只缓存静态应用壳；不会在 Service Worker 中保存身份密钥、解密消息、后台同步或发送队列。</small>
        </section>
        <section class="home-card"><button class="settings-row" aria-label="清理浏览器缓存" @click="ctx.clearBrowserCaches"><span>清理浏览器缓存</span><span class="chevron">›</span></button></section>
      </template>

      <template v-else-if="view === 'about'">
        <header class="detail-bar product-subbar"><button class="back-btn" @click="backHome">‹</button><h2>关于</h2><span></span></header>
        <section class="home-card about-card">
          <h2>LM Talk Web</h2>
          <p>{{ ctx.webVersionText }}</p>
        </section>
      </template>
    </div>
  </div>
</template>
