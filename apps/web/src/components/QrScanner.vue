<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import jsQR from 'jsqr'

const emit = defineEmits<{ scanned: [value: string]; close: [] }>()

const videoEl = ref<HTMLVideoElement | null>(null)
const canvasEl = ref<HTMLCanvasElement | null>(null)
const errorText = ref('')

let stream: MediaStream | null = null
let animationId = 0

async function startCamera() {
  try {
    stream = await navigator.mediaDevices.getUserMedia({
      video: { facingMode: 'environment', width: { ideal: 1280 }, height: { ideal: 720 } },
    })
    if (videoEl.value) {
      videoEl.value.srcObject = stream
      await videoEl.value.play()
      scanLoop()
    }
  } catch (e) {
    errorText.value = e instanceof DOMException && e.name === 'NotAllowedError'
      ? '摄像头权限被拒绝，请在浏览器设置中允许。'
      : `无法访问摄像头：${String(e)}`
  }
}

function scanLoop() {
  const video = videoEl.value
  const canvas = canvasEl.value
  if (!video || !canvas || video.readyState < 2) {
    animationId = requestAnimationFrame(scanLoop)
    return
  }
  const ctx = canvas.getContext('2d', { willReadFrequently: true })
  if (!ctx) return
  canvas.width = video.videoWidth
  canvas.height = video.videoHeight
  ctx.drawImage(video, 0, 0, canvas.width, canvas.height)
  const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height)
  const result = jsQR(imageData.data, imageData.width, imageData.height)
  if (result?.data) {
    emit('scanned', result.data)
    return
  }
  animationId = requestAnimationFrame(scanLoop)
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
}

onMounted(startCamera)
onUnmounted(stopCamera)
</script>

<template>
  <div class="qr-scanner-overlay">
    <div class="qr-scanner-header">
      <h2>扫描二维码</h2>
      <button class="icon-btn" aria-label="关闭扫描" @click="emit('close')">✕</button>
    </div>
    <div class="qr-scanner-body">
      <video ref="videoEl" class="qr-scanner-video" playsinline muted />
      <canvas ref="canvasEl" class="qr-scanner-canvas" />
      <div v-if="errorText" class="qr-scanner-error">{{ errorText }}</div>
    </div>
  </div>
</template>
