<script setup lang="ts">
import { computed, ref } from 'vue'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')

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
</script>

<template>
  <section class="simple-page contacts-page">
    <header class="simple-header contacts-header">
      <div>
        <h1>通讯录</h1>
      </div>
      <input v-model="keyword" class="contacts-search" placeholder="搜索好友或群聊" />
    </header>

    <div class="address-book-layout">
      <aside class="address-menu">
        <a href="#new-friends">新朋友</a>
        <a href="#groups">群聊</a>
        <a href="#friends">好友</a>
        <a href="#add-friend">添加好友</a>
        <a href="#create-group">发起群聊</a>
      </aside>

      <main class="address-content">
        <section v-if="ctx.activeContact.value" class="home-card contact-detail-card">
          <div class="detail-head">
            <span class="avatar large">{{ (ctx.activeContact.value.display_name || ctx.activeContact.value.user_id || '?').slice(0, 1).toUpperCase() }}</span>
            <div>
              <h3>{{ ctx.activeContact.value.display_name || '未命名' }}</h3>
              <small>{{ ctx.activeContact.value.state }} · {{ ctx.activeContact.value.user_id }}</small>
            </div>
          </div>
          <div class="row compact detail-actions">
            <button @click="ctx.goChatPage()">发消息</button>
            <button class="secondary" @click="ctx.showQr(ctx.activeContact.value.contact_card_text, '好友身份')">身份二维码</button>
            <button v-if="ctx.activeContact.value.state !== 'Blocked'" class="secondary" @click="ctx.blockActiveContact">拉黑</button>
            <button v-else class="secondary" @click="ctx.unblockActiveContact">解除拉黑</button>
            <button class="danger" @click="ctx.removeActiveContact">删除好友</button>
          </div>
        </section>

        <section v-else-if="ctx.activeGroup.value" class="home-card contact-detail-card">
          <div class="detail-head">
            <span class="avatar large group-avatar">群</span>
            <div>
              <h3>{{ ctx.activeGroup.value.name }}</h3>
              <small>{{ ctx.activeGroup.value.member_user_ids.length }} 人 · {{ ctx.activeGroup.value.group_id }}</small>
            </div>
          </div>
          <div class="member-list detail-members">
            <span v-for="m in ctx.activeGroupMembers.value" :key="m.user_id">{{ m.display_name || m.user_id }}</span>
          </div>
          <div class="row compact detail-actions">
            <button @click="ctx.goChatPage()">进入群聊</button>
            <button class="secondary" @click="ctx.createInviteForActiveGroup">生成邀请</button>
            <button class="secondary" @click="ctx.copyText(ctx.groupInviteText.value, '群邀请')">复制邀请</button>
            <button class="danger" @click="ctx.removeActiveGroup">删除群聊</button>
          </div>
        </section>

        <section id="new-friends" class="home-card">
          <h3>新朋友</h3>
          <div v-if="ctx.friendRequests.value.length" class="requests contact-requests">
            <div v-for="req in ctx.friendRequests.value" :key="req.request_id" class="request-item">
              <b>{{ req.from_user_id }}</b>
              <small>{{ req.note || '无备注' }}</small>
              <div class="row compact">
                <button @click="ctx.acceptInboxRequest(req)">接受</button>
                <button class="danger" @click="ctx.rejectInboxRequest(req)">忽略</button>
              </div>
            </div>
          </div>
          <div v-else class="empty">暂无好友请求</div>

          <details class="inline-details">
            <summary>离线添加</summary>
            <label>收到的好友请求</label>
            <textarea v-model="ctx.incomingFriendRequestText.value" rows="4" placeholder="粘贴好友请求" />
            <button @click="ctx.addIncomingFriendRequest">加入收件箱</button>
            <p class="hint">没有开启消息同步时，才需要用复制粘贴方式交换好友请求。</p>
          </details>

          <details class="inline-details">
            <summary>群邀请</summary>
            <label>收到的群邀请</label>
            <textarea v-model="ctx.incomingGroupInviteText.value" rows="3" placeholder="粘贴群邀请" />
            <button @click="ctx.addIncomingGroupInvite">加入收件箱</button>
            <div v-if="ctx.groupInvites.value.length" class="requests contact-requests">
              <div v-for="inv in ctx.groupInvites.value" :key="inv.invite_id" class="request-item">
                <b>{{ inv.group_name }}</b>
                <small>{{ inv.member_user_ids.length }} 人</small>
                <div class="row compact">
                  <button @click="ctx.acceptGroupInvite(inv)">接受入群</button>
                  <button class="danger" @click="ctx.ignoreGroupInvite(inv)">忽略</button>
                </div>
              </div>
            </div>
            <div v-else class="empty">暂无群邀请</div>
          </details>
        </section>

        <section id="groups" class="home-card">
          <h3>群聊</h3>
          <div v-if="filteredGroups.length" class="contacts-list">
            <button v-for="g in filteredGroups" :key="g.group_id" class="contact-row" @click="ctx.selectGroup(g.group_id)">
              <span class="avatar group-avatar">群</span>
              <span><b>{{ g.name }}</b><small>{{ g.member_user_ids.length }} 人</small></span>
            </button>
          </div>
          <div v-else class="empty">暂无群聊</div>
        </section>

        <section id="friends" class="home-card">
          <h3>好友</h3>
          <div v-if="filteredContacts.length" class="contacts-list">
            <button v-for="c in filteredContacts" :key="c.user_id" class="contact-row" @click="ctx.selectContact(c.user_id)">
              <span class="avatar">{{ (c.display_name || c.user_id || '?').slice(0, 1).toUpperCase() }}</span>
              <span><b>{{ c.display_name || '未命名' }}</b><small>{{ c.state }} · {{ c.user_id }}</small></span>
            </button>
          </div>
          <div v-else class="empty">暂无好友</div>
        </section>

        <section id="add-friend" class="home-card">
          <h3>添加好友</h3>
          <label>对方身份文本</label>
          <textarea v-model="ctx.addContactText.value" rows="5" placeholder="粘贴对方身份文本" />
          <button @click="ctx.addContact">添加好友</button>
        </section>

        <section id="create-group" class="home-card">
          <h3>发起群聊</h3>
          <label>群名</label>
          <input v-model="ctx.newGroupName.value" placeholder="例如：测试群" />
          <label>选择好友</label>
          <label v-for="c in ctx.friendContacts.value" :key="c.user_id" class="check-row">
            <input type="checkbox" :value="c.user_id" v-model="ctx.selectedGroupMembers.value" />
            {{ c.display_name || c.user_id }}
          </label>
          <div v-if="ctx.friendContacts.value.length === 0" class="empty">暂无可邀请好友</div>
          <button @click="ctx.createGroup">创建群聊</button>
        </section>
      </main>
    </div>
  </section>
</template>
