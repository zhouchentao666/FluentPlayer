<script setup lang="ts">
import { ref, onUnmounted, nextTick, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ControlAudioStore } from '@/store/ControlAudio'
import { MessagePlugin } from 'tdesign-vue-next'
import {
  MicrophoneIcon,
  StopCircleIcon,
  UploadIcon,
  PlayCircleIcon,
  SearchIcon,
  RefreshIcon,
  PlayIcon,
  DeleteIcon,
  TimeIcon
} from 'tdesign-icons-vue-next'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { playSong } from '@/utils/audio/globaPlayList'
import { searchValue } from '@/store/search'
import { useRouter } from 'vue-router'

const audioStore = ControlAudioStore()
const localUserStore = LocalUserDetailStore()
const searchStore = searchValue()
const router = useRouter()
const { t } = useI18n()

const MAX_DURATION = 15

const running = ref(false)
const status = ref('')
const currentDuration = ref(0)
const recognizedSongs = ref<any[]>([])
const wasPlaying = ref(false)

let unlistenChunk: UnlistenFn | null = null
let unlistenLevel: UnlistenFn | null = null
let unlistenError: UnlistenFn | null = null
let timer: any = null

// --- Recognition History ---
const HISTORY_KEY = 'mio-recognition-history'
const MAX_HISTORY = 20

interface HistoryItem {
  name: string
  singer: string
  img: string
  songmid: string
  albumName: string
  timestamp: number
}

const recognitionHistory = ref<HistoryItem[]>([])

function loadHistory() {
  try {
    const raw = localStorage.getItem(HISTORY_KEY)
    if (raw) recognitionHistory.value = JSON.parse(raw)
  } catch {}
}

function saveHistory() {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(recognitionHistory.value))
  } catch {}
}

function addToHistory(songs: any[]) {
  if (!songs.length) return
  const now = Date.now()
  const items: HistoryItem[] = songs.map((s) => ({
    name: s.name,
    singer: s.singer,
    img: s.img,
    songmid: s.songmid,
    albumName: s.albumName || '',
    timestamp: now
  }))
  recognitionHistory.value = [...items, ...recognitionHistory.value].slice(0, MAX_HISTORY)
  saveHistory()
}

function clearHistory() {
  recognitionHistory.value = []
  saveHistory()
}

function formatHistoryTime(ts: number): string {
  const diff = Date.now() - ts
  const minutes = Math.floor(diff / 60000)
  if (minutes < 1) return t('common.justNow')
  if (minutes < 60) return t('common.minutesAgo', { minutes })
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return t('common.hoursAgo', { hours })
  const days = Math.floor(hours / 24)
  if (days < 7) return t('common.daysAgo', { days })
  const d = new Date(ts)
  return `${d.getMonth() + 1}/${d.getDate()}`
}

const recentHistory = computed(() => recognitionHistory.value.slice(0, 6))

const showHistory = computed(() => recognitionHistory.value.length > 0 && !running.value && status.value !== 'success')

onMounted(() => {
  loadHistory()
})

// --- Canvas visualizer ---
const canvasRef = ref<HTMLCanvasElement | null>(null)
let animFrame = 0
let audioLevel = 0
let smoothLevel = 0

const BAR_COUNT = 64
const barPhases = Float32Array.from({ length: BAR_COUNT }, () => Math.random() * Math.PI * 2)

