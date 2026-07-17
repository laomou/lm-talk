<script setup lang="ts">
import { computed, ref } from 'vue'

const props = defineProps<{ ctx: any }>()
const syncRecoveryQuery = ref('')
const showRawSyncStatus = ref(false)
const showSyncServiceEditor = ref(false)
const showDataBackupEditor = ref(false)

const outboxItems = computed(() => props.ctx.outbox.value)
const pendingOutbox = computed(() => outboxItems.value.filter((item: any) => item.status !== 'sent'))
const failedOutbox = computed(() => outboxItems.value.filter((item: any) => item.status === 'failed'))
const contactName = (userId: string) => props.ctx.contacts.value.find((c: any) => c.user_id === userId)?.display_name || userId
const outboxKindLabel = (kind?: string) =>
  kind === 'group-fanout' ? '群消息' : kind === 'file-package' ? '文件' : kind === 'other' ? '系统消息' : '单聊消息'
function outboxExpiryText(item: any) {
  if (!item.expires_at) return '无过期时间'
  if (Date.now() > item.expires_at) return '已过期'
  return `过期 ${props.ctx.formatDateTime(item.expires_at)}`
}
function outboxRetryText(item: any) {
  const nextRetryAt = item.next_retry_at ?? item.created_at
  if (!nextRetryAt || Date.now() >= nextRetryAt) return '下次重试：可立即重试'
  return `下次重试：${props.ctx.formatDateTime(nextRetryAt)}`
}

function outboxSummaryText(summary: any) {
  if (!summary) return ''
  const parts = [
    `queued ${Number(summary.queued || 0)}`,
    `failed ${Number(summary.failed || 0)}`,
    `sent ${Number(summary.sent || 0)}`,
  ]
  if (summary.oldest_pending_at) parts.push(`最早待处理 ${new Date(summary.oldest_pending_at).toLocaleString('zh-CN')}`)
  if (summary.failed_kinds && Object.keys(summary.failed_kinds).length) {
    parts.push(`失败类型 ${Object.entries(summary.failed_kinds).map(([k, v]) => `${k}:${v}`).join('，')}`)
  }
  return parts.join('；')
}

function selfSyncCacheExpiryText(item: any) {
  if (!item?.expires_at) return '最近轻量包缓存未设置过期时间'
  if (Date.now() > item.expires_at) return '最近轻量包缓存已过期，下一次使用前会自动清理'
  return `最近轻量包缓存保留至：${props.ctx.formatDateTime(item.expires_at)}`
}
const localObjectCount = computed(() =>
  props.ctx.contacts.value.length +
  props.ctx.groups.value.length +
  props.ctx.messages.value.length +
  props.ctx.outbox.value.length
)
const filteredSyncRecoveryHistory = computed(() => {
  const q = syncRecoveryQuery.value.trim().toLowerCase()
  const history = props.ctx.syncRecoveryHistory.value
  return q ? history.filter((item: string) => item.toLowerCase().includes(q)) : history
})
const syncStatusText = computed(() => props.ctx.nodeControlStatus.value || '未连接')
const syncStatusSummary = computed(() => syncStatusText.value.split('\n')[0])
const hasRawSyncStatus = computed(() => syncStatusText.value.includes('\n') || syncStatusText.value.trim().startsWith('{') || syncStatusText.value.trim().startsWith('['))
const showSyncEditor = computed(() => showSyncServiceEditor.value || props.ctx.nodeEntrySummaries.value.length === 0)
</script>

