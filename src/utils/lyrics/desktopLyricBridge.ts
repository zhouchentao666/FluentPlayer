import { storeToRefs } from 'pinia'
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { watch, type WatchStopHandle } from 'vue'
import { ControlAudioStore } from '@/store/ControlAudio'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { findCurrentLine, getLineText } from '@/types/lyric'
import type { LyricLine } from '@/types/lyric'

let rafId: number | null = null
let installed = false
let readyUnlisten: UnlistenFn | null = null
let watchStopHandles: WatchStopHandle[] = []

// IPC 节流：行索引/播放状态变化时才发送对应事件
let lastIdx = -1
let lastPlayState = false

// 时间同步节流：每 ~100ms 发送一次进度，桌面端用本地 RAF 时钟插值
const SYNC_INTERVAL_MS = 100
let lastSyncTime = 0

function computeLyricIndex(lines: LyricLine[], timeMs: number): number {
  if (!lines.length) return -1
  for (let i = lines.length - 1; i >= 0; i--) {
    if (timeMs >= lines[i].startTime) return i
  }
  return -1
}

function pushLyricChange(lines: LyricLine[]) {
  emit('desktop-lyric-change', lines).catch(() => {})
}

function pushSongChange(name: string, singer: string) {
  emit('desktop-song-change', { name, singer }).catch(() => {})
}

function pushProgress(currentMs: number) {
  emit('desktop-lyric-progress', {
    currentMs,
    timestamp: performance.now()
  }).catch(() => {})
}

function pushIndexChange(index: number) {
  emit('desktop-lyric-index', index).catch(() => {})
}

function pushPlayStateChange(isPlaying: boolean) {
  emit('desktop-lyric-play-state', isPlaying).catch(() => {})
}

function pushSnapshot(audioStore: ReturnType<typeof ControlAudioStore>, playStatus: ReturnType<typeof useGlobalPlayStatusStore>) {
  const { Audio } = storeToRefs(audioStore)
  const { player } = storeToRefs(playStatus)

  const lines = player.value.lyrics?.lines || []
  pushLyricChange(lines)

  const song = player.value.songInfo as any
  if (song?.name || song?.singer) pushSongChange(song.name, song.singer)

  const currentTimeMs = (Audio.value.currentTime || 0) * 1000
  const idx = computeLyricIndex(lines, currentTimeMs)
  lastIdx = idx
  pushIndexChange(idx)
  pushProgress(currentTimeMs)
  pushPlayStateChange(Audio.value.isPlay)
}

export function installDesktopLyricBridge() {
  if (installed) return
  installed = true

  const audioStore = ControlAudioStore()
  const playStatus = useGlobalPlayStatusStore()
  const { Audio } = storeToRefs(audioStore)
  const { player } = storeToRefs(playStatus)

  // 歌词数据变化 → 发送完整 LyricLine[]
  watchStopHandles = [
    watch(
      () => player.value.lyrics.lines,
      (lines) => {
        lastIdx = -1
        pushLyricChange(lines)
      },
      { immediate: true }
    ),
    watch(
      () => player.value.songInfo,
      (song) => {
        const name = (song as any)?.name || ''
        const singer = (song as any)?.singer || ''
        if (name || singer) pushSongChange(name, singer)
      },
      { immediate: true }
    ),
    watch(
      () => Audio.value.isPlay,
      (isPlay) => {
        pushPlayStateChange(isPlay)
      },
      { immediate: true }
    )
  ]

  // 桌面歌词窗口准备好后，补发一次快照
  listen('desktop-lyric-ready', () => {
    pushSnapshot(audioStore, playStatus)
  }).then((unlisten) => {
    readyUnlisten = unlisten
  }).catch(() => {})

  // RAF 循环：周期性发送时间同步 + 行索引变化检测
  const tick = () => {
    if (!installed) return

    const lines = player.value.lyrics?.lines || []
    const currentTimeMs = (Audio.value.currentTime || 0) * 1000
    const currentIdx = computeLyricIndex(lines, currentTimeMs)
    const isPlaying = Audio.value.isPlay

    // 行索引变化 → 立即发送
    if (currentIdx !== lastIdx) {
      lastIdx = currentIdx
      pushIndexChange(currentIdx)
    }

    // 播放状态变化 → 立即发送
    if (isPlaying !== lastPlayState) {
      lastPlayState = isPlaying
      pushPlayStateChange(isPlaying)
    }

    // 周期性时间同步（桌面端用此锚定本地时钟）
    const now = performance.now()
    if (now - lastSyncTime >= SYNC_INTERVAL_MS) {
      lastSyncTime = now
      pushProgress(currentTimeMs)
    }

    rafId = requestAnimationFrame(tick)
  }

  rafId = requestAnimationFrame(tick)
}

export function uninstallDesktopLyricBridge() {
  installed = false
  if (rafId !== null) {
    cancelAnimationFrame(rafId)
    rafId = null
  }
  if (readyUnlisten) {
    readyUnlisten()
    readyUnlisten = null
  }
  watchStopHandles.forEach((stop) => stop())
  watchStopHandles = []
}

// 兼容旧接口
export const startDesktopLyricSync = installDesktopLyricBridge
export const stopDesktopLyricSync = uninstallDesktopLyricBridge
