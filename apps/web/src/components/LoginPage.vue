<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
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
  verifyBackup: []
  clear: []
  removeIdentity: [id: string]
}>()

const router = useRouter()
const { t } = useI18n()
const hasLocalIdentity = computed(() => props.localIdentities.length > 0)
const passphraseInput = ref<HTMLTextAreaElement | null>(null)

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

function focusPassphrase() {
  void nextTick(() => {
    passphraseInput.value?.focus()
    passphraseInput.value?.select()
  })
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

defineExpose({ focusPassphrase })

</script>

<template>
  <main v-if="!ready" class="login-page auth-page">
    <UiCard class="auth-card auth-loading-card"><UiEmptyState :title="t('auth.preparingTitle')" :description="t('auth.preparingDescription')" /></UiCard>
  </main>

  <main v-else-if="!loggedIn" class="login-page auth-page">
    <UiCard class="auth-card">
      <UiAuthHeader
        :title="props.mode === 'login' ? t('auth.login') : props.mode === 'register' ? t('auth.registerTitle') : t('auth.importTitle')"
        :description="props.mode === 'login' ? t('auth.loginDescription') : props.mode === 'register' ? t('auth.registerDescription') : t('auth.importDescription')"
      />


      <section v-if="props.mode === 'login'" class="auth-panel">
        <UiField :label="t('auth.passphrase')" for-id="login-passphrase">
          <textarea ref="passphraseInput" id="login-passphrase" v-model="passphrase" rows="2" :aria-label="t('auth.loginPassphrase')" :placeholder="t('auth.enterPassphrase')" autofocus />
        </UiField>

        <p class="auth-field-label">{{ t('auth.selectIdentity') }}</p>
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
        <UiEmptyState v-else :title="t('auth.noLocalIdentityTitle')" :description="t('auth.noLocalIdentityDescription')" />

        <UiActionGroup class="auth-actions" full-width>
          <button :disabled="!hasLocalIdentity" @click="login">{{ t('auth.login') }}</button>
        </UiActionGroup>
        <p class="auth-switch">
          {{ t('auth.noIdentity') }} <button class="link-button" @click="goRegister">{{ t('auth.registerLink') }}</button>，<button class="link-button" @click="goImport">{{ t('auth.importLink') }}</button>
        </p>
      </section>

      <section v-else-if="props.mode === 'register'" class="auth-panel register-page">
        <div v-if="registeredIdentity" class="auth-success-flow">
          <header class="auth-success-header">
            <span class="auth-success-mark">✓</span>
            <div><h2>{{ t('auth.identityCreated') }}</h2><p>{{ t('auth.identityCreatedDescription') }}</p></div>
          </header>
          <p class="auth-identity-summary">{{ registeredIdentity.display_name }} · {{ registeredIdentity.user_id }}</p>
          <ol class="onboarding-list">
            <li>下载身份文件。</li>
            <li>把提示词保存在密码管理器或离线安全位置。</li>
            <li>可选：点击“验证导入”确认备份可恢复。</li>
          </ol>
          <UiNotice compact>身份文件和提示词缺一不可；任意一项丢失都无法恢复这个身份。</UiNotice>
          <UiActionGroup align="center">
            <button @click="downloadRegisteredBackup">{{ t('auth.downloadIdentity') }}</button>
            <button class="secondary" @click="emit('verifyBackup')">{{ t('auth.verifyImport') }}</button>
            <button class="secondary" @click="goLogin">{{ t('auth.goLogin') }}</button>
          </UiActionGroup>
        </div>

        <template v-else>
          <UiField :label="t('auth.passphrase')" for-id="register-passphrase">
            <textarea id="register-passphrase" v-model="passphrase" rows="2" :aria-label="t('auth.registerPassphrase')" :placeholder="t('auth.setPassphrase')" />
          </UiField>
          <UiNotice compact>{{ t('auth.passphraseNotice') }}</UiNotice>
          <UiActionGroup class="auth-actions" full-width>
            <button @click="$emit('create')">{{ t('auth.register') }}</button>
          </UiActionGroup>
          <p class="auth-switch"><button class="link-button" @click="goLogin">{{ t('auth.backToLogin') }}</button></p>
        </template>
      </section>

      <section v-else class="auth-panel import-page">
        <UiField :label="t('auth.passphrase')" for-id="import-passphrase">
          <textarea id="import-passphrase" v-model="passphrase" rows="2" :aria-label="t('auth.importPassphrase')" :placeholder="t('auth.enterImportPassphrase')" />
        </UiField>
        <UiNotice compact>{{ t('auth.importNotice') }}</UiNotice>
        <UiField :label="t('auth.identityText')" for-id="import-backup-text">
          <textarea id="import-backup-text" v-model="backupText" rows="6" :aria-label="t('auth.importIdentityText')" :placeholder="t('auth.pasteIdentityText')" />
        </UiField>
        <UiActionGroup class="auth-actions" full-width>
          <button :disabled="!backupText.trim()" @click="emit('importIdentity')">{{ t('auth.import') }}</button>
        </UiActionGroup>
        <p class="auth-switch"><button class="link-button" @click="goLogin">{{ t('auth.backToLogin') }}</button></p>
      </section>
    </UiCard>
  </main>
</template>
