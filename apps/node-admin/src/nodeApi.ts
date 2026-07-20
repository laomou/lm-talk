// Thin fetch client for the lm_node HTTP control-plane API.
// Mirrors the pattern of fetchNodeOnce() in apps/web/src/App.vue:
// URL join, bearer token header, AbortController timeout, text-then-JSON,
// throw NodeError on non-2xx. No cookies/credentials (node CORS does not
// send access-control-allow-credentials).

import type {
  ControlStatsResponse,
  HealthResponse,
  NodeConfig,
  SyncStatusResponse,
} from './types'

export class NodeError extends Error {
  status?: number
  constructor(message: string, status?: number) {
    super(message)
    this.name = 'NodeError'
    this.status = status
  }
}

function classifyNodeError(status: number | undefined, raw: string, endpoint: string): string {
  const suffix = endpoint ? `（${endpoint}）` : ''
  if (status === 401 || /unauthorized/i.test(raw)) return `节点鉴权失败${suffix}：请检查控制面令牌是否与 lm_node --control-token 一致。`
  if (status === 403 || /cors origin not allowed|forbidden/i.test(raw)) return `节点拒绝访问${suffix}：请检查 CORS 白名单、访问来源或 /admin/ loopback 限制。`
  if (status === 429 || /rate limit|too many requests/i.test(raw)) return `节点限流${suffix}：请求过于频繁，请稍后重试。`
  if (status === 413 || /too large|payload too large/i.test(raw)) return `节点拒绝大载荷${suffix}：请求内容超过限制，请缩小快照或检查节点配额。`
  if (status === 404 || /not found/i.test(raw)) return `节点接口不存在${suffix}：请确认 lm_node 版本、/admin/ 挂载和 API 路径。`
  if (typeof status === 'number' && status >= 500) return `节点内部错误${suffix}：请稍后重试或查看 lm_node 日志。`
  return raw || (status ? `HTTP ${status}` : '节点请求失败')
}

export function createNodeApi(getConfig: () => NodeConfig, timeoutMs = 10_000) {
  async function request(path: string, init?: RequestInit): Promise<any> {
    const { url, token } = getConfig()
    if (!url) throw new NodeError('请先填写节点地址')
    const endpoint = `${url.replace(/\/$/, '')}${path}`
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), timeoutMs)
    let res: Response
    try {
      res = await fetch(endpoint, {
        ...init,
        signal: controller.signal,
        headers: {
          'content-type': 'application/json',
          ...(token ? { authorization: `Bearer ${token}` } : {}),
          ...init?.headers,
        },
      })
    } catch (err) {
      throw new NodeError(
        controller.signal.aborted
          ? `节点请求超时（${endpoint}）：请稍后重试或检查节点是否忙碌。`
          : `无法连接节点（${endpoint}）：请确认 lm_node 已启动、地址可访问，HTTPS 页面不要直连未代理的 HTTP 节点。${String(err)}`,
      )
    } finally {
      clearTimeout(timer)
    }
    const text = await res.text()
    let body: any = text
    try {
      body = text ? JSON.parse(text) : {}
    } catch {
      // keep raw text
    }
    if (!res.ok) {
      const message = typeof body === 'string' ? body : JSON.stringify(body)
      throw new NodeError(classifyNodeError(res.status, message, endpoint), res.status)
    }
    return body
  }

  const get = (path: string) => request(path)
  const post = (path: string, jsonBody: unknown) =>
    request(path, { method: 'POST', body: JSON.stringify(jsonBody) })

  return {
    health: (): Promise<HealthResponse> => get('/api/health'),
    stats: (): Promise<ControlStatsResponse> => get('/api/control/stats'),
    metricsText: (): Promise<string> => request('/api/control/metrics').then(String),
    syncStatus: (): Promise<SyncStatusResponse> => get('/api/sync/status'),
    resetPeer: (peerUrl: string) => post('/api/sync/peer/reset', { url: peerUrl }),
    dhtMaintenance: () => get('/api/dht/maintenance?factor=3&limit=8&max_targets=8'),
    dhtReplicate: () => get('/api/dht/replicate?factor=3'),
    dhtRoutingRefresh: () => get('/api/dht/routing-refresh?limit=8&max_targets=8'),
    dhtFindValue: (key: string) =>
      get(`/api/dht/find-value?key=${encodeURIComponent(key)}&limit=8&max_peers=8&alpha=3`),
    snapshotExport: () => get('/api/sync/snapshot'),
    snapshotImport: (snapshot: unknown) => post('/api/sync/import', { snapshot }),
  }
}

export type NodeApi = ReturnType<typeof createNodeApi>
