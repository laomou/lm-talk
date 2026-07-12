import { createRouter, createWebHashHistory } from 'vue-router'

export const router = createRouter({
  history: createWebHashHistory(import.meta.env.BASE_URL),
  routes: [
    { path: '/', redirect: '/login' },
    { path: '/login', component: { template: '<div />' } },
    { path: '/register', component: { template: '<div />' } },
    { path: '/import', component: { template: '<div />' } },
    { path: '/chat', component: { template: '<div />' } },
    { path: '/debug', component: { template: '<div />' } },
    { path: '/:pathMatch(.*)*', redirect: '/login' },
  ],
})
