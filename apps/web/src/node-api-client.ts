import type { NodeEntry } from './node-utils'

export class NodeRequestError extends Error {
  status?: number
  url?: string
  errorCode?: string
  recoveryHint?: string

  constructor(message: string, status?: number, url?: string, errorCode?: string, recoveryHint?: string) {
    super(message)
    this.name = 'NodeRequestError'
    this.status = status
    this.url = url
    this.errorCode = errorCode
    this.recoveryHint = recoveryHint
  }
}

export function nodeControlEndpoint(baseUrl: string, path: string): string {
  if (!baseUrl) throw new Error('请先填写同步节点')
  return `${baseUrl.replace(/\/$/, '')}${path}`
}

export async function fetchNodeOnce(options: {
  baseUrl: string
  path: string
  token?: string
  init?: RequestInit
  timeoutMs: number
  fetchImpl?: typeof fetch
}): Promise<any> {
  const { baseUrl, path, token, init, timeoutMs, fetchImpl = fetch } = options
  const endpoint = nodeControlEndpoint(baseUrl, path)
  const controller = new AbortController()
  const timeout = window.setTimeout(
    () => controller.abort(new DOMException('同步服务请求超时', 'AbortError')),
    timeoutMs,
  )
  let response: Response
  try {
    response = await fetchImpl(endpoint, {
      ...init,
      signal: init?.signal ?? controller.signal,
      headers: {
        'content-type': 'application/json',
        ...(token ? { authorization: `Bearer ${token}` } : {}),
        ...(init?.headers ?? {}),
      },
    })
  } catch (error) {
    if (init?.signal?.aborted) throw error
    throw new NodeRequestError(String(error), undefined, baseUrl)
  } finally {
    window.clearTimeout(timeout)
  }
  const text = await response.text()
  let body: any = text
  try { body = text ? JSON.parse(text) : {} } catch {}
  if (!response.ok) {
    if (body && typeof body === 'object' && !Array.isArray(body)) {
      throw new NodeRequestError(
        String(body.message ?? JSON.stringify(body)),
        response.status,
        baseUrl,
        typeof body.error_code === 'string' ? body.error_code : undefined,
        typeof body.recovery_hint === 'string' ? body.recovery_hint : undefined,
      )
    }
    throw new NodeRequestError(typeof body === 'string' ? body : JSON.stringify(body), response.status, baseUrl)
  }
  return body
}

export async function fetchNodeJsonWithFallback(options: {
  entries: NodeEntry[]
  path: string
  init?: RequestInit
  timeoutMs: number
  tokenFor: (url: string) => string
  formatError: (error: unknown) => string
  isAbortError: (error: unknown) => boolean
  onSuccess?: (entry: NodeEntry) => void
}): Promise<any> {
  const { entries, path, init, timeoutMs, tokenFor, formatError, isAbortError, onSuccess } = options
  if (entries.length === 0) throw new Error('请先填写同步节点')
  const errors: string[] = []
  for (const entry of entries) {
    try {
      const body = await fetchNodeOnce({
        baseUrl: entry.url,
        path,
        token: tokenFor(entry.url),
        init,
        timeoutMs,
      })
      onSuccess?.(entry)
      return body
    } catch (error) {
      if (init?.signal?.aborted || isAbortError(error)) throw error
      errors.push(`${entry.url}: ${formatError(error)}`)
    }
  }
  throw new Error(`所有同步服务都不可用：${errors.join('；')}`)
}
