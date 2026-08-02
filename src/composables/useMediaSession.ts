import { watch, onBeforeUnmount, type Ref } from 'vue'
import type { Song } from '../types'

/**
 * 系统音频 API 适配（参考 LyciaMusic 的系统媒体控制思路）。
 *
 * LyciaMusic 在 Rust 侧用 souvlaki 接管 SMTC，是因为它的解码/播放在 Rust。
 * fluentplayer 的音频由 WebView 里的 <audio> 播放，WebView2 会把标准
 * `navigator.mediaSession` 直接桥接到 Windows SMTC（任务栏/音量浮窗/媒体键），
 * macOS 上桥接到 Now Playing，Linux 上桥接到 MPRIS。
 * 因此这里直接适配标准 Media Session API，无需额外原生依赖。
 */

export interface MediaSessionHandlers {
  play: () => void
  pause: () => void
  next: () => void
  prev: () => void
  seek: (time: number) => void
  stop?: () => void
}

export interface MediaSessionOptions {
  currentSong: Ref<Song | null>
  coverUrl: Ref<string | null>
  isPlaying: Ref<boolean>
  currentTime: Ref<number>
  duration: Ref<number>
  /** 关闭时会清空系统媒体面板信息。 */
  enabled: Ref<boolean>
  handlers: MediaSessionHandlers
}

type ActionEntry = [MediaSessionAction, MediaSessionActionHandler | null]

function hasMediaSession(): boolean {
  return typeof navigator !== 'undefined' && 'mediaSession' in navigator
}

export function useMediaSession(options: MediaSessionOptions) {
  const { currentSong, coverUrl, isPlaying, currentTime, duration, enabled, handlers } = options

  let installed = false

  function actions(): ActionEntry[] {
    return [
      ['play', () => handlers.play()],
      ['pause', () => handlers.pause()],
      ['previoustrack', () => handlers.prev()],
      ['nexttrack', () => handlers.next()],
      ['stop', () => (handlers.stop ? handlers.stop() : handlers.pause())],
      [
        'seekto',
        (details) => {
          if (typeof details.seekTime === 'number') handlers.seek(details.seekTime)
        },
      ],
      [
        'seekbackward',
        (details) => {
          const offset = details.seekOffset ?? 10
          handlers.seek(Math.max(0, currentTime.value - offset))
        },
      ],
      [
        'seekforward',
        (details) => {
          const offset = details.seekOffset ?? 10
          handlers.seek(Math.min(duration.value || Infinity, currentTime.value + offset))
        },
      ],
    ]
  }

  function installHandlers() {
    if (!hasMediaSession() || installed) return
    for (const [action, handler] of actions()) {
      // 部分动作在某些平台不受支持，逐个 try 避免一个失败影响其余
      try {
        navigator.mediaSession.setActionHandler(action, handler)
      } catch {
        /* 该平台不支持此动作 */
      }
    }
    installed = true
  }

  function uninstallHandlers() {
    if (!hasMediaSession() || !installed) return
    for (const [action] of actions()) {
      try {
        navigator.mediaSession.setActionHandler(action, null)
      } catch {
        /* ignore */
      }
    }
    installed = false
  }

  function clear() {
    if (!hasMediaSession()) return
    uninstallHandlers()
    navigator.mediaSession.metadata = null
    navigator.mediaSession.playbackState = 'none'
  }

  function syncMetadata() {
    if (!hasMediaSession() || !enabled.value) return
    const song = currentSong.value
    if (!song) {
      navigator.mediaSession.metadata = null
      navigator.mediaSession.playbackState = 'none'
      return
    }
    const cover = coverUrl.value || song.cover || null
    navigator.mediaSession.metadata = new MediaMetadata({
      title: song.metadata?.title || song.title || '未知歌曲',
      artist: song.metadata?.artist || song.online?.singer || '未知艺术家',
      album: song.metadata?.album || song.online?.albumName || '',
      artwork: cover ? [{ src: cover, sizes: '512x512', type: 'image/jpeg' }] : [],
    })
  }

  function syncPlaybackState() {
    if (!hasMediaSession() || !enabled.value) return
    navigator.mediaSession.playbackState = isPlaying.value ? 'playing' : 'paused'
  }

  function syncPosition() {
    if (!hasMediaSession() || !enabled.value) return
    if (!navigator.mediaSession.setPositionState) return
    const total = duration.value
    if (!Number.isFinite(total) || total <= 0) return
    try {
      navigator.mediaSession.setPositionState({
        duration: total,
        position: Math.min(Math.max(currentTime.value, 0), total),
        playbackRate: 1,
      })
    } catch {
      /* 进度越界时浏览器会抛错，忽略即可 */
    }
  }

  watch(
    enabled,
    (on) => {
      if (on) {
        installHandlers()
        syncMetadata()
        syncPlaybackState()
        syncPosition()
      } else {
        clear()
      }
    },
    { immediate: true },
  )

  watch([currentSong, coverUrl], () => {
    syncMetadata()
    syncPosition()
  })
  watch(isPlaying, syncPlaybackState)
  // 进度每秒才会变化一次量级，这里按整秒节流，避免高频写系统面板
  let lastSecond = -1
  watch(currentTime, (t) => {
    const s = Math.floor(t)
    if (s === lastSecond) return
    lastSecond = s
    syncPosition()
  })
  watch(duration, syncPosition)

  onBeforeUnmount(clear)

  return { clear }
}
