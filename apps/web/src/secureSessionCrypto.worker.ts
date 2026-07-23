import init, {
  create_prekey_bundle,
  create_ratchet_dh_keypair,
  create_ratchet_session_from_shared_secret,
  create_ratchet_session_from_shared_secret_with_keys,
  create_ratchet_session_pair,
  create_signal_answer,
  create_signal_offer,
  create_x3dh_initial_message_with_one_time_prekey_id,
  create_x3dh_initial_message_with_one_time_prekey_record,
  derive_x3dh_responder_secret,
  inspect_prekey_bundle,
  inspect_ratchet_state,
  inspect_signal_answer,
  inspect_signal_offer,
  ratchet_dh_step,
  ratchet_next_receiving_key,
  ratchet_next_sending_key,
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
      case 'createPrekeyBundle':
        value = create_prekey_bundle(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          Number(request.signedId),
          Number(request.oneTimeCount),
          BigInt(required(request, 'expiresSeconds')),
        )
        break
      case 'inspectPrekeyBundle':
        value = inspect_prekey_bundle(required(request, 'prekeyBundleText'))
        break
      case 'createX3dhInitialMessageWithId':
        value = create_x3dh_initial_message_with_one_time_prekey_id(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          required(request, 'prekeyBundleText'),
          request.oneTimePrekeyId === '' || request.oneTimePrekeyId === undefined ? undefined : Number(request.oneTimePrekeyId),
        )
        break
      case 'createX3dhInitialMessageWithRecord':
        value = create_x3dh_initial_message_with_one_time_prekey_record(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          required(request, 'prekeyBundleText'),
          required(request, 'signedRecordText'),
        )
        break
      case 'deriveX3dhResponderSecret':
        value = derive_x3dh_responder_secret(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          required(request, 'privateBundleJson'),
          required(request, 'initialMessageJson'),
        )
        break
      case 'createRatchetDhKeyPair':
        value = create_ratchet_dh_keypair()
        break
      case 'createRatchetSessionFromSharedSecret':
        value = create_ratchet_session_from_shared_secret(
          required(request, 'localUserId'),
          required(request, 'remoteUserId'),
          required(request, 'sharedSecret'),
        )
        break
      case 'createRatchetSessionFromSharedSecretWithKeys':
        value = create_ratchet_session_from_shared_secret_with_keys(
          required(request, 'localUserId'),
          required(request, 'remoteUserId'),
          required(request, 'role'),
          required(request, 'sharedSecret'),
          required(request, 'localDhPrivateKey'),
          required(request, 'remoteDhPublicKey'),
        )
        break
      case 'createRatchetSessionPair':
        value = create_ratchet_session_pair(required(request, 'myContactCardText'), required(request, 'peerContactCardText'))
        break
      case 'inspectRatchetState':
        value = inspect_ratchet_state(required(request, 'stateText'))
        break
      case 'ratchetNextSendingKey':
        value = ratchet_next_sending_key(required(request, 'stateText'))
        break
      case 'ratchetNextReceivingKey':
        value = ratchet_next_receiving_key(required(request, 'stateText'), required(request, 'headerText'))
        break
      case 'ratchetDhStep':
        value = ratchet_dh_step(required(request, 'stateText'), required(request, 'remoteDhPublicKey'))
        break
      case 'createSignalOffer':
        value = create_signal_offer(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          required(request, 'toUserId'),
          required(request, 'sdp'),
          BigInt(required(request, 'ttlSeconds')),
        )
        break
      case 'inspectSignalOffer':
        value = inspect_signal_offer(required(request, 'signalText'), required(request, 'contactCardText'))
        break
      case 'createSignalAnswer':
        value = create_signal_answer(
          required(request, 'backupText'),
          required(request, 'passphrase'),
          required(request, 'offerText'),
          required(request, 'sdp'),
          BigInt(required(request, 'ttlSeconds')),
        )
        break
      case 'inspectSignalAnswer':
        value = inspect_signal_answer(required(request, 'signalText'), required(request, 'contactCardText'))
        break
      default:
        throw new Error(`未知安全会话 Worker 操作：${request.operation}`)
    }
    self.postMessage({ id: request.id, ok: true, value })
  } catch (error) {
    self.postMessage({
      id: request.id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
