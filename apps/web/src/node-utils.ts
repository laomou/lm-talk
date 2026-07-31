export type NodeEntry = { url: string; token: string }

export function nodeEntriesFromControlUrl(value: string): NodeEntry[] {
  return value
    .split(/[\n,]+/)
    .map((x, index) => ({ raw: x.trim(), line: index + 1 }))
    .filter((item) => item.raw)
    .map(({ raw, line }) => {
      // 每行：<url> 或 <url>|<令牌>（令牌对应节点的 --control-token）
      const [url, token] = raw.split('|').map((s) => s.trim())
      try {
        const parsed = new URL(url)
        if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') throw new Error('protocol')
      } catch {
        throw new Error(`同步服务第 ${line} 行地址无效，请使用 http:// 或 https:// 开头的完整地址`)
      }
      return { url, token: token || '' }
    })
    .filter((entry) => entry.url)
}

export function nodeEntryLine(entry: NodeEntry): string {
  return entry.token ? `${entry.url}|${entry.token}` : entry.url
}

export function nodeUrlListFromEntries(entries: NodeEntry[]): string[] {
  return entries.map((entry) => entry.url)
}

export function isLoopbackNodeUrl(url: string): boolean {
  try {
    const host = new URL(url).hostname.toLowerCase()
    return host === 'localhost' || host === '127.0.0.1' || host === '::1' || host === '[::1]'
  } catch {
    return false
  }
}

export function nodeTokenForUrl(entries: NodeEntry[], url: string): string {
  const base = url.replace(/\/$/, '')
  return entries.find((entry) => entry.url.replace(/\/$/, '') === base)?.token || ''
}
