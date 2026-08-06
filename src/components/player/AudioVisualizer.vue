<script setup lang="ts">
/**
 * 音频可视化频谱条（参考 LyciaMusic-main 的 AudioVisualizer）。
 * 不同于 Lycia（后端 playbackApi 采样），FluentPlayer 的音频由前端 Web Audio 播放，
 * 故直接用 AnalyserNode.getByteFrequencyData 做实时 FFT 频谱，无需后端改动。
 */
import { ref, onMounted, onBeforeUnmount, watch, inject, type Ref } from 'vue'

const props = withDefaults(
  defineProps<{
    /** 分析节点；为空时显示静态平条。 */
    analyser?: AnalyserNode | null
    /** 是否正在播放，用于决定是否滚动动画。 */
    playing?: boolean
    /** 柱状数量（默认 64）。 */
    bars?: number
    /** 柱高（CSS 高度）。 */
    height?: string
  }>(),
  { analyser: null, playing: true, bars: 64, height: '56px' },
)

// 若未显式传入 analyser，则从 App 注入
const injected = inject<Ref<AnalyserNode | null> | null>('audioAnalyser', null)
const activeAnalyser = () => props.analyser ?? injected?.value ?? null

const canvas = ref<HTMLCanvasElement | null>(null)
let rafId = 0
let freqData: Uint8Array | null = null

function resize() {
  const el = canvas.value
  if (!el) return
  const dpr = window.devicePixelRatio || 1
  const w = el.clientWidth
  const h = el.clientHeight
  el.width = Math.max(1, Math.floor(w * dpr))
  el.height = Math.max(1, Math.floor(h * dpr))
  const ctx = el.getContext('2d')
  if (ctx) ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
}

function accentColor(): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue('--fluent-accent')
  return v ? v.trim() : '#0078d4'
}

function draw() {
  const el = canvas.value
  const ctx = el?.getContext('2d')
  if (!el || !ctx) return
  const w = el.clientWidth
  const h = el.clientHeight
  const analyser = activeAnalyser()
  const n = props.bars
  const gap = 2
  const barW = (w - gap * (n - 1)) / n

  // 取实时频谱
  let values: number[]
  if (analyser && props.playing) {
    if (!freqData || freqData.length !== analyser.frequencyBinCount) {
      freqData = new Uint8Array(analyser.frequencyBinCount)
    }
    analyser.getByteFrequencyData(freqData)
    // 只取低中频段（高频通常没能量），映射到 n 根柱
    const usable = Math.floor(analyser.frequencyBinCount * 0.7)
    values = []
    for (let i = 0; i < n; i++) {
      const idx = Math.floor((i / n) * usable)
      values.push(freqData[idx] / 255)
    }
  } else {
    // 静态：中间略高、两侧低的平条
    values = Array.from({ length: n }, (_, i) => {
      const t = i / (n - 1)
      return 0.12 + 0.08 * Math.sin(t * Math.PI)
    })
  }

  ctx.clearRect(0, 0, w, h)
  const color = accentColor()
  for (let i = 0; i < n; i++) {
    const v = Math.max(0.04, values[i])
    const bh = v * h
    const x = i * (barW + gap)
    const y = (h - bh) / 2
    const radius = Math.min(barW / 2, 3)
    ctx.fillStyle = color
    ctx.globalAlpha = 0.55 + 0.45 * v
    roundRect(ctx, x, y, barW, bh, radius)
    ctx.fill()
  }
  ctx.globalAlpha = 1

  // 播放中持续刷新；静态时也轻量刷新（alpha 随时间微动）
  rafId = requestAnimationFrame(draw)
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  const rr = Math.min(r, w / 2, h / 2)
  ctx.beginPath()
  ctx.moveTo(x + rr, y)
  ctx.arcTo(x + w, y, x + w, y + h, rr)
  ctx.arcTo(x + w, y + h, x, y + h, rr)
  ctx.arcTo(x, y + h, x, y, rr)
  ctx.arcTo(x, y, x + w, y, rr)
  ctx.closePath()
}

watch(
  () => [props.analyser, injected?.value, props.playing],
  () => {
    // 参数变化无需重启循环，draw 每帧都会重读
  },
)

onMounted(() => {
  resize()
  window.addEventListener('resize', resize)
  rafId = requestAnimationFrame(draw)
})

onBeforeUnmount(() => {
  cancelAnimationFrame(rafId)
  window.removeEventListener('resize', resize)
})
</script>

<template>
  <canvas ref="canvas" class="audio-visualizer" :style="{ height }"></canvas>
</template>

<style scoped>
.audio-visualizer {
  display: block;
  width: 100%;
  pointer-events: none;
}
</style>
