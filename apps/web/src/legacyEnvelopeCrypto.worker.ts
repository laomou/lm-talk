import init, {
  decrypt_text_message,
  encrypt_text_message,
} from './wasm/lm_wasm.js'

type Request = {
  id: number
  operation: 'encrypt' | 'decrypt'
  backupText: string
  passphrase: string
  contactCardText: string
  conversationId?: string
  text?: string
  envelopeText?: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

function required(request: Request, key: 'conversationId' | 'text' | 'envelopeText'): string {
  const value = request[key]
  if (typeof value !== 'string') throw new Error(`缺少 Worker 参数：${key}`)
  return value
}

self.onmessage = async (event: MessageEvent<Request>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    const value = request.operation === 'encrypt'
      ? encrypt_text_message(
          request.backupText,
          request.passphrase,
          request.contactCardText,
          required(request, 'conversationId'),
          required(request, 'text'),
        )
      : decrypt_text_message(
          request.backupText,
          request.passphrase,
          request.contactCardText,
          required(request, 'envelopeText'),
        )
    self.postMessage({ id: request.id, ok: true, value })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
