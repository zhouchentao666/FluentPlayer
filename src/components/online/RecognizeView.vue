<script setup lang="ts">
import { ref, onUnmounted, computed } from 'vue'
import type { Song } from '../../types'
import { generateFingerprint, recognizeByFingerprint, type RecognizeCandidate } from '@online/lib/recognize'
import { toast } from '../../composables/useToast'

const emit = defineEmits<{
  (e: 'play', song: Song): void
  (e: 'queue', song: Song): void
  (e: 'add-playlist', song: Song): void
  (e: 'download', song: Song): void
  (e: 'back'): void
}>()

type State = 'idle' | 'recording' | 'recognizing' | 'done' | 'error'

const state = ref<State>('idle')
const candidates = ref<RecognizeCandidate[]>([])
const errorMsg = ref('')
const elapsed = ref(0)
const level = ref(0) // 录音音量可视化

let mediaStream: MediaStream | null = null
let mediaRecorder: MediaRecorder | null = null
let audioChunks: Blob[] = []
let audioContext: AudioContext | null = null
let analyser: AnalyserNode | null = null
let rafId = 0
let timerId: ReturnType<typeof setInterval> | null = null

const isRecording = computed(() => state.value === 'recording')
const isBusy = computed(() => state.value === 'recording' || state.value === 'recognizing')

const elapsedText = computed(() => {
  const s = elapsed.value
  const m = Math.floor(s / 60)
  const sec = s % 60
  return `${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}`
})

async function startRecording() {
  try {
    mediaStream = await navigator.mediaDevices.getUserMedia({ audio: true })
  } catch {
    toast('无法访问麦克风，请检查权限', 'error')
    return
  }
  audioChunks = []
  audioContext = new AudioContext()
  const source = audioContext.createMediaStreamSource(mediaStream)
  analyser = audioContext.createAnalyser()
  analyser.fftSize = 256
  source.connect(analyser)

  try {
    mediaRecorder = new MediaRecorder(mediaStream)
  } catch {
    toast('当前环境不支持录音', 'error')
    cleanup()
    return
  }
  mediaRecorder.ondataavailable = (e) => {
    if (e.data.size > 0) audioChunks.push(e.data)
  }
  mediaRecorder.onstop = () => {
    const blob = new Blob(audioChunks, { type: 'audio/webm' })
    void processAudioBlob(blob)
  }
  mediaRecorder.start()
  state.value = 'recording'
  elapsed.value = 0
  startVisualizer()
  timerId = setInterval(() => {
    elapsed.value++
    if (elapsed.value >= 20) stopRecording() // 最长 20s
  }, 1000)
}

function startVisualizer() {
  if (!analyser) return
  const data = new Uint8Array(analyser.frequencyBinCount)
  const tick = () => {
    if (!analyser) return
    analyser.getByteFrequencyData(data)
    const avg = data.reduce((a, b) => a + b, 0) / data.length
    level.value = Math.min(1, avg / 160)
    rafId = requestAnimationFrame(tick)
  }
  tick()
}

function stopRecording() {
  if (timerId) {
    clearInterval(timerId)
    timerId = null
  }
  if (mediaRecorder && mediaRecorder.state !== 'inactive') {
    mediaRecorder.stop()
  } else {
    void processAudioBlob(new Blob(audioChunks, { type: 'audio/webm' }))
  }
  stopVisualizer()
}

function stopVisualizer() {
  if (rafId) cancelAnimationFrame(rafId)
  rafId = 0
  level.value = 0
}

async function onPickFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  await processAudioBlob(file)
  input.value = ''
}

