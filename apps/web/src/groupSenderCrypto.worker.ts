import init, { group_sender_decrypt_text } from './wasm/lm_wasm.js'

type GroupSenderDecryptRequest = {
  id: number
  stateText: string
  envelopeText: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<GroupSenderDecryptRequest>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    const result = JSON.parse(group_sender_decrypt_text(
      request.stateText,
      request.envelopeText,
    )) as { state_json: string; plain_json: string }
    self.postMessage({ id: request.id, ok: true, ...result })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
