<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import UiNotice from './UiNotice.vue'
import UiField from './UiField.vue'
import UiActionGroup from './UiActionGroup.vue'
import UiCard from './UiCard.vue'
import UiAuthHeader from './UiAuthHeader.vue'
import UiIdentityChoice from './UiIdentityChoice.vue'
import UiEmptyState from './UiEmptyState.vue'

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

</script>

<template>
  <main v-if="!ready" class="login-page auth-page">
    <UiCard class="auth-card auth-loading-card"><UiEmptyState title="正在准备 LM Talk" description="正在加载安全组件…" /></UiCard>
  </main>

  <main v-else-if="!loggedIn" class="login-page auth-page">
    <UiCard class="auth-card">
      <UiAuthHeader
        :title="props.mode === 'login' ? '登录' : props.mode === 'register' ? '创建身份' : '导入身份'"
        :description="props.mode === 'login' ? '选择本机身份并输入提示词继续。' : props.mode === 'register' ? '创建身份后，请立即完成备份。' : '粘贴身份文本并输入对应提示词。'"
      />


      <section v-if="props.mode === 'login'" class="auth-panel">
        <UiField label="提示词" for-id="login-passphrase">
          <textarea id="login-passphrase" v-model="passphrase" rows="2" aria-label="登录提示词" placeholder="输入你的提示词" autofocus />
        </UiField>

        <p class="auth-field-label">选择身份</p>
        <div v-if="hasLocalIdentity" class="identity-list">
          <UiIdentityChoice
            v-for="item in localIdentities"
            :key="item.id"
            :id="item.id"
            :name="item.display_name"
            :user-id="item.user_id"
            :selected="selectedIdentityId === item.id"
            @select="selectedIdentityId = $event"
            @request-delete="emit('removeIdentity', $event)"
          />
        </div>
        <UiEmptyState v-else title="还没有本机身份" description="注册新身份，或导入已有身份后再登录。" />

        <UiActionGroup class="auth-actions" full-width>
          <button :disabled="!hasLocalIdentity" @click="login">登录</button>
        </UiActionGroup>
        <p class="auth-switch">
          还没有身份？<button class="link-button" @click="goRegister">注册</button>，<button class="link-button" @click="goImport">导入</button>
        </p>
      </section>

      <section v-else-if="props.mode === 'register'" class="auth-panel register-page">
        <div v-if="registeredIdentity" class="auth-success-flow">
          <header class="auth-success-header">
            <span class="auth-success-mark">✓</span>
            <div><h2>身份已创建</h2><p>身份已保存在本机。请先完成备份，再返回登录。</p></div>
          </header>
          <p class="auth-identity-summary">{{ registeredIdentity.display_name }} · {{ registeredIdentity.user_id }}</p>
          <ol class="onboarding-list">
            <li>下载或复制身份文件。</li>
            <li>把提示词保存在密码管理器或离线安全位置。</li>
            <li>可选：点击“验证导入”确认备份可恢复。</li>
          </ol>
          <UiNotice>身份文件和提示词缺一不可；任意一项丢失都无法恢复这个身份。</UiNotice>
          <UiActionGroup align="center">
            <button @click="downloadRegisteredBackup">下载身份</button>
            <button class="secondary" @click="goLogin">去登录</button>
          </UiActionGroup>
        </div>

        <template v-else>
          <UiField label="提示词" for-id="register-passphrase">
            <textarea id="register-passphrase" v-model="passphrase" rows="2" aria-label="注册提示词" placeholder="设置你的提示词" />
          </UiField>
          <UiNotice class="auth-hint-notice">提示词不会上传或找回；注册后请下载身份文件。</UiNotice>
          <UiActionGroup class="auth-actions" full-width>
            <button @click="$emit('create')">注册</button>
          </UiActionGroup>
          <p class="auth-switch">已有身份？<button class="link-button" @click="goLogin">返回登录</button></p>
        </template>
      </section>

      <section v-else class="auth-panel import-page">
        <UiField label="提示词" for-id="import-passphrase">
          <textarea id="import-passphrase" v-model="passphrase" rows="2" aria-label="导入身份提示词" placeholder="输入身份对应提示词" />
        </UiField>
        <UiNotice>导入需要身份文本和对应提示词；提示词错误或丢失时无法恢复。</UiNotice>
        <UiField label="身份文本" for-id="import-backup-text">
          <textarea id="import-backup-text" v-model="backupText" rows="6" aria-label="导入身份文本" placeholder="粘贴导出的身份文本" />
        </UiField>
        <UiActionGroup class="auth-actions" full-width>
          <button :disabled="!backupText.trim()" @click="emit('importIdentity')">导入</button>
        </UiActionGroup>
        <p class="auth-switch">导入后请回到登录页登录。<button class="link-button" @click="goLogin">返回登录</button></p>
      </section>
    </UiCard>
  </main>
</template>
