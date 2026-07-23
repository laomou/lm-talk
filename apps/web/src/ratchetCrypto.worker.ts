import init, { ratchet_decrypt_text_message } from './wasm/lm_wasm.js'

type RatchetDecryptRequest = {
  id: number
  stateText: string
  envelopeText: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<RatchetDecryptRequest>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    const result = JSON.parse(ratchet_decrypt_text_message(request.stateText, request.envelopeText)) as {
      state_text: string
      plain_json: string
    }
    self.postMessage({ id: request.id, ok: true, ...result })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
