import init, {
  create_group_sender_key,
  group_sender_decrypt_text,
  group_sender_encrypt_text,
  import_group_sender_key,
} from './wasm/lm_wasm.js'

type GroupSenderDecryptRequest = {
  id: number
  type: 'decrypt'
  stateText: string
  envelopeText: string
}

type GroupSenderEncryptRequest = {
  id: number
  type: 'encrypt'
  stateText: string
  text: string
}

type GroupSenderCreateRequest = {
  id: number
  type: 'create'
  backupText: string
  passphrase: string
  groupId: string
}

type GroupSenderImportRequest = {
  id: number
  type: 'import'
  distributionText: string
  senderContactCardText: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<GroupSenderDecryptRequest | GroupSenderEncryptRequest | GroupSenderCreateRequest | GroupSenderImportRequest>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    let result: Record<string, string>
    if (request.type === 'decrypt') {
      result = JSON.parse(group_sender_decrypt_text(
        request.stateText,
        request.envelopeText,
      )) as { state_json: string; plain_json: string }
    } else if (request.type === 'encrypt') {
      result = JSON.parse(group_sender_encrypt_text(
        request.stateText,
        request.text,
      )) as { state_json: string; envelope_json: string }
    } else if (request.type === 'create') {
      result = JSON.parse(create_group_sender_key(
        request.backupText,
        request.passphrase,
        request.groupId,
      )) as { state_json: string; distribution_text: string }
    } else {
      result = {
        state_json: import_group_sender_key(
          request.distributionText,
          request.senderContactCardText,
        ),
      }
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
