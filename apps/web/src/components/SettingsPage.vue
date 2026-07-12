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
          <button @click="ctx.showQr(ctx.myContactCardText.value, '我的 Contact Card')">卡片二维码</button>
          <button @click="ctx.showQr(ctx.backupText.value, '身份文件')">身份文件</button>
        </div>
      </section>

      <section class="home-card">
        <h3>消息同步</h3>
        <label>同步节点</label>
        <textarea v-model="ctx.nodeControlUrl.value" rows="4" placeholder="每行一个节点地址，例如：&#10;http://127.0.0.1:8787&#10;https://node.example.com" />
        <small>可填写多个，发送或收取消息时会自动尝试可用节点。</small>
        <div class="row compact">
          <button @click="ctx.toggleNodeEnabled">{{ ctx.nodeEnabled.value ? '关闭同步' : '开启同步' }}</button>
          <button @click="ctx.saveNetworkSettings">保存</button>
          <button @click="ctx.takeMailboxFromNode">收取消息</button>
        </div>
      </section>

      <section class="home-card">
        <h3>账号</h3>
        <button class="secondary" @click="ctx.logout">退出登录</button>
      </section>

      <section class="home-card">
        <h3>高级</h3>
        <button @click="ctx.goDebugPage">调试页面</button>
      </section>
    </div>
  </section>
</template>
