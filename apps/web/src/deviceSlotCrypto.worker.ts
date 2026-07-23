import init, { open_device_slot, seal_device_slot } from './wasm/lm_wasm.js'

type SealRequest = {
  id: number
  type: 'seal'
  deviceBoxPublicKey: string
  aad: string
  ciphertext: string
}

type OpenRequest = {
  id: number
  type: 'open'
  backupText: string
  passphrase: string
  deviceBackupText: string
  aad: string
  slotText: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<SealRequest | OpenRequest>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    const value = request.type === 'seal'
      ? seal_device_slot(request.deviceBoxPublicKey, request.aad, request.ciphertext)
      : open_device_slot(
        request.backupText,
        request.passphrase,
        request.deviceBackupText,
        request.aad,
        request.slotText,
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
