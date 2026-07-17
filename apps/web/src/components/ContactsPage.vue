<script setup lang="ts">
import { computed, ref } from 'vue'
import { avatarColor } from '../avatarColor'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')
const trustFilter = ref<'all' | 'unverified' | 'verified' | 'revoked' | 'strict-blocked'>('all')
type View = 'welcome' | 'requests' | 'detail' | 'add' | 'group'
const view = ref<View>('welcome')

const query = computed(() => keyword.value.trim().toLowerCase())
const filteredContacts = computed(() => {
  const q = query.value
  let list = props.ctx.contacts.value
  if (trustFilter.value === 'verified') list = list.filter((c: any) => c.state === 'Friend' && c.fingerprint_verified_at)
  else if (trustFilter.value === 'unverified') list = list.filter((c: any) => c.state === 'Friend' && !c.fingerprint_verified_at)
  else if (trustFilter.value === 'revoked') list = list.filter((c: any) => (c.revoked_device_ids || []).length > 0)
  else if (trustFilter.value === 'strict-blocked') list = list.filter((c: any) => c.state === 'Friend' && props.ctx.contactStrictE2eeRiskLevel(c) === 'high')
  if (!q) return list
  return list.filter((c: any) => `${c.display_name || ''} ${c.user_id || ''} ${c.state || ''}`.toLowerCase().includes(q))
})
const filteredGroups = computed(() => {
  const q = query.value
  const list = props.ctx.groups.value
  if (!q) return list
  return list.filter((g: any) => `${g.name || ''} ${g.group_id || ''}`.toLowerCase().includes(q))
})
const requestCount = computed(() => props.ctx.friendRequests.value.length + props.ctx.groupInvites.value.length)

function stateLabel(state: string) {
  return state === 'Friend' ? '好友' : state === 'RequestSent' ? '等待通过' : state === 'Blocked' ? '已拉黑' : '未验证'
}
function openContact(userId: string) {
  props.ctx.selectContact(userId)
  view.value = 'detail'
}
function openGroupDetail(groupId: string) {
  props.ctx.selectGroup(groupId)
  view.value = 'detail'
}
</script>

