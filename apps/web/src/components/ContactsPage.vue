<script setup lang="ts">
import { computed, ref } from 'vue'
import { avatarColor } from '../avatarColor'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')
type View = 'home' | 'friends' | 'search' | 'add' | 'detail'
const view = ref<View>('home')

const requestCount = computed(() => props.ctx.visibleFriendRequests.value.length)
const contactQuery = computed(() => keyword.value.trim().toLowerCase())
const friendContacts = computed(() => props.ctx.contacts.value.filter((c: any) => c.state === 'Friend'))
const visibleContacts = computed(() => {
  const q = contactQuery.value
  if (!q) return friendContacts.value
  return friendContacts.value.filter((c: any) => `${c.display_name || ''} ${c.user_id || ''}`.toLowerCase().includes(q))
})
const groupedContacts = computed(() => {
  const groups = new Map<string, any[]>()
  for (const contact of visibleContacts.value) {
    const key = contactInitial(contact)
    const list = groups.get(key) || []
    list.push(contact)
    groups.set(key, list)
  }
  const sortedKeys = [...groups.keys()].sort((a, b) => {
    if (a === '#') return 1
    if (b === '#') return -1
    return a.localeCompare(b)
  })
  return sortedKeys.map((key) => ({ key, items: groups.get(key) || [] }))
})

function contactInitial(contact: any) {
  const name = `${contact.display_name || contact.user_id || ''}`.trim()
  const first = name[0]?.toUpperCase()
  return first && /[A-Z]/.test(first) ? first : '#'
}
function trustText(contact: any) {
  if (contact.state !== 'Friend') return ''
  return contact.fingerprint_verified_at ? '✓ 已核验' : '⚠️ 未核验'
}
function trustClass(contact: any) {
  return contact.fingerprint_verified_at ? 'verified' : 'unverified'
}
function shortId(value?: string) {
  if (!value) return ''
  return value.length > 18 ? `${value.slice(0, 8)}…${value.slice(-6)}` : value
}
function openContact(userId: string) {
  props.ctx.selectContact(userId)
  view.value = 'detail'
}
function backHome() {
  view.value = 'home'
}
function addContact() {
  props.ctx.addContact()
}
</script>

