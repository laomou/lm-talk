<script setup lang="ts">
defineProps<{ ctx: any }>()
</script>

<template>
    <aside class="sidebar">
      <div class="me">
        <h2>{{ ctx.displayName.value }}</h2>
        <small>{{ ctx.identity.value?.user_id }}</small>
        <label>我的显示名</label>
        <input v-model="ctx.displayName.value" @change="ctx.refreshMyContactCard" />
      </div>
      <div class="row compact">
        <button @click="ctx.refreshMyContactCard">更新我的卡片</button>
        <button @click="ctx.copyText(ctx.myContactCardText.value, '我的 Contact Card')">复制我的卡片</button>
        <button @click="ctx.showQr(ctx.myContactCardText.value, '我的 Contact Card')">卡片二维码</button>
        <button @click="ctx.copyText(ctx.backupText.value, '身份备份包')">复制备份包</button>
        <button @click="ctx.showQr(ctx.backupText.value, '身份备份包')">备份包二维码</button>
      </div>

      <section class="add-box network-card">
        <label>网络设置</label>
        <input v-model="ctx.nodeControlUrl.value" placeholder="lm_node 控制面 URL，例如 http://127.0.0.1:8787" />
        <div class="row compact">
          <button @click="ctx.toggleNodeEnabled">{{ ctx.nodeEnabled.value ? '停用节点' : '启用节点' }}</button>
          <button @click="ctx.saveNetworkSettings">保存设置</button>
          <button @click="ctx.checkNodeHealth">检查连接</button>
          <button @click="ctx.takeMailboxFromNode">收取消息</button>
          <button @click="ctx.autoPublishPreKeyIfEnabled">发布 PreKey</button>
        </div>
        <label class="check-row"><input type="checkbox" v-model="ctx.autoMailboxTake.value" @change="ctx.saveNetworkSettings" /> 登录后自动收取 Mailbox</label>
        <label class="check-row"><input type="checkbox" v-model="ctx.autoPublishPreKey.value" @change="ctx.saveNetworkSettings" /> 登录后自动发布 PreKey</label>
        <label class="check-row"><input type="checkbox" v-model="ctx.autoNodeSync.value" @change="ctx.saveNetworkSettings" /> 自动从 Peer 节点同步 snapshot</label>
        <small>状态：{{ ctx.nodeEnabled.value ? '节点已启用' : '节点已停用' }} · {{ ctx.nodeControlStatus.value }}</small>
      </section>


      <details class="help-box">
        <summary>演示流程</summary>
        <ol>
          <li>双方分别创建身份，并交换 Contact Card。</li>
          <li>一方添加联系人并生成好友请求。</li>
          <li>另一方在好友请求收件箱接受，返回好友响应。</li>
          <li>请求方应用好友响应后，联系人状态变为 Friend。</li>
          <li>可在“安全会话建立”里创建 Offer/Response，建立 Ratchet Session。</li>
          <li>选择 Friend 联系人后发送消息；有 Ratchet Session 时自动双棘轮加密，WebRTC 已连接时会直发，否则复制密文。</li>
        </ol>
      </details>
      <details class="help-box">
        <summary>安全状态</summary>
        <ul>
          <li>消息内容端到端加密；已保存 Ratchet Session 的联系人优先使用 x3dh-double-ratchet-v1，否则回退 MVP 加密。</li>
          <li>Web 版本存在代码供应链风险。</li>
          <li>无服务器模式不保证离线可达。</li>
          <li>本地数据保存在 IndexedDB，可导出完整加密备份。</li>
          <li>协议对象有大小限制：文本 64KB，Signal 256KB，文件 MVP 16MB。</li>
        </ul>
      </details>

      <details class="add-box">
        <summary>安全会话建立（推荐）</summary>
        <p class="hint">无服务器复制粘贴流程：A 创建 Offer 发给 B；B 应用 Offer 后生成 Response 发回 A；A 应用 Response。完成后双方保存 Ratchet Session，普通聊天会自动双棘轮加密。</p>
        <div class="row compact">
          <button @click="ctx.createSecureSessionOfferText">1. 创建 Offer</button>
          <button @click="ctx.applySecureSessionOfferText">2. 应用 Offer 并生成 Response</button>
          <button @click="ctx.applySecureSessionResponseText">3. 应用 Response</button>
        </div>
        <label>发给对方的 Offer</label>
        <textarea v-model="ctx.secureSessionOfferText.value" rows="5" placeholder="lm-secure-session-offer-v1 JSON" />
        <label>收到的 Offer 或 Response</label>
        <textarea v-model="ctx.incomingSecureSessionText.value" rows="5" placeholder="粘贴对方发来的 Offer/Response JSON" />
        <label>发回对方的 Response</label>
        <textarea v-model="ctx.secureSessionResponseText.value" rows="5" placeholder="lm-secure-session-response-v1 JSON" />
        <div class="row compact">
          <button @click="ctx.copyText(ctx.secureSessionOfferText.value, 'Secure Session Offer')">复制 Offer</button>
          <button @click="ctx.copyText(ctx.secureSessionResponseText.value, 'Secure Session Response')">复制 Response</button>
          <button @click="ctx.showQr(ctx.secureSessionOfferText.value || ctx.secureSessionResponseText.value, 'Secure Session')">二维码</button>
        </div>
        <label>状态</label>
        <textarea v-model="ctx.secureSessionStatusText.value" rows="3" readonly />
      </details>



      <div class="add-box">
        <label>设备管理</label>
        <div class="row compact">
          <button @click="ctx.createMyDeviceCert">生成本设备证书</button>
          <button @click="ctx.copyText(ctx.myDeviceCertJson.value, '设备证书')">复制设备证书</button>
        </div>
        <small>本设备：{{ ctx.myDeviceId.value || '未生成' }}</small>
        <label>撤销 device_id</label>
        <input v-model="ctx.revokeDeviceId.value" placeholder="dev1_..." />
        <label>撤销原因</label>
        <input v-model="ctx.revokeReason.value" placeholder="lost / compromised / old device" />
        <div class="row compact">
          <button @click="ctx.createDeviceRevokeText">生成撤销事件</button>
          <button @click="ctx.copyText(ctx.deviceRevokeText.value, '设备撤销事件')">复制撤销事件</button>
        </div>
        <textarea v-model="ctx.deviceRevokeText.value" rows="3" placeholder="lm-device-revoke-v1:..." />
        <div class="empty">提示：身份备份包 + 提示词可在新设备恢复身份；每台设备应生成独立设备证书并更新 Contact Card。</div>
      </div>

      <div class="add-box">
        <label>完整数据备份</label>
        <div class="row compact">
          <button @click="ctx.exportFullDataBackup">导出完整备份</button>
          <button @click="ctx.importFullDataBackup">导入完整备份</button>
          <button @click="ctx.downloadText(ctx.dataBackupText.value, 'lm-data-backup.txt')">下载</button>
          <button @click="ctx.showQr(ctx.dataBackupText.value, '完整数据备份')">二维码</button>
        </div>
        <textarea v-model="ctx.dataBackupText.value" rows="3" placeholder="lm-data-backup-v1:..." />
      </div>

      <div class="add-box">
        <label>添加联系人 Contact Card</label>
        <textarea v-model="ctx.addContactText.value" rows="4" placeholder="lm-contact-card-v1:..." />
        <button @click="ctx.addContact">添加联系人</button>
      </div>


      <div class="add-box">
        <label>好友请求收件箱</label>
        <textarea v-model="ctx.incomingFriendRequestText.value" rows="3" placeholder="lm-friend-request-v1:..." />
        <button @click="ctx.addIncomingFriendRequest">加入收件箱</button>
        <div v-if="ctx.friendRequests.value.length" class="requests">
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
      </div>


      <div class="add-box">
        <label>群邀请收件箱</label>
        <textarea v-model="ctx.incomingGroupInviteText.value" rows="3" placeholder="lm-group-invite-v1:..." />
        <small>先在联系人列表选择邀请者，再加入群邀请以验签。</small>
        <button @click="ctx.addIncomingGroupInvite">加入群邀请</button>
        <div v-if="ctx.groupInvites.value.length" class="requests">
          <div v-for="inv in ctx.groupInvites.value" :key="inv.invite_id" class="request-item">
            <b>{{ inv.group_name }}</b>
            <small>邀请者：{{ inv.inviter_user_id }} · {{ inv.member_user_ids.length }} 人</small>
            <div class="row compact">
              <button @click="ctx.acceptGroupInvite(inv)">接受入群</button>
              <button class="danger" @click="ctx.ignoreGroupInvite(inv)">忽略</button>
            </div>
          </div>
        </div>
        <div v-else class="empty">暂无群邀请</div>
      </div>

      <h3>联系人</h3>
      <button
        v-for="c in ctx.contacts.value"
        :key="c.user_id"
        class="contact"
        :class="{ active: c.user_id === ctx.activePeerId.value }"
        @click="ctx.selectContact(c.user_id)"
      >
        <b>{{ c.display_name || '未命名' }} <em>{{ c.state }}</em></b>
        <small>{{ c.user_id }}</small>
      </button>

      <h3>创建群组</h3>
      <div class="add-box">
        <label>群名</label>
        <input v-model="ctx.newGroupName.value" placeholder="例如：测试群" />
        <label>选择 Friend 联系人</label>
        <label v-for="c in ctx.friendContacts.value" :key="c.user_id" class="check-row">
          <input type="checkbox" :value="c.user_id" v-model="ctx.selectedGroupMembers.value" />
          {{ c.display_name || c.user_id }}
        </label>
        <button @click="ctx.createGroup">创建群</button>
      </div>
      <h3>群组</h3>
      <button
        v-for="g in ctx.groups.value"
        :key="g.group_id"
        class="contact"
        :class="{ active: g.group_id === ctx.activeGroupId.value }"
        @click="ctx.selectGroup(g.group_id)"
      >
        <b>{{ g.name }}</b>
        <small>{{ g.member_user_ids.length }} 人</small>
      </button>
      <div v-if="ctx.groups.value.length === 0" class="empty">暂无群组</div>
    </aside>
</template>
