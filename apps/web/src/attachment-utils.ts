export type AttachmentDownload = {
  url: string
  name: string
  mime: string
  meta: string
  preview_kind: string
}

export function isDangerousFileName(name: string): boolean {
  return /\.(exe|bat|cmd|com|scr|ps1|vbs|js|jar|msi|apk|dmg|pkg|sh)$/i.test(name)
}

export function filePreviewKind(name: string, mime: string): string {
  const lower = name.toLowerCase()
  if (mime.startsWith('image/')) return '图片预览'
  if (mime.startsWith('audio/')) return '音频文件'
  if (mime.startsWith('video/')) return '视频文件'
  if (mime === 'application/pdf' || lower.endsWith('.pdf')) return 'PDF 文档'
  if (/(\.docx?|\.xlsx?|\.pptx?)$/.test(lower)) return 'Office 文档'
  if (/(\.zip|\.7z|\.rar|\.tar|\.gz)$/.test(lower)) return '压缩包'
  if (mime.startsWith('text/') || /\.(txt|md|csv|log)$/.test(lower)) return '文本文件'
  return '普通附件'
}

export function releaseAttachmentDownloads(
  downloads: Record<string, AttachmentDownload>,
  messageIds: Iterable<string>,
): Record<string, AttachmentDownload> {
  const ids = new Set(messageIds)
  if (ids.size === 0) return downloads
  const next = { ...downloads }
  for (const id of ids) {
    const download = next[id]
    if (!download) continue
    URL.revokeObjectURL(download.url)
    delete next[id]
  }
  return next
}

export async function readFileWithProgress(
  file: File,
  options: {
    mergeYieldBytes: number
    onProgress?: (loaded: number, total: number, percent: number) => void
    yieldToBrowser?: () => Promise<void>
  },
): Promise<Uint8Array> {
  if (!file.stream) {
    options.onProgress?.(0, file.size, 0)
    const bytes = new Uint8Array(await file.arrayBuffer())
    options.onProgress?.(bytes.length, file.size, 100)
    return bytes
  }
  const reader = file.stream().getReader()
  const chunks: Uint8Array[] = []
  let loaded = 0
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    if (!value) continue
    chunks.push(value)
    loaded += value.byteLength
    const percent = file.size > 0 ? Math.min(100, Math.round((loaded / file.size) * 100)) : 100
    options.onProgress?.(loaded, file.size, percent)
  }
  const out = new Uint8Array(loaded)
  let offset = 0
  for (const chunk of chunks) {
    out.set(chunk, offset)
    offset += chunk.byteLength
    if (offset < loaded && offset % options.mergeYieldBytes === 0) await options.yieldToBrowser?.()
  }
  return out
}
