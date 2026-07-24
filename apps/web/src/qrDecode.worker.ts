import jsQR from 'jsqr'

type DecodeRequest = {
  id: number
  pixels: ArrayBuffer
  width: number
  height: number
}

self.onmessage = (event: MessageEvent<DecodeRequest>) => {
  const { id, pixels, width, height } = event.data
  try {
    const result = jsQR(new Uint8ClampedArray(pixels), width, height)
    self.postMessage({ id, ok: true, value: result?.data ?? '' })
  } catch (error) {
    self.postMessage({ id, ok: false, error: error instanceof Error ? error.message : String(error) })
  }
}
