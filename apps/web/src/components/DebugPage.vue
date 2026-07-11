<script setup lang="ts">
defineProps<{ ctx: any }>()
</script>

<template>
    <section class="debug-page">
      <header class="debug-header">
        <div>
          <h1>调试页面</h1>
          <p class="hint">协议、节点、Mailbox、PreKey、双棘轮等开发排障工具。与日常聊天页面完全分开。</p>
        </div>
        <button @click="ctx.goChatPage">返回聊天</button>
      </header>
      <div class="debug-grid">
      <details class="add-box">
        <summary>X3DH / PreKey 调试（高级）</summary>
        <p class="hint">用于未来离线建链：发布 signed prekey + one-time prekeys，对方用它派生初始 shared secret，再初始化 Double Ratchet。Private Bundle 只能本地加密保存，不能发给别人。</p>
        <div class="row compact">
          <label>signed_prekey_id <input v-model.number="ctx.prekeySignedId.value" type="number" min="1" /></label>
          <label>one-time 数量 <input v-model.number="ctx.prekeyOneTimeCount.value" type="number" min="0" max="100" /></label>
        </div>
        <div class="row compact">
          <button @click="ctx.createMyPreKeyBundleText">生成我的 PreKey Bundle</button>
          <button @click="ctx.inspectPreKeyBundleText">解析/验签 Bundle</button>
          <button @click="ctx.copyText(ctx.prekeyBundleText.value, 'PreKey Bundle')">复制公开 Bundle</button>
          <button @click="ctx.showQr(ctx.prekeyBundleText.value, 'PreKey Bundle')">二维码</button>
        </div>
        <label>公开 PreKey Bundle（可发给联系人/放入 DHT）</label>
        <textarea v-model="ctx.prekeyBundleText.value" rows="4" placeholder="lm-prekey-bundle-v1:..." />
        <label>Private PreKey Bundle（仅本机保存，含私钥）</label>
        <textarea v-model="ctx.prekeyPrivateBundleJson.value" rows="4" placeholder="private json" />
        <label>Bundle 摘要</label>
        <textarea v-model="ctx.prekeyInfoText.value" rows="4" readonly />
        <hr />
        <div class="row compact">
          <button @click="ctx.createX3dhInitialMessageText">作为发起方创建 Initial Message</button>
          <button @click="ctx.deriveX3dhResponderSecretText">作为响应方派生 Shared Secret</button>
        </div>
        <label>X3DH Initial Message JSON</label>
        <textarea v-model="ctx.x3dhInitialMessageJson.value" rows="5" placeholder="initial message json" />
        <label>选中的 one-time prekey id</label>
        <input :value="ctx.selectedOneTimePreKeyId.value ?? ''" readonly placeholder="节点领取时自动填入" />
        <label>Shared Secret（调试显示；正式版不能暴露）</label>
        <textarea v-model="ctx.x3dhSharedSecretText.value" rows="2" readonly />
      </details>

      <details class="add-box">
        <summary>双棘轮状态调试（高级）</summary>
        <p class="hint">正式聊天会优先使用已保存的 Ratchet Session；没有会话时才回退 MVP 静态 X25519。Ratchet state 是敏感数据，只应放在加密本地库或加密备份里。</p>
        <div class="row compact">
          <button @click="ctx.createRatchetPairForActiveContact">为当前联系人生成测试状态对</button>
          <button @click="ctx.createRatchetFromSharedSecretText">用 Shared Secret 生成测试双端状态</button>
          <button @click="ctx.generateRatchetDhKeyPairText">生成本端 DH keypair</button>
          <button @click="ctx.createRatchetFromSharedSecretWithKeysText">用 Shared Secret + 双方 DH 初始化</button>
          <button @click="ctx.inspectRatchetStateText">解析状态</button>
          <button @click="ctx.ratchetNextSendKeyText">推进发送链</button>
          <button @click="ctx.ratchetNextRecvKeyText">按 Header 推进接收链</button>
        </div>
        <div class="row compact">
          <button @click="ctx.ratchetEncryptEnvelopeText">Ratchet 加密输入框消息</button>
          <button @click="ctx.ratchetDecryptEnvelopeText">Ratchet 解密 Envelope</button>
        </div>
        <label>本端 Ratchet State</label>
        <textarea v-model="ctx.ratchetStateText.value" rows="4" placeholder="lm-ratchet-state-v1:..." />
        <label>对端测试 State（仅本机调试，用来模拟另一端）</label>
        <textarea v-model="ctx.ratchetPeerStateText.value" rows="3" placeholder="生成测试状态对后出现；真实协议不会传输私钥状态" />
        <label>本端 Ratchet DH keypair（private_key 只本地保存；public_key 发给对方）</label>
        <textarea v-model="ctx.ratchetLocalDhKeyPairJson.value" rows="3" placeholder="{ private_key, public_key }" />
        <label>对方 Ratchet DH public_key</label>
        <input v-model="ctx.ratchetRemoteDhPublicKeyForInit.value" placeholder="base64 public_key" />
        <label>初始化角色</label>
        <select v-model="ctx.ratchetInitRole.value">
          <option value="Initiator">Initiator</option>
          <option value="Responder">Responder</option>
        </select>
        <label>Ratchet Header</label>
        <textarea v-model="ctx.ratchetHeaderText.value" rows="3" placeholder="发送链生成，或粘贴收到消息的 header JSON" />
        <label>Ratchet Envelope JSON</label>
        <textarea v-model="ctx.ratchetEnvelopeText.value" rows="5" placeholder="x3dh-double-ratchet-v1 envelope" />
        <label>Ratchet 解密明文 JSON</label>
        <textarea v-model="ctx.ratchetPlainText.value" rows="4" readonly />
        <label>最近 Message Key / Header</label>
        <textarea v-model="ctx.ratchetKeyText.value" rows="4" readonly />
        <label>远端新 DH public key（高级）</label>
        <input v-model="ctx.ratchetRemoteDhPublicKey.value" placeholder="base64 public key" />
        <button @click="ctx.ratchetDhStepText">DH Ratchet Step</button>
        <label>状态摘要</label>
        <textarea v-model="ctx.ratchetInfoText.value" rows="5" readonly />
      </details>

      <details class="add-box">
        <summary>本地安全策略 / 过滤</summary>
        <p class="hint">无社区管理员；这里只做本设备本地自治过滤，不上传、不审查全网。</p>
        <label class="check-row"><input type="checkbox" v-model="ctx.safetyPolicy.value.enableTextFilter" /> 启用文本过滤</label>
        <label>过滤级别</label>
        <select v-model="ctx.safetyPolicy.value.textFilterLevel">
          <option value="Off">Off</option>
          <option value="Relaxed">Relaxed</option>
          <option value="Standard">Standard</option>
          <option value="Strict">Strict</option>
        </select>
        <label class="check-row"><input type="checkbox" v-model="ctx.safetyPolicy.value.warnExternalLinks" /> 链接提示</label>
        <label class="check-row"><input type="checkbox" v-model="ctx.safetyPolicy.value.warnExecutableFiles" /> 可执行文件名提示</label>
        <label class="check-row"><input type="checkbox" v-model="ctx.safetyPolicy.value.dropFilteredIncoming" /> 严格命中时丢弃收到的消息</label>
        <button @click="ctx.saveSafetyPolicy">保存策略</button>
      </details>

      <details class="add-box">
        <summary>Public Peer / Mailbox 协议调试</summary>
        <p class="hint">这里只生成/验签协议对象，不启动中心服务器，也不使用摄像头。公网 IP 节点后续可发布 PublicPeerAnnounce，充当可选 bootstrap / DHT / relay / mailbox。</p>

        <label>我的 PeerAnnounce 地址（每行一个）</label>
        <textarea v-model="ctx.peerAddressesText.value" rows="2" />
        <label>Mailbox key（可选）</label>
        <input v-model="ctx.peerMailboxKey.value" placeholder="可选 mailbox key" />
        <div class="row compact">
          <button @click="ctx.createPeerAnnounceText">生成 PeerAnnounce</button>
          <button @click="ctx.inspectPeerAnnounceText">验签/解析</button>
          <button @click="ctx.copyText(ctx.peerAnnounceText.value, 'PeerAnnounce')">复制</button>
          <button @click="ctx.showQr(ctx.peerAnnounceText.value, 'PeerAnnounce')">二维码</button>
        </div>
        <textarea v-model="ctx.peerAnnounceText.value" rows="3" placeholder="lm-peer-announce-v1:..." />
        <label>发布者 identity_public_key</label>
        <input v-model="ctx.peerAnnounceInspectPublicKey.value" placeholder="base64 public key" />
        <textarea v-model="ctx.peerAnnounceInfoText.value" rows="4" placeholder="解析结果" readonly />

        <label>Public Peer ID</label>
        <input v-model="ctx.publicPeerId.value" placeholder="例如 public-peer-1" />
        <label>公网地址（每行一个）</label>
        <textarea v-model="ctx.publicPeerAddressesText.value" rows="2" />
        <label class="check-row"><input type="checkbox" value="bootstrap" v-model="ctx.publicPeerCapabilities.value" /> bootstrap</label>
        <label class="check-row"><input type="checkbox" value="dht" v-model="ctx.publicPeerCapabilities.value" /> dht</label>
        <label class="check-row"><input type="checkbox" value="signaling" v-model="ctx.publicPeerCapabilities.value" /> signaling</label>
        <label class="check-row"><input type="checkbox" value="relay" v-model="ctx.publicPeerCapabilities.value" /> relay</label>
        <label class="check-row"><input type="checkbox" value="mailbox" v-model="ctx.publicPeerCapabilities.value" /> mailbox</label>
        <div class="row compact">
          <button @click="ctx.createPublicPeerAnnounceText">生成 PublicPeerAnnounce</button>
          <button @click="ctx.inspectPublicPeerAnnounceText">验签/解析</button>
          <button @click="ctx.copyText(ctx.publicPeerAnnounceText.value, 'PublicPeerAnnounce')">复制</button>
          <button @click="ctx.showQr(ctx.publicPeerAnnounceText.value, 'PublicPeerAnnounce')">二维码</button>
        </div>
        <textarea v-model="ctx.publicPeerAnnounceText.value" rows="3" placeholder="lm-public-peer-announce-v1:..." />
        <label>发布者 identity_public_key</label>
        <input v-model="ctx.publicPeerAnnounceInspectPublicKey.value" placeholder="base64 public key" />
        <textarea v-model="ctx.publicPeerAnnounceInfoText.value" rows="4" placeholder="解析结果" readonly />

        <label>MailboxMessage 类型</label>
        <select v-model="ctx.mailboxKind.value">
          <option value="signal-offer">signal-offer</option>
          <option value="signal-answer">signal-answer</option>
          <option value="direct-envelope">direct-envelope</option>
          <option value="group-fanout">group-fanout</option>
          <option value="other">other</option>
        </select>
        <small>收件人使用当前选中的联系人。</small>
        <label>密文/信令载荷</label>
        <textarea v-model="ctx.mailboxCiphertext.value" rows="3" placeholder="Envelope / Signal 文本" />
        <div class="row compact">
          <button @click="ctx.createMailboxMessageText">生成 MailboxMessage</button>
          <button @click="ctx.inspectMailboxMessageText">验签/解析</button>
          <button @click="ctx.copyText(ctx.mailboxMessageText.value, 'MailboxMessage')">复制</button>
          <button @click="ctx.showQr(ctx.mailboxMessageText.value, 'MailboxMessage')">二维码</button>
        </div>
        <textarea v-model="ctx.mailboxMessageText.value" rows="3" placeholder="lm-mailbox-message-v1:..." />
        <label>发送者 identity_public_key</label>
        <input v-model="ctx.mailboxMessageInspectPublicKey.value" placeholder="base64 public key" />
        <textarea v-model="ctx.mailboxMessageInfoText.value" rows="4" placeholder="解析结果" readonly />

        <hr />
        <label>lm_node 控制面 URL</label>
        <input v-model="ctx.nodeControlUrl.value" placeholder="http://127.0.0.1:8787" />
        <div class="row compact">
          <button @click="ctx.checkNodeHealth">Health</button>
          <button @click="ctx.submitPublicPeerToNode">提交 PublicPeer</button>
          <button @click="ctx.pushMailboxToNode">提交 Mailbox</button>
        </div>
        <textarea v-model="ctx.nodeControlStatus.value" rows="4" placeholder="控制面响应" readonly />

        <label>查询最近节点 target peer_id</label>
        <input v-model="ctx.nodeClosestTarget.value" placeholder="peer id" />
        <button @click="ctx.queryNodeClosestPeers">查询 /peers/closest</button>
        <textarea v-model="ctx.nodeClosestInfoText.value" rows="5" placeholder="closest peers" readonly />

        <label>领取 mailbox 的 UserID</label>
        <input v-model="ctx.nodeMailboxTakeUserId.value" placeholder="默认当前 UserID" />
        <div class="row compact">
          <button @click="ctx.takeMailboxFromNode">领取 /mailbox/take</button>
          <button @click="ctx.processMailboxTakeInfoText">处理下方 mailbox JSON</button>
        </div>
        <textarea v-model="ctx.nodeMailboxTakeInfoText.value" rows="5" placeholder="mailbox messages" />

        <hr />
        <label>PreKey UserID</label>
        <input v-model="ctx.nodePreKeyUserId.value" placeholder="默认当前选中联系人；发布时使用当前身份" />
        <div class="row compact">
          <button @click="ctx.publishPreKeyToNode">发布我的 PreKey</button>
          <button @click="ctx.fetchPreKeyFromNode">查询联系人 PreKey</button>
          <button @click="ctx.consumePreKeyFromNode">领取并消费 PreKey</button>
        </div>
        <textarea v-model="ctx.nodePreKeyStatusText.value" rows="5" placeholder="prekey response" readonly />

        <hr />
        <label>节点同步 Peer URL</label>
        <input v-model="ctx.nodeSyncPeerUrl.value" placeholder="http://127.0.0.1:8788" />
        <div class="row compact">
          <button @click="ctx.exportNodeSnapshot">导出当前节点快照</button>
          <button @click="ctx.importNodeSnapshot">导入下方快照</button>
          <button @click="ctx.pullSnapshotFromPeerNode">从 Peer 拉取并导入</button>
        </div>
        <textarea v-model="ctx.nodeSyncSnapshotText.value" rows="5" placeholder="NodeStateSnapshot JSON" />
        <textarea v-model="ctx.nodeSyncStatusText.value" rows="4" placeholder="sync status" readonly />
      </details>
      </div>
    </section>
</template>
