type SetKeyRequest = {
  id: number
  type: 'set-key'
  keyId: string
  key: CryptoKey
}

type CryptoRequest = {
  id: number
  type: 'encrypt' | 'decrypt'
  keyId: string
  value: string | { iv: string; ct: string }
}

const keys = new Map<string, CryptoKey>()

function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  for (let index = 0; index < bytes.length; index += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(index, index + 0x8000))
  }
  return btoa(binary)
}

function base64ToBytes(value: string): Uint8Array {
  const binary = atob(value)
  const out = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) out[index] = binary.charCodeAt(index)
  return out
}

self.onmessage = async (event: MessageEvent<SetKeyRequest | CryptoRequest>) => {
  const request = event.data
  try {
    if (request.type === 'set-key') {
      keys.set(request.keyId, request.key)
      self.postMessage({ id: request.id, ok: true })
      return
    }
    const key = keys.get(request.keyId)
    if (!key) throw new Error('本地存储加密密钥未初始化')
    if (request.type === 'encrypt') {
      const iv = crypto.getRandomValues(new Uint8Array(12))
      const ct = new Uint8Array(await crypto.subtle.encrypt(
        { name: 'AES-GCM', iv },
        key,
        new TextEncoder().encode(request.value as string),
      ))
      self.postMessage({
        id: request.id,
        ok: true,
        value: {
          __lm_enc_v1: true,
          alg: 'AES-GCM',
          kdf: 'PBKDF2-SHA-256',
          iv: bytesToBase64(iv),
          ct: bytesToBase64(ct),
        },
      })
      return
    }
    const value = request.value as { iv: string; ct: string }
    const iv = base64ToBytes(value.iv)
    const ciphertext = base64ToBytes(value.ct)
    const plain = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: iv as BufferSource },
      key,
      ciphertext as BufferSource,
    )
    self.postMessage({ id: request.id, ok: true, value: new TextDecoder().decode(plain) })
  } catch (error) {
    self.postMessage({ id: request.id, ok: false, error: error instanceof Error ? error.message : String(error) })
  }
}
