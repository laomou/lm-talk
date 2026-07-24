import { createRouter, createWebHashHistory } from 'vue-router'

function normalizeInitialHashRoute() {
  if (typeof window === 'undefined') return
  const hash = window.location.hash
  // Accept product links like #login as well as Vue Router links like #/login.
  if (/^#[^/!]/.test(hash)) {
    window.history.replaceState(null, '', `${window.location.pathname}${window.location.search}#/${hash.slice(1)}`)
  }
}

normalizeInitialHashRoute()

export const router = createRouter({
  history: createWebHashHistory(import.meta.env.BASE_URL),
  routes: [
    { path: '/', redirect: '/login' },
    { path: '/login', component: { template: '<div />' } },
    { path: '/register', component: { template: '<div />' } },
    { path: '/import', component: { template: '<div />' } },
    { path: '/chat', component: { template: '<div />' } },
    { path: '/chat/:userId', component: { template: '<div />' } },
    { path: '/chat/search', component: { template: '<div />' } },
    { path: '/chat/search/messages', component: { template: '<div />' } },
    { path: '/contacts', component: { template: '<div />' } },
    { path: '/contacts/new-friends', component: { template: '<div />' } },
    { path: '/contacts/group-invites', component: { template: '<div />' } },
    { path: '/contacts/add', component: { template: '<div />' } },
    { path: '/contacts/:userId', component: { template: '<div />' } },
    { path: '/contacts/search', component: { template: '<div />' } },
    { path: '/me', component: { template: '<div />' } },
    { path: '/me/profile', component: { template: '<div />' } },
    { path: '/me/backup', component: { template: '<div />' } },
    { path: '/me/security', component: { template: '<div />' } },
    { path: '/me/sync', component: { template: '<div />' } },
    { path: '/me/preferences', component: { template: '<div />' } },
    { path: '/me/about', component: { template: '<div />' } },
    { path: '/settings', redirect: '/me' },
    { path: '/diagnostics', component: { template: '<div />' } },
    { path: '/:pathMatch(.*)*', redirect: '/login' },
  ],
})
