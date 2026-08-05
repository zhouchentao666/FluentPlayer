<script setup lang="ts">
/**
 * 听歌识曲组件（参考 Mio-Music-main AudioMatch / recognize）。
 * 用浏览器麦克风录 3 秒 → 8kHz 重采样 → 网易云指纹 WASM → audio/match 识曲。
 */
import { ref, computed, onBeforeUnmount } from "vue"
import type { MusicInfo } from "@online/types/music"
import { recognizeSong, type RecordHandle } from "@online/lib/recognition"

const emit = defineEmits<{
  (e: "play", musics: MusicInfo[], index: number): void
  (e: "search", keyword: string): void
  (e: "close"): void
}>()

type Status = "idle" | "loading" | "recording" | "done" | "error"

const status = ref<Status>("idle")
const errorMsg = ref("")
const songs = ref<MusicInfo[]>([])
const elapsed = ref(0)

let handle: RecordHandle | null = null
let timer: number | null = null

const HISTORY_KEY = "fluent-recognize-history"

interface HistoryItem {
  id: string
  name: string
  singer: string
  cover: string | null
  ts: number
}

const history = ref<HistoryItem[]>(loadHistory())

function loadHistory(): HistoryItem[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY)
    return raw ? (JSON.parse(raw) as HistoryItem[]) : []
  } catch {
    return []
  }
}

function saveHistory(items: HistoryItem[]) {
  history.value = items
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(items.slice(0, 30)))
  } catch {
    /* ignore */
  }
}

function addHistory(m: MusicInfo) {
  const item: HistoryItem = {
    id: m.id,
    name: m.name,
    singer: m.singer,
    cover: m.meta?.picUrl ?? null,
    ts: Date.now(),
  }
  const next = [item, ...history.value.filter((h) => h.id !== item.id)]
  saveHistory(next)
}

const statusText = computed(() => {
  switch (status.value) {
    case "loading":
      return "正在初始化音频指纹模块…"
    case "recording":
      return `正在聆听… ${elapsed.value}s`
    case "done":
      return songs.value.length ? `识别到 ${songs.value.length} 首` : "未能识别，换一段更清晰的声音试试"
    case "error":
      return errorMsg.value
    default:
      return "点击麦克风，对着正在播放的音乐录制 3 秒"
  }
})

const recordTip = computed(() => {
  if (status.value === "recording") return "保持安静，正在采集环境声音"
  return "建议在歌曲副歌段落、环境尽量安静时识别"
})

async function start() {
  if (status.value === "recording" || status.value === "loading") return
  errorMsg.value = ""
  songs.value = []
  status.value = "loading"
  try {
    // 先确保指纹模块加载完成再进入录音，避免录音时卡在 await
    status.value = "recording"
    elapsed.value = 0
    timer = window.setInterval(() => {
      elapsed.value++
    }, 1000)

    const result = await recognizeSong(() => {
      if (timer) {
        clearInterval(timer)
        timer = null
      }
    })

    songs.value = result.songs
    status.value = "done"
    result.songs.forEach(addHistory)
  } catch (err: any) {
    status.value = "error"
    errorMsg.value = err?.message || String(err)
  } finally {
    if (timer) {
      clearInterval(timer)
      timer = null
    }
    handle = null
  }
}

function stop() {
  handle?.stop()
  if (timer) {
    clearInterval(timer)
    timer = null
  }
  if (status.value === "recording") status.value = "idle"
}

function play(m: MusicInfo, index: number) {
  emit("play", [m], index)
}

function openSearch(m: MusicInfo) {
  emit("search", `${m.name} ${m.singer}`.trim())
}

function retry() {
  status.value = "idle"
  errorMsg.value = ""
  start()
}

function recover(item: HistoryItem) {
  // 从历史恢复：用搜索兜底（历史仅存基本信息，直接跳搜索更稳妥）
  emit("search", `${item.name} ${item.singer}`.trim())
}

function close() {
  stop()
  emit("close")
}

onBeforeUnmount(() => {
  stop()
})
</script>