function drawVisualizer() {
  const canvas = canvasRef.value
  if (!canvas) return

  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const dpr = Math.min(window.devicePixelRatio || 1, 2)
  const bounds = canvas.parentElement?.getBoundingClientRect()
  const size = Math.max(1, Math.round(bounds?.width || 280))
  canvas.width = size * dpr
  canvas.height = size * dpr
  canvas.style.width = `${size}px`
  canvas.style.height = `${size}px`
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

  const cx = size / 2
  const cy = size / 2
  const innerR = size * 0.215
  const maxBarH = size * 0.19

  ctx.clearRect(0, 0, size, size)

  // Smooth level interpolation
  smoothLevel += (audioLevel - smoothLevel) * 0.25

  const now = performance.now() / 1000

  // Draw circular spectrum bars
  for (let i = 0; i < BAR_COUNT; i++) {
    const angle = (i / BAR_COUNT) * Math.PI * 2 - Math.PI / 2
    const phase = barPhases[i]

    const baseH = 0.15 + 0.25 * Math.abs(Math.sin(phase + now * 1.2))
    const levelH = smoothLevel * (0.3 + 0.7 * Math.abs(Math.sin(phase * 2.3 + now * 0.8)))
    const h = baseH + levelH
    const barH = Math.max(2, h * maxBarH)

    const x1 = cx + Math.cos(angle) * (innerR + 4)
    const y1 = cy + Math.sin(angle) * (innerR + 4)
    const x2 = cx + Math.cos(angle) * (innerR + 4 + barH)
    const y2 = cy + Math.sin(angle) * (innerR + 4 + barH)

    const alpha = 0.3 + h * 0.7
    ctx.beginPath()
    ctx.moveTo(x1, y1)
    ctx.lineTo(x2, y2)
    ctx.strokeStyle = `rgba(var(--td-brand-color-rgb, 3, 222, 109), ${alpha})`
    ctx.lineWidth = 2.5
    ctx.lineCap = 'round'
    ctx.stroke()
  }

  // Inner glow ring
  if (smoothLevel > 0.05) {
    const glowAlpha = smoothLevel * 0.5
    ctx.beginPath()
    ctx.arc(cx, cy, innerR + 2, 0, Math.PI * 2)
    ctx.strokeStyle = `rgba(var(--td-brand-color-rgb, 3, 222, 109), ${glowAlpha})`
    ctx.lineWidth = 2
    ctx.shadowColor = `rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.6)`
    ctx.shadowBlur = 15 * smoothLevel
    ctx.stroke()
    ctx.shadowBlur = 0
  }

  if (running.value) {
    animFrame = requestAnimationFrame(drawVisualizer)
  }
}

function startVisualizerLoop() {
  stopVisualizerLoop()
  animFrame = requestAnimationFrame(drawVisualizer)
}

function stopVisualizerLoop() {
  if (animFrame) {
    cancelAnimationFrame(animFrame)
    animFrame = 0
  }
}

// --- Script utilities ---
function loadScript(src: string) {
  return new Promise<void>((resolve, reject) => {
    const s = document.createElement('script')
    s.src = src
    s.onload = () => resolve()
    s.onerror = (e) => reject(e)
    document.head.appendChild(s)
  })
}

async function ensureAFP() {
  const g = window as any
  if (typeof g.GenerateFP === 'function') return
  if (!g.__afp_wasm_loaded__) {
    try {
      await loadScript('afp.wasm.js')
      g.__afp_wasm_loaded__ = true
    } catch {}
  }
  if (!g.__afp_runtime_loaded__) {
    try {
      await loadScript('afp.js')
      g.__afp_runtime_loaded__ = true
    } catch {}
  }
}

async function resampleTo8kMono(audioBuffer: AudioBuffer): Promise<Float32Array> {
  const ctx = new (window.OfflineAudioContext || (window as any).webkitOfflineAudioContext)(
    1,
    Math.floor(audioBuffer.duration * 8000),
    8000
  )
  const source = ctx.createBufferSource()
  source.buffer = audioBuffer
  source.connect(ctx.destination)
  source.start()
  const renderedBuffer = await ctx.startRendering()
  return renderedBuffer.getChannelData(0)
}

async function queryNetease(fp: string, duration: number) {
  const params = new URLSearchParams({
    sessionId: crypto.randomUUID?.() || Math.random().toString(36).slice(2),
    algorithmCode: 'shazam_v2',
    duration: String(duration),
    rawdata: fp,
    times: '1',
    decrypt: '1'
  })
  const url = `https://interface.music.163.com/api/music/audio/match?${params.toString()}`

  // Use Tauri http_proxy to bypass CORS
  const resp = await invoke<any>('http_proxy', {
    args: {
      url,
      method: 'POST',
      timeout: 15000
    }
  })

  // http_proxy wraps response in { statusCode, headers, body }
  const json = resp.body ?? resp
  return json
}

function parseNeteaseTypes(song: any): string[] {
  const types: string[] = []
  if (song?.hrMusic?.bitrate || song?.hrMusic?.id) types.push('hires')
  if (song?.sqMusic?.bitrate || song?.sqMusic?.id) types.push('flac')
  if (song?.hMusic?.bitrate || song?.hMusic?.id) types.push('320k')
  if (song?.mMusic?.bitrate || song?.mMusic?.id) types.push('320k')
  if (song?.lMusic?.bitrate || song?.lMusic?.id) types.push('128k')
  return [...new Set(types)]
}