async function processAudioBlob(blob: Blob) {
  if (blob.size === 0) {
    toast('录音为空，请重试', 'error')
    state.value = 'idle'
    return
  }
  state.value = 'recognizing'
  try {
    const arrayBuffer = await blob.arrayBuffer()
    const ctx = audioContext ?? new AudioContext()
    audioContext = ctx
    const audioBuffer = await ctx.decodeAudioData(arrayBuffer)
    const durationSec = audioBuffer.duration
    const fp = await generateFingerprint(audioBuffer)
    const result = await recognizeByFingerprint(fp, durationSec)
    candidates.value = result
    if (result.length === 0) {
      state.value = 'done'
      toast('未识别到歌曲，请靠近音源重试', 'info')
    } else {
      state.value = 'done'
      toast(`识别到 ${result.length} 首候选`, 'success')
    }
  } catch (err) {
    state.value = 'error'
    errorMsg.value = (err as Error).message || '识别失败'
    toast(errorMsg.value, 'error')
  } finally {
    cleanup()
  }
}

function cleanup() {
  if (mediaStream) {
    mediaStream.getTracks().forEach((t) => t.stop())
    mediaStream = null
  }
  if (audioContext && audioContext.state !== 'closed') {
    audioContext.close().catch(() => {})
  }
  audioContext = null
  analyser = null
  mediaRecorder = null
}

function onPlay(song: Song) {
  emit('play', song)
}
function onQueue(song: Song) {
  emit('queue', song)
}
function onAddPlaylist(song: Song) {
  emit('add-playlist', song)
}
function onDownload(song: Song) {
  emit('download', song)
}

function reset() {
  state.value = 'idle'
  candidates.value = []
  errorMsg.value = ''
}

onUnmounted(() => {
  if (timerId) clearInterval(timerId)
  stopVisualizer()
  cleanup()
})
</script>

<template>
  <div class="recognize-view">
    <div class="head">
      <button class="back" @click="emit('back')">← 返回</button>
      <div class="title">听歌识曲</div>
    </div>

    <div class="panel">
      <div class="mic-wrap" :class="{ active: isRecording, busy: isBusy }">
        <div class="mic-ring" :style="{ '--lv': level }">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="3" width="6" height="11" rx="3" />
            <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
          </svg>
        </div>
        <div class="state-text">
          <template v-if="state === 'idle'">点击麦克风，对着正在播放的音乐录制</template>
          <template v-else-if="state === 'recording'">录音中… {{ elapsedText }}</template>
          <template v-else-if="state === 'recognizing'">正在识别…</template>
          <template v-else-if="state === 'done'">识别完成</template>
          <template v-else>识别失败</template>
        </div>
      </div>

      <div class="controls">
        <button
          v-if="!isRecording"
          class="ctrl primary"
          :disabled="state === 'recognizing'"
          @click="startRecording"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="3" width="6" height="11" rx="3" />
            <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
          </svg>
          开始识别
        </button>
        <button v-else class="ctrl danger" @click="stopRecording">
          <svg viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2" /></svg>
          停止
        </button>

        <label class="ctrl file">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 3v12M7 10l5 5 5-5M5 21h14" />
          </svg>
          上传音频
          <input type="file" accept="audio/*" hidden @change="onPickFile" />
        </label>

        <button v-if="state === 'done' || state === 'error'" class="ctrl" @click="reset">重新识别</button>
      </div>

      <div v-if="state === 'error'" class="error-msg">{{ errorMsg }}</div>
    </div>

    <div v-if="candidates.length" class="result">
      <div class="result-title">识别结果</div>
      <div
        v-for="(c, i) in candidates"
        :key="c.song.id"
        class="result-item"
        :class="{ top: i === 0 }"
      >
        <div class="cover" v-if="c.song.cover || c.song.online?.meta?.picUrl">
          <img
            :src="c.song.cover || c.song.online?.meta?.picUrl || ''"
            @error="(e: any) => (e.target.style.visibility = 'hidden')"
          />
        </div>
        <div class="cover placeholder" v-else>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="9" /><path d="M9 9l5 3-5 3z" fill="currentColor" stroke="none" /></svg>
        </div>
        <div class="meta">
          <div class="name">{{ c.song.title }}</div>
          <div class="sub">{{ c.song.metadata?.artist }} <span v-if="c.song.metadata?.album">· {{ c.song.metadata.album }}</span></div>
        </div>
        <div class="ops">
          <button class="op" title="播放" @click="onPlay(c.song)">
            <svg viewBox="0 0 16 16"><path d="M4 2.5v11l9-5.5z" fill="currentColor" /></svg>
          </button>
          <button class="op" title="加入队列" @click="onQueue(c.song)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg>
          </button>
          <button class="op" title="收藏到歌单" @click="onAddPlaylist(c.song)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14" transform="rotate(45 12 12)" /></svg>
          </button>
          <button class="op" title="下载" @click="onDownload(c.song)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12M7 10l5 5 5-5M5 21h14" /></svg>
          </button>
        </div>
      </div>
    </div>

    <div v-else-if="state === 'done'" class="empty">没有匹配的歌曲，换个更清晰的环境试试。</div>
  </div>