<template>
  <div class="audio-match">
    <div class="am-header">
      <div class="am-title">听歌识曲</div>
      <button class="am-close" title="关闭" @click="close">×</button>
    </div>

    <div class="am-stage">
      <button
        class="am-mic"
        :class="{ recording: status === 'recording', busy: status === 'loading' }"
        :disabled="status === 'loading'"
        @click="status === 'recording' ? stop() : start()"
      >
        <svg viewBox="0 0 24 24" width="40" height="40" fill="currentColor">
          <path
            d="M12 14a3 3 0 0 0 3-3V5a3 3 0 0 0-6 0v6a3 3 0 0 0 3 3zm5-3a5 5 0 0 1-10 0H5a7 7 0 0 0 6 6.92V21h2v-3.08A7 7 0 0 0 19 11h-2z"
          />
        </svg>
        <span v-if="status === 'recording'" class="am-ring"></span>
      </button>

      <div class="am-status" :class="status">{{ statusText }}</div>
      <div class="am-tip">{{ recordTip }}</div>

      <div class="am-actions">
        <button
          v-if="status === 'done' && songs.length === 0"
          class="am-btn"
          @click="retry"
        >
          重新识别
        </button>
        <button
          v-if="status === 'error'"
          class="am-btn"
          @click="retry"
        >
          重试
        </button>
      </div>
    </div>

    <div v-if="songs.length" class="am-result">
      <div class="am-label">识别结果</div>
      <div
        v-for="(m, i) in songs"
        :key="m.id"
        class="am-song"
        :class="{ 'is-hover': true }"
      >
        <img
          v-if="m.meta?.picUrl"
          :src="m.meta.picUrl"
          class="am-cover"
          alt=""
        />
        <div v-else class="am-cover am-cover--empty"></div>
        <div class="am-info" @click="play(m, i)">
          <div class="am-name">{{ m.name }}</div>
          <div class="am-singer">{{ m.singer || "未知歌手" }}</div>
        </div>
        <button class="am-mini" title="搜索" @click="openSearch(m)">搜索</button>
        <button class="am-mini am-play" title="播放" @click="play(m, i)">播放</button>
      </div>
    </div>

    <div v-if="history.length" class="am-history">
      <div class="am-label">识曲历史</div>
      <div
        v-for="h in history"
        :key="h.id + h.ts"
        class="am-song is-hover"
        @click="recover(h)"
      >
        <img v-if="h.cover" :src="h.cover" class="am-cover" alt="" />
        <div v-else class="am-cover am-cover--empty"></div>
        <div class="am-info">
          <div class="am-name">{{ h.name }}</div>
          <div class="am-singer">{{ h.singer || "未知歌手" }}</div>
        </div>
        <span class="am-time">{{ new Date(h.ts).toLocaleDateString() }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.audio-match {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  background: transparent;
  color: var(--fluent-text);
  padding: 16px;
  box-sizing: border-box;
}

.am-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.am-title {
  font-size: 18px;
  font-weight: 600;
}
.am-close {
  background: transparent;
  border: none;
  color: var(--fluent-text-secondary);
  font-size: 22px;
  cursor: pointer;
  line-height: 1;
  padding: 4px 8px;
  border-radius: 10px;
}
.am-close:hover {
  background: var(--fluent-bg-hover);
}

.am-stage {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 28px 0 20px;
}

.am-mic {
  position: relative;
  width: 92px;
  height: 92px;
  border-radius: 50%;
  border: none;
  cursor: pointer;
  color: #fff;
  background: var(--fluent-accent, #0a84ff);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 6px 20px rgba(10, 132, 255, 0.35);
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}
.am-mic:hover {
  transform: scale(1.04);
}
.am-mic.busy {
  opacity: 0.7;
  cursor: default;
}
.am-mic.recording {
  background: #ff453a;
  box-shadow: 0 6px 20px rgba(255, 69, 58, 0.35);
}
.am-ring {
  position: absolute;
  inset: -6px;
  border-radius: 50%;
  border: 3px solid rgba(255, 69, 58, 0.5);
  animation: am-pulse 1.4s ease-out infinite;
}
@keyframes am-pulse {
  0% {
    transform: scale(0.9);
    opacity: 0.9;
  }
  100% {
    transform: scale(1.4);
    opacity: 0;
  }
}

.am-status {
  font-size: 15px;
  font-weight: 600;
}
.am-status.error {
  color: #ff453a;
}
.am-tip {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  text-align: center;
  max-width: 280px;
}

.am-actions {
  display: flex;
  gap: 10px;
}
.am-btn {
  background: var(--fluent-bg-hover);
  color: var(--fluent-text-100);
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  padding: 7px 16px;
  cursor: pointer;
  font-size: 13px;
}
.am-btn:hover {
  border-color: var(--fluent-accent, #0a84ff);
  color: var(--fluent-accent, #0a84ff);
}

.am-result,
.am-history {
  margin-top: 8px;
  overflow-y: auto;
}
.am-label {
  font-size: 13px;
  color: var(--fluent-text-secondary);
  margin: 8px 2px;
}

.am-song {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px;
  border-radius: 10px;
  cursor: pointer;
}
.am-song.is-hover:hover {
  background: var(--fluent-bg-hover);
}
.am-cover {
  width: 44px;
  height: 44px;
  border-radius: 6px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--fluent-bg-hover);
}
.am-cover--empty {
  opacity: 0.4;
}
.am-info {
  flex: 1;
  min-width: 0;
}
.am-name {
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.am-singer {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.am-mini {
  background: var(--fluent-bg-hover);
  color: var(--fluent-text-100);
  border: none;
  border-radius: 6px;
  padding: 5px 10px;
  font-size: 12px;
  cursor: pointer;
}
.am-mini:hover {
  color: var(--fluent-accent, #0a84ff);
}
.am-mini.am-play {
  background: var(--fluent-accent, #0a84ff);
  color: #fff;
}
.am-mini.am-play:hover {
  filter: brightness(1.1);
}
.am-time {
  font-size: 11px;
  color: var(--fluent-text-secondary);
  flex-shrink: 0;
}
</style>
