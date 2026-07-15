const BUILD_REF = new URL(self.location.href).searchParams.get('v') || 'dev'
const CACHE_NAME = `lm-talk-pwa-${BUILD_REF}`
const BASE_PATH = new URL(self.registration.scope).pathname
const BACKGROUND_SYNC_TAG = 'lm-talk-mailbox-sync'
const PERIODIC_SYNC_TAG = 'lm-talk-periodic-mailbox-sync'
const appUrl = (path) => new URL(path.replace(/^\//, ''), self.registration.scope).toString()
const APP_SHELL = [appUrl(''), appUrl('manifest.webmanifest')]
const STATIC_EXTENSIONS = /\.(?:js|css|wasm|json|webmanifest|png|jpg|jpeg|svg|ico|woff2?)$/i

function isStaticAsset(request, url) {
  return request.destination === 'script'
    || request.destination === 'style'
    || request.destination === 'worker'
    || request.destination === 'manifest'
    || request.destination === 'font'
    || request.destination === 'image'
    || STATIC_EXTENSIONS.test(url.pathname)
}

async function cacheResponse(request, response) {
  if (!response || !response.ok) return
  const cache = await caches.open(CACHE_NAME)
  await cache.put(request, response.clone())
}

async function notifyBackgroundSync(tag) {
  const clientsList = await self.clients.matchAll({ type: 'window', includeUncontrolled: true })
  for (const client of clientsList) {
    client.postMessage({ type: 'lm-talk-background-sync', tag, at: Date.now() })
  }
  if (self.registration.showNotification && self.Notification?.permission === 'granted') {
    await self.registration.showNotification('LM Talk 可同步新消息', {
      body: '为保护端到端加密数据，后台不会读取本地密钥。请打开应用完成 Mailbox 同步。',
      tag: 'lm-talk-background-sync',
      data: { url: appUrl('#/contacts') },
    })
  }
}

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
  if (isStaticAsset(event.request, url)) {
    event.respondWith(caches.match(event.request).then((cached) => {
      if (cached) return cached
      return fetch(event.request).then((response) => {
        event.waitUntil(cacheResponse(event.request, response))
        return response
      })
    }))
    return
  }
  event.respondWith(fetch(event.request).then((response) => {
    event.waitUntil(cacheResponse(event.request, response))
    return response
  }).catch(() => caches.match(event.request).then((cached) => cached || caches.match(appUrl('')))))
})

self.addEventListener('sync', (event) => {
  if (event.tag !== BACKGROUND_SYNC_TAG) return
  event.waitUntil(notifyBackgroundSync(event.tag))
})

self.addEventListener('periodicsync', (event) => {
  if (event.tag !== PERIODIC_SYNC_TAG) return
  event.waitUntil(notifyBackgroundSync(event.tag))
})

self.addEventListener('push', (event) => {
  event.waitUntil(notifyBackgroundSync('push'))
})

self.addEventListener('notificationclick', (event) => {
  event.notification.close()
  const targetUrl = event.notification.data?.url || appUrl('#/contacts')
  event.waitUntil((async () => {
    const clientsList = await self.clients.matchAll({ type: 'window', includeUncontrolled: true })
    for (const client of clientsList) {
      if ('focus' in client) return client.focus()
    }
    if (self.clients.openWindow) return self.clients.openWindow(targetUrl)
  })())
})
