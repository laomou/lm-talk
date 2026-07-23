import init, {
  create_device_cert,
  create_device_revoke,
  export_contact_card,
  import_contact_as_json,
  inspect_contact_card,
  inspect_device_revoke,
} from './wasm/lm_wasm.js'

type Request = { id: number; operation: string; [key: string]: string | number | undefined }
let wasmReady: Promise<unknown> | null = null
function ensureWasmReady(): Promise<unknown> { wasmReady ??= init(); return wasmReady }
function required(request: Request, key: string): string {
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
      case 'inspectContactCard': value = inspect_contact_card(required(request, 'cardText')); break
      case 'importContact': value = import_contact_as_json(required(request, 'cardText'), required(request, 'state')); break
      case 'exportContactCard': value = export_contact_card(required(request, 'backupText'), required(request, 'passphrase'), typeof request.displayName === 'string' ? request.displayName || undefined : undefined, typeof request.deviceCertsJson === 'string' ? request.deviceCertsJson || undefined : undefined); break
      case 'createDeviceCert': value = create_device_cert(required(request, 'backupText'), required(request, 'passphrase'), required(request, 'label')); break
      case 'createDeviceRevoke': value = create_device_revoke(required(request, 'backupText'), required(request, 'passphrase'), required(request, 'deviceId'), typeof request.reason === 'string' ? request.reason || undefined : undefined); break
      case 'inspectDeviceRevoke': value = inspect_device_revoke(required(request, 'revokeText'), required(request, 'identityPublicKey')); break
      default: throw new Error(`未知 Contact Card Worker 操作：${request.operation}`)
    }
    self.postMessage({ id: request.id, ok: true, value })
  } catch (error) {
    self.postMessage({ id: request.id, ok: false, error: error instanceof Error ? error.message : String(error) })
  }
}
