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
    { path: '/contacts', component: { template: '<div />' } },
    { path: '/me', component: { template: '<div />' } },
    { path: '/settings', redirect: '/me' },
    { path: '/diagnostics', component: { template: '<div />' } },
    { path: '/:pathMatch(.*)*', redirect: '/login' },
  ],
})
