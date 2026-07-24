<script setup lang="ts">
import { computed, ref } from 'vue'
import { avatarColor } from '../avatarColor'
import UiPageHeader from './UiPageHeader.vue'
import UiListRow from './UiListRow.vue'
import UiStatusBadge from './UiStatusBadge.vue'
import UiIcon from './UiIcon.vue'
import UiEmptyState from './UiEmptyState.vue'
import UiCard from './UiCard.vue'
import UiField from './UiField.vue'
import UiSection from './UiSection.vue'
import UiActionGroup from './UiActionGroup.vue'
import UiListGroup from './UiListGroup.vue'
import UiNavRow from './UiNavRow.vue'
import QrScanner from './QrScanner.vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

const props = defineProps<{ ctx: any }>()
const keyword = ref('')
const route = useRoute()
const router = useRouter()
const { t } = useI18n()
type View = 'home' | 'friends' | 'search' | 'add' | 'detail' | 'group-invites'
const view = ref<View>('home')
const scannerOpen = ref(false)
const scannedCard = ref<{ text: string; displayName: string; userId: string } | null>(null)
const isSearchPage = computed(() => route.path === '/contacts/search')

const requestCount = computed(() => props.ctx.visibleFriendRequests.value.length)
const groupInviteCount = computed(() => props.ctx.groupInvites.value.length)
const contactQuery = computed(() => keyword.value.trim().toLowerCase())
const friendContacts = computed(() => props.ctx.contacts.value.filter((c: any) => c.state === 'Friend'))
const visibleContacts = computed(() => {
  const q = contactQuery.value
  if (!q) return friendContacts.value
  return friendContacts.value.filter((c: any) => `${c.display_name || ''} ${c.user_id || ''}`.toLowerCase().includes(q))
})
const groupedContacts = computed(() => {
  const groups = new Map<string, any[]>()
  for (const contact of visibleContacts.value) {
    const key = contactInitial(contact)
    const list = groups.get(key) || []
    list.push(contact)
    groups.set(key, list)
  }
  const sortedKeys = [...groups.keys()].sort((a, b) => {
    if (a === '#') return 1
    if (b === '#') return -1
    return a.localeCompare(b)
  })
  return sortedKeys.map((key) => ({ key, items: groups.get(key) || [] }))
})

function contactInitial(contact: any) {
  const name = `${contact.display_name || contact.user_id || ''}`.trim()
  const first = name[0]?.toUpperCase()
  return first && /[A-Z]/.test(first) ? first : '#'
}
function trustIconName(contact: any) {
  return props.ctx.contactAllKnownDevicesRevoked(contact) ? 'alert' : 'lock'
}
function trustTitle(contact: any) {
  return props.ctx.contactAllKnownDevicesRevoked(contact) ? t('securityStatus.abnormal') : t('securityStatus.normal')
}
function shortId(value?: string) {
  if (!value) return ''
  return value.length > 18 ? `${value.slice(0, 8)}…${value.slice(-6)}` : value
}
function openContact(userId: string) {
  props.ctx.selectContact(userId)
  view.value = 'detail'
}
function backHome() {
  view.value = 'home'
  void router.push('/contacts')
}
function addContact() {
  props.ctx.addContact()
  if (!props.ctx.addContactText.value.trim() && props.ctx.activePeerId.value) view.value = 'detail'
}
async function onQrScanned(value: string) {
  scannerOpen.value = false
  if (!value.startsWith('lm-contact-card-v1:')) {
    props.ctx.showAlert(t('contactsView.scanResultTitle'), t('contactsView.scanInvalidCard'), 'warning')
    return
  }
  try {
    const info = await props.ctx.inspectContactCardForAdd(value)
    scannedCard.value = {
      text: value,
      displayName: info.display_name || t('contactsView.unnamed'),
      userId: info.user_id,
    }
  } catch {
    props.ctx.showAlert(t('contactsView.scanResultTitle'), t('contactsView.scanInvalidCard'), 'warning')
  }
}
function confirmScannedCard() {
  if (!scannedCard.value) return
  props.ctx.addContactText.value = scannedCard.value.text
  scannedCard.value = null
  addContact()
}
</script>

