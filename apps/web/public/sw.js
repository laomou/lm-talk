const CACHE_NAME = 'lm-talk-pwa-v1'
const BASE_PATH = new URL(self.registration.scope).pathname
const appUrl = (path) => new URL(path.replace(/^\//, ''), self.registration.scope).toString()
const APP_SHELL = [appUrl(''), appUrl('manifest.webmanifest')]

self.addEventListener('install', (event) => {
  event.waitUntil(caches.open(CACHE_NAME).then((cache) => cache.addAll(APP_SHELL)).then(() => self.skipWaiting()))
})

self.addEventListener('activate', (event) => {
  event.waitUntil(caches.keys().then((keys) => Promise.all(keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)))).then(() => self.clients.claim()))
})

self.addEventListener('fetch', (event) => {
  if (event.request.method !== 'GET') return
  const url = new URL(event.request.url)
  if (url.origin !== self.location.origin || !url.pathname.startsWith(BASE_PATH)) return
  // Network-first: always try the network so freshly rebuilt assets (e.g. the
  // wasm module) are picked up immediately; fall back to cache only when offline.
  event.respondWith(fetch(event.request).then((response) => {
    if (response && response.ok) {
      const copy = response.clone()
      caches.open(CACHE_NAME).then((cache) => cache.put(event.request, copy))
    }
    return response
  }).catch(() => caches.match(event.request).then((cached) => cached || caches.match(appUrl('')))))
})
