<script setup lang="ts">
import DebugPage from './DebugPage.vue'
const props = defineProps<{ ctx: any }>()
const contactName = (userId: string) => props.ctx.contacts.value.find((c: any) => c.user_id === userId)?.display_name || userId
const pendingOutbox = () => props.ctx.outbox.value.filter((o: any) => o.peer_user_id === props.ctx.activeContact.value?.user_id && o.status !== 'sent')
</script>

<template>
    <section class="chat-main">
      <header class="chat-header">
        <div v-if="ctx.activeContact.value">
          <h2>{{ ctx.activeContact.value.display_name || '未命名联系人' }}</h2>
          <small>{{ ctx.activeContact.value.user_id }} · {{ ctx.activeContact.value.fingerprint }} · {{ ctx.activeContact.value.state }} · devices {{ ctx.activeContact.value.device_count }} · revoked {{ ctx.activeContact.value.revoked_device_ids?.length || 0 }}</small>
        </div>
        <div v-else-if="ctx.activeGroup.value">
          <h2>{{ ctx.activeGroup.value.name }}</h2>
          <small>群组 · {{ ctx.activeGroup.value.member_user_ids.length }} 人 · admin {{ ctx.activeGroup.value.admin_user_ids?.length || 0 }} · seq {{ ctx.activeGroup.value.sequence || 0 }} · {{ ctx.activeGroup.value.group_id }}</small>
          <div class="member-list">
            <span v-for="m in ctx.activeGroupMembers.value" :key="m.user_id">{{ m.display_name || m.user_id }}</span>
          </div>
        </div>
        <div v-else>
          <h2>请选择联系人或群组</h2>
          <small>左侧只保留会话列表；添加、建群、网络和调试入口在这里。</small>
        </div>
        <div class="row compact" v-if="ctx.activeContact.value">
          <button @click="ctx.createFriendRequestForActive">生成好友请求</button>
          <input v-model="ctx.blockReason.value" placeholder="拉黑原因" />
          <button v-if="ctx.activeContact.value.state !== 'Blocked'" class="danger" @click="ctx.blockActiveContact">拉黑</button>
          <button v-else @click="ctx.unblockActiveContact">解除拉黑</button>
          <button @click="ctx.removeActiveContact" class="danger">删除联系人</button>
        </div>
        <div class="row compact" v-if="ctx.activeGroup.value">
          <button @click="ctx.createInviteForActiveGroup">生成群邀请</button>
          <button @click="ctx.copyText(ctx.groupInviteText.value, '群邀请')">复制群邀请</button>
          <button @click="ctx.showQr(ctx.groupInviteText.value, '群邀请')">群邀请二维码</button>
          <button @click="ctx.copyText(ctx.groupFanoutJson.value, '群发 fanout')">复制群发密文</button>
          <button @click="ctx.showQr(ctx.groupFanoutJson.value, '群发 fanout')">fanout 二维码</button>
          <button @click="ctx.removeActiveGroup" class="danger">删除群</button>
        </div>
      </header>

      <div class="messages">
        <div
          v-for="m in ctx.activeMessages.value"
          :key="m.id"
          class="bubble"
          :class="m.direction"
        >
          <div class="text">{{ m.text }}</div>
          <small>{{ m.direction === 'out' ? '我' : (contactName(m.peer_user_id)) }} · {{ ctx.formatTime(m.created_at) }} · {{ ctx.statusLabel(m.status) }}</small>
          <button v-if="m.envelope_json" @click="ctx.copyMessageEnvelope(m)">复制密文</button>
          <button v-if="m.envelope_json" @click="ctx.showQr(m.envelope_json, 'Envelope')">二维码</button>
        </div>
        <div v-if="(ctx.activeContact.value || ctx.activeGroup.value) && ctx.activeMessages.value.length === 0" class="empty center">还没有消息</div>
        <section v-if="!ctx.activeContact.value && !ctx.activeGroup.value" class="chat-home-panel">
          <div class="home-card identity-card">
            <h3>我的账号</h3>
            <label>我的显示名</label>
            <input v-model="ctx.displayName.value" @change="ctx.refreshMyContactCard" />
            <div class="row compact">
              <button @click="ctx.refreshMyContactCard">更新我的卡片</button>
              <button @click="ctx.copyText(ctx.myContactCardText.value, '我的 Contact Card')">复制我的卡片</button>
              <button @click="ctx.showQr(ctx.myContactCardText.value, '我的 Contact Card')">卡片二维码</button>
              <button @click="ctx.copyText(ctx.backupText.value, '身份备份包')">复制备份包</button>
              <button @click="ctx.showQr(ctx.backupText.value, '身份备份包')">备份包二维码</button>
            </div>
          </div>

          <div class="home-grid">
            <section class="home-card">
              <h3>添加联系人</h3>
              <label>添加联系人 Contact Card</label>
              <textarea v-model="ctx.addContactText.value" rows="4" placeholder="lm-contact-card-v1:..." />
              <button @click="ctx.addContact">添加联系人</button>
            </section>

            <section class="home-card">
              <h3>创建群</h3>
              <label>群名</label>
              <input v-model="ctx.newGroupName.value" placeholder="例如：测试群" />
              <label>选择 Friend 联系人</label>
              <label v-for="c in ctx.friendContacts.value" :key="c.user_id" class="check-row">
                <input type="checkbox" :value="c.user_id" v-model="ctx.selectedGroupMembers.value" />
                {{ c.display_name || c.user_id }}
              </label>
              <button @click="ctx.createGroup">创建群</button>
            </section>

            <section class="home-card">
              <h3>网络设置</h3>
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

            <section class="home-card">
              <h3>好友请求</h3>
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
            </section>
          </div>

          <details class="home-card">
            <summary>安全会话建立（推荐）</summary>
            <p class="hint">无服务器复制粘贴流程：A 创建 Offer 发给 B；B 应用 Offer 后生成 Response 发回 A；A 应用 Response。</p>
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

          <details class="home-card">
            <summary>群邀请 / 设备 / 数据备份</summary>
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
            <hr />
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
            <hr />
            <label>完整数据备份</label>
            <div class="row compact">
              <button @click="ctx.exportFullDataBackup">导出完整备份</button>
              <button @click="ctx.importFullDataBackup">导入完整备份</button>
              <button @click="ctx.downloadText(ctx.dataBackupText.value, 'lm-data-backup.txt')">下载</button>
              <button @click="ctx.showQr(ctx.dataBackupText.value, '完整数据备份')">二维码</button>
            </div>
            <textarea v-model="ctx.dataBackupText.value" rows="3" placeholder="lm-data-backup-v1:..." />
          </details>

          <details class="home-card advanced-debug-gate">
            <summary>高级调试入口（默认隐藏）</summary>
            <p class="hint">日常聊天不需要打开。这里保留协议 JSON、节点、Mailbox、PreKey、双棘轮、文件等高级排障工具；也可点击顶部“调试”进入独立调试页。</p>
            <div class="debug-panel">
              <DebugPage :ctx="ctx" />
            </div>
          </details>
        </section>
      </div>

      <footer class="composer" v-if="ctx.activeContact.value || ctx.activeGroup.value">
        <textarea v-model="ctx.composerText.value" rows="3" placeholder="输入消息。WebRTC 已连接则直发；否则启用节点时走 Mailbox；都不可用则进入 ctx.outbox.value。" />
        <button @click="ctx.sendMessage">发送/生成密文</button>
      </footer>

      <section class="tools" v-if="ctx.activeContact.value || ctx.activeGroup.value">
        <details class="right-quick-actions" open>
          <summary>添加 / 网络 / 调试</summary>
          <div class="right-tools-grid">
            <section>
              <h3>添加联系人</h3>
              <label>添加联系人 Contact Card</label>
              <textarea v-model="ctx.addContactText.value" rows="3" placeholder="lm-contact-card-v1:..." />
              <button @click="ctx.addContact">添加联系人</button>
            </section>
            <section class="add-box">
              <h3>创建群</h3>
              <label>群名</label>
              <input v-model="ctx.newGroupName.value" placeholder="例如：测试群" />
              <label>选择 Friend 联系人</label>
              <label v-for="c in ctx.friendContacts.value" :key="c.user_id" class="check-row">
                <input type="checkbox" :value="c.user_id" v-model="ctx.selectedGroupMembers.value" />
                {{ c.display_name || c.user_id }}
              </label>
              <button @click="ctx.createGroup">创建群</button>
            </section>
            <section>
              <h3>网络设置</h3>
              <label class="check-row"><input type="checkbox" v-model="ctx.autoNodeSync.value" @change="ctx.saveNetworkSettings" /> 自动从 Peer 节点同步 snapshot</label>
              <div class="row compact">
                <button @click="ctx.takeMailboxFromNode">收取消息</button>
                <button @click="ctx.checkNodeHealth">检查连接</button>
              </div>
            </section>
          </div>
        </details>

        <details class="advanced-debug-gate">
          <summary>高级调试入口（默认隐藏）</summary>
          <p class="hint">协议 JSON、节点、Mailbox、PreKey、双棘轮、文件等高级排障工具。</p>
          <div class="debug-panel">
            <DebugPage :ctx="ctx" />
          </div>
        </details>

        <details v-if="ctx.activeContact.value">
          <summary>设备撤销</summary>
          <label>收到的设备撤销事件</label>
          <textarea v-model="ctx.incomingDeviceRevokeText.value" rows="3" placeholder="lm-device-revoke-v1:..." />
          <button @click="ctx.applyDeviceRevokeToActiveContact">验证并应用到当前联系人</button>
          <div v-if="ctx.activeContact.value.revoked_device_ids?.length" class="empty">
            已撤销设备：{{ ctx.activeContact.value.revoked_device_ids.join(', ') }}
          </div>
          <div v-if="ctx.activeContact.value.device_certs?.length" class="empty">
            已知设备：<span v-for="d in ctx.activeContact.value.device_certs" :key="d.device_id">{{ d.device_name || 'device' }} / {{ d.device_id }} </span>
          </div>
        </details>

        <details v-if="ctx.activeContact.value">
          <summary>WebRTC 直连</summary>
          <p class="hint">状态：{{ ctx.rtcStatus.value }}</p>
          <div class="row compact">
            <button @click="ctx.createRtcOfferForActive">A 创建 Offer</button>
            <button @click="ctx.acceptRtcOfferForActive">B 接受 Offer 并生成 Answer</button>
            <button @click="ctx.applyRtcAnswerForActive">A 应用 Answer</button>
            <button class="danger" @click="ctx.resetRtc">重置</button>
          </div>
          <label>本地 Signal</label>
          <textarea v-model="ctx.localSignalText.value" rows="3" />
          <button @click="ctx.copySignal(ctx.localSignalText.value)">复制本地 Signal</button>
          <button @click="ctx.showQr(ctx.localSignalText.value, '本地 Signal')">Signal 二维码</button>
          <label>远端 Signal</label>
          <textarea v-model="ctx.remoteSignalText.value" rows="3" />
        </details>


        <details v-if="ctx.activeContact.value">
          <summary>待发送队列</summary>
          <div class="row compact">
            <button @click="ctx.flushOutboxForActive">重发当前联系人</button>
            <button @click="ctx.clearSentOutbox">清理已发送</button>
          </div>
          <div v-if="pendingOutbox().length === 0" class="empty">暂无待发送</div>
          <div v-for="item in pendingOutbox()" :key="item.id" class="fanout-item">
            <small>{{ item.status }} · retry {{ item.retry_count }}</small>
            <button @click="ctx.copyText(item.envelope_json, '待发送 Envelope')">复制</button>
          </div>
        </details>

        <details v-if="ctx.activeContact.value">
          <summary>交换区：好友请求 / 收到的密文 Envelope</summary>
          <label>好友请求</label>
          <textarea v-model="ctx.friendRequestText.value" rows="3" />
          <button @click="ctx.copyText(ctx.friendRequestText.value, '好友请求')">复制好友请求</button>
          <button @click="ctx.showQr(ctx.friendRequestText.value, '好友请求')">好友请求二维码</button>
          <label>收到的好友响应</label>
          <textarea v-model="ctx.incomingFriendResponseText.value" rows="3" placeholder="lm-friend-response-v1:..." />
          <button @click="ctx.applyFriendResponse">应用好友响应</button>
          <label>收到的 Envelope JSON</label>
          <textarea v-model="ctx.inboundEnvelopeText.value" rows="5" />
          <button @click="ctx.receiveEnvelope">解密并加入聊天</button>
        </details>

        <details v-if="ctx.activeContact.value">
          <summary>文件传输 MVP</summary>
          <p class="hint">文件会被切块加密成 JSON 包；WebRTC 已连接时可直接通过 DataChannel 发送，也可复制/下载后离线传输。</p>
          <input type="file" @change="ctx.onFileSelected" />
          <div class="row compact">
            <button @click="ctx.createFilePackageForActive">加密文件包</button>
            <button @click="ctx.sendFilePackageOverRtc">WebRTC 发送文件包</button>
            <button @click="ctx.copyText(ctx.filePackageText.value, '文件包')">复制文件包</button>
            <button @click="ctx.downloadText(ctx.filePackageText.value, 'lm-file-package.json')">下载文件包</button>
          </div>
          <small>{{ ctx.rtcFileStatus.value }}</small>
          <textarea v-model="ctx.filePackageText.value" rows="5" placeholder="生成的文件包 JSON" />
          <label>收到的文件包 JSON</label>
          <textarea v-model="ctx.incomingFilePackageText.value" rows="5" placeholder="粘贴文件包 JSON" />
          <div class="row compact">
            <button @click="ctx.inspectIncomingFilePackage">解析文件包</button>
            <button @click="ctx.decryptIncomingFilePackage">解密文件包</button>
            <a v-if="ctx.receivedFileUrl.value" :href="ctx.receivedFileUrl.value" :download="ctx.receivedFileName.value">下载解密文件：{{ ctx.receivedFileName.value }}</a>
          </div>
          <textarea v-model="ctx.filePackageInfoText.value" rows="5" placeholder="文件包信息" readonly />
        </details>
        <details v-if="ctx.activeGroup.value">
          <summary>群邀请 / 群发 fanout 密文</summary>
          <label>群邀请</label>
          <textarea v-model="ctx.groupInviteText.value" rows="3" />
          <button @click="ctx.copyText(ctx.groupInviteText.value, '群邀请')">复制群邀请</button>
          <button @click="ctx.showQr(ctx.groupInviteText.value, '群邀请')">群邀请二维码</button>
          <p class="hint">如果本群已有自己的 Sender Key，发送会生成一份群 Sender Envelope；否则回退为每个成员生成一份单聊 Envelope。</p>
          <div class="row compact">
            <button @click="ctx.createGroupSenderKeyForActiveGroup">创建我的群 Sender Key</button>
            <button @click="ctx.copyText(ctx.groupSenderDistributionText.value, 'Sender Key Distribution')">复制 Sender Key</button>
            <button @click="ctx.importGroupSenderKeyForActiveContact">导入当前联系人的 Sender Key</button>
            <button @click="ctx.groupSenderEncryptDebug">Sender Key 加密输入框</button>
            <button @click="ctx.groupSenderDecryptDebug">解密 Sender Envelope</button>
          </div>
          <label>Sender Key Distribution</label>
          <textarea v-model="ctx.groupSenderDistributionText.value" rows="3" placeholder="lm-group-sender-key-v1:..." />
          <button @click="ctx.createGroupSenderDistributionFanoutForActiveGroup">生成 Sender Key fanout</button>
          <label>Sender Key Distribution fanout</label>
          <textarea v-model="ctx.groupSenderDistributionFanoutJson.value" rows="5" placeholder="Sender Key fanout JSON" />
          <button @click="ctx.copyText(ctx.groupSenderDistributionFanoutJson.value, 'Sender Key fanout')">复制 Sender Key fanout</button>
          <div v-if="ctx.groupSenderDistributionFanoutItems.value.length" class="fanout-list">
            <div v-for="item in ctx.groupSenderDistributionFanoutItems.value" :key="item.to_user_id" class="fanout-item">
              <small>to: {{ contactName(item.to_user_id) }}</small>
              <button @click="ctx.copyText(item.envelope, 'Sender Key 给 ' + item.to_user_id)">复制单条</button>
            </div>
          </div>
          <label>Sender Envelope JSON</label>
          <textarea v-model="ctx.groupSenderEnvelopeText.value" rows="5" placeholder="lm-group-sender-envelope-v1 JSON" />
          <label>Sender Plain JSON</label>
          <textarea v-model="ctx.groupSenderPlainText.value" rows="4" readonly />
          <label>群事件：新群名</label>
          <input v-model="ctx.groupRenameText.value" placeholder="新群名" />
          <button @click="ctx.createRenameGroupEvent">生成改名事件</button>
          <label>群事件文本</label>
          <textarea v-model="ctx.groupEventText.value" rows="3" placeholder="lm-group-event-v1:..." />
          <div class="row compact">
            <button @click="ctx.copyText(ctx.groupEventText.value, '群事件')">复制群事件</button>
            <button @click="ctx.showQr(ctx.groupEventText.value, '群事件')">群事件二维码</button>
            <button @click="ctx.applyGroupEventText">应用本地群事件</button>
            <button @click="ctx.createGroupEventFanout">生成群事件 fanout</button>
          </div>
          <label>群事件 fanout</label>
          <textarea v-model="ctx.groupEventFanoutJson.value" rows="5" placeholder="群事件 fanout JSON" />
          <button @click="ctx.copyText(ctx.groupEventFanoutJson.value, '群事件 fanout')">复制群事件 fanout</button>
          <div v-if="ctx.groupEventFanoutItems.value.length" class="fanout-list">
            <div v-for="item in ctx.groupEventFanoutItems.value" :key="item.to_user_id" class="fanout-item">
              <small>to: {{ contactName(item.to_user_id) }}</small>
              <button @click="ctx.copyText(item.envelope, '群事件给 ' + item.to_user_id)">复制单条</button>
            </div>
          </div>
          <label>收到的群事件</label>
          <textarea v-model="ctx.incomingGroupEventText.value" rows="3" placeholder="lm-group-event-v1:..." />
          <label>事件发起者 UserID</label>
          <input v-model="ctx.groupEventActorUserId.value" placeholder="默认当前联系人或自己" />
          <button @click="ctx.applyGroupEventText">验证并应用收到的群事件</button>
          <label>成员事件</label>
          <div v-for="c in ctx.friendContacts.value" :key="c.user_id" class="fanout-item">
            <small>{{ c.display_name || c.user_id }}</small>
            <button v-if="!ctx.activeGroup.value.member_user_ids.includes(c.user_id)" @click="ctx.createAddMemberGroupEvent(c.user_id)">生成加入事件</button>
            <button v-else class="danger" @click="ctx.createRemoveMemberGroupEvent(c.user_id)">生成移除事件</button>
            <button v-if="!(ctx.activeGroup.value.admin_user_ids || []).includes(c.user_id)" @click="ctx.createPromoteAdminGroupEvent(c.user_id)">提升管理员</button>
            <button v-else @click="ctx.createDemoteAdminGroupEvent(c.user_id)">取消管理员</button>
          </div>
          <textarea v-model="ctx.groupFanoutJson.value" rows="8" />
          <button @click="ctx.copyText(ctx.groupFanoutJson.value, '群发 fanout')">复制群发 fanout</button>
          <div v-if="ctx.fanoutItems.value.length" class="fanout-list">
            <div v-for="item in ctx.fanoutItems.value" :key="item.to_user_id" class="fanout-item">
              <small>to: {{ contactName(item.to_user_id) }}</small>
              <button @click="ctx.copyText(item.envelope, '给 ' + item.to_user_id + ' 的 Envelope')">复制单条</button>
              <button @click="ctx.showQr(item.envelope, '给 ' + item.to_user_id + ' 的 Envelope')">二维码</button>
            </div>
          </div>
        </details>
      </section>
    </section>
</template>
