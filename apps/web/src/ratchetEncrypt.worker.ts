import init, { ratchet_encrypt_text_message } from './wasm/lm_wasm.js'

type RatchetEncryptRequest = {
  id: number
  stateText: string
  conversationId: string
  text: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<RatchetEncryptRequest>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    const result = JSON.parse(ratchet_encrypt_text_message(
      request.stateText,
      request.conversationId,
      request.text,
    )) as { state_text: string; envelope_json: string }
    self.postMessage({ id: request.id, ok: true, ...result })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
