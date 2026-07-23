import init, { create_file_package, decrypt_file_package } from './wasm/lm_wasm.js'

type CreateFilePackageRequest = {
  id: number
  type: 'create'
  backupText: string
  passphrase: string
  contactCardText: string
  name: string
  mimeType: string
  bytes: ArrayBuffer
}

type DecryptFilePackageRequest = {
  id: number
  type: 'decrypt'
  backupText: string
  passphrase: string
  contactCardText: string
  filePackageText: string
}

type FileCryptoRequest = CreateFilePackageRequest | DecryptFilePackageRequest

let wasmReady: Promise<unknown> | null = null
const postToMain = self.postMessage.bind(self) as (message: unknown, transfer?: Transferable[]) => void

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

function bytesToBase64(bytes: Uint8Array): string {
  const parts: string[] = []
  const chunkBytes = 3 * 256 * 1024
  for (let offset = 0; offset < bytes.length; offset += chunkBytes) {
    const chunk = bytes.subarray(offset, Math.min(offset + chunkBytes, bytes.length))
    let binary = ''
    for (let index = 0; index < chunk.length; index += 0x8000) {
      binary += String.fromCharCode(...chunk.subarray(index, index + 0x8000))
    }
    parts.push(btoa(binary))
  }
  return parts.join('')
}

function base64ToBytes(value: string): Uint8Array {
  const byteLength = Math.floor((value.length * 3) / 4) - (value.endsWith('==') ? 2 : value.endsWith('=') ? 1 : 0)
  const out = new Uint8Array(byteLength)
  const chunkChars = 4 * 256 * 1024
  let offset = 0
  for (let index = 0; index < value.length; index += chunkChars) {
    const binary = atob(value.slice(index, index + chunkChars))
    for (let byteIndex = 0; byteIndex < binary.length; byteIndex += 1) out[offset + byteIndex] = binary.charCodeAt(byteIndex)
    offset += binary.length
  }
  return out
}

self.onmessage = async (event: MessageEvent<FileCryptoRequest>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    if (request.type === 'create') {
      const filePackageText = create_file_package(
        request.backupText,
        request.passphrase,
        request.contactCardText,
        request.name,
        request.mimeType,
        bytesToBase64(new Uint8Array(request.bytes)),
        16 * 1024,
      )
      self.postMessage({ id: request.id, ok: true, type: request.type, filePackageText })
      return
    }
    const out = JSON.parse(decrypt_file_package(
      request.backupText,
      request.passphrase,
      request.contactCardText,
      request.filePackageText,
    )) as { name: string; mime_type: string; size?: number; bytes_base64: string }
    const bytes = base64ToBytes(out.bytes_base64)
    const transferableBytes = bytes.buffer as ArrayBuffer
    postToMain({
      id: request.id,
      ok: true,
      type: request.type,
      name: out.name,
      mimeType: out.mime_type,
      size: out.size,
      bytes: transferableBytes,
    }, [transferableBytes])
  } catch (error) {
    postToMain({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