<template>
  <div class="contacts-shell">
    <aside class="sidebar contacts-list-col">
      <header class="list-col-header">
        <h2>通讯录</h2>
        <div class="header-actions">
          <button class="ghost-btn" @click="view = 'add'">添加</button>
          <button class="ghost-btn" @click="view = 'group'">建群</button>
        </div>
      </header>
      <div class="list-col-search">
        <input v-model="keyword" type="search" aria-label="搜索好友或群聊" placeholder="搜索好友或群聊" />
        <select v-model="trustFilter" aria-label="联系人信任筛选">
          <option value="all">全部联系人</option>
          <option value="unverified">未核验好友</option>
          <option value="verified">已核验好友</option>
          <option value="revoked">有撤销设备</option>
          <option value="strict-blocked">严格 E2EE 阻塞</option>
        </select>
        <button v-if="trustFilter === 'strict-blocked'" class="secondary" @click="ctx.repairStrictE2eeBlockers">批量处理阻塞</button>
      </div>

      <div class="conversation-list">
        <button class="contact" :class="{ active: view === 'requests' }" :aria-current="view === 'requests' ? 'true' : undefined" @click="view = 'requests'">
          <span class="avatar" style="background:#f59e0b">新</span>
          <span class="contact-main">
            <b>收件箱 <em v-if="requestCount">{{ requestCount }}</em></b>
            <small>好友请求 / 群邀请 / Mailbox</small>
          </span>
        </button>

        <h3 v-if="filteredGroups.length">群聊</h3>
        <button
          v-for="g in filteredGroups"
          :key="g.group_id"
          class="contact"
          :class="{ active: view === 'detail' && g.group_id === ctx.activeGroupId.value }"
          :aria-current="view === 'detail' && g.group_id === ctx.activeGroupId.value ? 'true' : undefined"
          @click="openGroupDetail(g.group_id)"
        >
          <span class="avatar group-avatar">群</span>
          <span class="contact-main">
            <b>{{ g.name }}</b>
            <small>{{ g.member_user_ids.length }} 人</small>
          </span>
        </button>

        <h3 v-if="filteredContacts.length">好友</h3>
        <button
          v-for="c in filteredContacts"
          :key="c.user_id"
          class="contact"
          :class="{ active: view === 'detail' && c.user_id === ctx.activePeerId.value }"
          :aria-current="view === 'detail' && c.user_id === ctx.activePeerId.value ? 'true' : undefined"
          @click="openContact(c.user_id)"
        >
          <span class="avatar" :style="{ background: avatarColor(c.user_id) }">{{ (c.display_name || c.user_id || '?').slice(0, 1).toUpperCase() }}</span>
          <span class="contact-main">
            <b>{{ c.display_name || '未命名' }}</b>
            <small>{{ stateLabel(c.state) }} · {{ c.user_id }}</small>
            <small v-if="c.state === 'Friend' && c.fingerprint_verified_at">指纹已核验</small>
            <small v-else-if="c.state === 'Friend'" class="danger-text">指纹未核验</small>
            <small v-if="c.state === 'Friend' && ctx.contactStrictE2eeStatusText(c)" :class="{ 'danger-text': ctx.contactStrictE2eeRiskLevel(c) === 'high' }">{{ ctx.contactStrictE2eeStatusText(c) }}</small>
          </span>
        </button>

        <div v-if="filteredGroups.length === 0 && filteredContacts.length === 0" class="empty">暂无好友或群聊</div>
      </div>
    </aside>

    <main class="detail-col">
      <!-- 新的朋友 -->
      <section v-if="view === 'requests'" class="detail-scroll">
        <header class="detail-bar"><h2>收件箱</h2><button class="secondary" @click="ctx.syncNow">同步</button></header>
        <div class="detail-body">
          <section class="home-card inbox-status-card">
            <h3>Mailbox 状态</h3>
            <div class="inbox-status-grid">
              <span>好友请求 {{ ctx.visibleFriendRequests.value.length }}</span>
              <span>垃圾请求 {{ ctx.quarantinedFriendRequests.value.length }}</span>
              <span>群邀请 {{ ctx.groupInvites.value.length }}</span>
              <span>24小时计数 {{ ctx.friendRequestRateRecords.value.length }}</span>
            </div>
            <small v-if="ctx.friendRequestRateSummaryText.value" class="danger-text">请求频率：{{ ctx.friendRequestRateSummaryText.value }}</small>
            <small>{{ ctx.mailboxInboxStatus.value }}</small>
            <small :class="{ 'danger-text': ctx.mailboxQuotaPressureLevel.value !== 'ok' }">{{ ctx.mailboxQuotaStatusText.value }}</small>
            <small>{{ ctx.mailboxDedupeStatusText.value }}</small>
            <small v-if="ctx.mailboxFailureSummaryText.value" class="danger-text">{{ ctx.mailboxFailureSummaryText.value }}</small>
            <small v-if="ctx.mailboxInboxErrorText.value" class="danger-text">{{ ctx.mailboxInboxErrorText.value }}</small>
            <div class="row compact">
              <button class="secondary" :disabled="ctx.mailboxDedupeCount.value === 0" @click="ctx.clearProcessedMailboxIds">清空去重记录</button>
              <button class="secondary" :disabled="ctx.mailboxFailedCount.value === 0" @click="ctx.retryFailedMailboxItems">重试失败</button>
              <button class="secondary danger" :disabled="ctx.mailboxFailedCount.value === 0" @click="ctx.clearFailedMailboxItems">清空失败</button>
              <button class="secondary danger" :disabled="ctx.friendRequestRateRecords.value.length === 0" @click="ctx.clearFriendRequestRateRecords">清空请求计数</button>
            </div>
          </section>

          <section class="home-card">
            <div class="section-title-row">
              <h3>好友请求</h3>
              <div v-if="ctx.visibleFriendRequests.value.length" class="row compact">
                <button class="secondary" @click="ctx.rejectAllInboxRequests">全部忽略</button>
                <button class="secondary danger" @click="ctx.blockAllInboxRequests">全部拉黑</button>
              </div>
            </div>
            <div v-if="ctx.visibleFriendRequests.value.length" class="request-grid">
              <div v-for="req in ctx.visibleFriendRequests.value" :key="req.request_id" class="request-item">
                <b>{{ req.from_user_id }}</b>
                <small>{{ req.note || '无备注' }}</small>
                <div class="row compact">
                  <button @click="ctx.acceptInboxRequest(req)">接受</button>
                  <button class="secondary danger" @click="ctx.rejectInboxRequest(req)">忽略</button>
                </div>
              </div>
            </div>
            <div v-else class="empty">暂无好友请求</div>
          </section>

          <section class="home-card">
            <div class="section-title-row">
              <h3>垃圾请求</h3>
              <div v-if="ctx.quarantinedFriendRequests.value.length" class="row compact">
                <button class="secondary" @click="ctx.restoreAllQuarantinedFriendRequests">全部恢复</button>
                <button class="secondary danger" @click="ctx.clearQuarantinedFriendRequests">清空</button>
              </div>
            </div>
            <div v-if="ctx.quarantinedFriendRequests.value.length" class="request-grid">
              <div v-for="req in ctx.quarantinedFriendRequests.value" :key="req.request_id" class="request-item">
                <b>{{ req.from_user_id }}</b>
                <small>{{ req.quarantine_reason || '本地规则隔离' }}</small>
                <small>{{ req.note || '无备注' }}</small>
                <div class="row compact">
                  <button class="secondary" @click="ctx.restoreQuarantinedFriendRequest(req)">恢复</button>
                  <button class="secondary danger" @click="ctx.rejectInboxRequest(req)">忽略</button>
                </div>
              </div>
            </div>
            <div v-else class="empty">暂无垃圾请求</div>
          </section>

          <section class="home-card">
            <h3>群邀请</h3>
            <div v-if="ctx.groupInvites.value.length" class="request-grid">
              <div v-for="inv in ctx.groupInvites.value" :key="inv.invite_id" class="request-item">
                <b>{{ inv.group_name }}</b>
                <small>{{ inv.member_user_ids.length }} 人</small>
                <small>接受后只显示本机收到的新消息，历史消息不会自动同步。</small>
                <small v-if="ctx.groupInviteStrictE2eeRiskText(inv)" class="danger-text">{{ ctx.groupInviteStrictE2eeRiskText(inv) }}</small>
                <div class="row compact">
                  <button @click="ctx.acceptGroupInvite(inv)">接受入群</button>
                  <button class="secondary danger" @click="ctx.ignoreGroupInvite(inv)">忽略</button>
                </div>
              </div>
            </div>
            <div v-else class="empty">暂无群邀请</div>
          </section>
        </div>
      </section>

      <!-- 联系人详情 -->
      <section v-else-if="view === 'detail' && ctx.activeContact.value" class="detail-scroll">
        <div class="detail-hero">
          <span class="avatar large" :style="{ background: avatarColor(ctx.activeContact.value.user_id) }">{{ (ctx.activeContact.value.display_name || ctx.activeContact.value.user_id || '?').slice(0, 1).toUpperCase() }}</span>
          <div class="detail-hero-text">
            <h2>{{ ctx.activeContact.value.display_name || '未命名' }}</h2>
            <small>{{ stateLabel(ctx.activeContact.value.state) }} · {{ ctx.activeContact.value.user_id }}</small>
            <small v-if="ctx.activeContact.value.state === 'Friend'">端到端会话：{{ ctx.activeRatchetStatusText.value }}</small>
            <small>指纹：{{ ctx.activeContact.value.fingerprint }}</small>
            <small v-if="ctx.activeContact.value.fingerprint_verified_at">指纹已核验：{{ ctx.formatDateTime(ctx.activeContact.value.fingerprint_verified_at) }}</small>
            <small v-if="ctx.activeContact.value.device_certs?.length">活跃设备：{{ ctx.contactActiveDeviceIds(ctx.activeContact.value).length }}/{{ ctx.activeContact.value.device_certs.length }}</small>
            <small v-if="ctx.contactRevokedDeviceCount(ctx.activeContact.value)" class="danger-text">已撤销设备：{{ ctx.contactRevokedDeviceCount(ctx.activeContact.value) }}（已知 {{ ctx.contactKnownRevokedDeviceCount(ctx.activeContact.value) }}/{{ ctx.activeContact.value.device_certs?.length || 0 }}）</small>
            <small v-if="ctx.contactAllKnownDevicesRevoked(ctx.activeContact.value)" class="danger-text">所有已知设备均已撤销，已阻止发送/建链</small>
            <small v-if="ctx.activeContact.value.mailbox_hint_url">MailboxHint：{{ ctx.activeContact.value.mailbox_hint_url }}</small>
            <small v-if="ctx.activeContact.value.last_dht_discovery_attempt_at">最近 DHT 发现尝试：{{ ctx.formatDateTime(ctx.activeContact.value.last_dht_discovery_attempt_at) }}</small>
            <small v-if="ctx.activeContact.value.last_prekey_dht_found_at">PreKey DHT 发现：{{ ctx.formatDateTime(ctx.activeContact.value.last_prekey_dht_found_at) }}</small>
            <small v-if="ctx.activeContact.value.last_mailbox_hint_dht_found_at">MailboxHint DHT 发现：{{ ctx.formatDateTime(ctx.activeContact.value.last_mailbox_hint_dht_found_at) }}</small>
        <small v-if="ctx.activeContact.value.last_contact_card_dht_found_at">ContactCard DHT 发现：{{ ctx.formatDateTime(ctx.activeContact.value.last_contact_card_dht_found_at) }}</small>
            <small v-if="ctx.activeContact.value.next_dht_discovery_retry_at">DHT 下次自动重试：{{ ctx.formatDateTime(ctx.activeContact.value.next_dht_discovery_retry_at) }}</small>
            <small v-if="ctx.activeContact.value.dht_discovery_failure_count">DHT 连续失败：{{ ctx.activeContact.value.dht_discovery_failure_count }} 次</small>
            <small v-if="ctx.activeContact.value.dht_discovery_risk_level === 'high'" class="danger-text">DHT 安全风险：记录验签或格式异常，请谨慎信任该联系人发现结果</small>
            <small v-if="ctx.activeContact.value.last_dht_discovery_error" class="danger-text">DHT 发现失败<span v-if="ctx.activeContact.value.last_dht_discovery_error_kind">({{ ctx.activeContact.value.last_dht_discovery_error_kind }})</span>：{{ ctx.activeContact.value.last_dht_discovery_error }}</small>
            <small v-if="ctx.activeContact.value.last_secure_session_attempt_at">最近建链尝试：{{ ctx.formatDateTime(ctx.activeContact.value.last_secure_session_attempt_at) }}</small>
            <small v-if="ctx.activeContact.value.last_secure_session_success_at">最近建链成功：{{ ctx.formatDateTime(ctx.activeContact.value.last_secure_session_success_at) }}</small>
            <small v-if="ctx.activeContact.value.secure_session_failure_count">连续建链失败：{{ ctx.activeContact.value.secure_session_failure_count }} 次</small>
            <small v-if="ctx.activeSecureSessionOutboxCount.value">安全建链待重试：{{ ctx.activeSecureSessionOutboxCount.value }} 条</small>
            <small v-if="ctx.activeContact.value.last_secure_session_error" class="danger-text">安全建链失败：{{ ctx.activeContact.value.last_secure_session_error }}</small>
            <small v-if="ctx.activeContact.value.last_friend_request_error" class="danger-text">好友请求失败：{{ ctx.activeContact.value.last_friend_request_error }}</small>
            <small v-if="ctx.activeContact.value.state === 'Friend'" :class="{ 'danger-text': ctx.contactStrictE2eeRiskLevel(ctx.activeContact.value) === 'high' }">{{ ctx.contactStrictE2eeStatusText(ctx.activeContact.value) }}</small>
            <small v-if="ctx.contactCardUpdateAckStatusFor(ctx.activeContact.value).pending" class="danger-text">设备证书更新 ACK：待确认 {{ ctx.contactCardUpdateAckStatusFor(ctx.activeContact.value).pending }}，过期 {{ ctx.contactCardUpdateAckStatusFor(ctx.activeContact.value).stale }}</small>
          </div>
        </div>
        <div class="detail-body">
          <section v-if="ctx.activeContact.value.device_certs?.length" class="home-card">
            <h3>已知设备</h3>
            <div class="outbox-list">
              <div v-for="cert in ctx.activeContact.value.device_certs" :key="cert.device_id" class="outbox-row">
                <b>{{ cert.device_name || cert.device_id }}</b>
                <small>{{ cert.device_id }}</small>
                <small :class="{ 'danger-text': (ctx.activeContact.value.revoked_device_ids || []).includes(cert.device_id) }">{{ (ctx.activeContact.value.revoked_device_ids || []).includes(cert.device_id) ? '已撤销' : '活跃' }}</small>
              </div>
            </div>
          </section>
          <section v-if="ctx.contactRevokedDeviceIds(ctx.activeContact.value).length" class="home-card">
            <h3>已撤销设备</h3>
            <div class="outbox-list">
              <div v-for="item in ctx.contactRevokedDeviceDetails(ctx.activeContact.value)" :key="item.device_id" class="outbox-row">
                <b>{{ item.device_id }}</b>
                <small>该设备已标记撤销，不应再用于信任或建链。</small>
                <small v-if="!(ctx.activeContact.value.device_certs || []).some((cert: any) => cert.device_id === item.device_id)" class="danger-text">该 device_id 不在当前联系人名片的已知设备中。</small>
                <small v-if="item.reason">原因：{{ item.reason }}</small>
                <small v-if="item.created_at">撤销时间：{{ ctx.formatDateTime(item.created_at * 1000) }}</small>
                <button class="secondary danger" @click="ctx.unmarkActiveContactRevokedDevice(item.device_id)">解除撤销标记</button>
              </div>
            </div>
          </section>
          <section v-if="ctx.activeContact.value.state === 'Friend'" class="home-card">
            <h3>指纹核验</h3>
            <textarea v-model="ctx.activeFingerprintVerificationText.value" rows="3" placeholder="粘贴对方通过可信渠道展示的 lm-contact-fingerprint-v1 核验码，或直接粘贴指纹文本"></textarea>
            <div class="row detail-actions">
              <button class="secondary" @click="ctx.verifyActiveContactFingerprintFromText">核验并标记可信</button>
              <button class="secondary" @click="ctx.startFingerprintQrScan">扫码核验</button>
              <button class="secondary" @click="ctx.showActiveContactFingerprintQr">显示当前联系人核验码</button>
              <button class="secondary" @click="ctx.copyActiveContactFingerprintProof">复制当前联系人核验码</button>
            </div>
          </section>
          <section v-if="ctx.activeContact.value.state === 'Friend'" class="home-card">
            <div class="section-title-row">
              <h3>严格 E2EE 修复向导</h3>
              <span :class="{ 'danger-text': ctx.contactStrictE2eeRiskLevel(ctx.activeContact.value) === 'high' }">{{ ctx.contactStrictE2eeStatusText(ctx.activeContact.value) }}</span>
            </div>
            <div class="outbox-list">
              <div class="outbox-row">
                <b>1. 身份指纹核验</b>
                <small v-if="ctx.activeContact.value.fingerprint_verified_at">已核验 · {{ ctx.formatDateTime(ctx.activeContact.value.fingerprint_verified_at) }}</small>
                <small v-else class="danger-text">未核验。请通过线下/可信渠道核对核验码，避免中间人冒充。</small>
                <div class="row compact">
                  <button class="secondary" @click="ctx.showActiveContactFingerprintQr">显示核验码</button>
                  <button class="secondary" @click="ctx.copyActiveContactFingerprintProof">复制核验码</button>
                  <button v-if="!ctx.activeContact.value.fingerprint_verified_at" class="secondary" @click="ctx.verifyActiveContactFingerprint">已人工核验，标记可信</button>
                </div>
              </div>
              <div class="outbox-row">
                <b>2. ContactCard DHT 新鲜度</b>
                <small v-if="ctx.activeContact.value.last_contact_card_dht_found_at">最近发现：{{ ctx.formatDateTime(ctx.activeContact.value.last_contact_card_dht_found_at) }}</small>
                <small v-else class="danger-text">尚未发现 ContactCard DHT，可能缺少最新设备证书或撤销状态。</small>
                <small v-if="ctx.contactCardDhtDiscoveryIsStale(ctx.activeContact.value)" class="danger-text">发现结果已过期或缺失，建议刷新。</small>
                <button class="secondary" @click="ctx.findActiveContactContactCard">刷新 ContactCard</button>
              </div>
              <div class="outbox-row">
                <b>3. 分设备 sealed slot 覆盖</b>
                <small :class="{ 'danger-text': ctx.activeContactSealedSlotRiskLevel.value === 'high' }">{{ ctx.activeContactSealedSlotStatusText.value }}</small>
                <div class="row compact">
                  <button class="secondary" @click="ctx.discoverActiveContactDht">发现全部 DHT</button>
                  <button class="secondary" @click="ctx.findActiveContactPreKey">查找 PreKey</button>
                </div>
              </div>
              <div class="outbox-row">
                <b>4. 设备证书更新 ACK</b>
                <small>待确认 {{ ctx.contactCardUpdateAckStatusFor(ctx.activeContact.value).pending }}；过期 {{ ctx.contactCardUpdateAckStatusFor(ctx.activeContact.value).stale }}；已确认 {{ ctx.contactCardUpdateAckStatusFor(ctx.activeContact.value).acked }}</small>
                <button class="secondary" :disabled="ctx.contactCardUpdateAckStatusFor(ctx.activeContact.value).pending === 0" @click="ctx.retryStaleContactCardUpdateAcks">重试过期 ACK</button>
              </div>
            </div>
          </section>
          <div class="row detail-actions">
            <button @click="ctx.goChatPage()">发消息</button>
            <button v-if="ctx.activeContact.value.state === 'Friend'" class="secondary" @click="ctx.discoverActiveContactDht">发现 DHT</button>
            <button v-if="ctx.activeContact.value.state === 'Friend'" class="secondary" @click="ctx.findActiveContactPreKey">查找 PreKey</button>
            <button v-if="ctx.activeContact.value.state === 'Friend' && !ctx.activeContact.value.fingerprint_verified_at" class="secondary" @click="ctx.verifyActiveContactFingerprint">标记指纹已核验</button>
            <button v-if="ctx.activeContact.value.state === 'Friend'" class="secondary" @click="ctx.showActiveContactFingerprintQr">指纹核验码</button>
            <button v-if="ctx.activeContact.value.state === 'Friend'" class="secondary" @click="ctx.findActiveContactMailboxHint">查找 MailboxHint</button>
            <button v-if="ctx.activeContact.value.dht_discovery_risk_level === 'high'" class="secondary danger" @click="ctx.clearActiveContactDhtRisk">已核验，清除 DHT 风险</button>
            <button v-if="ctx.activeContact.value.state === 'Friend' && ctx.activeContact.value.last_secure_session_error" class="secondary" @click="ctx.retrySecureSessionForActiveContact">重试建链</button>
            <button v-if="ctx.activeContact.value.state === 'Friend' && ctx.activeContact.value.last_secure_session_error" class="secondary" @click="ctx.clearActiveSecureSessionError">清除建链错误</button>
            <button v-if="ctx.activeContact.value.state === 'Friend' && !ctx.activeRatchetSession.value" class="secondary" @click="ctx.recreateActiveRatchetSession">本地建链</button>
            <button v-if="ctx.activeContact.value.last_friend_request_error" class="secondary" @click="ctx.clearActiveFriendRequestError">清除请求错误</button>
            <button class="secondary" @click="ctx.showQr(ctx.activeContact.value.contact_card_text, '好友身份')">查看名片</button>
            <button v-if="ctx.activeContact.value.state !== 'Blocked'" class="secondary" @click="ctx.blockActiveContact">拉黑</button>
            <button v-else class="secondary" @click="ctx.unblockActiveContact">解除拉黑</button>
            <button class="secondary danger" @click="ctx.removeActiveContact">删除好友</button>
          </div>
        </div>
      </section>

      <!-- 群详情 -->
      <section v-else-if="view === 'detail' && ctx.activeGroup.value" class="detail-scroll">
        <div class="detail-hero">
          <span class="avatar large group-avatar">群</span>
          <div class="detail-hero-text">
            <h2>{{ ctx.activeGroup.value.name }}</h2>
            <small>{{ ctx.activeGroup.value.member_user_ids.length }} 人 · {{ ctx.activeGroup.value.group_id }}</small>
          </div>
        </div>
        <div class="detail-body">
          <section class="home-card">
            <h3>群成员</h3>
            <div class="group-state-grid">
              <span>事件序列 {{ ctx.activeGroup.value.sequence ?? 0 }}</span>
              <span>管理员 {{ ctx.activeGroup.value.admin_user_ids?.length ?? 0 }}</span>
              <span v-if="ctx.activeGroup.value.last_event_at">本地视图更新 {{ ctx.formatDateTime(ctx.activeGroup.value.last_event_at) }}</span>
            </div>
            <div v-if="ctx.activeGroup.value.removed_self_at" class="group-event-summary">
              <b class="danger-text">你已被移出群聊</b>
              <small>{{ ctx.activeGroup.value.removed_self_by || '未知发起者' }} · {{ ctx.formatDateTime(ctx.activeGroup.value.removed_self_at) }}</small>
            </div>
            <div v-if="ctx.activeGroup.value.last_sender_key_error" class="group-event-summary">
              <b class="danger-text">Sender Key 异常：{{ ctx.activeGroup.value.last_sender_key_error }}</b>
              <small>{{ ctx.formatDateTime(ctx.activeGroup.value.last_sender_key_error_at || Date.now()) }}</small>
            </div>
            <div class="member-list">
              <span v-for="m in ctx.activeGroupMembers.value" :key="m.user_id">{{ m.display_name || m.user_id }}</span>
            </div>
          </section>
          <section class="home-card">
            <h3>最近群事件</h3>
            <div v-if="ctx.activeGroup.value.last_event_error" class="group-event-summary">
              <b class="danger-text">群事件失败：{{ ctx.activeGroup.value.last_event_error }}</b>
              <small>{{ ctx.formatDateTime(ctx.activeGroup.value.last_event_error_at || Date.now()) }}</small>
              <small v-if="ctx.activeGroup.value.last_event_recovery_hint">{{ ctx.activeGroup.value.last_event_recovery_hint }}</small>
              <button class="secondary" @click="ctx.clearActiveGroupEventError">清除错误</button>
            </div>
            <div v-if="ctx.activeGroup.value.last_event_summary" class="group-event-summary">
              <b>{{ ctx.activeGroup.value.last_event_summary }}</b>
              <small>{{ ctx.activeGroup.value.last_event_actor_user_id || '未知发起者' }} · {{ ctx.formatDateTime(ctx.activeGroup.value.last_event_at || Date.now()) }}</small>
            </div>
            <div v-if="!ctx.activeGroup.value.last_event_summary && !ctx.activeGroup.value.last_event_error" class="empty">暂无群事件</div>
          </section>
          <div class="row detail-actions">
            <button @click="ctx.goChatPage()">进入群聊</button>
            <button class="secondary" @click="ctx.leaveActiveGroupWithNotice">通知退群</button>
            <button class="secondary danger" @click="ctx.removeActiveGroup">仅本机退出</button>
          </div>
          <small class="sync-note">通知退群会向其他成员发送退群事件；仅本机退出不会通知其他成员。</small>
        </div>
      </section>

      <!-- 添加好友 -->
      <section v-else-if="view === 'add'" class="detail-scroll">
        <header class="detail-bar"><h2>添加好友</h2></header>
        <div class="detail-body narrow">
          <section class="home-card">
            <label for="contact-card-input">对方名片</label>
            <textarea id="contact-card-input" v-model="ctx.addContactText.value" rows="6" aria-label="对方名片文本" placeholder="粘贴对方发来的名片文本" />
            <div class="row"><button @click="ctx.addContact">添加好友</button></div>
          </section>
        </div>
      </section>

      <!-- 发起群聊 -->
      <section v-else-if="view === 'group'" class="detail-scroll">
        <header class="detail-bar"><h2>发起群聊</h2></header>
        <div class="detail-body narrow">
          <section class="home-card">
            <label for="new-group-name">群名</label>
            <input id="new-group-name" v-model="ctx.newGroupName.value" aria-label="群名" placeholder="例如：项目讨论组" />
            <div v-if="ctx.createGroupStrictE2eeRiskText.value" class="callout warning">
              <b>新建群聊严格 E2EE 预检</b>
              <small>{{ ctx.createGroupStrictE2eeRiskText.value }}</small>
            </div>
            <label>选择好友</label>
            <div v-if="ctx.friendContacts.value.length" class="member-picker">
              <label v-for="c in ctx.friendContacts.value" :key="c.user_id" class="check-row">
                <input type="checkbox" :value="c.user_id" v-model="ctx.selectedGroupMembers.value" />
                {{ c.display_name || c.user_id }}
              </label>
            </div>
            <div v-else class="empty">暂无可邀请好友</div>
            <div class="row"><button @click="ctx.createGroup">创建群聊</button></div>
          </section>
        </div>
      </section>

      <!-- 默认欢迎 -->
      <section v-else class="detail-empty">
        <p>选择左侧联系人查看详情</p>
      </section>
    </main>
  </div>
</template>
