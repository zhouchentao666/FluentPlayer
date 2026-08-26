<script lang="ts" setup>
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { ControlAudioStore } from '@/store/ControlAudio'
import { storeToRefs } from 'pinia'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

interface Props {
  show?: boolean
  height?: number
  barCount?: number
  color?: string
  backgroundColor?: string
}

const props = withDefaults(defineProps<Props>(), {
  show: true,
  height: 80,
  barCount: 128,
  color: 'rgba(255, 255, 255, 0.8)',
  backgroundColor: 'transparent'
})

const canvasRef = ref<HTMLCanvasElement>()
const resizeObserver = ref<ResizeObserver>()

const controlAudio = ControlAudioStore()
const { Audio } = storeToRefs(controlAudio)

let unlisten: UnlistenFn | null = null
let animationId: number | undefined
let lastDrawTime = 0

const BAND_COUNT = 128
const SILENCE_DB = -80
const ATTACK = 0.3
const RELEASE = 0.85
const PEAK_FALL = 0.6
const PEAK_BAR_H = 2
const DRAW_INTERVAL_MS = 1000 / 30

interface SpectrumPayload {
  bands?: unknown
}

const targetBands = new Float64Array(BAND_COUNT).fill(SILENCE_DB)
const smoothedBands = new Float64Array(BAND_COUNT).fill(SILENCE_DB)
const peakBands = new Float64Array(BAND_COUNT).fill(SILENCE_DB)

let drawWidth = 0
let drawHeight = 0
let barWidth = 0
let halfBars = 0
let centerX = 0
let maxBarH = 0

function dbToNorm(db: number): number {
  const clamped = Math.max(SILENCE_DB, Math.min(10, db))
  const norm = (clamped - SILENCE_DB) / 90
  return Math.pow(norm, 0.55)
}

const setupSpectrumListener = async () => {
  unlisten = await listen<SpectrumPayload>('player:spectrum', (event) => {
    const { bands } = event.payload
    if (!Array.isArray(bands)) return

    targetBands.fill(SILENCE_DB)
    const len = Math.min(bands.length, BAND_COUNT)
    for (let i = 0; i < len; i++) {
      const band = bands[i]
      targetBands[i] = typeof band === 'number' && Number.isFinite(band) ? band : SILENCE_DB
    }
  })
}

function updateSmoothed() {
  const activeBands = Math.max(1, Math.min(BAND_COUNT, halfBars))
  for (let i = 0; i < activeBands; i++) {
    const target = targetBands[i]
    const prev = smoothedBands[i]
    if (target > prev) {
      smoothedBands[i] = prev * ATTACK + target * (1 - ATTACK)
    } else {
      smoothedBands[i] = prev * RELEASE + target * (1 - RELEASE)
    }
    // peak hold
    if (smoothedBands[i] > peakBands[i]) {
      peakBands[i] = smoothedBands[i]
    } else {
      peakBands[i] = Math.max(smoothedBands[i], peakBands[i] - PEAK_FALL)
    }
  }
}

function drawRoundedBar(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  if (h < 1) return
  const radius = Math.min(r, w / 2, h / 2)
  ctx.beginPath()
  ctx.moveTo(x, y + h)
  ctx.lineTo(x, y + radius)
  ctx.arcTo(x, y, x + radius, y, radius)
  ctx.arcTo(x + w, y, x + w, y + radius, radius)
  ctx.lineTo(x + w, y + h)
  ctx.closePath()
  ctx.fill()
}

let cachedGradient: CanvasGradient | null = null
let lastColor = ''

function getGradient(ctx: CanvasRenderingContext2D): CanvasGradient | CanvasPattern {
  if (lastColor === props.color && cachedGradient) return cachedGradient
  const g = ctx.createLinearGradient(0, drawHeight, 0, 0)
  g.addColorStop(0, props.color)
  g.addColorStop(1, props.color.replace(/[\d.]+\)$/, '0.2)'))
  cachedGradient = g
  lastColor = props.color
  return g
}

function scheduleNextFrame() {
  if (props.show && Audio.value.isPlay) {
    animationId = requestAnimationFrame(draw)
  }
}

