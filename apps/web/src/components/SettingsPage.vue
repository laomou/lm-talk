<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{ ctx: any }>()
const showSyncServiceEditor = ref(false)
const showDataBackupEditor = ref(false)
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
      </section>

      <section class="home-card sync-card">
        <div class="section-title-row">
          <h3>PWA 应用</h3>
          <button class="secondary" @click="ctx.refreshPwaStatus">刷新状态</button>
        </div>
        <small>{{ ctx.pwaStatusText.value }}</small>
        <small>安全边界：PWA 只缓存静态应用壳；不会在 Service Worker 中保存身份密钥、解密消息、后台同步 Mailbox 或发送 Outbox。</small>
      </section>

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

      <section class="home-card">
        <div class="section-title-row">
          <h3>完整数据备份</h3>
          <button class="secondary" @click="showDataBackupEditor = !showDataBackupEditor">{{ showDataBackupEditor ? '隐藏' : '显示备份' }}</button>
        </div>
        <small>加密导出本机联系人、群聊、消息和设置；可用于换设备恢复。</small>
        <div class="row compact">
          <button class="secondary" @click="ctx.exportFullDataBackup">生成备份</button>
          <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.downloadText(ctx.dataBackupText.value, 'lm-talk-data-backup.txt')">下载备份</button>
          <button class="secondary" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackupMerge">导入合并</button>
          <button class="secondary danger" :disabled="!ctx.dataBackupText.value.trim()" @click="ctx.importFullDataBackup">导入覆盖</button>
        </div>
        <textarea
          v-if="showDataBackupEditor"
          v-model="ctx.dataBackupText.value"
          class="mono"
          rows="5"
          aria-label="完整数据备份文本"
          placeholder="点击生成备份，或粘贴 lm-data-backup-v1 文本后导入"
        />
      </section>

      <section class="home-card">
        <h3>账号</h3>
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

      <footer class="app-version" aria-label="应用版本信息">· {{ ctx.webVersionText }}</footer>
    </div>
  </div>
</template>
