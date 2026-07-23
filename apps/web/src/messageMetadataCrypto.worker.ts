import init, {
  create_mailbox_message,
  create_message_receipt,
  create_peer_announce,
  create_public_peer_announce,
  inspect_mailbox_message,
  inspect_message_receipt,
  inspect_peer_announce,
  inspect_public_peer_announce,
} from './wasm/lm_wasm.js'

type Request = {
  id: number
  operation: string
  [key: string]: string | number | undefined
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

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
      case 'createMailboxMessage':
        value = create_mailbox_message(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          required(request, 'toUserId'),
          required(request, 'kind'),
          required(request, 'ciphertext'),
          BigInt(required(request, 'ttlSeconds')),
        )
        break
      case 'inspectMailboxMessage':
        value = inspect_mailbox_message(required(request, 'messageText'), required(request, 'identityPublicKey'))
        break
      case 'createMessageReceipt':
        value = create_message_receipt(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          required(request, 'toUserId'),
          required(request, 'targetMessageId'),
          required(request, 'conversationId'),
          typeof request.mailboxDeliveryId === 'string' && request.mailboxDeliveryId ? request.mailboxDeliveryId : undefined,
          required(request, 'kind'),
          BigInt(required(request, 'ttlSeconds')),
        )
        break
      case 'inspectMessageReceipt':
        value = inspect_message_receipt(required(request, 'receiptText'), required(request, 'identityPublicKey'))
        break
      case 'createPeerAnnounce':
        value = create_peer_announce(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          required(request, 'addressesJson'),
          typeof request.mailboxKey === 'string' && request.mailboxKey ? request.mailboxKey : undefined,
          BigInt(required(request, 'ttlSeconds')),
        )
        break
      case 'inspectPeerAnnounce':
        value = inspect_peer_announce(required(request, 'announceText'), required(request, 'identityPublicKey'))
        break
      case 'createPublicPeerAnnounce':
        value = create_public_peer_announce(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          required(request, 'peerId'),
          required(request, 'addressesJson'),
          required(request, 'capabilitiesJson'),
          BigInt(required(request, 'ttlSeconds')),
        )
        break
      case 'inspectPublicPeerAnnounce':
        value = inspect_public_peer_announce(required(request, 'announceText'), required(request, 'identityPublicKey'))
        break
      default:
        throw new Error(`未知消息元数据 Worker 操作：${request.operation}`)
    }
    self.postMessage({ id: request.id, ok: true, value })
  } catch (error) {
    self.postMessage({ id: request.id, ok: false, error: error instanceof Error ? error.message : String(error) })
  }
}
