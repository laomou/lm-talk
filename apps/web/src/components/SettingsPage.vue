<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ ctx: any }>()

const outboxItems = computed(() => props.ctx.outbox.value)
const pendingOutbox = computed(() => outboxItems.value.filter((item: any) => item.status !== 'sent'))
const failedOutbox = computed(() => outboxItems.value.filter((item: any) => item.status === 'failed'))
const contactName = (userId: string) => props.ctx.contacts.value.find((c: any) => c.user_id === userId)?.display_name || userId
const outboxKindLabel = (kind?: string) =>
  kind === 'group-fanout' ? '群消息' : kind === 'file-package' ? '文件' : kind === 'other' ? '系统消息' : '单聊消息'
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
          <button class="secondary" @click="ctx.showQr(ctx.backupText.value, '导出身份')">导出身份</button>
        </div>
      </header>

      <section class="home-card">
        <h3>我的资料</h3>
        <label>显示名</label>
        <div class="inline-field">
          <input v-model="ctx.displayName.value" @change="ctx.refreshMyContactCard" />
          <button @click="ctx.refreshMyContactCard">保存</button>
        </div>
      </section>

      <section class="home-card sync-card">
        <div class="section-title-row">
          <h3>消息同步</h3>
          <span class="sync-pill" :class="{ on: ctx.nodeEnabled.value }">{{ ctx.nodeEnabled.value ? '已开启' : '未开启' }}</span>
        </div>
        <label>同步服务</label>
        <textarea v-model="ctx.nodeControlUrl.value" rows="4" placeholder="每行一个同步服务地址，例如：&#10;http://127.0.0.1:8787&#10;http://192.168.1.23:8787|令牌&#10;http://[fd00::1234]:8787|令牌" />
        <small>开启后可自动收发好友请求和离线消息。支持局域网 IPv4/IPv6，可填多个。<br>跨设备访问时节点需设 <code>--control-token</code>，在地址后用 <code>|令牌</code> 附上（与节点一致）；仅本机(127.0.0.1)可不填令牌。</small>
        <div class="row compact">
          <button @click="ctx.toggleNodeEnabled">{{ ctx.nodeEnabled.value ? '关闭同步' : '开启同步' }}</button>
          <button class="secondary" @click="ctx.saveNetworkSettings">保存</button>
          <button class="secondary" @click="ctx.syncNow">立即同步</button>
        </div>
        <div class="sync-status">
          <b>同步状态</b>
          <small>{{ ctx.nodeControlStatus.value || '未连接' }}</small>
        </div>
        <div class="row compact">
          <button class="secondary" @click="ctx.enableNotifications">开启通知</button>
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
            <small v-if="item.last_error" class="danger-text">{{ item.last_error }}</small>
          </div>
        </div>
        <div v-else class="empty">没有待发送内容</div>
        <div class="row compact">
          <button class="secondary" @click="ctx.clearSentOutbox">清理已发送</button>
        </div>
      </section>

      <section class="home-card">
        <h3>账号与高级</h3>
        <div class="settings-rows">
          <button class="settings-row" @click="ctx.goDiagnosticsPage">
            <span>诊断工具</span><span class="chevron">›</span>
          </button>
          <button class="settings-row" @click="ctx.clearBrowserCaches">
            <span>清理浏览器缓存</span><span class="chevron">›</span>
          </button>
          <button class="settings-row danger-row" @click="ctx.logout">
            <span>退出登录</span><span class="chevron">›</span>
          </button>
        </div>
      </section>
    </div>
  </div>
</template>
