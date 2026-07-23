import init, {
  create_identity,
  reencrypt_identity_backup,
  restore_identity,
} from './wasm/lm_wasm.js'

type Request = {
  id: number
  operation: 'createIdentity' | 'restoreIdentity' | 'reencryptIdentityBackup'
  passphrase: string
  backupText?: string
  newPassphrase?: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

function required(request: Request, key: 'backupText' | 'newPassphrase'): string {
  const value = request[key]
  if (typeof value !== 'string') throw new Error(`缺少 Worker 参数：${key}`)
  return value
}

self.onmessage = async (event: MessageEvent<Request>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    let value: string
    switch (request.operation) {
      case 'createIdentity':
        value = create_identity(request.passphrase)
        break
      case 'restoreIdentity':
        value = restore_identity(required(request, 'backupText'), request.passphrase)
        break
      case 'reencryptIdentityBackup':
        value = reencrypt_identity_backup(
          required(request, 'backupText'),
          request.passphrase,
          required(request, 'newPassphrase'),
        )
        break
    }
    self.postMessage({ id: request.id, ok: true, value })
  } catch (error) {
    self.postMessage({ id: request.id, ok: false, error: error instanceof Error ? error.message : String(error) })
  }
}
