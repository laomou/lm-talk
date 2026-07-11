<script setup lang="ts">
defineProps<{
  ready: boolean
  loggedIn: boolean
  normalized: string
  log: string[]
}>()

const passphrase = defineModel<string>('passphrase', { required: true })
const backupText = defineModel<string>('backupText', { required: true })
const displayName = defineModel<string>('displayName', { required: true })
defineEmits<{
  create: []
  restore: []
  clear: []
}>()
</script>

<template>
  <main v-if="!ready" class="login-page">
    <div class="login-card"><h1>LM Talk</h1><p>正在加载 WASM...</p></div>
  </main>

  <main v-else-if="!loggedIn" class="login-page">
    <section class="login-card">
      <h1>LM Talk</h1>
      <p>输入提示词。新身份会生成身份备份包；已有身份需要粘贴身份备份包恢复。</p>
      <p class="warning">原型提示：已支持实验性 Double Ratchet；没有建立 Ratchet 会话时仍回退 MVP 加密。请勿用于真实敏感通信。</p>
      <label>提示词</label>
      <textarea v-model="passphrase" rows="2" placeholder="输入你的提示词" />
      <small>归一化：{{ normalized }}</small>
      <label>身份备份包（恢复已有身份时粘贴）</label>
      <textarea v-model="backupText" rows="6" placeholder="lm-identity-backup-v1:..." />
      <label>我的显示名</label>
      <input v-model="displayName" />
      <div class="row">
        <button @click="$emit('create')">创建新身份</button>
        <button @click="$emit('restore')">恢复并进入</button>
        <button class="danger" @click="$emit('clear')">清空本地</button>
      </div>
      <div class="log"><div v-for="(line, i) in log" :key="i">{{ line }}</div></div>
    </section>
  </main>
</template>
