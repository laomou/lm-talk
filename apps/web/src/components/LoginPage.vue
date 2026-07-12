<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'

type LocalIdentityRecord = {
  id: string
  user_id: string
  display_name: string
  backup_text: string
  updated_at: number
}

const props = defineProps<{
  ready: boolean
  loggedIn: boolean
  normalized: string
  log: string[]
  localIdentities: LocalIdentityRecord[]
  registeredIdentity: LocalIdentityRecord | null
  mode: 'login' | 'register' | 'import'
}>()

const passphrase = defineModel<string>('passphrase', { required: true })
const backupText = defineModel<string>('backupText', { required: true })
const displayName = defineModel<string>('displayName', { required: true })
const selectedIdentityId = defineModel<string>('selectedIdentityId', { required: true })
const emit = defineEmits<{
  create: []
  login: []
  importIdentity: []
  clear: []
  resetRegister: []
  removeIdentity: [id: string]
}>()

const router = useRouter()
const hasLocalIdentity = computed(() => props.localIdentities.length > 0)

function goRegister() {
  void router.push('/register')
}

function goLogin() {
  void router.push('/login')
}

function goImport() {
  void router.push('/import')
}

function login() {
  emit('login')
}

function fallbackCopyText(value: string) {
  const textarea = document.createElement('textarea')
  textarea.value = value
  textarea.setAttribute('readonly', 'true')
  textarea.style.position = 'fixed'
  textarea.style.left = '-9999px'
  document.body.appendChild(textarea)
  textarea.select()
  document.execCommand('copy')
  document.body.removeChild(textarea)
}

async function copyRegisteredBackup() {
  if (!props.registeredIdentity) return
  if ((navigator.clipboard as Clipboard | undefined)?.writeText) await navigator.clipboard.writeText(props.registeredIdentity.backup_text)
  else fallbackCopyText(props.registeredIdentity.backup_text)
}

function downloadRegisteredBackup() {
  if (!props.registeredIdentity) return
  const blob = new Blob([props.registeredIdentity.backup_text], { type: 'text/plain;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${props.registeredIdentity.display_name || props.registeredIdentity.user_id}.lm-identity-backup.txt`
  a.click()
  URL.revokeObjectURL(url)
}

function resetRegister() {
  emit('resetRegister')
}
</script>

<template>
  <main v-if="!ready" class="login-page">
    <div class="login-card"><h1>LM Talk</h1><p>正在加载 WASM...</p></div>
  </main>

  <main v-else-if="!loggedIn" class="login-page auth-page">
    <section class="login-card auth-card">
      <header class="auth-header">
        <h1 v-if="props.mode === 'login'">登录 LM Talk</h1>
        <h1 v-else-if="props.mode === 'register'">注册 LM Talk</h1>
        <h1 v-else>导入身份</h1>
        <p v-if="props.mode === 'login'">选择本机保存的身份，输入提示词登录。</p>
        <p v-else-if="props.mode === 'register'">注册新身份只需要提示词，用户名可以进入聊天后再改。</p>
        <p v-else>粘贴导出的身份文本，输入对应提示词导入到本机。</p>
      </header>


      <section v-if="props.mode === 'login'" class="auth-panel">
        <label>提示词</label>
        <textarea v-model="passphrase" rows="2" placeholder="输入你的提示词" autofocus />

        <label>选择身份</label>
        <div v-if="hasLocalIdentity" class="identity-list">
          <div v-for="item in localIdentities" :key="item.id" class="identity-choice">
            <label class="identity-select">
              <input type="radio" :value="item.id" v-model="selectedIdentityId" />
              <span>
                <b>{{ item.display_name || '未命名' }}</b>
                <small>{{ item.user_id }}</small>
              </span>
            </label>
            <button class="identity-delete" title="删除本地身份" @click="emit('removeIdentity', item.id)">×</button>
          </div>
        </div>
        <div v-else class="empty auth-empty">
          本机还没有保存的身份。
        </div>

        <div class="row auth-actions">
          <button :disabled="!hasLocalIdentity" @click="login">登录</button>
        </div>
        <p class="auth-switch">
          还没有身份？<button class="link-button" @click="goRegister">注册</button>，<button class="link-button" @click="goImport">导入</button>
        </p>
      </section>

      <section v-else-if="props.mode === 'register'" class="auth-panel register-page">
        <div v-if="registeredIdentity" class="registered-result">
          <h2>注册成功</h2>
          <p>身份已保存在本机，也建议下载一份身份文件。</p>
          <small>{{ registeredIdentity.display_name }} · {{ registeredIdentity.user_id }}</small>
          <div class="row compact">
            <button @click="downloadRegisteredBackup">下载身份</button>
            <button class="secondary" @click="copyRegisteredBackup">复制身份</button>
            <button class="secondary" @click="goLogin">去登录</button>
            <button @click="resetRegister">返回注册</button>
          </div>
        </div>

        <template v-else>
          <label>提示词</label>
          <textarea v-model="passphrase" rows="2" placeholder="设置你的提示词" />
          <div class="row auth-actions">
            <button @click="$emit('create')">注册</button>
          </div>
          <p class="auth-switch">已有身份？<button class="link-button" @click="goLogin">返回登录</button></p>
        </template>
      </section>

      <section v-else class="auth-panel import-page">
        <label>提示词</label>
        <textarea v-model="passphrase" rows="2" placeholder="输入身份对应提示词" />
        <label>身份文本</label>
        <textarea v-model="backupText" rows="6" placeholder="粘贴导出的身份文本" />
        <div class="row auth-actions">
          <button :disabled="!backupText.trim()" @click="emit('importIdentity')">导入</button>
        </div>
        <p class="auth-switch">导入后请回到登录页登录。<button class="link-button" @click="goLogin">返回登录</button></p>
      </section>
    </section>
  </main>
</template>