</template>

<style scoped>
.recognize-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 18px 28px;
  overflow-y: auto;
}
.head {
  display: flex;
  align-items: center;
  gap: 16px;
}
.back {
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  cursor: pointer;
  font-size: 14px;
}
.back:hover {
  color: var(--fluent-text);
}
.title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fluent-text);
}
.panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 22px;
  padding: 40px 0 24px;
}
.mic-wrap {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
}
.mic-ring {
  width: 120px;
  height: 120px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--fluent-accent);
  background: var(--fluent-bg-card);
  border: 2px solid var(--fluent-border);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}
.mic-ring svg {
  width: 46px;
  height: 46px;
}
.mic-wrap.active .mic-ring {
  border-color: var(--fluent-accent);
  transform: scale(calc(1 + var(--lv) * 0.12));
  box-shadow: 0 0 calc(20px + var(--lv) * 40px) var(--fluent-accent);
  animation: pulse 1.4s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { box-shadow: 0 0 18px var(--fluent-accent); }
  50% { box-shadow: 0 0 38px var(--fluent-accent); }
}
.state-text {
  color: var(--fluent-text-secondary);
  font-size: 14px;
  text-align: center;
  max-width: 320px;
}
.controls {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  justify-content: center;
}
.ctrl {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 40px;
  padding: 0 18px;
  border-radius: 12px;
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 14px;
  cursor: pointer;
}
.ctrl:hover {
  background: var(--fluent-bg-hover);
}
.ctrl.primary {
  background: var(--fluent-accent);
  color: #fff;
  border-color: var(--fluent-accent);
}
.ctrl.danger {
  background: #ef4444;
  color: #fff;
  border-color: #ef4444;
}
.ctrl:disabled {
  opacity: 0.5;
  cursor: default;
}
.ctrl svg {
  width: 18px;
  height: 18px;
}
.file {
  cursor: pointer;
}
.error-msg {
  color: #f87171;
  font-size: 13px;
}
.result {
  margin-top: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.result-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--fluent-text);
  margin-bottom: 4px;
}
.result-item {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 10px 12px;
  border-radius: 12px;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
}
.result-item.top {
  border-color: var(--fluent-accent);
}
.cover {
  width: 48px;
  height: 48px;
  border-radius: 8px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--fluent-bg-hover);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--fluent-text-secondary);
}
.cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.cover.placeholder svg {
  width: 24px;
  height: 24px;
}
.meta {
  flex: 1;
  min-width: 0;
}
.name {
  font-size: 15px;
  font-weight: 600;
  color: var(--fluent-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sub {
  font-size: 12.5px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ops {
  display: inline-flex;
  gap: 6px;
  flex-shrink: 0;
}
.op {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: none;
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.op:hover {
  background: var(--fluent-accent);
  color: #fff;
}
.op svg {
  width: 16px;
  height: 16px;
}
.empty {
  margin-top: 24px;
  text-align: center;
  color: var(--fluent-text-secondary);
  font-size: 14px;
}
</style>
