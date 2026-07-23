import init, {
  sign_identity_text,
  verify_identity_text_signature,
} from './wasm/lm_wasm.js'

type SignRequest = {
  id: number
  type: 'sign'
  backupText: string
  passphrase: string
  payload: string
}

type VerifyRequest = {
  id: number
  type: 'verify'
  identityPublicKey: string
  payload: string
  signature: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<SignRequest | VerifyRequest>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    const value = request.type === 'sign'
      ? sign_identity_text(request.backupText, request.passphrase, request.payload)
      : verify_identity_text_signature(request.identityPublicKey, request.payload, request.signature)
    self.postMessage({ id: request.id, ok: true, value })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
