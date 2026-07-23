import init, { inspect_file_package } from './wasm/lm_wasm.js'

type Request = {
  id: number
  filePackageText: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<Request>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    const value = inspect_file_package(request.filePackageText)
    self.postMessage({ id: request.id, ok: true, value })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
