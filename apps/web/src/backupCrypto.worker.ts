import init, { export_data_backup, import_data_backup } from './wasm/lm_wasm.js'

type ExportRequest = {
  id: number
  type: 'export'
  backupText: string
  passphrase: string
  dataJson: string
}

type ImportRequest = {
  id: number
  type: 'import'
  backupText: string
  passphrase: string
  dataBackupText: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<ExportRequest | ImportRequest>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    const value = request.type === 'export'
      ? export_data_backup(request.backupText, request.passphrase, request.dataJson)
      : import_data_backup(request.backupText, request.passphrase, request.dataBackupText)
    self.postMessage({ id: request.id, ok: true, value })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