function parseResult(resp: any): any[] {
  const code = resp?.code
  const data = resp?.data
  const list = data?.result
  if (!Array.isArray(list) || list.length === 0) return []
  return list.map((item: any) => {
    const album = item?.song?.album
    let img =
      album?.picUrl ||
      album?.cover ||
      album?.img1v1Url ||
      ''
    if (img && !img.startsWith('http')) {
      img = `https://p1.music.126.net/${img}/${img}.jpg`
    }
    if (img && !img.includes('param=')) {
      img += '?param=100y100'
    }
    return {
      songmid: String(item?.song?.id || ''),
      name: String(item?.song?.name || ''),
      singer: (item?.song?.artists || []).map((a: any) => a.name).join(' / '),
      albumName: String(album?.name || ''),
      img: img.replace(/^http:/, 'https:'),
      source: 'wy',
      types: parseNeteaseTypes(item?.song),
      startTime: item?.startTime || 0
    }
  })
}

async function start() {
  if (running.value) return

  try {
    status.value = 'initializing'
    await ensureAFP()

    const granted = await invoke<boolean>('request_mic_permission')
    if (!granted) {
      MessagePlugin.error(t('music.recognize.needMic'))
      reset()
      return
    }

    if (audioStore.Audio.isPlay) {
      wasPlaying.value = true
      await audioStore.stop()
    } else {
      wasPlaying.value = false
    }

    running.value = true
    status.value = 'recording'
    currentDuration.value = 0
    recognizedSongs.value = []

    // Listen for Rust capture errors
    unlistenError = await listen<{ error: string }>('audio-capture:error', (event) => {
      stopRecording(false)
      const msg = event.payload.error
      if (msg.includes('未找到') || msg.includes('麦克风设备')) {
        MessagePlugin.warning(t('music.recognize.noMic'))
      } else {
        MessagePlugin.error(msg)
      }
    })

    // Listen for audio level updates
    unlistenLevel = await listen<{ level: number }>('audio-capture:level', (event) => {
      audioLevel = event.payload.level
    })

    let chunkCount = 0
    unlistenChunk = await listen<{ data: number[]; sampleRate: number }>(
      'audio-capture:chunk',
      async (event) => {
        chunkCount++
        const pcm8k = new Float32Array(event.payload.data)
        await tryRecognizePCM(pcm8k)
      }
    )

    await invoke('audio_capture_start', { chunkDurationMs: 3000 })

    await nextTick()
    startVisualizerLoop()

    const startTime = Date.now()
    timer = setInterval(() => {
      const elapsed = (Date.now() - startTime) / 1000
      currentDuration.value = Math.floor(elapsed)
      if (elapsed >= MAX_DURATION) {
        stopRecording(false)
      }
    }, 1000)
  } catch {
    MessagePlugin.error(t('music.recognize.startFailed'))
    reset()
  }
}

async function tryRecognizePCM(pcm8k: Float32Array) {
  if (!running.value) return

  try {
    // Sound detection
    let hasSound = false
    let maxAmp = 0
    let soundSamples = 0
    for (let i = 0; i < pcm8k.length; i += 100) {
      const amp = Math.abs(pcm8k[i])
      if (amp > maxAmp) maxAmp = amp
      if (amp > 0.002) soundSamples++
      if (amp > 0.002) {
        hasSound = true
      }
    }
    if (!hasSound) return

    const gen = (window as any).GenerateFP
    if (typeof gen !== 'function') return

    const fp = await gen(pcm8k)
    if (!fp) return

    const duration = pcm8k.length / 8000
    const resp = await queryNetease(fp, duration)
    const result = parseResult(resp)

    if (result.length > 0) {
      recognizedSongs.value = result
      status.value = 'success'
      addToHistory(result)
      stopRecording(true)
    }
  } catch {}
}

async function stopRecording(success: boolean = false) {
  if (!running.value) return

  try {
    await invoke('audio_capture_stop')
  } catch {}

  stopVisualizerLoop()
  audioLevel = 0
  smoothLevel = 0

  if (unlistenLevel) { unlistenLevel(); unlistenLevel = null }
  if (unlistenChunk) { unlistenChunk(); unlistenChunk = null }
  if (unlistenError) { unlistenError(); unlistenError = null }
  if (timer) { clearInterval(timer); timer = null }

  running.value = false
  if (!success) status.value = 'failed'

  if (wasPlaying.value) {
    setTimeout(() => audioStore.start(), 500)
    wasPlaying.value = false
  }
}

const fileInput = ref<HTMLInputElement | null>(null)