<template>
  <div class="me-page">
    <div class="me-inner">
      <header class="me-hero">
        <span class="avatar large">{{ (ctx.displayName.value || ctx.identity.value?.user_id || '?').slice(0, 1).toUpperCase() }}</span>
        <div class="me-hero-text">
          <h2>{{ ctx.displayName.value || '未命名' }}</h2>
          <small>{{ ctx.identity.value?.user_id }}</small>
        </div>
        <div class="row compact me-hero-actions">
          <button class="secondary" @click="ctx.showQr(ctx.myContactCardText.value, '我的名片')">我的名片</button>
          <button class="secondary" @click="ctx.showMyFingerprintQr">我的指纹核验码</button>
          <button class="secondary" @click="ctx.copyMyFingerprintProof">复制指纹核验码</button>
          <button class="secondary" @click="ctx.showQr(ctx.backupText.value, '导出身份')">导出身份</button>
        </div>
      </header>

      <section class="home-card">
        <h3>我的资料</h3>
        <label for="display-name-input">显示名</label>
        <div class="inline-field">
          <input id="display-name-input" v-model="ctx.displayName.value" aria-label="显示名" @change="ctx.refreshMyContactCard" />
          <button @click="ctx.refreshMyContactCard">保存</button>
        </div>
        <label for="new-identity-passphrase">新提示词</label>
        <div class="inline-field">
          <input id="new-identity-passphrase" v-model="ctx.newIdentityPassphrase.value" type="password" aria-label="新身份备份提示词" autocomplete="new-password" placeholder="重新加密身份备份" />
          <button class="secondary" :disabled="!ctx.newIdentityPassphrase.value.trim()" @click="ctx.reencryptCurrentIdentityBackup">重加密身份</button>
        </div>
        <small>重加密后请重新导出身份；本机保存的登录入口会同步更新。</small>
      </section>

      <section class="home-card">
        <div class="section-title-row">
          <h3>设备与撤销</h3>
          <button class="secondary" @click="ctx.createMyDeviceCert">生成本设备证书</button>
        </div>
        <small>设备证书用于后续多设备信任；撤销事件可分发给好友，提醒对方停止信任已丢失或废弃设备。</small>
        <small v-if="ctx.myDeviceId.value">当前设备：{{ ctx.myDeviceId.value }}</small>
        <small v-if="ctx.contactCardDhtAutoRefreshCount.value || ctx.lastContactCardDhtAutoRefreshError.value" :class="{ 'danger-text': ctx.lastContactCardDhtAutoRefreshError.value }">后台 ContactCard DHT 刷新：{{ ctx.contactCardDhtAutoRefreshCount.value }} 次<span v-if="ctx.lastContactCardDhtAutoRefreshAt.value">，最近 {{ ctx.formatDateTime(ctx.lastContactCardDhtAutoRefreshAt.value) }}</span><span v-if="ctx.lastContactCardDhtAutoRefreshError.value">，错误：{{ ctx.lastContactCardDhtAutoRefreshError.value }}</span></small>
        <small v-for="item in ctx.contactCardDhtAutoRefreshHistory.value.slice(0, 3)" :key="`${item.user_id}-${item.refreshed_at}`" :class="{ 'danger-text': item.status === 'failed' }">后台 DHT：{{ item.display_name || item.user_id }} · {{ item.status === 'success' ? '成功' : `失败：${item.error}` }} · {{ ctx.formatDateTime(item.refreshed_at) }}</small>
        <small v-if="ctx.contactCardUpdateFanoutCount.value || ctx.contactCardUpdateFanoutSkipCount.value">设备证书更新分发：{{ ctx.contactCardUpdateFanoutCount.value }} 次，确认 {{ ctx.contactCardUpdateFanoutAckCount.value }}/{{ ctx.contactCardUpdateFanoutRecords.value.length }}，待确认 {{ ctx.contactCardUpdatePendingAckCount.value }}，过期 {{ ctx.contactCardUpdateStaleAckCount.value }}，节流 {{ ctx.contactCardUpdateFanoutSkipCount.value }} 次<span v-if="ctx.lastContactCardUpdateFanoutAt.value">，最近 {{ ctx.formatDateTime(ctx.lastContactCardUpdateFanoutAt.value) }}</span></small>
        <button v-if="ctx.contactCardUpdateStaleAckCount.value" class="secondary" @click="ctx.retryStaleContactCardUpdateAcks">重试过期确认</button>
        <div class="row compact" v-if="ctx.myDeviceCertJson.value">
          <button class="secondary" @click="ctx.copyText(ctx.myDeviceCertJson.value, '设备证书')">复制设备证书</button>
          <button class="secondary" @click="ctx.showQr(ctx.myDeviceCertJson.value, '设备证书')">设备证书二维码</button>
          <button class="secondary" @click="ctx.fanoutMyContactCardUpdateToFriends({ force: true })">分发设备证书更新</button>
        </div>
        <label for="device-revoke-id-input">撤销 Device ID</label>
        <input id="device-revoke-id-input" v-model="ctx.revokeDeviceId.value" placeholder="输入要撤销的 device_id" />
        <label for="device-revoke-reason-input">撤销原因</label>
        <input id="device-revoke-reason-input" v-model="ctx.revokeReason.value" placeholder="可选，例如：设备丢失 / 已更换" />
        <div class="row compact">
          <button class="secondary" @click="ctx.createDeviceRevokeText">生成撤销事件</button>
          <button class="secondary" :disabled="!ctx.deviceRevokeText.value" @click="ctx.copyText(ctx.deviceRevokeText.value, '设备撤销事件')">复制撤销事件</button>
          <button class="secondary" :disabled="!ctx.deviceRevokeText.value" @click="ctx.showQr(ctx.deviceRevokeText.value, '设备撤销事件')">撤销二维码</button>
          <button class="secondary danger" :disabled="!ctx.deviceRevokeText.value" @click="ctx.fanoutDeviceRevokeToFriends">分发给好友</button>
        </div>
        <textarea v-if="ctx.deviceRevokeText.value" v-model="ctx.deviceRevokeText.value" class="mono" rows="3" readonly />
      </section>

      <section class="home-card sync-card">
        <div class="section-title-row">
          <h3>消息同步</h3>
          <span class="sync-pill" :class="{ on: ctx.nodeEnabled.value }">{{ ctx.nodeEnabled.value ? '已开启' : '未开启' }}</span>
        </div>
        <label for="sync-service-input">同步服务</label>
        <div v-if="ctx.nodeEntrySummaries.value.length && !showSyncEditor" class="outbox-list">
          <div v-for="entry in ctx.nodeEntrySummaries.value" :key="entry.url" class="outbox-row">
            <b>{{ entry.url }}</b>
            <small>{{ entry.token_configured ? '令牌已配置' : entry.missing_remote_token ? '远端缺令牌' : '本机无需令牌' }}</small>
            <small v-if="entry.missing_remote_token" class="danger-text">点击“编辑地址/令牌”，在地址后追加 |令牌。</small>
          </div>
        </div>
        <textarea v-if="showSyncEditor" id="sync-service-input" v-model="ctx.nodeControlUrl.value" rows="4" aria-label="同步服务地址列表" placeholder="每行一个同步服务地址，例如：&#10;http://127.0.0.1:8787&#10;http://192.168.1.23:8787|令牌&#10;http://[fd00::1234]:8787|令牌" />
        <small>{{ ctx.nodeSettingsSummaryText.value }}</small>
        <small v-if="ctx.nodeTokenStorageText.value">{{ ctx.nodeTokenStorageText.value }}</small>
        <small>{{ ctx.syncTriggerPolicyText.value }}</small>
        <small>开启后可自动收发好友请求和离线消息。支持局域网 IPv4/IPv6，可填多个。<br>跨设备访问时节点需设 <code>--control-token</code>，在地址后用 <code>|令牌</code> 附上（与节点一致）；仅本机(127.0.0.1)可不填令牌。</small>
        <div class="row compact">
          <button class="secondary" @click="showSyncServiceEditor = !showSyncServiceEditor">{{ showSyncEditor ? '隐藏地址/令牌' : '编辑地址/令牌' }}</button>
          <button @click="ctx.toggleNodeEnabled">{{ ctx.nodeEnabled.value ? '关闭同步' : '开启同步' }}</button>
          <button v-if="showSyncEditor" class="secondary" @click="ctx.saveNetworkSettings">保存</button>
          <button class="secondary" @click="ctx.syncNow">立即同步</button>
        </div>
        <div class="policy-grid sync-options">
          <label class="identity-select">
            <input v-model="ctx.autoPublishPreKey.value" type="checkbox" />
            <span>登录/同步时发布 PreKey</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.autoMailboxTake.value" type="checkbox" />
            <span>自动收取 Mailbox</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.autoReadReceipts.value" type="checkbox" />
            <span>当前会话自动发送已读回执</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.autoNodeSync.value" type="checkbox" />
            <span>自动同步节点快照</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.autoSelfMailboxSync.value" type="checkbox" />
            <span>手动同步时发送轻量自同步包</span>
          </label>
        </div>
        <section class="home-card">
          <h3>节点快照同步</h3>
          <small>用于把当前 lm_node 的 mailbox、PreKey、DHT records 和 routing peers 与另一节点同步；不包含身份私钥。</small>
          <label for="node-sync-peer-input">对端节点</label>
          <input id="node-sync-peer-input" v-model="ctx.nodeSyncPeerUrl.value" placeholder="http://127.0.0.1:8788" />
          <div class="row compact">
            <button class="secondary" @click="ctx.exportNodeSnapshot">导出本节点快照</button>
            <button class="secondary" @click="ctx.importNodeSnapshot">导入到本节点</button>
            <button class="secondary" @click="ctx.pullSnapshotFromPeerNode">从对端拉取并导入</button>
          </div>
          <small :class="{ 'danger-text': ctx.nodeSnapshotSyncFreshnessLevel.value !== 'ok' }">{{ ctx.nodeSnapshotSyncFreshnessText.value }}</small>
          <small v-if="ctx.lastNodeSnapshotSyncAt.value">最近快照导入：{{ ctx.formatDateTime(ctx.lastNodeSnapshotSyncAt.value) }}</small>
          <small v-if="ctx.nodeSyncStatusText.value">{{ ctx.nodeSyncStatusText.value.slice(0, 180) }}</small>
          <textarea v-model="ctx.nodeSyncSnapshotText.value" class="mono" rows="4" placeholder="节点 snapshot JSON，可粘贴后导入" />
        </section>
        <div class="sync-status">
          <b>同步状态</b>
          <small>{{ showRawSyncStatus ? syncStatusText : syncStatusSummary }}</small>
          <small>{{ ctx.nodeHealthSummaryText.value }}</small>
          <small :class="{ 'danger-text': ctx.nodeStateDbSecurityLevel.value !== 'ok' }">{{ ctx.nodeStateDbSecurityText.value }}</small>
          <small :class="{ 'danger-text': ctx.nodeStateFileSecurityLevel.value !== 'ok' }">{{ ctx.nodeStateFileSecurityText.value }}</small>
          <small :class="{ 'danger-text': ctx.nodePeerHealthRiskLevel.value !== 'ok' }">{{ ctx.nodePeerHealthStatusText.value }}</small>
          <div v-if="ctx.nodePeerHealthPeers.value.length" class="outbox-list">
            <div v-for="peer in ctx.nodePeerHealthPeers.value.filter((p: any) => p.consecutive_failures > 0 || p.quarantined).slice(0, 4)" :key="peer.url" class="outbox-row">
              <b>{{ peer.url }}</b>
              <small :class="{ 'danger-text': peer.quarantined }">连续失败 {{ peer.consecutive_failures }} · 累计失败 {{ peer.failures }}{{ peer.quarantined ? ' · 已隔离' : '' }}</small>
              <small v-if="peer.last_error" class="danger-text">{{ peer.last_error }}</small>
              <button class="secondary" @click="ctx.resetDhtPeerHealth(peer.url)">重置 {{ peer.url }}</button>
            </div>
          </div>

          <section class="home-card">
            <h3>PublicPeer 公网发现</h3>
            <small>填写可被其他节点访问的 multiaddr；登录/同步时会自动把 PublicPeer 发布到 DHT。能力可包含 bootstrap / dht / relay / mailbox。</small>
            <label for="public-peer-id-input">Public peer id</label>
            <input id="public-peer-id-input" v-model="ctx.publicPeerId.value" placeholder="留空自动生成 public-..." />
            <label for="public-peer-addresses-input">公网地址</label>
            <textarea id="public-peer-addresses-input" v-model="ctx.publicPeerAddressesText.value" rows="3" placeholder="每行一个 multiaddr，例如 /dns4/example.com/tcp/443/wss" />
            <div class="policy-grid sync-options">
              <label v-for="cap in ['bootstrap', 'dht', 'relay', 'mailbox']" :key="cap" class="identity-select">
                <input v-model="ctx.publicPeerCapabilities.value" type="checkbox" :value="cap" />
                <span>{{ cap }}</span>
              </label>
            </div>
            <div class="row compact">
              <button class="secondary" @click="ctx.createPublicPeerAnnounceText">生成 PublicPeer</button>
              <button class="secondary" :disabled="!ctx.publicPeerAnnounceText.value" @click="ctx.inspectPublicPeerAnnounceText">验签</button>
              <button class="secondary" :disabled="!ctx.publicPeerAnnounceText.value" @click="ctx.copyText(ctx.publicPeerAnnounceText.value, 'PublicPeerAnnounce')">复制</button>
              <button class="secondary" :disabled="!ctx.publicPeerAnnounceText.value" @click="ctx.publishAndCheckMyPublicPeerDht">发布 PublicPeer DHT</button>
            </div>
            <small v-if="ctx.publicPeerAnnounceInfoText.value">{{ ctx.publicPeerAnnounceInfoText.value.slice(0, 180) }}</small>
          </section>
          <label for="dht-key-value-input">DHT key 派生</label>
          <div class="inline-field">
            <select v-model="ctx.nodeDhtKeyKind.value" aria-label="DHT key 类型">
              <option value="prekey">PreKey(UserID)</option>
              <option value="mailbox-hint">MailboxHint(UserID)</option>
              <option value="public-peer">PublicPeer(peer_id)</option>
              <option value="contact-card">ContactCard(UserID)</option>
            </select>
            <input id="dht-key-value-input" v-model="ctx.nodeDhtKeyValue.value" aria-label="DHT key 输入值" placeholder="UserID 或 peer_id" />
            <button class="secondary" @click="ctx.fillMyPreKeyDhtKeyInput">我的 PreKey</button>
            <button class="secondary" @click="ctx.fillMyMailboxHintDhtKeyInput">我的 MailboxHint</button>
            <button class="secondary" @click="ctx.fillMyContactCardDhtKeyInput">我的 ContactCard</button>
            <button class="secondary" @click="ctx.fillCurrentPublicPeerDhtKeyInput">当前 PublicPeer</button>
            <button class="secondary" @click="ctx.deriveDhtKeyForFindValue">派生 key</button>
            <button class="secondary" @click="ctx.deriveAndFindDhtValueNow">派生并查找</button>
          </div>
          <label for="dht-find-key-input">DHT record key</label>
          <div class="inline-field">
            <input id="dht-find-key-input" v-model="ctx.nodeDhtFindValueKey.value" aria-label="DHT record key" placeholder="64 位十六进制 key" />
            <button class="secondary" @click="ctx.runDhtFindValueNow">查找 DHT 记录</button>
            <button class="secondary" @click="ctx.publishAndCheckMyMailboxHintDht">发布并查 MailboxHint</button>
            <button class="secondary" @click="ctx.publishAndCheckMyContactCardDht">发布并查 ContactCard</button>
            <button class="secondary" @click="ctx.publishAndCheckMyPublicPeerDht">发布并查 PublicPeer</button>
          </div>
          <div class="row compact">
            <button class="secondary" @click="ctx.checkNodeHealth">刷新节点健康</button>
            <button class="secondary" @click="ctx.runDhtMaintenanceNow">运行 DHT 维护</button>
            <button class="secondary" @click="ctx.runDhtReplicationNow">复制 DHT 记录</button>
            <button class="secondary" @click="ctx.runDhtRoutingRefreshNow">刷新 DHT 路由</button>
            <button v-if="hasRawSyncStatus" class="secondary" @click="showRawSyncStatus = !showRawSyncStatus">{{ showRawSyncStatus ? '隐藏原始状态' : '显示原始状态' }}</button>
          </div>
          <small>{{ ctx.nodeDhtFindValueStatusText.value }}</small>
          <div v-if="ctx.discoveredMailboxHintUrl.value" class="row compact">
            <small>发现 MailboxHint：{{ ctx.discoveredMailboxHintUrl.value }}</small>
            <button class="secondary" @click="ctx.addDiscoveredMailboxHintToSyncServices">加入同步服务</button>
          </div>
          <small>{{ ctx.nodeDhtMaintenanceStatusText.value }}</small>
          <small>{{ ctx.nodeDhtReplicationStatusText.value }}</small>
          <small>{{ ctx.nodeRoutingRefreshStatusText.value }}</small>
          <small v-if="ctx.nodeDhtOperationHistory.value.length">DHT 操作历史：{{ ctx.nodeDhtOperationHistory.value.slice(0, 4).join(' ｜ ') }}</small>
          <div v-if="ctx.nodeDhtOperationHistory.value.length" class="row compact">
            <button class="secondary" @click="ctx.copyDhtOperationHistory">复制 DHT 历史</button>
            <button class="secondary" @click="ctx.exportDhtOperationHistory">导出 DHT 历史</button>
            <button class="secondary" @click="ctx.clearDhtOperationHistory">清空 DHT 历史</button>
          </div>
          <details class="advanced-block">
            <summary>导入 DHT 历史</summary>
            <textarea v-model="ctx.nodeDhtOperationHistoryImportText.value" class="mono" rows="3" aria-label="DHT 操作历史 JSON" placeholder='粘贴 {"history":[...]}、诊断报告 JSON 或 JSON 数组' />
            <small :class="{ 'danger-text': ctx.nodeDhtOperationHistoryImportStatus.value.includes('失败') }">{{ ctx.nodeDhtOperationHistoryImportStatus.value }}</small>
            <button class="secondary" :disabled="!ctx.nodeDhtOperationHistoryImportText.value.trim()" @click="ctx.importDhtOperationHistory">导入 DHT 历史</button>
          </details>
          <small :class="{ 'danger-text': ctx.syncFailureSummaryText.value !== '暂无同步失败' }">{{ ctx.syncFailureSummaryText.value }}</small>
          <small>{{ ctx.syncRecoveryStatusText.value }}</small>
          <input v-if="ctx.syncRecoveryHistory.value.length" v-model="syncRecoveryQuery" type="search" aria-label="筛选同步恢复历史" placeholder="筛选恢复历史" />
          <small v-if="filteredSyncRecoveryHistory.length">历史：{{ filteredSyncRecoveryHistory.join(' ｜ ') }}</small>
          <div v-if="ctx.syncRecoveryHistory.value.length" class="row compact">
            <button class="secondary" @click="ctx.exportSyncRecoveryHistory">导出恢复历史</button>
            <button class="secondary danger" @click="ctx.clearSyncRecoveryHistory">清空恢复历史</button>
          </div>
          <button v-if="ctx.syncFailureSummaryText.value !== '暂无同步失败'" class="secondary" @click="ctx.recoverSyncFailures">恢复同步失败</button>
        </div>
        <div class="sync-status">
          <b>运行环境</b>
          <small>{{ ctx.runtimeStatusText.value }}</small>
          <small>{{ ctx.notificationRuntimePolicyText.value }}</small>
        </div>
        <div class="sync-status">
          <b>PreKey</b>
          <small>{{ ctx.prekeyStatusSummary.value }}</small>
          <small>自动状态：{{ ctx.prekeyAutoStateText.value }}</small>
          <small v-if="ctx.prekeyAutoErrorText.value" class="danger-text">{{ ctx.prekeyAutoErrorText.value }}</small>
        </div>
        <div class="row compact">
          <button class="secondary" @click="ctx.publishPreKeyToNode">发布 PreKey</button>
          <button class="secondary" @click="ctx.publishAndCheckMyPreKeyDht">发布并查 DHT</button>
          <button class="secondary" @click="ctx.publishAndCheckAllMyDht">发布并查全部 DHT</button>
          <button class="secondary" @click="ctx.refreshPreKeyStatusFromNode">刷新 PreKey</button>
          <button class="secondary" @click="ctx.replenishPreKeyIfLow">检查补货</button>
          <button class="secondary" @click="ctx.clearPreKeyRawState">清除公开原文</button>
          <button class="secondary" @click="ctx.clearSecureSessionRawText">清除会话原文</button>
          <button v-if="ctx.prekeyAutoErrorText.value" class="secondary" @click="ctx.retryPreKeyAutoPublish">重试自动发布</button>
        </div>
        <small>清除公开原文只移除节点返回和 selected record 等临时文本；清除会话原文只移除 Offer/Response 输入输出文本，不删除已建立会话。</small>
        <div class="row compact">
          <button class="secondary" @click="ctx.enableNotifications">开启通知</button>
          <button class="secondary" @click="ctx.refreshRuntimeStatus">刷新状态</button>
          <small class="sync-note">通知：{{ ctx.notificationPermission.value || '未知' }}</small>
        </div>
      </section>

      <section class="home-card outbox-card">
        <div class="section-title-row">
          <h3>待发送队列</h3>
          <span class="sync-pill" :class="{ on: pendingOutbox.length === 0 }">
            {{ pendingOutbox.length ? `${pendingOutbox.length} 待处理` : '已清空' }}
          </span>
        </div>
        <div class="outbox-summary">
          <span>总数 {{ outboxItems.length }}</span>
          <span>失败 {{ failedOutbox.length }}</span>
        </div>
        <div v-if="pendingOutbox.length" class="outbox-list">
          <div v-for="item in pendingOutbox.slice(0, 6)" :key="item.id" class="outbox-row">
            <b>{{ contactName(item.peer_user_id) }}</b>
            <small>{{ outboxKindLabel(item.kind) }} · 重试 {{ item.retry_count }}</small>
            <small>{{ outboxRetryText(item) }}</small>
            <small>{{ outboxExpiryText(item) }}</small>
            <small v-if="item.last_error" class="danger-text">{{ item.last_error }}</small>
          </div>
        </div>
        <div v-else class="empty">没有待发送内容</div>
        <div class="row compact">
          <button class="secondary" :disabled="pendingOutbox.length === 0" @click="ctx.retryAllOutbox">重试全部</button>
          <button class="secondary" @click="ctx.clearSentOutbox">清理已发送</button>
        </div>
      </section>

      <section class="home-card storage-card">
        <div class="section-title-row">
          <h3>本地存储</h3>
          <button class="secondary" @click="ctx.refreshStorageEstimate">刷新</button>
        </div>
        <div class="outbox-summary">
          <span>对象 {{ localObjectCount }}</span>
          <span>消息 {{ ctx.messages.value.length }}</span>
          <span>队列 {{ ctx.outbox.value.length }}</span>
        </div>
        <div class="sync-status">
          <b>浏览器估算</b>
          <small>{{ ctx.storageEstimateText.value }}</small>
        </div>
      </section>

      <section class="home-card">
        <div class="section-title-row">
          <h3>PWA 状态</h3>
          <button class="secondary" @click="ctx.refreshPwaStatus">刷新</button>
          <button class="secondary" @click="ctx.registerPeriodicMailboxSync">注册后台同步</button>
        </div>
        <div class="sync-status">
          <b>版本</b>
          <small>{{ ctx.webVersionText }}</small>
        </div>
        <div class="sync-status">
          <b>离线缓存</b>
          <small>{{ ctx.pwaStatusText.value }}</small>
          <small>{{ ctx.pwaBackgroundCapabilityText.value }}</small>
          <small>最近后台事件：{{ ctx.pwaLastBackgroundEventText.value }}</small>
          <small v-if="ctx.pwaBackgroundEventHistory.value.length > 1">后台事件历史：{{ ctx.pwaBackgroundEventHistory.value.slice(0, 5).join('；') }}</small>
        </div>
      </section>


      <section class="home-card">
        <div class="section-title-row">
          <h3>完整数据备份</h3>
          <button class="secondary" @click="showDataBackupEditor = !showDataBackupEditor">{{ showDataBackupEditor ? '隐藏备份文本' : '显示备份文本' }}</button>
        </div>
        <small>完整数据备份会加密导出本机联系人、群聊、消息、待发送队列、同步设置和安全会话状态；可选择“导入合并”只补缺失数据，或“导入覆盖”替换当前身份的本地数据。</small>
        <small :class="{ 'danger-text': ctx.fullDataBackupFreshnessLevel.value !== 'ok' }">{{ ctx.fullDataBackupFreshnessText.value }}</small>
        <small v-if="ctx.lastFullDataBackupAt.value">最近生成备份：{{ ctx.formatDateTime(ctx.lastFullDataBackupAt.value) }}</small>
        <small v-if="ctx.lastSelfMailboxBackupPushedAt.value">最近备份到自己的 Mailbox：{{ ctx.formatDateTime(ctx.lastSelfMailboxBackupPushedAt.value) }}</small>
        <small>{{ ctx.selfMailboxBackupStatusText.value }}</small>
        <small v-if="ctx.lastSelfMailboxBackupReceivedAt.value" :class="{ 'danger-text': ctx.selfMailboxBackupMergePending.value }">{{ ctx.selfMailboxBackupMergeStatusText.value }}</small>
        <small v-if="ctx.lastSelfMailboxBackupReceivedAt.value">最近收到自己的 Mailbox 备份：{{ ctx.formatDateTime(ctx.lastSelfMailboxBackupReceivedAt.value) }}</small>
        <small v-if="ctx.lastSelfMailboxBackupMergedAt.value">最近合并自己的 Mailbox 备份：{{ ctx.formatDateTime(ctx.lastSelfMailboxBackupMergedAt.value) }}</small>
        <small>{{ ctx.selfSyncStatusText.value }}</small>
        <small v-if="ctx.selfSyncRequestSentCount.value || ctx.selfSyncRequestHitCount.value || ctx.selfSyncRequestMissCount.value">缺包请求：发送 {{ ctx.selfSyncRequestSentCount.value }}，命中 {{ ctx.selfSyncRequestHitCount.value }}，未命中 {{ ctx.selfSyncRequestMissCount.value }}</small>
        <small v-if="ctx.selfSyncRecentPackages.value.length">最近轻量包缓存：{{ ctx.selfSyncRecentPackages.value.length }}/10；{{ selfSyncCacheExpiryText(ctx.selfSyncRecentPackages.value[0]) }}</small>
        <small v-if="ctx.lastSelfSyncPushedAt.value">最近投递轻量自同步：#{{ ctx.lastSelfSyncSequenceSent.value }} · {{ ctx.formatDateTime(ctx.lastSelfSyncPushedAt.value) }}</small>
        <small v-if="ctx.lastSelfSyncMergedAt.value">最近合并轻量自同步：#{{ ctx.lastSelfSyncSequenceMerged.value }} · {{ ctx.formatDateTime(ctx.lastSelfSyncMergedAt.value) }}</small>
        <small v-if="ctx.lastSelfSyncReceiptStatesSent.value || ctx.lastSelfSyncReceiptStatesMerged.value">回执自同步：最近发送 {{ ctx.lastSelfSyncReceiptStatesSent.value }}，最近合并 {{ ctx.lastSelfSyncReceiptStatesMerged.value }}，累计合并 {{ ctx.totalSelfSyncReceiptStatesMerged.value }}</small>
        <small v-if="ctx.lastSelfSyncOutboxSummary.value" :class="{ 'danger-text': Number(ctx.lastSelfSyncOutboxSummary.value.failed || 0) > 0 }">远端设备 outbox：{{ outboxSummaryText(ctx.lastSelfSyncOutboxSummary.value) }}</small>
        <div v-if="ctx.selfSyncGapCount.value" class="row compact">
          <small class="danger-text">轻量自同步可能缺口 {{ ctx.selfSyncGapCount.value }} 次<span v-if="ctx.lastSelfSyncGapAt.value">，最近：{{ ctx.formatDateTime(ctx.lastSelfSyncGapAt.value) }}</span><span v-if="ctx.lastSelfSyncMissingPreviousId.value">，缺失：{{ ctx.lastSelfSyncMissingPreviousId.value.slice(0, 8) }}</span></small>
          <button class="secondary" :disabled="!ctx.nodeEnabled.value" @click="ctx.repairSelfSyncGapNow">补发自同步包</button>
          <button class="secondary" @click="ctx.clearSelfSyncGapStats">清空缺口统计</button>
        </div>
        <div class="row compact">
          <button class="secondary" @click="ctx.exportFullDataBackup">生成备份</button>
          <button class="secondary" :disabled="!ctx.nodeEnabled.value" @click="ctx.pushSelfSyncPackageToOwnMailbox">轻量状态到自己 Mailbox</button>
          <button class="secondary" :disabled="!ctx.nodeEnabled.value || ctx.selfSyncRecentPackages.value.length === 0" @click="ctx.resendLatestSelfSyncPackageToOwnMailbox">重发最近轻量包</button>
          <button class="secondary" :disabled="ctx.selfSyncRecentPackages.value.length === 0" @click="ctx.clearSelfSyncRecentPackages">清空轻量包缓存</button>
          <button class="secondary" :disabled="!ctx.nodeEnabled.value" @click="ctx.pushFullDataBackupToOwnMailbox">备份到自己 Mailbox</button>
          <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.downloadText(ctx.dataBackupText.value, 'lm-talk-data-backup.txt')">下载备份</button>
          <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackupMerge">导入合并</button>
          <button class="secondary" :disabled="!ctx.selfMailboxBackupMergePending.value || !ctx.dataBackupText.value.trim()" @click="ctx.mergeSelfMailboxBackupNow">合并收到的 Mailbox 备份</button>
          <button class="secondary danger" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackup">导入覆盖</button>
        </div>
        <textarea
          v-if="showDataBackupEditor"
          v-model="ctx.dataBackupText.value"
          class="mono"
          rows="6"
          aria-label="完整数据备份文本"
          placeholder="点击生成备份，或粘贴 lm-data-backup-v1 文本后导入"
        />
      </section>

      <section class="home-card">
        <div class="section-title-row">
          <h3>本地安全策略</h3>
          <div class="row compact">
            <button class="secondary" @click="ctx.enableStrictE2eePolicy">一键严格 E2EE</button>
            <button class="secondary" @click="ctx.saveSafetyPolicy">保存</button>
          </div>
        </div>
        <label class="identity-select">
          <input v-model="ctx.safetyPolicy.value.enableTextFilter" type="checkbox" />
          <span>启用文本过滤</span>
        </label>
        <p class="muted">好友指纹核验：已核验 {{ ctx.verifiedFriendContactCount.value }}，未核验 {{ ctx.unverifiedFriendContactCount.value }}</p>
        <p class="muted">严格 E2EE 策略：{{ ctx.strictE2eePolicyEnabled.value ? '已启用' : '未完全启用' }}；{{ ctx.strictE2eeReadiness.value.text }}</p>
        <div v-if="ctx.strictE2eeReadinessIssues.value.length" class="callout warning">
          <b>严格 E2EE 启用前检查</b>
          <div v-for="item in ctx.strictE2eeReadinessIssues.value" :key="`${item.user_id}-${item.issue}`" class="row compact">
            <small>{{ item.display_name }}：{{ item.issue }}</small>
            <button class="secondary" @click="ctx.openStrictE2eeReadinessIssue(item)">处理</button>
          </div>
        </div>
        <p class="muted">分设备 sealed slot：{{ ctx.sealedSlotCoverageSummary.value.text }}</p>
        <div v-if="ctx.sealedSlotRiskContacts.value.length" class="callout warning">
          <b>存在 sealed slot 降级风险</b>
          <small v-for="item in ctx.sealedSlotRiskContacts.value" :key="item.user_id">{{ item.display_name }}：{{ item.status }}</small>
        </div>
        <div class="policy-grid">
          <label>
            <span>过滤级别</span>
            <select v-model="ctx.safetyPolicy.value.textFilterLevel">
              <option value="Off">关闭</option>
              <option value="Relaxed">宽松</option>
              <option value="Standard">标准</option>
              <option value="Strict">严格</option>
            </select>
          </label>
          <label class="identity-select">
            <input v-model="ctx.safetyPolicy.value.warnExternalLinks" type="checkbox" />
            <span>提示外部链接</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.safetyPolicy.value.warnExecutableFiles" type="checkbox" />
            <span>提示可执行文件</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.safetyPolicy.value.dropFilteredIncoming" type="checkbox" />
            <span>丢弃高风险入站消息</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.safetyPolicy.value.requireVerifiedContactsForSend" type="checkbox" />
            <span>仅向已核验指纹的联系人发送</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.safetyPolicy.value.requireVerifiedContactsForReceive" type="checkbox" />
            <span>仅接收已核验指纹的联系人消息</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.safetyPolicy.value.requireSealedPerDeviceSlotsForSend" type="checkbox" />
            <span>仅发送到支持 sealed slot 的设备</span>
          </label>
          <label class="identity-select">
            <input v-model="ctx.safetyPolicy.value.requireSealedPerDeviceSlotsForReceive" type="checkbox" />
            <span>仅接收 sealed slot 入站消息</span>
          </label>
          <div v-if="ctx.unverifiedIncomingDropCount.value" class="row compact">
            <small class="danger-text">已丢弃未核验联系人入站消息 {{ ctx.unverifiedIncomingDropCount.value }} 条<span v-if="ctx.lastUnverifiedIncomingDropAt.value">，最近：{{ ctx.lastUnverifiedIncomingDropFrom.value }} · {{ ctx.formatDateTime(ctx.lastUnverifiedIncomingDropAt.value) }}</span></small>
            <button class="secondary" @click="ctx.clearUnverifiedIncomingDropStats">清空丢弃统计</button>
          </div>
          <div v-if="ctx.revokedDeviceIncomingDropCount.value" class="row compact">
            <small class="danger-text">已丢弃撤销设备联系人入站消息 {{ ctx.revokedDeviceIncomingDropCount.value }} 条<span v-if="ctx.lastRevokedDeviceIncomingDropAt.value">，最近：{{ ctx.lastRevokedDeviceIncomingDropFrom.value }} · {{ ctx.formatDateTime(ctx.lastRevokedDeviceIncomingDropAt.value) }}</span></small>
            <button class="secondary" @click="ctx.clearRevokedDeviceIncomingDropStats">清空撤销丢弃统计</button>
          </div>
        </div>
      </section>

      <section class="home-card">
        <h3>账号与高级</h3>
        <div class="settings-rows">
          <button class="settings-row" aria-label="打开诊断工具" @click="ctx.goDiagnosticsPage">
            <span>诊断工具</span><span class="chevron">›</span>
          </button>
          <button class="settings-row" aria-label="清理浏览器缓存" @click="ctx.clearBrowserCaches">
            <span>清理浏览器缓存</span><span class="chevron">›</span>
          </button>
          <button class="settings-row danger-row" aria-label="退出登录" @click="ctx.logout">
            <span>退出登录</span><span class="chevron">›</span>
          </button>
        </div>
      </section>
    </div>
  </div>
</template>
