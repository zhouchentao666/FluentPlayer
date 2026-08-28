import { Events } from '@bridge/runtime'
import { ref, watch, type Ref } from 'vue'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import type { Song } from '../types'

export interface DesktopLyricBridgeHandlers {
  onPrev: () => void
  onNext: () => void
  onToggle: () => void
  onShowMain: () => void
  onClose: () => void
  onLockChange: (locked: boolean) => void
}

export interface DesktopLyricBridgeOptions {
  currentSong: Ref<Song | null>
  lyrics: Ref<LyricLine[]>
  isPlaying: Ref<boolean>
  currentTime: Ref<number>
  handlers: DesktopLyricBridgeHandlers
}

export function useDesktopLyricBridge(options: DesktopLyricBridgeOptions) {
  const { currentSong, lyrics, isPlaying, currentTime, handlers } = options

  const playSeekMs = ref(0)
  let baseMs = 0
  let anchorTick = 0
  let rafId = 0
  let lastIndex = -1

  function computeLyricIndex(ms: number, lines: LyricLine[]): number {
    if (!lines.length) return -1
    const idx = lines.findIndex((l) => (l.startTime || 0) > ms)
    if (idx === -1) return lines.length - 1
    if (idx > 0) return idx - 1
    return -1
  }

  function emitProgress() {
    const lines = lyrics.value
    const ms = playSeekMs.value
    const idx = computeLyricIndex(ms, lines)
    let progress = 0
    if (idx >= 0 && lines[idx]) {
      const line = lines[idx]
      const dur = Math.max(1, (line.endTime || line.startTime + 1) - line.startTime)
      progress = Math.min(1, Math.max(0, (ms - line.startTime) / dur))
    }
    Events.Emit('desktop-lyric:time', { currentMs: ms }).catch(() => {})
    if (idx !== lastIndex) {
      lastIndex = idx
      Events.Emit('desktop-lyric:index', { index: idx, progress }).catch(() => {})
    } else {
      Events.Emit('desktop-lyric:progress', { index: idx, progress }).catch(() => {})
    }
  }

  function startLoop() {
    if (rafId) return
    anchorTick = performance.now()
    baseMs = playSeekMs.value
    const loop = () => {
      if (!isPlaying.value) {
        rafId = 0
        return
      }
      playSeekMs.value = baseMs + (performance.now() - anchorTick)
      emitProgress()
      rafId = requestAnimationFrame(loop)
    }
    rafId = requestAnimationFrame(loop)
  }

  function stopLoop() {
    if (rafId) {
      cancelAnimationFrame(rafId)
      rafId = 0
    }
    baseMs = playSeekMs.value
    anchorTick = performance.now()
  }

  function buildYrcData(lines: LyricLine[]): LyricLine[] {
    return lines.some((l) => Array.isArray(l.words) && l.words.length > 1) ? lines : []
  }

  function pushSong() {
    const s = currentSong.value
    Events.Emit('desktop-lyric:song', {
      name: s?.title || '',
      artist: s?.metadata?.artist || '',
    }).catch(() => {})
  }

  function pushLyrics() {
    const lines = lyrics.value
    Events.Emit('desktop-lyric:lyrics', {
      lrcData: lines,
      yrcData: buildYrcData(lines),
    }).catch(() => {})
    lastIndex = -1
  }

  function pushPlayState() {
    Events.Emit('desktop-lyric:play', isPlaying.value).catch(() => {})
  }

  function pushSnapshot() {
    pushSong()
    pushLyrics()
    playSeekMs.value = Math.round(currentTime.value * 1000)
    baseMs = playSeekMs.value
    anchorTick = performance.now()
    pushPlayState()
    emitProgress()
  }

  const unwatchSong = watch(currentSong, pushSong, { immediate: true })
  const unwatchLyrics = watch(lyrics, pushLyrics, { immediate: true })
  const unwatchPlaying = watch(
    isPlaying,
    (playing) => {
      pushPlayState()
      if (playing) startLoop()
      else stopLoop()
    },
    { immediate: true }
  )
  const unwatchTime = watch(currentTime, (time) => {
    playSeekMs.value = Math.round(time * 1000)
    baseMs = playSeekMs.value
    anchorTick = performance.now()
  })

  const offControl = Events.On('desktop-lyric:control', (event: any) => {
    const action = event?.data?.action
    if (action === 'prev') handlers.onPrev()
    else if (action === 'next') handlers.onNext()
    else if (action === 'toggle') handlers.onToggle()
    else if (action === 'show-main') handlers.onShowMain()
  })
  const offClose = Events.On('desktop-lyric:close', () => handlers.onClose())
  const offReady = Events.On('desktop-lyric:ready', () => pushSnapshot())
  const offLock = Events.On('desktop-lyric:lock-changed', (event: any) => {
    handlers.onLockChange(!!event?.data?.locked)
  })

  function dispose() {
    unwatchSong()
    unwatchLyrics()
    unwatchPlaying()
    unwatchTime()
    stopLoop()
    offControl?.()
    offClose?.()
    offReady?.()
    offLock?.()
  }

  return { dispose }
}
