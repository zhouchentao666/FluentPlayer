import { ref, computed, nextTick, onMounted, onUnmounted, watch, type Ref } from 'vue'
import { ReadCoverArt, AudioSrc } from '@bridge/app'
import { type Song } from '../types'
import { localMetadata } from './useLocalMetadata'

interface AudioPlayerOptions {
  audioRef?: Ref<HTMLAudioElement | null>
  onEnded?: () => void
}

export function useAudioPlayer(options: AudioPlayerOptions = {}) {
  const internalAudioRef = ref<HTMLAudioElement | null>(null)
  const audioRef = options.audioRef || internalAudioRef
  const currentSong = ref<Song | null>(null)
  const isPlaying = ref(false)
  const currentTime = ref(0)
  const duration = ref(0)
  const volume = ref(100)
  const playbackRate = ref(1)
  const coverUrl = ref<string | null>(null)
  // 播放列表（与歌单解耦的播放队列）
  const queue = ref<Song[]>([])
  const index = ref(-1)
  // 上下文标签（用于展示/恢复），不再用于队列导航
  const playlistId = ref<string | null>(null)

  const hasSong = computed(() => currentSong.value !== null)

  async function loadCover(path: string) {
    const override = localMetadata.value[path]?.cover
    if (override) {
      coverUrl.value = override
      return
    }
    try {
      coverUrl.value = await ReadCoverArt(path)
    } catch {
      coverUrl.value = null
    }
  }

  watch(() => localMetadata.value[currentSong.value?.path ?? '']?.cover, () => {
    if (currentSong.value) loadCover(currentSong.value.path)
  })

  // 播放本地歌曲
  async function playLocal(song: Song, autoPlay = true) {
    currentSong.value = song
    currentTime.value = 0
    duration.value = song.metadata?.duration || 0
    await loadCover(song.path)
    await nextTick()
    if (!audioRef.value) return
    try {
      audioRef.value.src = AudioSrc(song.path)
      audioRef.value.load()
      audioRef.value.playbackRate = playbackRate.value
      if (autoPlay) {
        await audioRef.value.play()
        isPlaying.value = true
      } else {
        isPlaying.value = false
      }
    } catch {
      isPlaying.value = false
    }
  }

  // 播放队列中第 i 首
  async function playQueueAt(i: number, autoPlay = true) {
    const song = queue.value[i]
    if (!song) return
    index.value = i
    await playLocal(song, autoPlay)
  }

  // 替换播放列表：用 songs 替换整个队列并从 startIndex 开始播放
  async function playSongs(songs: Song[], startIndex: number, context?: string | null, autoPlay = true) {
    queue.value = songs
    playlistId.value = context ?? null
    await playQueueAt(startIndex, autoPlay)
  }

  // 添加到播放列表：追加到队尾，若当前未播放则立即播放
  function addToQueue(song: Song) {
    if (queue.value.length && queue.value[queue.value.length - 1]?.id === song.id) return
    queue.value = [...queue.value, song]
    if (index.value < 0 && !currentSong.value) {
      playQueueAt(queue.value.length - 1, true)
    }
  }

  // 从播放列表删除第 i 首，自动维护 index
  function removeFromQueue(i: number) {
    if (i < 0 || i >= queue.value.length) return
    const wasCurrent = i === index.value
    queue.value = queue.value.filter((_, idx) => idx !== i)
    if (i < index.value) {
      index.value = index.value - 1
    } else if (wasCurrent) {
      if (queue.value.length === 0) {
        index.value = -1
        currentSong.value = null
        isPlaying.value = false
        audioRef.value?.pause()
      } else {
        const nextIdx = Math.min(i, queue.value.length - 1)
        playQueueAt(nextIdx, isPlaying.value)
      }
    }
  }

  function clearQueue() {
    queue.value = []
    index.value = -1
    currentSong.value = null
    isPlaying.value = false
    audioRef.value?.pause()
  }

  function togglePlay() {
    if (!currentSong.value || !audioRef.value) return
    if (isPlaying.value) {
      audioRef.value.pause()
    } else {
      audioRef.value.play().catch(() => {})
    }
  }

  function pause() {
    audioRef.value?.pause()
  }

  function seek(time: number) {
    if (!audioRef.value) return
    audioRef.value.currentTime = time
    currentTime.value = time
  }

  function setVolume(value: number) {
    volume.value = value
    if (audioRef.value) audioRef.value.volume = value / 100
  }

  function setPlaybackRate(rate: number) {
    const clamped = Math.min(16, Math.max(0.25, rate))
    playbackRate.value = clamped
    if (audioRef.value) audioRef.value.playbackRate = clamped
  }

  function bindAudioEvents() {
    const audio = audioRef.value
    if (!audio) return
    audio.volume = volume.value / 100
    audio.addEventListener('timeupdate', () => {
      currentTime.value = audio.currentTime || 0
    })
    audio.addEventListener('loadedmetadata', () => {
      duration.value = audio.duration || currentSong.value?.metadata?.duration || 0
    })
    if (options.onEnded) {
      audio.addEventListener('ended', options.onEnded)
    }
    audio.addEventListener('play', () => { isPlaying.value = true })
    audio.addEventListener('pause', () => { isPlaying.value = false })
  }

  onMounted(() => {
    nextTick(bindAudioEvents)
  })

  onUnmounted(() => {
    audioRef.value?.pause()
  })

  return {
    audioRef,
    currentSong,
    isPlaying,
    currentTime,
    duration,
    volume,
    playbackRate,
    coverUrl,
    queue,
    index,
    playlistId,
    hasSong,
    playSongs,
    playQueueAt,
    addToQueue,
    removeFromQueue,
    clearQueue,
    togglePlay,
    pause,
    seek,
    setVolume,
    setPlaybackRate,
  }
}
