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
    { path: '/chat/search', component: { template: '<div />' } },
    { path: '/chat/search/messages', component: { template: '<div />' } },
    { path: '/contacts', component: { template: '<div />' } },
    { path: '/contacts/search', component: { template: '<div />' } },
    { path: '/me', component: { template: '<div />' } },
    { path: '/settings', redirect: '/me' },
    { path: '/diagnostics', component: { template: '<div />' } },
    { path: '/:pathMatch(.*)*', redirect: '/login' },
  ],
})

// 登录态只保存在内存中。用户在登录之外的页面下拉刷新时，不保留原页面，
// 统一回到登录页，避免注册/导入表单或已退出的应用页面被刷新后继续展示。
let isInitialNavigation = true
const isBrowserReload = typeof performance !== 'undefined'
  && performance.getEntriesByType('navigation').some((entry) => (entry as PerformanceNavigationTiming).type === 'reload')

router.beforeEach((to) => {
  if (isInitialNavigation) {
    isInitialNavigation = false
    if (isBrowserReload && to.path !== '/login') return '/login'
  }
})