function triggerUpload() {
  fileInput.value?.click()
}

async function onFilePicked(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!file) return
  if (running.value) return

  status.value = 'processing'
  running.value = true
  recognizedSongs.value = []

  try {
    await ensureAFP()
    const arrayBuffer = await file.arrayBuffer()
    const ctx = new (window.AudioContext || (window as any).webkitAudioContext)()

    try {
      const audioBuffer = await ctx.decodeAudioData(arrayBuffer)

      const pcm8k = await resampleTo8kMono(audioBuffer)

      const targetLength = MAX_DURATION * 8000
      const slice = new Float32Array(Math.min(pcm8k.length, targetLength))
      slice.set(pcm8k.subarray(0, slice.length))

      const gen = (window as any).GenerateFP
      if (typeof gen === 'function') {
        const fp = await gen(slice)
        if (!fp) {
          status.value = 'failed'
          MessagePlugin.warning(t('music.recognize.notRecognized'))
          return
        }
        const resp = await queryNetease(fp, slice.length / 8000)
        const result = parseResult(resp)
        if (result.length > 0) {
          recognizedSongs.value = result
          status.value = 'success'
          addToHistory(result)
        } else {
          status.value = 'failed'
          MessagePlugin.warning(t('music.recognize.notRecognized'))
        }
      } else {
        status.value = 'failed'
        MessagePlugin.error(t('music.recognize.moduleNotLoaded'))
      }
    } finally {
      ctx.close()
    }
  } catch {
    status.value = 'failed'
    MessagePlugin.error(t('music.recognize.recognizeFailed'))
  } finally {
    running.value = false
  }
}

function reset() {
  running.value = false
  status.value = ''
  currentDuration.value = 0
  audioLevel = 0
  smoothLevel = 0

  stopVisualizerLoop()

  try { invoke('audio_capture_stop') } catch {}
  if (unlistenLevel) { unlistenLevel(); unlistenLevel = null }
  if (unlistenChunk) { unlistenChunk(); unlistenChunk = null }
  if (unlistenError) { unlistenError(); unlistenError = null }
  if (timer) { clearInterval(timer); timer = null }
}

function backToInitial() {
  reset()
  recognizedSongs.value = []
}

async function handlePlayResult(song: any) {
  if (!song) return
  localUserStore.addSongToFirst(song)
  await playSong(song)

  if (song.startTime && song.startTime > 0) {
    const seconds = song.startTime / 1000
    setTimeout(async () => {
      audioStore.setCurrentTime(seconds)
      try {
        await invoke('player__seek', { position: seconds })
      } catch {}
      MessagePlugin.success(t('music.recognize.jumpedTo', { time: formatTime(seconds) }))
    }, 500)
  }
}

function handleSearchResult(song: any) {
  if (!song) return
  searchStore.setValue(song.name)
  router.push({ name: 'search' })
}

function handlePlayHistory(item: HistoryItem) {
  handlePlayResult({
    songmid: item.songmid,
    name: item.name,
    singer: item.singer,
    img: item.img,
    albumName: item.albumName
  })
}

function formatTime(seconds: number) {
  const m = Math.floor(seconds / 60)
  const s = Math.floor(seconds % 60)
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
}

onUnmounted(() => {
  reset()
})
</script>

