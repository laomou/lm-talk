export type PwaStatus = {
  supported: boolean
  standalone: boolean
  online: boolean
  serviceWorker: 'unsupported' | 'registering' | 'active' | 'inactive' | 'error'
  cacheCount?: number
  message: string
}

export async function registerPwa(): Promise<PwaStatus> {
  if (!('serviceWorker' in navigator)) return pwaStatus('unsupported')
  try {
    const base = import.meta.env.BASE_URL || '/'
    await navigator.serviceWorker.register(`${base}sw.js`, { scope: base })
    return readPwaStatus()
  } catch {
    return pwaStatus('error')
  }
}

export async function readPwaStatus(): Promise<PwaStatus> {
  if (!('serviceWorker' in navigator)) return pwaStatus('unsupported')
  const registration = await navigator.serviceWorker.getRegistration().catch(() => undefined)
  const state = registration?.active ? 'active' : registration ? 'registering' : 'inactive'
  let cacheCount: number | undefined
  if ('caches' in window) cacheCount = (await caches.keys().catch(() => [])).length
  return pwaStatus(state, cacheCount)
}

function pwaStatus(serviceWorker: PwaStatus['serviceWorker'], cacheCount?: number): PwaStatus {
  const standalone = window.matchMedia?.('(display-mode: standalone)').matches || Boolean((navigator as any).standalone)
  const supported = serviceWorker !== 'unsupported'
  const online = navigator.onLine
  const message = supported
    ? `PWA：${standalone ? '独立窗口' : '浏览器标签'} · ${online ? '在线' : '离线'} · Service Worker ${serviceWorker === 'active' ? '已启用' : serviceWorker === 'registering' ? '注册中' : serviceWorker === 'inactive' ? '未激活' : '异常'}${cacheCount !== undefined ? ` · 缓存 ${cacheCount} 组` : ''}；仅缓存静态应用壳，不在后台处理身份密钥、消息明文或同步。`
    : `PWA：当前浏览器不支持 Service Worker；不会后台处理身份密钥、消息明文或同步。`
  return { supported, standalone, online, serviceWorker, cacheCount, message }
}