function draw(timestamp = 0) {
  if (timestamp - lastDrawTime < DRAW_INTERVAL_MS) {
    scheduleNextFrame()
    return
  }
  lastDrawTime = timestamp

  if (!canvasRef.value) return
  const canvas = canvasRef.value
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  updateSmoothed()

  if (props.backgroundColor === 'transparent') {
    ctx.clearRect(0, 0, canvas.width, canvas.height)
  } else {
    ctx.fillStyle = props.backgroundColor
    ctx.fillRect(0, 0, canvas.width, canvas.height)
  }

  if (drawWidth === 0) {
    scheduleNextFrame()
    return
  }

  const gradient = getGradient(ctx)
  ctx.fillStyle = gradient

  // glow effect via shadowBlur
  ctx.shadowColor = props.color.replace(/[\d.]+\)$/, '0.6)')
  ctx.shadowBlur = 8
  ctx.shadowOffsetX = 0
  ctx.shadowOffsetY = 0

  const cornerR = Math.max(1, barWidth * 0.3)

  for (let i = 0; i < halfBars; i++) {
    const norm = dbToNorm(smoothedBands[i])
    const barH = norm * maxBarH
    const y = drawHeight - barH

    const lx = centerX - (i + 1) * barWidth
    drawRoundedBar(ctx, lx, y, barWidth - 0.5, barH, cornerR)

    const rx = centerX + i * barWidth
    drawRoundedBar(ctx, rx, y, barWidth - 0.5, barH, cornerR)
  }

  // peak indicators
  ctx.shadowBlur = 0
  ctx.fillStyle = props.color
  for (let i = 0; i < halfBars; i++) {
    const peakNorm = dbToNorm(peakBands[i])
    const peakH = peakNorm * maxBarH
    const peakY = drawHeight - peakH - PEAK_BAR_H

    const lx = centerX - (i + 1) * barWidth
    ctx.fillRect(lx, peakY, barWidth - 0.5, PEAK_BAR_H)

    const rx = centerX + i * barWidth
    ctx.fillRect(rx, peakY, barWidth - 0.5, PEAK_BAR_H)
  }

  scheduleNextFrame()
}

function startVisualization() {
  if (!props.show || !Audio.value.isPlay) return
  if (animationId !== undefined) cancelAnimationFrame(animationId)
  lastDrawTime = 0
  animationId = requestAnimationFrame(draw)
}

function stopVisualization() {
  if (animationId !== undefined) {
    cancelAnimationFrame(animationId)
    animationId = undefined
  }
  if (canvasRef.value) {
    const ctx = canvasRef.value.getContext('2d')
    if (ctx) ctx.clearRect(0, 0, canvasRef.value.width, canvasRef.value.height)
  }
}

function disconnectResizeObserver() {
  if (resizeObserver.value) {
    resizeObserver.value.disconnect()
    resizeObserver.value = undefined
  }
}

function bindResizeObserver() {
  disconnectResizeObserver()
  if (!canvasRef.value) return

  resizeCanvas()
  const container = canvasRef.value.parentElement
  if (!container) return

  resizeObserver.value = new ResizeObserver(() => {
    nextTick(resizeCanvas)
  })
  resizeObserver.value.observe(container)
}

let visualizationRequestId = 0

async function showVisualization() {
  const requestId = ++visualizationRequestId
  if (!props.show || !Audio.value.isPlay) return
  await nextTick()
  if (requestId !== visualizationRequestId || !props.show || !Audio.value.isPlay) return
  bindResizeObserver()
  startVisualization()
}

function hideVisualization() {
  visualizationRequestId++
  stopVisualization()
  disconnectResizeObserver()
}

watch(() => Audio.value.isPlay, (playing) => {
  if (playing && props.show) showVisualization()
  else hideVisualization()
})

watch(() => props.show, (show) => {
  if (show && Audio.value.isPlay) showVisualization()
  else hideVisualization()
})

watch(() => props.color, () => { cachedGradient = null })

function resizeCanvas() {
  if (!canvasRef.value) return
  const canvas = canvasRef.value
  const container = canvas.parentElement
  if (!container) return

  const rect = container.getBoundingClientRect()
  const dpr = window.devicePixelRatio || 1
  canvas.width = Math.round(rect.width * dpr)
  canvas.height = Math.round(props.height * dpr)
  canvas.style.width = rect.width + 'px'
  canvas.style.height = props.height + 'px'

  const ctx = canvas.getContext('2d')
  if (ctx) {
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.scale(dpr, dpr)
  }

  drawWidth = rect.width
  drawHeight = props.height
  halfBars = Math.floor(props.barCount / 2)
  barWidth = drawWidth / 2 / halfBars
  centerX = drawWidth / 2
  maxBarH = drawHeight * 0.92
  cachedGradient = null
}

onMounted(async () => {
  await setupSpectrumListener()

  if (props.show) {
    await nextTick()
    bindResizeObserver()
  }

  if (props.show && Audio.value.isPlay) {
    startVisualization()
  }
})

onBeforeUnmount(() => {
  hideVisualization()
  if (unlisten) { unlisten(); unlisten = null }
})
</script>

<template>
  <div v-if="show" class="audio-visualizer" :style="{ height: `${height}px` }">
    <canvas ref="canvasRef" class="visualizer-canvas" :style="{ height: `${height}px` }" />
  </div>
</template>

<style lang="scss" scoped>
.audio-visualizer {
  width: 100%;
  position: relative;
  overflow: hidden;

  .visualizer-canvas {
    width: 100%;
    display: block;
    border-radius: 4px;
  }
}
</style>
