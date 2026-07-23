import init, {
  accept_friend_request,
  create_friend_request,
  inspect_friend_request,
  inspect_friend_response,
} from './wasm/lm_wasm.js'

type CreateFriendRequest = {
  id: number
  type: 'create'
  backupText: string
  passphrase: string
  myContactCardText: string
  peerContactCardText: string
  message: string
}

type InspectFriendRequest = {
  id: number
  type: 'inspectRequest'
  requestText: string
}

type AcceptFriendRequest = {
  id: number
  type: 'accept'
  backupText: string
  passphrase: string
  requestText: string
}

type InspectFriendResponse = {
  id: number
  type: 'inspectResponse'
  responseText: string
  contactCardText: string
}

let wasmReady: Promise<unknown> | null = null

function ensureWasmReady(): Promise<unknown> {
  wasmReady ??= init()
  return wasmReady
}

self.onmessage = async (event: MessageEvent<CreateFriendRequest | InspectFriendRequest | AcceptFriendRequest | InspectFriendResponse>) => {
  const request = event.data
  try {
    await ensureWasmReady()
    if (request.type === 'create') {
      const requestText = create_friend_request(
        request.backupText,
        request.passphrase,
        request.myContactCardText,
        request.peerContactCardText,
        request.message,
      )
      self.postMessage({
        id: request.id,
        ok: true,
        request_text: requestText,
        info_json: inspect_friend_request(requestText),
      })
      return
    }
    if (request.type === 'inspectRequest') {
      self.postMessage({ id: request.id, ok: true, info_json: inspect_friend_request(request.requestText) })
      return
    }
    if (request.type === 'accept') {
      self.postMessage({
        id: request.id,
        ok: true,
        response_text: accept_friend_request(request.backupText, request.passphrase, request.requestText),
      })
      return
    }
    self.postMessage({ id: request.id, ok: true, info_json: inspect_friend_response(request.responseText, request.contactCardText) })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
