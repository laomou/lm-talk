import { createRouter, createWebHashHistory } from 'vue-router'

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
    { path: '/debug', component: { template: '<div />' } },
    { path: '/:pathMatch(.*)*', redirect: '/login' },
  ],
})