<template>
  <div class="contacts-shell product-contacts-shell">
    <main class="detail-col contacts-wide product-contacts-main">
      <section v-if="view === 'home' && !isSearchPage" class="detail-scroll">
        <header class="contacts-mobile-bar">
          <span></span>
          <h2>{{ t('contactsView.title') }}</h2>
          <div class="header-actions icon-actions">
            <button class="icon-btn" :aria-label="t('contactsView.searchContacts')" :title="t('contactsView.searchContacts')" @click="router.push('/contacts/search')"><UiIcon name="search" /></button>
            <button class="icon-btn" :aria-label="t('contactsView.addFriend')" :title="t('contactsView.addFriend')" @click="view = 'add'"><UiIcon name="add" /></button>
          </div>
        </header>

        <div class="contact-directory">
          <UiNavRow :icon="t('contactsView.newFriendsIcon')" :badge="requestCount || undefined" :aria-label="t('contactsView.openNewFriends')" @click="view = 'friends'">{{ t('contactsView.newFriends') }}</UiNavRow>
          <UiNavRow :icon="t('contactsView.groupInvitesIcon')" :badge="groupInviteCount || undefined" :aria-label="t('contactsView.openGroupInvites')" @click="view = 'group-invites'">{{ t('contactsView.groupInvites') }}</UiNavRow>

          <template v-for="group in groupedContacts" :key="group.key">
            <h3 class="alpha-heading">{{ group.key }}</h3>
            <button
              v-for="c in group.items"
              :key="c.user_id"
              class="directory-row contact-row"
              :class="{ active: c.user_id === ctx.activePeerId.value }"
              @click="openContact(c.user_id)"
            >
              <span class="avatar" :style="{ background: avatarColor(c.user_id) }">{{ (c.display_name || c.user_id || '?').slice(0, 1).toUpperCase() }}</span>
              <span class="directory-main">
                <b>{{ c.display_name || t('contactsView.unnamed') }}</b>
                <small>{{ shortId(c.user_id) }}</small>
              </span>
              <UiStatusBadge :tone="ctx.contactAllKnownDevicesRevoked(c) ? 'warning' : 'success'" :title="trustTitle(c)" :aria-label="trustTitle(c)"><UiIcon :name="trustIconName(c)" size="13" /></UiStatusBadge>
              <span class="chevron">›</span>
            </button>
          </template>

          <UiEmptyState v-if="groupedContacts.length === 0" :title="t('contactsView.noFriendsTitle')" :description="t('contactsView.noFriendsDescription')" />
        </div>
      </section>

      <section v-else-if="view === 'friends'" class="detail-scroll">
        <UiPageHeader :title="t('contactsView.newFriends')" :back-label="t('contactsView.backToContacts')" @back="backHome">
          <template #end><button class="secondary" @click="ctx.syncNow">{{ t('contactsView.sync') }}</button></template>
        </UiPageHeader>
        <div class="detail-body narrow friend-requests-body">
          <UiCard>
            <h3>{{ t('contactsView.friendRequests') }}</h3>
            <div v-if="ctx.visibleFriendRequests.value.length" class="request-grid">
              <UiCard v-for="req in ctx.visibleFriendRequests.value" :key="req.request_id" variant="inset">
                <b>{{ req.from_user_id }}</b>
                <small>{{ req.note || t('contactsView.requestNoteFallback') }}</small>
                <UiActionGroup>
                  <button @click="ctx.acceptInboxRequest(req)">{{ t('contactsView.accept') }}</button>
                  <button class="secondary danger" @click="ctx.rejectInboxRequest(req)">{{ t('contactsView.reject') }}</button>
                </UiActionGroup>
              </UiCard>
            </div>
            <UiEmptyState v-else :title="t('contactsView.noFriendRequestsTitle')" :description="t('contactsView.noFriendRequestsDescription')" />
          </UiCard>

          <UiSection v-if="ctx.quarantinedFriendRequests.value.length" :title="t('contactsView.quarantinedRequests')">
            <template #actions><button class="secondary danger" @click="ctx.clearQuarantinedFriendRequests">{{ t('contactsView.clear') }}</button></template>
            <div class="request-grid">
              <UiCard v-for="req in ctx.quarantinedFriendRequests.value" :key="req.request_id" variant="inset">
                <b>{{ req.from_user_id }}</b>
                <small>{{ req.quarantine_reason || t('contactsView.localRuleQuarantined') }}</small>
                <UiActionGroup>
                  <button class="secondary" @click="ctx.restoreQuarantinedFriendRequest(req)">{{ t('contactsView.restore') }}</button>
                  <button class="secondary danger" @click="ctx.rejectInboxRequest(req)">{{ t('contactsView.reject') }}</button>
                </UiActionGroup>
              </UiCard>
            </div>
          </UiSection>
        </div>
      </section>

      <section v-else-if="isSearchPage" class="detail-scroll">
        <UiPageHeader :back-label="t('contactsView.backToContacts')" @back="backHome">
          <template #title><input v-model="keyword" class="subbar-search" type="search" :aria-label="t('contactsView.searchContacts')" :placeholder="t('contactsView.searchContacts')" autofocus /></template>
        </UiPageHeader>
        <div class="contact-directory search-directory">
          <h3 class="alpha-heading">{{ t('contactsView.contacts') }}</h3>
          <button v-for="c in visibleContacts" :key="c.user_id" class="directory-row contact-row" @click="openContact(c.user_id)">
            <span class="avatar" :style="{ background: avatarColor(c.user_id) }">{{ (c.display_name || c.user_id || '?').slice(0, 1).toUpperCase() }}</span>
            <span class="directory-main"><b>{{ c.display_name || t('contactsView.unnamed') }}</b><small>{{ shortId(c.user_id) }}</small></span>
            <UiStatusBadge :tone="ctx.contactAllKnownDevicesRevoked(c) ? 'warning' : 'success'" :title="trustTitle(c)" :aria-label="trustTitle(c)"><UiIcon :name="trustIconName(c)" size="13" /></UiStatusBadge>
            <span class="chevron">›</span>
          </button>
          <UiEmptyState v-if="visibleContacts.length === 0" icon="search" :title="t('contactsView.noContactMatchesTitle')" :description="t('contactsView.noContactMatchesDescription')" />
        </div>
      </section>

      <section v-else-if="view === 'add'" class="detail-scroll">
        <UiPageHeader :title="t('contactsView.add')" :back-label="t('contactsView.backToContacts')" @back="backHome" />
        <div class="detail-body narrow add-page-body">
          <UiSection :title="t('contactsView.pasteCardAddFriend')">
            <UiField :label="t('contactsView.contactCard')" for-id="contact-card-input">
              <textarea id="contact-card-input" v-model="ctx.addContactText.value" rows="7" :aria-label="t('contactsView.contactCard')" :placeholder="t('contactsView.pasteContactCard')" />
            </UiField>
            <UiActionGroup><button @click="addContact">{{ t('contactsView.addFriend') }}</button></UiActionGroup>
          </UiSection>
          <UiListRow class="mobile-only-row" :aria-label="t('contactsView.scanAdd')" @click="scannerOpen = true">{{ t('contactsView.scanAdd') }}</UiListRow>
          <UiCard v-if="scannedCard" class="scanned-contact-preview">
            <small>{{ t('contactsView.scanResultTitle') }}</small>
            <b>{{ scannedCard.displayName }}</b>
            <small>{{ shortId(scannedCard.userId) }}</small>
            <UiActionGroup>
              <button @click="confirmScannedCard">{{ t('contactsView.addFriend') }}</button>
              <button class="secondary" @click="scannedCard = null">{{ t('common.cancel') }}</button>
            </UiActionGroup>
          </UiCard>
        </div>
      </section>

      <section v-else-if="view === 'group-invites'" class="detail-scroll">
        <UiPageHeader :title="t('contactsView.groupInvites')" :back-label="t('contactsView.backToContacts')" @back="backHome" />
        <div class="detail-body narrow">
          <div v-if="ctx.groupInvites.value.length" class="request-grid">
            <UiCard v-for="invite in ctx.groupInvites.value" :key="invite.invite_id" variant="inset">
              <b>{{ invite.group_name || t('contactsView.groupNameFallback') }}</b>
              <small>{{ t('contactsView.inviteFrom', { name: invite.inviter_display_name || invite.inviter_user_id || '?' }) }}</small>
              <UiActionGroup>
                <button @click="ctx.acceptGroupInvite(invite)">{{ t('contactsView.joinGroup') }}</button>
                <button class="secondary danger" @click="ctx.ignoreGroupInvite(invite)">{{ t('contactsView.ignoreInvite') }}</button>
              </UiActionGroup>
            </UiCard>
          </div>
          <UiEmptyState v-else :title="t('contactsView.noGroupInvitesTitle')" :description="t('contactsView.noGroupInvitesDescription')" />
        </div>
      </section>

      <section v-else-if="view === 'detail' && ctx.activeContact.value" class="detail-scroll">
        <UiPageHeader :title="t('contactsView.contactDetail')" :back-label="t('contactsView.backToContacts')" @back="backHome" />
        <div class="detail-hero product-contact-hero contact-detail-centered">
          <span class="avatar large" :style="{ background: avatarColor(ctx.activeContact.value.user_id) }">{{ (ctx.activeContact.value.display_name || ctx.activeContact.value.user_id || '?').slice(0, 1).toUpperCase() }}</span>
          <div class="detail-hero-text">
            <h2>{{ ctx.activeContact.value.display_name || t('contactsView.unnamed') }}</h2>
            <UiStatusBadge v-if="ctx.activeContact.value.state === 'Friend'" :tone="ctx.contactAllKnownDevicesRevoked(ctx.activeContact.value) ? 'warning' : 'success'" :title="trustTitle(ctx.activeContact.value)" :aria-label="trustTitle(ctx.activeContact.value)"><UiIcon :name="trustIconName(ctx.activeContact.value)" size="13" /></UiStatusBadge>
            <small>{{ shortId(ctx.activeContact.value.user_id) }}</small>
          </div>
        </div>
        <div class="detail-body narrow contact-detail-centered">
          <button class="primary-action" @click="ctx.selectContact(ctx.activeContact.value.user_id); ctx.goChatPage()">{{ t('contactsView.sendMessage') }}</button>
          <UiSection v-if="ctx.activeContact.value.state === 'Friend'" :title="t('contactsView.securityStatus')">
            <template #actions><UiStatusBadge :tone="ctx.contactAllKnownDevicesRevoked(ctx.activeContact.value) ? 'warning' : 'success'" :title="trustTitle(ctx.activeContact.value)" :aria-label="trustTitle(ctx.activeContact.value)"><UiIcon :name="trustIconName(ctx.activeContact.value)" size="13" /></UiStatusBadge></template>
            <small v-if="!ctx.contactAllKnownDevicesRevoked(ctx.activeContact.value)">{{ t('contactsView.secureSessionEstablished') }}</small>
            <small v-if="ctx.contactRevokedDeviceCount(ctx.activeContact.value)" class="danger-text">{{ t('contactsView.revokedDevices', { count: ctx.contactRevokedDeviceCount(ctx.activeContact.value) }) }}</small>
          </UiSection>
          <UiSection v-if="ctx.activeContact.value.state === 'Friend'" :title="t('contactsView.readReceipts')">
            <small>{{ t('contactsView.readReceiptsDescription') }}</small>
            <UiActionGroup>
              <button
                class="secondary"
                :aria-pressed="ctx.readReceiptsEnabledFor(ctx.activeContact.value) ? 'true' : 'false'"
                @click="ctx.setActiveContactReadReceipts(ctx.readReceiptsEnabledFor(ctx.activeContact.value) ? 'disabled' : 'enabled')"
              >{{ ctx.readReceiptsEnabledFor(ctx.activeContact.value) ? t('contactsView.readReceiptsEnabled') : t('contactsView.enableReadReceipts') }}</button>
            </UiActionGroup>
          </UiSection>
          <UiCard>
            <UiListGroup>
              <UiListRow @click="ctx.showQr(ctx.activeContact.value.contact_card_text, t('contactsView.friendIdentity'))">{{ t('contactsView.viewCard') }}</UiListRow>
              <UiListRow v-if="ctx.activeContact.value.state !== 'Blocked'" @click="ctx.blockActiveContact">{{ t('contactsView.block') }}</UiListRow>
              <UiListRow v-else @click="ctx.unblockActiveContact">{{ t('contactsView.unblock') }}</UiListRow>
              <UiListRow danger @click="ctx.removeActiveContact">{{ t('contactsView.deleteFriend') }}</UiListRow>
            </UiListGroup>
          </UiCard>
        </div>
      </section>
    </main>
    <QrScanner v-if="scannerOpen" @scanned="onQrScanned" @close="scannerOpen = false" />
  </div>
</template>
