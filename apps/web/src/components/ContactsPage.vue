<script setup lang="ts">
import { computed, ref } from 'vue'
import { avatarColor } from '../avatarColor'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')
type View = 'welcome' | 'requests' | 'detail' | 'add' | 'group'
const view = ref<View>('welcome')

const query = computed(() => keyword.value.trim().toLowerCase())
const filteredContacts = computed(() => {
  const q = query.value
  const list = props.ctx.contacts.value
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
        <input v-model="keyword" placeholder="搜索好友或群聊" />
      </div>

      <div class="conversation-list">
        <button class="contact" :class="{ active: view === 'requests' }" @click="view = 'requests'">
          <span class="avatar" style="background:#f59e0b">新</span>
          <span class="contact-main">
            <b>新的朋友 <em v-if="requestCount">{{ requestCount }}</em></b>
            <small>好友请求 / 群邀请</small>
          </span>
        </button>

        <h3 v-if="filteredGroups.length">群聊</h3>
        <button
          v-for="g in filteredGroups"
          :key="g.group_id"
          class="contact"
          :class="{ active: view === 'detail' && g.group_id === ctx.activeGroupId.value }"
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
          @click="openContact(c.user_id)"
        >
          <span class="avatar" :style="{ background: avatarColor(c.user_id) }">{{ (c.display_name || c.user_id || '?').slice(0, 1).toUpperCase() }}</span>
          <span class="contact-main">
            <b>{{ c.display_name || '未命名' }}</b>
            <small>{{ stateLabel(c.state) }} · {{ c.user_id }}</small>
          </span>
        </button>

        <div v-if="filteredGroups.length === 0 && filteredContacts.length === 0" class="empty">暂无好友或群聊</div>
      </div>
    </aside>

    <main class="detail-col">
      <!-- 新的朋友 -->
      <section v-if="view === 'requests'" class="detail-scroll">
        <header class="detail-bar"><h2>新的朋友</h2><button class="secondary" @click="ctx.syncNow">刷新</button></header>
        <div class="detail-body">
          <section class="home-card">
            <h3>好友请求</h3>
            <div v-if="ctx.friendRequests.value.length" class="request-grid">
              <div v-for="req in ctx.friendRequests.value" :key="req.request_id" class="request-item">
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
            <h3>群邀请</h3>
            <div v-if="ctx.groupInvites.value.length" class="request-grid">
              <div v-for="inv in ctx.groupInvites.value" :key="inv.invite_id" class="request-item">
                <b>{{ inv.group_name }}</b>
                <small>{{ inv.member_user_ids.length }} 人</small>
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
          </div>
        </div>
        <div class="detail-body">
          <div class="row detail-actions">
            <button @click="ctx.goChatPage()">发消息</button>
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
            <div class="member-list">
              <span v-for="m in ctx.activeGroupMembers.value" :key="m.user_id">{{ m.display_name || m.user_id }}</span>
            </div>
          </section>
          <div class="row detail-actions">
            <button @click="ctx.goChatPage()">进入群聊</button>
            <button class="secondary danger" @click="ctx.removeActiveGroup">删除群聊</button>
          </div>
        </div>
      </section>

      <!-- 添加好友 -->
      <section v-else-if="view === 'add'" class="detail-scroll">
        <header class="detail-bar"><h2>添加好友</h2></header>
        <div class="detail-body narrow">
          <section class="home-card">
            <label>对方名片</label>
            <textarea v-model="ctx.addContactText.value" rows="6" placeholder="粘贴对方发来的名片文本" />
            <div class="row"><button @click="ctx.addContact">添加好友</button></div>
          </section>
        </div>
      </section>

      <!-- 发起群聊 -->
      <section v-else-if="view === 'group'" class="detail-scroll">
        <header class="detail-bar"><h2>发起群聊</h2></header>
        <div class="detail-body narrow">
          <section class="home-card">
            <label>群名</label>
            <input v-model="ctx.newGroupName.value" placeholder="例如：项目讨论组" />
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