<template>
  <div class="contacts-shell product-contacts-shell">
    <main class="detail-col contacts-wide product-contacts-main">
      <section v-if="view === 'home'" class="detail-scroll">
        <header class="contacts-mobile-bar">
          <span></span>
          <h2>通讯录</h2>
          <div class="header-actions icon-actions">
            <button class="icon-btn" aria-label="搜索联系人" title="搜索联系人" @click="view = 'search'">🔍</button>
            <button class="icon-btn" aria-label="添加好友" title="添加好友" @click="view = 'add'">＋</button>
          </div>
        </header>

        <div class="contact-directory">
          <button class="directory-row primary-row" aria-label="打开新的朋友" @click="view = 'friends'">
            <span class="directory-icon">新</span>
            <span class="directory-main"><b>新的朋友</b></span>
            <em v-if="requestCount" class="request-badge">{{ requestCount }}</em>
            <span class="chevron">›</span>
          </button>

          <template v-for="group in groupedContacts" :key="group.key">
            <h3 class="alpha-heading">{{ group.key }}</h3>
            <button
              v-for="c in group.items"
              :key="c.user_id"
              class="directory-row contact-row"
              :class="{ active: c.user_id === ctx.activePeerId.value }"
              @click="openContact(c.user_id)"
            >
              <span class="avatar" :style="{ background: avatarColor(c.user_id) }">{{ (c.display_name || c.user_id || '?').slice(0, 1).toUpperCase() }}</span>
              <span class="directory-main">
                <b>{{ c.display_name || '未命名' }}</b>
                <small>{{ shortId(c.user_id) }}</small>
              </span>
              <span class="trust-mark" :class="trustClass(c)">{{ trustText(c) }}</span>
              <span class="chevron">›</span>
            </button>
          </template>

          <div v-if="groupedContacts.length === 0" class="empty product-empty">
            暂无好友，点击右上角 ＋ 添加好友。
          </div>
        </div>
      </section>

      <section v-else-if="view === 'friends'" class="detail-scroll">
        <header class="detail-bar product-subbar">
          <button class="back-btn" aria-label="返回通讯录" @click="backHome">‹</button>
          <h2>新的朋友</h2>
          <button class="secondary" @click="ctx.syncNow">同步</button>
        </header>
        <div class="detail-body narrow">
          <section class="home-card">
            <h3>好友申请</h3>
            <div v-if="ctx.visibleFriendRequests.value.length" class="request-grid">
              <div v-for="req in ctx.visibleFriendRequests.value" :key="req.request_id" class="request-item">
                <b>{{ req.from_user_id }}</b>
                <small>{{ req.note || '申请添加你为好友' }}</small>
                <div class="row compact">
                  <button @click="ctx.acceptInboxRequest(req)">同意</button>
                  <button class="secondary danger" @click="ctx.rejectInboxRequest(req)">拒绝</button>
                </div>
              </div>
            </div>
            <div v-else class="empty">暂无好友申请</div>
          </section>

          <section v-if="ctx.quarantinedFriendRequests.value.length" class="home-card">
            <div class="section-title-row">
              <h3>已隔离请求</h3>
              <button class="secondary danger" @click="ctx.clearQuarantinedFriendRequests">清空</button>
            </div>
            <div class="request-grid">
              <div v-for="req in ctx.quarantinedFriendRequests.value" :key="req.request_id" class="request-item">
                <b>{{ req.from_user_id }}</b>
                <small>{{ req.quarantine_reason || '本地规则隔离' }}</small>
                <div class="row compact">
                  <button class="secondary" @click="ctx.restoreQuarantinedFriendRequest(req)">恢复</button>
                  <button class="secondary danger" @click="ctx.rejectInboxRequest(req)">拒绝</button>
                </div>
              </div>
            </div>
          </section>
        </div>
      </section>

      <section v-else-if="view === 'search'" class="detail-scroll">
        <header class="detail-bar product-subbar">
          <button class="back-btn" aria-label="返回通讯录" @click="backHome">‹</button>
          <input v-model="keyword" class="subbar-search" type="search" aria-label="搜索联系人" placeholder="搜索联系人" autofocus />
        </header>
        <div class="contact-directory search-directory">
          <h3 class="alpha-heading">联系人</h3>
          <button v-for="c in visibleContacts" :key="c.user_id" class="directory-row contact-row" @click="openContact(c.user_id)">
            <span class="avatar" :style="{ background: avatarColor(c.user_id) }">{{ (c.display_name || c.user_id || '?').slice(0, 1).toUpperCase() }}</span>
            <span class="directory-main"><b>{{ c.display_name || '未命名' }}</b><small>{{ shortId(c.user_id) }}</small></span>
            <span class="trust-mark" :class="trustClass(c)">{{ trustText(c) }}</span>
            <span class="chevron">›</span>
          </button>
          <div v-if="visibleContacts.length === 0" class="empty">没有匹配的联系人</div>
        </div>
      </section>

      <section v-else-if="view === 'add'" class="detail-scroll">
        <header class="detail-bar product-subbar">
          <button class="back-btn" aria-label="返回通讯录" @click="backHome">‹</button>
          <h2>添加</h2>
          <span></span>
        </header>
        <div class="detail-body narrow add-page-body">
          <section class="home-card">
            <h3>添加好友（粘贴名片）</h3>
            <label for="contact-card-input">对方名片</label>
            <textarea id="contact-card-input" v-model="ctx.addContactText.value" rows="7" aria-label="对方名片文本" placeholder="粘贴对方发来的名片文本" />
            <div class="row"><button @click="addContact">添加好友</button></div>
          </section>
          <button class="settings-row mobile-only-row" aria-label="扫码添加" @click="ctx.showAlert('扫码添加', '扫码添加后续接入；也可以先使用“添加好友（粘贴名片）”。', 'info')">
            <span>扫码添加</span><span class="chevron">›</span>
          </button>
        </div>
      </section>

      <section v-else-if="view === 'detail' && ctx.activeContact.value" class="detail-scroll">
        <header class="detail-bar product-subbar">
          <button class="back-btn" aria-label="返回通讯录" @click="backHome">‹</button>
          <h2>联系人详情</h2>
          <span></span>
        </header>
        <div class="detail-hero product-contact-hero">
          <span class="avatar large" :style="{ background: avatarColor(ctx.activeContact.value.user_id) }">{{ (ctx.activeContact.value.display_name || ctx.activeContact.value.user_id || '?').slice(0, 1).toUpperCase() }}</span>
          <div class="detail-hero-text">
            <h2>{{ ctx.activeContact.value.display_name || '未命名' }}</h2>
            <span v-if="ctx.activeContact.value.state === 'Friend'" class="trust-chip" :class="trustClass(ctx.activeContact.value)">{{ trustText(ctx.activeContact.value) }}</span>
            <small>{{ shortId(ctx.activeContact.value.user_id) }}</small>
          </div>
        </div>
        <div class="detail-body narrow">
          <button class="primary-action" @click="ctx.goChatPage()">发消息</button>
          <section v-if="ctx.activeContact.value.state === 'Friend'" class="home-card">
            <div class="section-title-row">
              <h3>安全与设备</h3>
              <span class="trust-chip" :class="trustClass(ctx.activeContact.value)">{{ trustText(ctx.activeContact.value) }}</span>
            </div>
            <small v-if="ctx.activeContact.value.fingerprint_verified_at">指纹已核验：{{ ctx.formatDateTime(ctx.activeContact.value.fingerprint_verified_at) }}</small>
            <small v-else class="danger-text">指纹未核验。请通过可信渠道核对，确认对方就是本人。</small>
            <small v-if="ctx.contactRevokedDeviceCount(ctx.activeContact.value)" class="danger-text">已撤销设备：{{ ctx.contactRevokedDeviceCount(ctx.activeContact.value) }}</small>
            <div class="row compact">
              <button v-if="!ctx.activeContact.value.fingerprint_verified_at" class="secondary" @click="ctx.verifyActiveContactFingerprint">标记已核验</button>
              <button class="secondary" @click="ctx.showActiveContactFingerprintQr">指纹核验码</button>
              <button class="secondary" @click="ctx.copyActiveContactFingerprintProof">复制核验码</button>
            </div>
          </section>
          <section class="home-card">
            <div class="settings-rows">
              <button class="settings-row" @click="ctx.showQr(ctx.activeContact.value.contact_card_text, '好友身份')"><span>查看名片</span><span class="chevron">›</span></button>
              <button v-if="ctx.activeContact.value.state !== 'Blocked'" class="settings-row" @click="ctx.blockActiveContact"><span>拉黑</span><span class="chevron">›</span></button>
              <button v-else class="settings-row" @click="ctx.unblockActiveContact"><span>解除拉黑</span><span class="chevron">›</span></button>
              <button class="settings-row danger-row" @click="ctx.removeActiveContact"><span>删除好友</span><span class="chevron">›</span></button>
            </div>
          </section>
        </div>
      </section>
    </main>
  </div>
</template>
