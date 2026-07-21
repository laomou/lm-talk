const CACHE_NAME = 'lm-talk-web-shell-v1'
const STATIC_EXTENSIONS = /\.(?:js|css|wasm|svg|png|jpg|jpeg|webp|ico|json|webmanifest)$/i

function scopeUrl(path) {
  return new URL(path, self.registration.scope).toString()
}

async function cacheShell() {
  const cache = await caches.open(CACHE_NAME)
  await cache.addAll([
    scopeUrl('./'),
    scopeUrl('manifest.webmanifest'),
    scopeUrl('icons/lm-talk-icon.svg'),
  ])
}

self.addEventListener('install', (event) => {
  // A new shell waits for explicit user action from the page before taking
  // control. Reloading automatically could discard a draft or an import.
  event.waitUntil(cacheShell())
})

self.addEventListener('message', (event) => {
  if (event.data?.type === 'LM_TALK_SKIP_WAITING') self.skipWaiting()
})

self.addEventListener('activate', (event) => {
  event.waitUntil((async () => {
    const keys = await caches.keys()
    await Promise.all(keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)))
    await self.clients.claim()
  })())
})

async function networkFirst(request) {
  const cache = await caches.open(CACHE_NAME)
  try {
    const response = await fetch(request)
    if (response && response.ok) cache.put(request, response.clone())
    return response
  } catch {
    return (await cache.match(request)) || cache.match(scopeUrl('./'))
  }
}

async function cacheFirst(request) {
  const cache = await caches.open(CACHE_NAME)
  const cached = await cache.match(request)
  if (cached) return cached
  const response = await fetch(request)
  if (response && response.ok) cache.put(request, response.clone())
  return response
}

self.addEventListener('fetch', (event) => {
  const request = event.request
  if (request.method !== 'GET') return
  const url = new URL(request.url)
  if (url.origin !== self.location.origin) return
  // 安全边界：Service Worker 只缓存 Web 静态壳，不代理 lm_node/API、Mailbox、DHT、同步或明文内容。
  if (url.pathname.includes('/api/') || url.pathname.includes('/mailbox/') || url.pathname.includes('/dht/') || url.pathname.includes('/sync/')) return
  if (request.mode === 'navigate') {
    event.respondWith(networkFirst(request))
    return
  }
  if (STATIC_EXTENSIONS.test(url.pathname)) {
    event.respondWith(cacheFirst(request))
  }
})
