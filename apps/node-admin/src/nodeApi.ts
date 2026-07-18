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
        controller.signal.aborted ? '请求超时' : `无法连接节点：${String(err)}`,
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
      throw new NodeError(message || `HTTP ${res.status}`, res.status)
    }
    return body
  }

  const get = (path: string) => request(path)
  const post = (path: string, jsonBody: unknown) =>
    request(path, { method: 'POST', body: JSON.stringify(jsonBody) })

  return {
    health: (): Promise<HealthResponse> => get('/health'),
    stats: (): Promise<ControlStatsResponse> => get('/control/stats'),
    metricsText: (): Promise<string> => request('/control/metrics').then(String),
    syncStatus: (): Promise<SyncStatusResponse> => get('/sync/status'),
    resetPeer: (peerUrl: string) => post('/sync/peer/reset', { url: peerUrl }),
    dhtMaintenance: () => get('/dht/maintenance?factor=3&limit=8&max_targets=8'),
    dhtReplicate: () => get('/dht/replicate?factor=3'),
    dhtRoutingRefresh: () => get('/dht/routing-refresh?limit=8&max_targets=8'),
    dhtFindValue: (key: string) =>
      get(`/dht/find-value?key=${encodeURIComponent(key)}&limit=8&max_peers=8&alpha=3`),
    snapshotExport: () => get('/sync/snapshot'),
    snapshotImport: (snapshot: unknown) => post('/sync/import', { snapshot }),
  }
}

export type NodeApi = ReturnType<typeof createNodeApi>
