<script setup lang="ts">
defineProps<{ ctx: any }>()
</script>

<template>
  <section class="simple-page settings-page">
    <header class="simple-header">
      <h1>我</h1>
    </header>

    <div class="simple-grid">
      <section class="home-card identity-card">
        <h3>我的资料</h3>
        <label>显示名</label>
        <input v-model="ctx.displayName.value" @change="ctx.refreshMyContactCard" />
        <div class="row compact">
          <button @click="ctx.refreshMyContactCard">保存</button>
          <button @click="ctx.showQr(ctx.myContactCardText.value, '我的名片')">我的名片</button>
          <button @click="ctx.showQr(ctx.backupText.value, '导出身份')">导出身份</button>
        </div>
      </section>

      <section class="home-card sync-card">
        <div class="section-title-row"><h3>消息同步</h3><span class="sync-pill" :class="{ on: ctx.nodeEnabled.value }">{{ ctx.nodeEnabled.value ? '已开启' : '未开启' }}</span></div>
        <label>同步服务</label>
        <textarea v-model="ctx.nodeControlUrl.value" rows="4" placeholder="每行一个同步服务地址，例如：&#10;http://127.0.0.1:8787&#10;https://node.example.com" />
        <small>开启后可自动收发好友请求和离线消息。可填写多个同步服务地址。</small>
        <div class="row compact">
          <button @click="ctx.toggleNodeEnabled">{{ ctx.nodeEnabled.value ? '关闭同步' : '开启同步' }}</button>
          <button @click="ctx.saveNetworkSettings">保存</button>
          <button @click="ctx.syncNow">立即同步</button>
        </div>
        <div class="sync-status">
          <b>同步状态</b>
          <small>{{ ctx.nodeControlStatus.value || '未连接' }}</small>
        </div>
        <div class="sync-log">
          <b>最近记录</b>
          <div v-if="ctx.log.value.length" class="sync-log-lines">
            <small v-for="line in ctx.log.value.slice(0, 6)" :key="line">{{ line }}</small>
          </div>
          <small v-else>暂无同步记录</small>
        </div>
      </section>

      <section class="home-card">
        <h3>账号</h3>
        <button class="secondary" @click="ctx.logout">退出登录</button>
      </section>

      <section class="home-card">
        <h3>高级</h3>
        <button @click="ctx.goDebugPage">诊断工具</button>
      </section>
    </div>
  </section>
</template>