<template>
  <div class="recognize-page">
    <div class="recognize-container">
      <!-- Recognition Stage -->
      <section class="recognition-stage">
        <div class="viz-area" :class="{ 'is-active': running }">
          <!-- Background glow -->
          <div class="viz-glow"></div>
          <!-- Ping ring -->
          <div class="viz-ring ping-ring"></div>
          <!-- Static ring -->
          <div class="viz-ring static-ring"></div>
          <!-- Ripple waves -->
          <div class="viz-wave w1"></div>
          <div class="viz-wave w2"></div>
          <div class="viz-wave w3"></div>
          <!-- Canvas visualizer -->
          <canvas ref="canvasRef" class="viz-canvas"></canvas>
          <!-- Center button -->
          <button
            class="center-btn"
            :class="{
              'is-recording': running,
              'is-loading': status === 'initializing' || status === 'processing'
            }"
            :disabled="status === 'initializing' || status === 'processing'"
            :aria-label="running ? t('music.recognize.stopRecognize') : t('music.recognize.startRecognize')"
            :aria-busy="status === 'initializing' || status === 'processing'"
            @click="running ? stopRecording(false) : start()"
          >
            <StopCircleIcon v-if="running" size="48px" />
            <t-loading v-else-if="status === 'initializing' || status === 'processing'" size="48px" />
            <MicrophoneIcon v-else size="48px" />
          </button>
        </div>

        <!-- Status text -->
        <div class="status-area">
          <template v-if="running">
            <h3 class="status-title">
              <span class="recording-dot"></span>
              {{ t('music.recognize.recognizing') }}
            </h3>
            <p class="status-desc">{{ t('music.recognize.recognizingTip') }}</p>
            <div class="progress-bar">
              <div class="progress-fill" :style="{ transform: `scaleX(${currentDuration / MAX_DURATION})` }"></div>
            </div>
            <span class="progress-time">{{ currentDuration }}s / {{ MAX_DURATION }}s</span>
          </template>
          <template v-else-if="status === 'failed'">
            <h3 class="status-title error">{{ t('music.recognize.notFound') }}</h3>
            <div class="failed-actions">
              <button class="action-chip" @click="start">
                <RefreshIcon size="16px" /> {{ t('common.retry') }}
              </button>
            </div>
          </template>
          <template v-else-if="!recognizedSongs.length">
            <h3 class="status-title">{{ t('music.recognize.tapToStart') }}</h3>
            <p class="status-desc">{{ t('music.recognize.recordTip') }}</p>
            <div class="upload-chip" @click="triggerUpload">
              <UploadIcon size="16px" /> {{ t('music.recognize.orUpload') }}
            </div>
          </template>
        </div>
        <input ref="fileInput" type="file" accept="audio/*" hidden @change="onFilePicked" />
      </section>

      <!-- Results Section -->
      <section v-if="recognizedSongs.length > 0" class="results-section">
        <div class="result-list">
          <div
            v-for="(song, idx) in recognizedSongs"
            :key="song.songmid"
            class="result-card"
            :style="{ animationDelay: `${idx * 0.1}s` }"
          >
            <div class="result-cover-wrapper">
              <img :src="song.img" class="result-cover" :alt="song.name" loading="lazy" />
            </div>
            <div class="result-info">
              <h4 class="result-name">{{ song.name }}</h4>
              <p class="result-artist">{{ song.singer }}</p>
              <span v-if="song.startTime > 0" class="result-tag">
                {{ t('music.recognize.segment', { time: formatTime(song.startTime / 1000) }) }}
              </span>
            </div>
            <div class="result-actions">
              <button class="result-btn primary" :aria-label="`${t('common.play')} ${song.name}`" @click="handlePlayResult(song)">
                <PlayCircleIcon size="22px" />
              </button>
              <button class="result-btn" :aria-label="`${t('common.search')} ${song.name}`" @click="handleSearchResult(song)">
                <SearchIcon size="18px" />
              </button>
            </div>
          </div>
        </div>
        <div class="result-footer">
          <button class="action-chip" @click="backToInitial">
            <RefreshIcon size="16px" /> {{ t('music.recognize.continueRecognize') }}
          </button>
        </div>
      </section>

      <!-- History Section -->
      <section v-if="showHistory" class="history-section">
        <div class="history-header">
          <h4 class="history-title">
            <TimeIcon size="18px" />
            {{ t('music.recognize.history') }}
          </h4>
          <button class="history-clear" @click="clearHistory">
            <DeleteIcon size="14px" />
            {{ t('music.recognize.clearHistory') }}
          </button>
        </div>
        <div class="history-grid">
          <div
            v-for="(item, idx) in recentHistory"
            :key="`${item.songmid}-${idx}`"
            class="history-card"
            @click="handlePlayHistory(item)"
          >
            <div class="history-cover-wrapper">
              <img :src="item.img" class="history-cover" :alt="item.name" loading="lazy" />
            </div>
            <div class="history-info">
              <h5 class="history-name">{{ item.name }}</h5>
              <p class="history-artist">{{ item.singer }}</p>
            </div>
            <div class="history-meta">
              <span class="history-time">{{ formatHistoryTime(item.timestamp) }}</span>
              <PlayCircleIcon size="20px" class="history-play" />
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
/* ===== Page Layout ===== */
.recognize-page {
  height: 100%;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  overflow: hidden;
}

.recognize-container {
  width: 100%;
  height: 100%;
  max-width: 960px;
  margin: 0 auto;
  padding: 24px 32px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  overflow-x: hidden;
}

