<script lang="ts" setup>
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue'
import { getAudioVisualizerSamples, getIdleSamples, sampleToHeight } from './audioVisualizerMath'

const props = defineProps<{
  /** 共享的 <audio> 元素，用于接入 Web Audio 分析节点。 */
  audioEl: HTMLAudioElement | null
  /** 是否启用可视化。 */
  enabled: boolean
  /** 强调色（十六进制），用于频谱着色。 */
  accentColor?: string
  /** 当前是否正在播放。 */
  isPlaying?: boolean
}>()

const BAR_COUNT = 64
const canvasRef = ref<HTMLCanvasElement | null>(null)

let audioCtx: AudioContext | null = null
let analyser: AnalyserNode | null = null
let sourceNode: MediaElementAudioSourceNode | null = null
let dataArray: Uint8Array<ArrayBuffer> | null = null
let rafId = 0
// 上次真实样本是否全为 0（CORS 阻断等情况），用于切换到伪频谱。
let lastRealActive = false

const accent = computed(() => props.accentColor || '#0078d4')

function ensureAudioGraph() {
  if (!props.audioEl || analyser) return
  try {
    const Ctx = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext
    if (!Ctx) return
    audioCtx = new Ctx()
    analyser = audioCtx.createAnalyser()
    analyser.fftSize = 256
    analyser.smoothingTimeConstant = 0.8
    dataArray = new Uint8Array(new ArrayBuffer(analyser.frequencyBinCount))
    // 将 <audio> 接入分析节点。createMediaElementSource 会把音频输出重定向到图内，
    // 因此必须再连到 destination，否则会没有声音。
    sourceNode = audioCtx.createMediaElementSource(props.audioEl)
    sourceNode.connect(analyser)
    analyser.connect(audioCtx.destination)
  } catch {
    // 创建失败（如已被其它地方接管、或浏览器限制）时静默降级为伪频谱。
    analyser = null
  }
}

function resumeIfNeeded() {
  if (audioCtx && audioCtx.state === 'suspended') {
    audioCtx.resume().catch(() => {})
  }
}

function readRawSamples(): number[] {
  if (analyser && dataArray) {
    analyser.getByteFrequencyData(dataArray)
    const sum = dataArray.reduce((a, b) => a + b, 0)
    lastRealActive = sum > 0
    // 取低中频段（高频能量低，跳过上半部分让条形更富表现力）。
    const usable = Math.floor(dataArray.length * 0.72)
    const step = usable / BAR_COUNT
    const out: number[] = []
    for (let i = 0; i < BAR_COUNT; i++) {
      const idx = Math.floor(i * step)
      out.push(dataArray[idx] / 255)
    }
    return out
  }
  lastRealActive = false
  return []
}

function render() {
  const canvas = canvasRef.value
  if (!canvas) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const dpr = window.devicePixelRatio || 1
  const w = canvas.clientWidth
  const h = canvas.clientHeight
  if (canvas.width !== Math.floor(w * dpr) || canvas.height !== Math.floor(h * dpr)) {
    canvas.width = Math.floor(w * dpr)
    canvas.height = Math.floor(h * dpr)
  }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, w, h)

  const playing = props.isPlaying ?? false
  let raw = playing ? readRawSamples() : []

  // 真实数据全为 0（CORS 阻断 / 无分析节点）且仍在播放时，使用柔和的伪频谱。
  if (raw.length === 0) {
    raw = getIdleSamples(BAR_COUNT, performance.now())
  } else if (!lastRealActive && playing) {
    // 有节点但无真实能量：混入伪频谱，避免完全静止。
    const idle = getIdleSamples(BAR_COUNT, performance.now())
    raw = raw.map((v, i) => (v > 0.02 ? v : idle[i]))
  }

  const samples = getAudioVisualizerSamples(raw)

  const gap = Math.max(2, w / BAR_COUNT * 0.28)
  const barW = (w - gap * (BAR_COUNT - 1)) / BAR_COUNT
  const baseY = h

  for (let i = 0; i < BAR_COUNT; i++) {
    const value = sampleToHeight(samples[i])
    const barH = Math.max(2, value * (h * 0.92))
    const x = i * (barW + gap)
    const y = baseY - barH

    const grad = ctx.createLinearGradient(0, baseY, 0, y)
    grad.addColorStop(0, hexToRgba(accent.value, 0.35))
    grad.addColorStop(1, hexToRgba(accent.value, 0.95))

    const radius = Math.min(barW / 2, 4)
    ctx.fillStyle = grad
    roundedTopRect(ctx, x, y, barW, barH, radius)
    ctx.fill()
  }

  rafId = requestAnimationFrame(render)
}

function roundedTopRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath()
  ctx.moveTo(x, y + h)
  ctx.lineTo(x, y + r)
  ctx.quadraticCurveTo(x, y, x + r, y)
  ctx.lineTo(x + w - r, y)
  ctx.quadraticCurveTo(x + w, y, x + w, y + r)
  ctx.lineTo(x + w, y + h)
  ctx.closePath()
}

function hexToRgba(hex: string, alpha: number): string {
  let h = hex.replace('#', '')
  if (h.length === 3) h = h.split('').map((c) => c + c).join('')
  const r = parseInt(h.slice(0, 2), 16) || 0
  const g = parseInt(h.slice(2, 4), 16) || 0
  const b = parseInt(h.slice(4, 6), 16) || 0
  return `rgba(${r}, ${g}, ${b}, ${alpha})`
}

function start() {
  if (rafId) return
  rafId = requestAnimationFrame(render)
}

function stop() {
  if (rafId) {
    cancelAnimationFrame(rafId)
    rafId = 0
  }
}

function teardown() {
  stop()
  try {
    sourceNode?.disconnect()
    analyser?.disconnect()
  } catch {
    /* ignore */
  }
  sourceNode = null
  analyser = null
  if (audioCtx) {
    audioCtx.close().catch(() => {})
    audioCtx = null
  }
  dataArray = null
}

watch(
  () => [props.enabled, props.audioEl] as const,
  ([enabled, el]) => {
    if (enabled && el) {
      ensureAudioGraph()
      resumeIfNeeded()
      start()
    } else {
      stop()
    }
  },
  { immediate: true },
)

watch(
  () => props.isPlaying,
  (playing) => {
    if (playing) resumeIfNeeded()
  },
)

onMounted(() => {
  if (props.enabled && props.audioEl) {
    ensureAudioGraph()
    start()
  }
})

onBeforeUnmount(() => {
  teardown()
})

// 暴露给父组件：在用户手势（如点击播放）时解锁音频上下文。
defineExpose({ ensureAudioGraph, resumeIfNeeded })
</script>

<template>
  <canvas
    v-show="enabled"
    ref="canvasRef"
    class="audio-visualizer"
  ></canvas>
</template>

<style scoped>
.audio-visualizer {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  width: 100%;
  height: 120px;
  pointer-events: none;
  z-index: 1;
  opacity: 0.9;
}
</style>
