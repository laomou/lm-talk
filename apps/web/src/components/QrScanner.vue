<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import UiIcon from './UiIcon.vue'
import { useI18n } from 'vue-i18n'

const emit = defineEmits<{ scanned: [value: string]; close: [] }>()
const { t } = useI18n()

const videoEl = ref<HTMLVideoElement | null>(null)
const canvasEl = ref<HTMLCanvasElement | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)
const errorText = ref('')
const scanningImage = ref(false)
const cameraReady = ref(false)

let stream: MediaStream | null = null
let animationId = 0
let decodeWorker: Worker | null = null
let nextRequestId = 1
let decoding = false
let lastDecodeAt = 0

const cameraSupported = computed(() => typeof navigator !== 'undefined' && Boolean(navigator.mediaDevices?.getUserMedia))

function getDecodeWorker() {
  if (decodeWorker) return decodeWorker
  decodeWorker = new Worker(new URL('../qrDecode.worker.ts', import.meta.url), { type: 'module' })
  return decodeWorker
}

function decodePixels(pixels: Uint8ClampedArray, width: number, height: number): Promise<string> {
  const id = nextRequestId++
  const worker = getDecodeWorker()
  return new Promise((resolve, reject) => {
    const onMessage = (event: MessageEvent<{ id: number; ok: boolean; value?: string; error?: string }>) => {
      if (event.data.id !== id) return
      worker.removeEventListener('message', onMessage)
      if (event.data.ok) resolve(event.data.value || '')
      else reject(new Error(event.data.error || 'QR decode failed'))
    }
    worker.addEventListener('message', onMessage)
    worker.postMessage({ id, pixels: pixels.buffer, width, height }, [pixels.buffer])
  })
}

function cameraErrorText(error: unknown) {
  if (!window.isSecureContext) return t('contactsView.scanNeedsSecureContext')
  if (error instanceof DOMException && error.name === 'NotAllowedError') return t('contactsView.scanPermissionDenied')
  if (error instanceof DOMException && error.name === 'NotFoundError') return t('contactsView.scanNoCamera')
  return t('contactsView.scanCameraUnavailable')
}

async function startCamera() {
  if (!cameraSupported.value) {
    errorText.value = t('contactsView.scanNoCamera')
    return
  }
  try {
    stream = await navigator.mediaDevices.getUserMedia({
      video: { facingMode: 'environment', width: { ideal: 1280 }, height: { ideal: 720 } },
    })
    if (videoEl.value) {
      videoEl.value.srcObject = stream
      await videoEl.value.play()
      cameraReady.value = true
      scanLoop()
    }
  } catch (error) {
    errorText.value = cameraErrorText(error)
  }
}

function scanLoop() {
  const video = videoEl.value
  const canvas = canvasEl.value
  if (!video || !canvas || video.readyState < 2 || !cameraReady.value) {
    animationId = requestAnimationFrame(scanLoop)
    return
  }
  const now = performance.now()
  if (decoding || now - lastDecodeAt < 125) {
    animationId = requestAnimationFrame(scanLoop)
    return
  }
  const context = canvas.getContext('2d', { willReadFrequently: true })
  if (!context) return
  const scale = Math.min(1, 640 / video.videoWidth)
  canvas.width = Math.max(1, Math.floor(video.videoWidth * scale))
  canvas.height = Math.max(1, Math.floor(video.videoHeight * scale))
  context.drawImage(video, 0, 0, canvas.width, canvas.height)
  const imageData = context.getImageData(0, 0, canvas.width, canvas.height)
  decoding = true
  lastDecodeAt = now
  void decodePixels(imageData.data, imageData.width, imageData.height)
    .then((value) => {
      if (!value) return
      stopCamera()
      emit('scanned', value)
    })
    .catch(() => {
      // Ignore a single decode frame and continue scanning.
    })
    .finally(() => {
      decoding = false
      if (cameraReady.value) animationId = requestAnimationFrame(scanLoop)
    })
}

function chooseImage() {
  fileInput.value?.click()
}

async function onImageSelected(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  scanningImage.value = true
  errorText.value = ''
  try {
    const bitmap = await createImageBitmap(file)
    const scale = Math.min(1, 1280 / bitmap.width)
    const width = Math.max(1, Math.floor(bitmap.width * scale))
    const height = Math.max(1, Math.floor(bitmap.height * scale))
    const canvas = document.createElement('canvas')
    canvas.width = width
    canvas.height = height
    const context = canvas.getContext('2d', { willReadFrequently: true })
    if (!context) throw new Error('Canvas unavailable')
    context.drawImage(bitmap, 0, 0, width, height)
    bitmap.close()
    const imageData = context.getImageData(0, 0, width, height)
    const value = await decodePixels(imageData.data, width, height)
    if (!value) {
      errorText.value = t('contactsView.scanImageNotFound')
      return
    }
    stopCamera()
    emit('scanned', value)
  } catch {
    errorText.value = t('contactsView.scanImageFailed')
  } finally {
    scanningImage.value = false
  }
}

function stopCamera() {
  if (animationId) {
    cancelAnimationFrame(animationId)
    animationId = 0
  }
  if (stream) {
    for (const track of stream.getTracks()) track.stop()
    stream = null
  }
  cameraReady.value = false
}

onMounted(startCamera)
onUnmounted(() => {
  stopCamera()
  decodeWorker?.terminate()
  decodeWorker = null
})
</script>

<template>
  <div class="qr-scanner-overlay" role="dialog" aria-modal="true" :aria-label="t('contactsView.scanAdd')">
    <div class="qr-scanner-header">
      <h2>{{ t('contactsView.scanAdd') }}</h2>
      <button class="icon-btn" :aria-label="t('common.close')" :title="t('common.close')" @click="emit('close')"><UiIcon name="close" /></button>
    </div>
    <div class="qr-scanner-body">
      <video ref="videoEl" class="qr-scanner-video" playsinline muted />
      <canvas ref="canvasEl" class="qr-scanner-canvas" />
      <div v-if="cameraReady && !errorText" class="qr-scanner-guide" aria-hidden="true"></div>
      <div v-if="errorText" class="qr-scanner-error">{{ errorText }}</div>
    </div>
    <footer class="qr-scanner-actions">
      <input ref="fileInput" class="sr-only" type="file" accept="image/*" @change="onImageSelected" />
      <small>{{ t('contactsView.scanHint') }}</small>
      <button class="secondary" :disabled="scanningImage" @click="chooseImage">
        <UiIcon name="image" size="16" /> {{ scanningImage ? t('contactsView.scanningImage') : t('contactsView.scanImage') }}
      </button>
    </footer>
  </div>
</template>