.recognize-container::-webkit-scrollbar {
  width: 6px;
}
.recognize-container::-webkit-scrollbar-thumb {
  background-color: var(--td-scrollbar-color);
  border-radius: 3px;
}
.recognize-container::-webkit-scrollbar-track {
  background: transparent;
}

/* ===== Recognition Stage ===== */
.recognition-stage {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 24px 0 16px;
  flex-shrink: 0;
}

/* ===== Visualizer Area ===== */
.viz-area {
  position: relative;
  width: 280px;
  height: 280px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 24px;
}

.viz-glow {
  position: absolute;
  inset: -20px;
  background: radial-gradient(
    circle,
    rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.08) 0%,
    transparent 70%
  );
  border-radius: 50%;
  opacity: 0;
  transition: opacity 0.6s ease;
}

.is-active .viz-glow {
  opacity: 1;
}

.viz-ring {
  position: absolute;
  border-radius: 50%;
  pointer-events: none;
}

.ping-ring {
  inset: 0;
  border: 3px solid rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.15);
  opacity: 0;
  transition: opacity 0.3s;
}

.is-active .ping-ring {
  opacity: 1;
  will-change: transform, opacity; animation: viz-ping 2.5s cubic-bezier(0, 0, 0.2, 1) infinite;
}

.static-ring {
  inset: 16px;
  border: 2px solid rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.25);
  opacity: 0;
  transition: opacity 0.3s;
}

.is-active .static-ring {
  opacity: 1;
}

@keyframes viz-ping {
  0% {
    transform: scale(1);
    opacity: 0.6;
  }
  75%,
  100% {
    transform: scale(1.15);
    opacity: 0;
  }
}

/* Ripple waves */
.viz-wave {
  position: absolute;
  width: 120px;
  height: 120px;
  border-radius: 50%;
  border: 1.5px solid rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.3);
  opacity: 0;
  pointer-events: none;
}

.is-active .viz-wave {
  will-change: transform, opacity; animation: wave-ripple 3s ease-out infinite;
}
.is-active .w2 { animation-delay: 0.75s; }
.is-active .w3 { animation-delay: 1.5s; }

@keyframes wave-ripple {
  0% {
    transform: scale(1);
    opacity: 0.4;
  }
  100% {
    transform: scale(2.33);
    opacity: 0;
  }
}

/* Canvas */
.viz-canvas {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 1;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.5s ease;
}

.is-active .viz-canvas {
  opacity: 1;
}

/* Center button */
.center-btn {
  position: relative;
  z-index: 10;
  width: 120px;
  height: 120px;
  border-radius: 50%;
  border: none;
  background: var(--td-brand-color);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  box-shadow: 0 4px 16px rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.3);
  transition: background-color 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275), border-color 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275), color 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275), box-shadow 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275), opacity 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275), transform 0.5s cubic-bezier(0.175, 0.885, 0.32, 1.275);
  outline: none;
}

.center-btn:hover {
  transform: scale(1.05);
  box-shadow: 0 6px 16px rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.4);
}

.center-btn:active {
  transform: scale(0.95);
}

.center-btn.is-recording {
  transform: scale(1.08);
  box-shadow:
    0 0 0 4px rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.15),
    0 6px 16px rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.35);
  will-change: transform, opacity; animation: btn-breathe 2s ease-in-out infinite;
}

.center-btn.is-loading {
  pointer-events: none;
  opacity: 0.8;
}

.center-btn:disabled {
  cursor: not-allowed;
}

@keyframes btn-breathe {
  0%, 100% { box-shadow: 0 0 0 0 rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.3); }
  50% { box-shadow: 0 0 0 12px rgba(var(--td-brand-color-rgb, 3, 222, 109), 0); }
}

/* ===== Status Area ===== */
.status-area {
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.status-title {
  font-size: 24px;
  font-weight: 700;
  color: var(--td-text-color-primary);
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  letter-spacing: -0.3px;
}

.status-title.error {
  color: var(--td-error-color);
}

.status-desc {
  font-size: 14px;
  color: var(--td-text-color-secondary);
  margin: 0;
  max-width: 320px;
}

.recording-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--td-error-color);
  will-change: opacity; animation: pulse-dot 1.2s ease-in-out infinite;
}

@keyframes pulse-dot {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(0.8); }
}

.progress-bar {
  width: 200px;
  height: 4px;
  background: var(--td-bg-color-component);
  border-radius: 2px;
  overflow: hidden;
  margin-top: 4px;
}

.progress-fill {
  width: 100%;
  height: 100%;
  background: var(--td-brand-color);
  border-radius: 2px;
  transform-origin: left center;
  transition: transform 1s linear;
}

.progress-time {
  font-size: 12px;
  color: var(--td-text-color-placeholder);
  margin-top: 2px;
}

/* Action chips */
.action-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 16px;
  border-radius: 999px;
  border: 1px solid var(--td-border-level-2-color);
  background: var(--td-bg-color-container);
  color: var(--td-brand-color);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
}

.action-chip:hover {
  background: var(--td-brand-color-light);
  border-color: var(--td-brand-color-light);
}

.upload-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 16px;
  border-radius: 999px;
  border: 1px dashed var(--td-border-level-2-color);
  color: var(--td-text-color-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
}

.upload-chip:hover {
  border-color: var(--td-brand-color);
  color: var(--td-brand-color);
  background: var(--td-brand-color-light);
}

.failed-actions {
  display: flex;
  gap: 12px;
  margin-top: 8px;
}

/* ===== Results Section ===== */
.results-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
  animation: fadeIn 0.4s ease;
}

.result-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.result-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px;
  background: var(--td-bg-color-container);
  border: 1px solid var(--td-border-level-1-color);
  border-radius: 16px;
  cursor: pointer;
  transition: background-color 0.25s ease, border-color 0.25s ease, color 0.25s ease, box-shadow 0.25s ease, opacity 0.25s ease, transform 0.25s ease;
  animation: slideUp 0.45s cubic-bezier(0.22, 1, 0.36, 1) both;
}

.result-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 16px rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.1);
  border-color: rgba(var(--td-brand-color-rgb, 3, 222, 109), 0.2);
}

.result-cover-wrapper {
  width: 72px;
  height: 72px;
  border-radius: 12px;
  overflow: hidden;
  flex-shrink: 0;
}

.result-cover {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: transform 0.4s ease;
}

.result-card:hover .result-cover {
  transform: scale(1.08);
}

.result-info {
  flex: 1;
  min-width: 0;
  text-align: left;
}

.result-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--td-text-color-primary);
  margin: 0 0 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.result-artist {
  font-size: 14px;
  color: var(--td-text-color-secondary);
  margin: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.result-tag {
  display: inline-block;
  margin-top: 6px;
  padding: 2px 8px;
  font-size: 12px;
  color: var(--td-brand-color);
  background: var(--td-brand-color-light);
  border-radius: 4px;
}

.result-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.result-btn {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: 1px solid var(--td-border-level-2-color);
  background: var(--td-bg-color-container);
  color: var(--td-text-color-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
}

.result-btn:hover {
  color: var(--td-brand-color);
  border-color: var(--td-brand-color);
  background: var(--td-brand-color-light);
}

.result-btn.primary {
  width: 42px;
  height: 42px;
  background: var(--td-brand-color);
  color: #fff;
  border: none;
}

.result-btn.primary:hover {
  background: var(--td-brand-color-hover);
  color: #fff;
}

.result-footer {
  display: flex;
  justify-content: center;
  padding-top: 4px;
}

/* ===== History Section ===== */
.history-section {
  padding-top: 8px;
  border-top: 1px solid var(--td-border-level-1-color);
  min-width: 0;
  max-width: 100%;
}

.history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.history-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 700;
  color: var(--td-text-color-primary);
  margin: 0;
}

.history-clear {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border-radius: 999px;
  border: none;
  background: transparent;
  color: var(--td-text-color-placeholder);
  font-size: 12px;
  cursor: pointer;
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
}

.history-clear:hover {
  color: var(--td-error-color);
  background: rgba(var(--td-error-color-rgb, 180, 26, 26), 0.06);
}

.history-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  min-width: 0;
  max-width: 100%;
}

.history-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  min-width: 0;
  max-width: 100%;
  box-sizing: border-box;
  background: var(--td-bg-color-container);
  border: 1px solid var(--td-border-level-1-color);
  border-radius: 14px;
  cursor: pointer;
  transition: background-color 0.25s ease, border-color 0.25s ease, color 0.25s ease, box-shadow 0.25s ease, opacity 0.25s ease, transform 0.25s ease;
}

.history-card:hover {
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.06);
  transform: translateY(-2px);
}

.history-cover-wrapper {
  width: 56px;
  height: 56px;
  border-radius: 10px;
  overflow: hidden;
  flex-shrink: 0;
}

.history-cover {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: transform 0.4s ease;
}

.history-card:hover .history-cover {
  transform: scale(1.1);
}

.history-info {
  flex: 1;
  min-width: 0;
}

.history-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--td-text-color-primary);
  margin: 0 0 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.history-artist {
  font-size: 12px;
  color: var(--td-text-color-secondary);
  margin: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.history-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  flex-shrink: 0;
}

.history-time {
  font-size: 11px;
  color: var(--td-text-color-placeholder);
  white-space: nowrap;
}

.history-play {
  color: var(--td-brand-color);
  opacity: 0;
  transition: opacity 0.2s;
}

.history-card:hover .history-play {
  opacity: 1;
}

/* ===== Animations ===== */
@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(16px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@media (max-width: 768px) {
  .recognize-page {
    min-width: 0;
    align-items: stretch;
    overflow: hidden;
  }

  .recognize-container {
    min-width: 0;
    max-width: 100vw;
    padding: var(--mobile-page-top-gutter) var(--mobile-page-gutter) calc(var(--mobile-page-top-gutter) + 4px);
    gap: 14px;
    box-sizing: border-box;
    -webkit-overflow-scrolling: touch;
  }

  .recognition-stage {
    padding: 8px 0 12px;
  }

  .viz-area {
    width: min(74vw, 280px);
    height: min(74vw, 280px);
    max-width: 100%;
    margin-bottom: 18px;
  }

  .viz-wave {
    width: 43%;
    height: 43%;
  }

  @keyframes wave-ripple {
    0% {
      transform: scale(1);
      opacity: 0.4;
    }
    100% {
      transform: scale(2.33);
      opacity: 0;
    }
  }

  .center-btn {
    width: clamp(88px, 32vw, 120px);
    height: clamp(88px, 32vw, 120px);
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    touch-action: manipulation;
  }

  .status-title {
    font-size: clamp(1.5rem, 7vw, 2rem);
    line-height: 1.15;
  }

  .status-desc {
    max-width: 100%;
    font-size: 15px;
    line-height: 1.45;
  }

  .action-chip,
  .upload-chip,
  .history-clear {
    min-height: var(--mobile-touch-target);
    padding: 0 16px;
    touch-action: manipulation;
  }

  .progress-bar {
    width: min(72vw, 260px);
  }

  .results-section,
  .result-list,
  .history-section,
  .history-grid {
    min-width: 0;
    max-width: 100%;
  }

  .result-card {
    gap: 12px;
    padding: 12px;
    border-radius: var(--mobile-card-radius-small);
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
    touch-action: manipulation;
  }

  .result-cover-wrapper {
    width: 60px;
    height: 60px;
    border-radius: 12px;
  }

  .result-actions {
    gap: 6px;
  }

  .result-info {
    min-width: 0;
  }

  .result-tag {
    max-width: 100%;
    white-space: normal;
    overflow-wrap: anywhere;
  }

  .result-actions {
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .result-btn,
  .result-btn.primary {
    width: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    touch-action: manipulation;
  }

  .history-grid {
    grid-template-columns: 1fr;
  }

  .history-card {
    min-height: 72px;
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
    border-radius: var(--mobile-card-radius-small);
    touch-action: manipulation;
  }

  .history-play {
    opacity: 1;
  }
}

@media (max-width: 360px) {
  .recognize-container {
    padding-right: max(12px, env(safe-area-inset-right, 0px));
    padding-left: max(12px, env(safe-area-inset-left, 0px));
    gap: 12px;
  }

  .recognition-stage {
    padding-top: 4px;
  }

  .viz-area {
    width: min(68vw, 232px);
    height: min(68vw, 232px);
    margin-bottom: 14px;
  }

  .status-title {
    font-size: clamp(1.25rem, 6.2vw, 1.5rem);
  }

  .result-card {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr);
    align-items: center;
    gap: 10px 12px;
  }

  .result-cover-wrapper {
    width: 56px;
    height: 56px;
  }

  .result-actions {
    grid-column: 1 / -1;
    justify-content: stretch;
    gap: 8px;
  }

  .result-btn,
  .result-btn.primary {
    flex: 1 1 0;
    border-radius: 999px;
  }

  .history-card {
    gap: 10px;
    padding: 10px;
  }

  .history-cover-wrapper {
    width: 48px;
    height: 48px;
  }
}

@media (prefers-reduced-motion: reduce) {
  .is-active .ping-ring,
  .is-active .viz-wave,
  .center-btn.is-recording,
  .recording-dot,
  .result-card {
    animation: none;
  }
}
</style>
