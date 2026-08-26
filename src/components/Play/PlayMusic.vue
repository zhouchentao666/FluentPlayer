<script setup lang="ts">
import {
  ref,
  computed,
  onMounted,
  watch,
  nextTick,
  onActivated,
  onDeactivated,
  onBeforeUnmount,
  toRaw
} from 'vue'
import { useLifecycle } from '@/composables/useEventListener'
import { withViewTransition } from '@/composables/useViewTransition'
import { ControlAudioStore } from '@/store/ControlAudio'
import {
  installDesktopLyricBridge,
  uninstallDesktopLyricBridge
} from '@/utils/lyrics/desktopLyricBridge'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import icons from '@/assets/icon_font/icons'
const { liebiao, shengyin } = icons
import { storeToRefs } from 'pinia'
import FullPlay from './FullPlay.vue'
import PlaylistDrawer from './PlaylistDrawer.vue'
import { PlayMode } from '@/types/audio'
import { MessagePlugin } from 'tdesign-vue-next'
import {
  playNext,
  playPrevious,
  updatePlayMode,
  togglePlayPause,
  isLoadingSong,
  setVolume,
  seekTo,
  playSong,
  playMode
} from '@/utils/audio/globaPlayList'
import songCover from '@/assets/images/song.jpg'
import { downloadSingleSong } from '@/utils/audio/download'
import {
  HeartIcon,
  DownloadIcon,
  CheckIcon,
  LockOnIcon,
  ChatBubble1Icon
} from 'tdesign-icons-vue-next'
import cloneDeep from 'lodash/cloneDeep'
import { songListAPI } from '@/api/songList'
import { useDlnaStore } from '@/store/dlna'
import { crossfadeState } from '@/utils/audio/crossfade'
import CrossfadeHint from './CrossfadeHint.vue'

const { t } = useI18n()

const dlnaStore = useDlnaStore()
const controlAudio = ControlAudioStore()
const localUserStore = LocalUserDetailStore()
const globalPlayStatus = useGlobalPlayStatusStore()
const lifecycle = useLifecycle()
const { Audio } = storeToRefs(controlAudio)
const { list, userInfo } = storeToRefs(localUserStore)
const { player } = storeToRefs(globalPlayStatus)
const songInfo = computed(() => player.value.songInfo || ({} as any))

let clearDlnaSync: (() => void) | null = null
let dlnaDeviceGeneration = 0
let dlnaHandoffToken = 0
let positionRequestPending = false
let expectedLocalPlayState: boolean | null = null
let dlnaResumeRequested = Audio.value.isPlay
let activeDlnaHandoffToken: number | null = null

const isCurrentDlnaDevice = (generation: number, usn: string) =>
  dlnaDeviceGeneration === generation && dlnaStore.currentDevice?.usn === usn

const isCurrentHandoff = (token: number, generation: number, usn: string, url: string) =>
  dlnaHandoffToken === token &&
  Audio.value.url === url &&
  isCurrentDlnaDevice(generation, usn)

const setLocalPlayState = async (isPlay: boolean, isCurrent: () => boolean) => {
  expectedLocalPlayState = isPlay
  try {
    await invoke(isPlay ? 'player__resume' : 'player__pause')
    return isCurrent()
  } catch (error) {
    throw error
  }
}

const handleTogglePlayPause = () => {
  dlnaResumeRequested = !Audio.value.isPlay
  togglePlayPause()
}

const syncDlnaPosition = async (generation: number, usn: string, handoffToken: number) => {
  if (positionRequestPending || !Audio.value.isPlay) return
  positionRequestPending = true
  try {
    const positionInfo = await dlnaStore.getPosition()
    if (
      !isCurrentDlnaDevice(generation, usn) ||
      dlnaHandoffToken !== handoffToken ||
      !Audio.value.isPlay
    ) return
    if (positionInfo.duration > 0) controlAudio.setDuration(positionInfo.duration)
    if (Math.abs(Audio.value.currentTime - positionInfo.position) > 2) {
      await invoke('player__seek', { position: positionInfo.position })
    }
  } catch (error) {
    if (isCurrentDlnaDevice(generation, usn)) {
      console.error('DLNA position sync failed', error)
    }
  } finally {
    positionRequestPending = false
  }
}

watch(
  () => dlnaStore.currentDevice,
  (device) => {
    dlnaDeviceGeneration += 1
    dlnaHandoffToken += 1
    const generation = dlnaDeviceGeneration
    if (clearDlnaSync) { clearDlnaSync(); clearDlnaSync = null }
    if (device) {
      const usn = device.usn
      clearDlnaSync = lifecycle.addInterval(() => {
        void syncDlnaPosition(generation, usn, dlnaHandoffToken)
      }, 1000)
    }
  }
)

watch(
  () => Audio.value.url,
  async (newUrl) => {
    const token = ++dlnaHandoffToken
    const deviceUsn = dlnaStore.currentDevice?.usn
    if (!deviceUsn || !newUrl) return
    const generation = dlnaDeviceGeneration
    activeDlnaHandoffToken = token
    dlnaResumeRequested = Audio.value.isPlay
    try {
      if (!await setLocalPlayState(
        false,
        () => isCurrentHandoff(token, generation, deviceUsn, newUrl)
      )) return
      await dlnaStore.play(newUrl, songInfo.value.name || 'CeruMusic')
      if (!isCurrentHandoff(token, generation, deviceUsn, newUrl)) return
      await new Promise(resolve => setTimeout(resolve, 1500))
      if (!isCurrentHandoff(token, generation, deviceUsn, newUrl)) return
      const positionInfo = await dlnaStore.getPosition()
      if (!isCurrentHandoff(token, generation, deviceUsn, newUrl)) return
      if (positionInfo.duration > 0) controlAudio.setDuration(positionInfo.duration)
      await invoke('player__seek', { position: positionInfo.position })
      if (!isCurrentHandoff(token, generation, deviceUsn, newUrl)) return
      if (dlnaResumeRequested) {
        if (!await setLocalPlayState(
          true,
          () => isCurrentHandoff(token, generation, deviceUsn, newUrl)
        )) return
      } else {
        await dlnaStore.pause()
        if (!isCurrentHandoff(token, generation, deviceUsn, newUrl)) return
      }
    } catch (error) {
      if (!isCurrentHandoff(token, generation, deviceUsn, newUrl)) return
      dlnaStore.selectDevice(null)
      console.error('DLNA track handoff failed', error)
    } finally {
      if (activeDlnaHandoffToken === token) activeDlnaHandoffToken = null
    }
  }
)

watch(
  () => Audio.value.isPlay,
  async (isPlay) => {
    if (expectedLocalPlayState === isPlay) {
      expectedLocalPlayState = null
      if (!isPlay && activeDlnaHandoffToken !== null && dlnaResumeRequested) {
        expectedLocalPlayState = true
        Audio.value.isPlay = true
      }
      return
    }
    expectedLocalPlayState = null
    dlnaResumeRequested = isPlay
    if (activeDlnaHandoffToken !== null) return
    const deviceUsn = dlnaStore.currentDevice?.usn
    if (deviceUsn) {
      const generation = dlnaDeviceGeneration
      try {
        if (isPlay) await dlnaStore.resume()
        else await dlnaStore.pause()
        if (!isCurrentDlnaDevice(generation, deviceUsn)) return
      } catch (error) {
        if (isCurrentDlnaDevice(generation, deviceUsn)) dlnaStore.selectDevice(null)
        console.error('DLNA play state sync failed', error)
      }
    }
  }
)

watch(
  () => Audio.value.volume,
  async (newVol) => {
    const deviceUsn = dlnaStore.currentDevice?.usn
    if (deviceUsn) {
      const generation = dlnaDeviceGeneration
      try {
        await dlnaStore.setVolume(newVol)
        if (!isCurrentDlnaDevice(generation, deviceUsn)) return
      } catch (error) {
        if (isCurrentDlnaDevice(generation, deviceUsn)) dlnaStore.selectDevice(null)
        console.error('DLNA volume sync failed', error)
      }
    }
  }
)

// 当前歌曲是否已在"我的喜欢"
const likeState = ref(false)
const isLiked = computed(() => likeState.value)

const refreshLikeState = async () => {
  try {
    if (!userInfo.value.lastPlaySongId) {
      likeState.value = false
      return
    }
    const favIdRes = await window.api.songList.getFavoritesId()
    const favoritesId: string | null = (favIdRes && favIdRes.data) || null
    if (!favoritesId) {
      likeState.value = false
      return
    }
    const hasRes = await songListAPI.hasSong(favoritesId, userInfo.value.lastPlaySongId)
    likeState.value = !!(hasRes.success && hasRes.data)
  } catch {
    likeState.value = false
  }
}

watch(
  () => userInfo.value.lastPlaySongId,
  () => refreshLikeState()
)
onMounted(() => refreshLikeState())
const showFullPlay = ref(false)
const showComments = ref(false)

const toggleComments = () => {
  showComments.value = !showComments.value
  if (showComments.value && !showFullPlay.value) {
    showFullPlay.value = true
  }
}

// 桌面歌词开关与锁定状态
const desktopLyricOpen = ref(false)
const desktopLyricLocked = ref(false)

// 桌面歌词按钮逻辑：
// - 若未打开：打开桌面歌词
// - 若已打开且锁定：先解锁，不关闭
// - 若已打开且未锁定：关闭桌面歌词
const toggleDesktopLyric = async () => {
  try {
    if (!desktopLyricOpen.value) {
      window.electron?.ipcRenderer?.send?.('change-desktop-lyric', true)
      desktopLyricOpen.value = true
      installDesktopLyricBridge()
      // 恢复最新锁定状态
      const lock = await window.electron?.ipcRenderer?.invoke?.('get-lyric-lock-state')
      desktopLyricLocked.value = !!lock
      return
    }
    // 已打开
    const lock = await window.electron?.ipcRenderer?.invoke?.('get-lyric-lock-state')
    desktopLyricLocked.value = !!lock
    if (desktopLyricLocked.value) {
      // 先解锁，本次不关闭
      window.electron?.ipcRenderer?.send?.('toogle-desktop-lyric-lock', false)
      desktopLyricLocked.value = false
      return
    }
    // 未锁定则关闭
    window.electron?.ipcRenderer?.send?.('change-desktop-lyric', false)
    desktopLyricOpen.value = false
    uninstallDesktopLyricBridge()
  } catch (e) {
    console.error('切换桌面歌词失败:', e)
  }
}
// 等待音频准备就绪
// 播放位置恢复逻辑由全局播放管理器处理

// 记录组件被停用前的播放状态
// let wasPlaying = false

// let playbackPosition = 0
let isFull = false

// 获取播放模式图标类名
const playModeIconClass = computed(() => {
  switch (playMode.value) {
    case PlayMode.LIST:
      return 'iconfont icon-shunxubofangtubiao'
    case PlayMode.SEQUENCE:
      return 'iconfont icon-bofang-xunhuanbofang'
    case PlayMode.RANDOM:
      return 'iconfont icon-suijibofang'
    case PlayMode.SINGLE:
      return 'iconfont icon-bofang-xunhuanbofang'
    default:
      return 'iconfont icon-shunxubofangtubiao'
  }
})
const playModeTip = computed(() => {
  switch (playMode.value) {
    case PlayMode.LIST: return t('play.modeList')
    case PlayMode.SEQUENCE: return t('play.modeSequence')
    case PlayMode.RANDOM: return t('play.modeRandom')
    case PlayMode.SINGLE: return t('play.modeSingle')
    default: return t('play.modeList')
  }
})

// 音量控制相关
const showVolumeSlider = ref(false)
const volumeBarRef = ref<HTMLDivElement | null>(null)
const isDraggingVolume = ref(false)

const volumeValue = computed({
  get: () => Audio.value.volume,
  set: (val) => {
    setVolume(val)
  }
})

// 音量控制拖动处理
const handleVolumeClick = (event: MouseEvent) => {
  if (!volumeBarRef.value) return

  const rect = volumeBarRef.value.getBoundingClientRect()
  const offsetY = rect.bottom - event.clientY
  const percentage = Math.max(0, Math.min(100, (offsetY / rect.height) * 100))

  volumeValue.value = Math.round(percentage)
}

const handleVolumeDragStart = (event: MouseEvent) => {
  event.preventDefault()
  isDraggingVolume.value = true
  window.addEventListener('mousemove', handleVolumeDragMove)
  window.addEventListener('mouseup', handleVolumeDragEnd)
}

const handleVolumeDragMove = (event: MouseEvent) => {
  if (!isDraggingVolume.value || !volumeBarRef.value) return

  const rect = volumeBarRef.value.getBoundingClientRect()
  const offsetY = rect.bottom - event.clientY
  const percentage = Math.max(0, Math.min(100, (offsetY / rect.height) * 100))

  volumeValue.value = Math.round(percentage)
}

const handleVolumeDragEnd = () => {
  isDraggingVolume.value = false
  window.removeEventListener('mousemove', handleVolumeDragMove)
  window.removeEventListener('mouseup', handleVolumeDragEnd)
}

const handleVolumeWheel = (event: WheelEvent) => {
  event.preventDefault()

  const volumeStep = event.deltaY > 0 ? -5 : 5
  const updatedVolume = Math.max(0, Math.min(100, volumeValue.value + volumeStep))

  if (updatedVolume === volumeValue.value) {
    return
  }

  volumeValue.value = updatedVolume
}

// 播放列表相关
const showPlaylist = ref(false)
const playlistDrawerRef = ref<InstanceType<typeof PlaylistDrawer> | null>(null)

const togglePlaylist = (e: MouseEvent) => {
  e.stopPropagation()
  withViewTransition(() => {
    showPlaylist.value = !showPlaylist.value
  })

  // 如果打开播放列表，滚动到当前播放歌曲
  if (showPlaylist.value) {
    nextTick(() => {
      playlistDrawerRef.value?.scrollToCurrentSong()
    })
  }
}

// 播放列表中的歌曲
const currentSongId = computed(() => userInfo.value.lastPlaySongId)

// 关闭播放列表
const closePlaylist = () => {
  showPlaylist.value = false
}

// 播放上一首
// 上一首/下一首由全局播放管理器提供

// 定期保存当前播放位置
// 全局快捷控制事件由全局播放管理器处理
// 初始化播放器

function globalControls(e: Event) {
  const customEvent = e as CustomEvent<{ name?: string; val?: any }>
  const { name, val } = customEvent.detail || {}
  if (name === 'toggleFullPlay') {
    toggleFullPlay()
  } else if (name === 'toggle') {
    handleTogglePlayPause()
  } else if (name === 'playPrev') {
    playPrevious()
  } else if (name === 'playNext') {
    playNext()
  } else if (name === 'seekDelta') {
    const delta = val || 0
    const newTime = Math.max(0, Math.min(Audio.value.duration, Audio.value.currentTime + delta))
    seekTo(newTime)
  } else if (name === 'volumeDelta') {
    const delta = val || 0
    setVolume(Math.max(0, Math.min(100, Audio.value.volume + delta)))
  } else if (name === 'toggleDesktopLyric') {
    toggleDesktopLyric()
  }
}

onMounted(async () => {
  // Electron IPC 监听器（精确注册/清理，不用 removeAllListeners）
  lifecycle.addIpcListener('toogle-desktop-lyric-lock', (_, lock) => {
    desktopLyricLocked.value = !!lock
  })
  lifecycle.addIpcListener('desktop-lyric-open-change', async (_: any, visible: boolean) => {
    desktopLyricOpen.value = !!visible
    if (desktopLyricOpen.value) {
      installDesktopLyricBridge()
      try {
        const lock = await window.electron?.ipcRenderer?.invoke?.('get-lyric-lock-state')
        desktopLyricLocked.value = !!lock
      } catch {}
    } else {
      desktopLyricLocked.value = false
      uninstallDesktopLyricBridge()
    }
  })
  lifecycle.addIpcListener('closeDesktopLyric', () => {
    desktopLyricOpen.value = false
    desktopLyricLocked.value = false
    uninstallDesktopLyricBridge()
  })

  // 初始化同步当前打开与锁定状态（Tauri 无此后端，静默降级）
  try {
    const open = await window.electron?.ipcRenderer?.invoke?.('get-lyric-open-state')
    desktopLyricOpen.value = !!open
    const lock = await window.electron?.ipcRenderer?.invoke?.('get-lyric-lock-state')
    desktopLyricLocked.value = !!lock
  } catch {}

  // Window 事件监听器（自动清理）
  lifecycle.addEventListener(window, 'global-music-control', globalControls)
  const openPlaylistHandler = () => {
    showPlaylist.value = true
    nextTick(() => {
      playlistDrawerRef.value?.scrollToCurrentSong?.()
    })
  }
  const closePlaylistHandler = () => {
    showPlaylist.value = false
  }
  lifecycle.addEventListener(window, 'open-playlist', openPlaylistHandler)
  lifecycle.addEventListener(window, 'close-playlist', closePlaylistHandler)

  // Tauri 事件监听（自动清理）
  lifecycle.addTauriListen(
    listen('desktop-lyric-control', (event) => {
      const { name, value } = event.payload as any
      if (name === 'close') {
        window.electron?.ipcRenderer?.send?.('change-desktop-lyric', false)
        desktopLyricOpen.value = false
        uninstallDesktopLyricBridge()
        return
      }
      if (name === 'lock') {
        window.electron?.ipcRenderer?.send?.('toogle-desktop-lyric-lock', value)
        desktopLyricLocked.value = !!value
        return
      }
      window.dispatchEvent(new CustomEvent('global-music-control', { detail: { name } }))
    })
  )

  // 拖动监听器兜底清理（拖动开始时添加，拖动结束时应已自清理，此处兜底）
  lifecycle.addCleanup(() => {
    window.removeEventListener('mousemove', handleVolumeDragMove)
    window.removeEventListener('mouseup', handleVolumeDragEnd)
    window.removeEventListener('mousemove', handleProgressDragMove)
    window.removeEventListener('mouseup', handleProgressDragEnd)
  })
})

// 组件被激活时（从缓存中恢复）
onActivated(async () => {
  if (isFull) {
    showFullPlay.value = true
  }
})

// 组件被停用时（缓存但不销毁）
onDeactivated(() => {
  // 仅记录状态，不主动暂停，避免页面切换导致音乐暂停
  // wasPlaying = Audio.value.isPlay
  isFull = showFullPlay.value
})

// 监听用户信息变化，更新音量
watch(
  () => userInfo.value.volume,
  (newVolume) => {
    if (newVolume !== undefined) {
      setVolume(newVolume)
    }
  },
  { immediate: true }
)

// 全屏展示相关
const toggleFullPlay = () => {
  if (!songInfo.value.songmid) return
  withViewTransition(() => {
    showFullPlay.value = !showFullPlay.value
  })
}

// 全屏闲置状态
const isFullPlayIdle = ref(false)
const handleIdleChange = (idle: boolean) => {
  isFullPlayIdle.value = idle
}

// 左侧操作：喜欢/取消喜欢（支持切换）
const onToggleLike = async () => {
  try {
    // 获取当前播放歌曲对象
    const currentSong = list.value.find((s) => s.songmid === userInfo.value.lastPlaySongId)
    if (!currentSong) {
      MessagePlugin.warning(t('play.noSongPlaying'))
      return
    }

    // 读取持久化的"我的喜欢"歌单ID
    const favIdRes = await window.api.songList.getFavoritesId()
    let favoritesId: string | null = (favIdRes && favIdRes.data) || null

    // 如果已有ID但歌单不存在，则置空
    if (favoritesId) {
      const existsRes = await songListAPI.exists(favoritesId)
      if (!existsRes.success || !existsRes.data) {
        favoritesId = null
      }
    }

    // 如果没有ID，尝试查找同名歌单；找不到则创建
    if (!favoritesId) {
      const searchRes = await songListAPI.search('我的喜欢', 'local')
      if (searchRes.success && Array.isArray(searchRes.data)) {
        const exact = searchRes.data.find((pl) => pl.name === '我的喜欢' && pl.source === 'local')
        favoritesId = exact?.id || null
      }
      if (!favoritesId) {
        const createRes = await songListAPI.create('我的喜欢', '', 'local')
        if (!createRes.success || !createRes.data?.id) {
          MessagePlugin.error(createRes.error || t('play.createFavoritesFailed'))
          return
        }
        favoritesId = createRes.data.id
      }
      // 持久化ID到主进程配置
      await window.api.songList.setFavoritesId(favoritesId)
    }

    // 根据当前状态决定添加或移除
    if (likeState.value) {
      const removeRes = await songListAPI.removeSong(
        favoritesId!,
        userInfo.value.lastPlaySongId as any
      )
      if (removeRes.success && removeRes.data) {
        likeState.value = false
        // MessagePlugin.success('已取消喜欢')
      } else {
        MessagePlugin.error(removeRes.error || t('play.unlikeFailed'))
      }
    } else {
      const addRes = await songListAPI.addSongs(favoritesId!, [
        cloneDeep(toRaw(currentSong)) as any
      ])
      if (addRes.success) {
        likeState.value = true
        // MessagePlugin.success('已添加到"我的喜欢"')
      } else {
        MessagePlugin.error(addRes.error || t('play.addToFavoritesFailed'))
      }
    }
  } catch (error: any) {
    console.error('切换喜欢状态失败:', error)
    MessagePlugin.error(t('play.operationFailedRetry'))
  }
}

const onDownload = async () => {
  try {
    await downloadSingleSong(cloneDeep(toRaw(songInfo.value)) as any)
    MessagePlugin.success(t('play.startDownload'))
  } catch (e: any) {
    console.error('下载失败:', e)
    MessagePlugin.error(t('play.downloadFailedRetry'))
  }
}

// 进度条相关
const progressRef = ref<HTMLDivElement | null>(null)
const isDraggingProgress = ref(false)
const hasDragged = ref(false)
const tempProgressPercentage = ref(Audio.value.currentTime || 0)

const progressPercentage = computed(() => {
  if (isDraggingProgress.value) {
    return tempProgressPercentage.value
  }
  if (Audio.value.duration === 0) return 0
  return (Audio.value.currentTime / Audio.value.duration) * 100
})

// 无感过渡预告区间在进度条上的百分比位置
const crossfadeMarkVisible = computed(() => {
  return (
    crossfadeState.markEnd > crossfadeState.markStart &&
    Audio.value.duration > 0 &&
    !dlnaStore.currentDevice
  )
})
const crossfadeMarkLeft = computed(() => {
  if (!crossfadeMarkVisible.value) return 0
  return (crossfadeState.markStart / Audio.value.duration) * 100
})
const crossfadeMarkWidth = computed(() => {
  if (!crossfadeMarkVisible.value) return 0
  const d = Audio.value.duration
  return ((crossfadeState.markEnd - crossfadeState.markStart) / d) * 100
})
// 过渡激活时也标注实际正在发生淡化的区段
const crossfadeActiveMarkVisible = computed(() => {
  return crossfadeState.active && crossfadeState.fadeDuration > 0 && Audio.value.duration > 0
})
const crossfadeActiveMarkLeft = computed(() => {
  if (!crossfadeActiveMarkVisible.value) return 0
  return (crossfadeState.fadeStart / Audio.value.duration) * 100
})
const crossfadeActiveMarkWidth = computed(() => {
  if (!crossfadeActiveMarkVisible.value) return 0
  return (crossfadeState.fadeDuration / Audio.value.duration) * 100
})

// 过渡完成后，新歌开头的淡入区段标记（从 0 到 fadeInMarkEnd 秒）
const crossfadeFadeInMarkVisible = computed(() => {
  return crossfadeState.fadeInMarkEnd > 0 && Audio.value.duration > 0 && !dlnaStore.currentDevice
})
const crossfadeFadeInMarkWidth = computed(() => {
  if (!crossfadeFadeInMarkVisible.value) return 0
  return (crossfadeState.fadeInMarkEnd / Audio.value.duration) * 100
})

// 格式化时间显示
const formatTime = (seconds: number) => {
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

// 当前播放时间和总时长的格式化显示
const currentTimeFormatted = computed(() => formatTime(Audio.value.currentTime))
const durationFormatted = computed(() => formatTime(Audio.value.duration))

// 进度条拖动处理
const handleProgressClick = (event: MouseEvent) => {
  if (dlnaStore.currentDevice) {
    MessagePlugin.warning(t('play.screenCastingNoSeek'))
    return
  }
  // 如果刚刚发生了拖动，忽略点击事件（避免重复 seek）
  if (hasDragged.value) {
    hasDragged.value = false
    return
  }
  if (!progressRef.value) return

  const rect = progressRef.value.getBoundingClientRect()
  const offsetX = event.clientX - rect.left
  const percentage = (offsetX / rect.width) * 100

  // 更新临时进度值，使UI立即响应
  tempProgressPercentage.value = percentage

  const newTime = (percentage / 100) * Audio.value.duration
  seekTo(newTime)
}

const handleProgressDragMove = (event: MouseEvent) => {
  if (!isDraggingProgress.value || !progressRef.value) return
  const rect = progressRef.value.getBoundingClientRect()
  const offsetX = Math.max(0, Math.min(event.clientX - rect.left, rect.width))
  const percentage = (offsetX / rect.width) * 100

  // 标记发生了拖动（用于区分点击和拖动）
  hasDragged.value = true

  // 拖动时只更新UI，不频繁设置audio.currentTime
  tempProgressPercentage.value = percentage
}

const handleProgressDragEnd = (event: MouseEvent) => {
  document.querySelector('.progress-handle')?.classList.remove('dragging')

  if (!isDraggingProgress.value || !progressRef.value) {
    isDraggingProgress.value = false
    window.removeEventListener('mousemove', handleProgressDragMove)
    window.removeEventListener('mouseup', handleProgressDragEnd)
    return
  }

  const rect = progressRef.value.getBoundingClientRect()
  const offsetX = Math.max(0, Math.min(event.clientX - rect.left, rect.width))
  const percentage = (offsetX / rect.width) * 100
  const newTime = (percentage / 100) * Audio.value.duration
  seekTo(newTime)

  isDraggingProgress.value = false
  window.removeEventListener('mousemove', handleProgressDragMove)
  window.removeEventListener('mouseup', handleProgressDragEnd)
}

const handleProgressDragStart = (event: MouseEvent) => {
  if (dlnaStore.currentDevice) {
    MessagePlugin.warning(t('play.screenCastingNoSeek'))
    return
  }
  event.preventDefault()
  document.querySelector('.progress-handle')?.classList.add('dragging')

  hasDragged.value = false
  isDraggingProgress.value = true
  window.addEventListener('mousemove', handleProgressDragMove)
  window.addEventListener('mouseup', handleProgressDragEnd)
}

const getTouchPercentage = (touch: Touch): number => {
  if (!progressRef.value) return 0
  const rect = progressRef.value.getBoundingClientRect()
  const offsetX = Math.max(0, Math.min(touch.clientX - rect.left, rect.width))
  return (offsetX / rect.width) * 100
}

const handleProgressTouchStart = (event: TouchEvent) => {
  if (dlnaStore.currentDevice) {
    MessagePlugin.warning(t('play.screenCastingNoSeek'))
    return
  }
  event.stopPropagation()
  document.querySelector('.progress-handle')?.classList.add('dragging')
  hasDragged.value = false
  isDraggingProgress.value = true
  tempProgressPercentage.value = getTouchPercentage(event.touches[0])
}

const handleProgressTouchMove = (event: TouchEvent) => {
  if (!isDraggingProgress.value) return
  event.preventDefault()
  hasDragged.value = true
  tempProgressPercentage.value = getTouchPercentage(event.touches[0])
}

const handleProgressTouchEnd = () => {
  document.querySelector('.progress-handle')?.classList.remove('dragging')
  if (!isDraggingProgress.value) return
  const percentage = tempProgressPercentage.value
  const newTime = (percentage / 100) * Audio.value.duration
  seekTo(newTime)
  isDraggingProgress.value = false
}

// 歌曲信息由全局播放管理器提供
const maincolor = computed(() => player.value.coverDetail.mainColor || 'var(--td-brand-color-5)')
const startmaincolor = computed(() => {
  const c = player.value.coverDetail.ColorObject
  if (c) return `rgba(${c.r},${c.g},${c.b},.2)`
  return 'rgba(0, 0, 0, 1)'
})
const contrastTextColor = computed(
  () => player.value.coverDetail.textColor || 'var(--player-text-idle)'
)
const hoverColor = computed(
  () => player.value.coverDetail.hoverColor || 'var(--player-text-hover-idle)'
)
const playbg = computed(() => player.value.coverDetail.playBg || 'var(--player-btn-bg-idle)')
const playbghover = computed(
  () => player.value.coverDetail.playBgHover || 'var(--player-btn-bg-hover-idle)'
)
const mobilePlayerBg = computed(() => {
  if (showFullPlay.value) return '#00000020'
  if (!songInfo.value.songmid) return 'var(--player-bg-idle)'

  const color = player.value.coverDetail.ColorObject
  if (!color) return 'var(--player-bg-default)'

  if (player.value.coverDetail.useBlackText) {
    const r = Math.round(color.r + (255 - color.r) * 0.78)
    const g = Math.round(color.g + (255 - color.g) * 0.78)
    const b = Math.round(color.b + (255 - color.b) * 0.78)
    return `rgba(${r}, ${g}, ${b}, 0.88)`
  }

  const r = Math.round(color.r * 0.35)
  const g = Math.round(color.g * 0.35)
  const b = Math.round(color.b * 0.35)
  return `rgba(${r}, ${g}, ${b}, 0.88)`
})

const bg = ref('var(--player-bg-default)')

// 封面图切换淡入效果
const coverOpacity = ref(1)
let coverFadeRaf: number | null = null
let coverFadeNextRaf: number | null = null

const cancelCoverFade = () => {
  if (coverFadeRaf !== null) {
    cancelAnimationFrame(coverFadeRaf)
    coverFadeRaf = null
  }
  if (coverFadeNextRaf !== null) {
    cancelAnimationFrame(coverFadeNextRaf)
    coverFadeNextRaf = null
  }
}

watch(
  () => player.value.cover,
  () => {
    cancelCoverFade()
    coverOpacity.value = 0
    coverFadeRaf = requestAnimationFrame(() => {
      coverFadeRaf = null
      coverFadeNextRaf = requestAnimationFrame(() => {
        coverFadeNextRaf = null
        coverOpacity.value = 1
      })
    })
  }
)

onBeforeUnmount(cancelCoverFade)

watch(
  () => player.value.songInfo?.songmid,
  (songmid) => {
    bg.value = bg.value === 'var(--player-bg-idle)' ? 'var(--player-bg-default)' : toRaw(bg.value)
    if (!songmid) {
      bg.value = 'var(--player-bg-idle)'
    }
  },
  { immediate: true }
)

watch(showFullPlay, (val) => {
  globalPlayStatus.setFullPlayOpen(val)
  if (val) {
    bg.value = '#00000020'
  } else {
    bg.value = 'var(--player-bg-default)'
  }
})

onBeforeUnmount(() => {
  dlnaDeviceGeneration += 1
  dlnaHandoffToken += 1
  globalPlayStatus.setFullPlayOpen(false)
})
</script>

<template>
  <div
    class="player-container"
    :style="!showFullPlay && 'box-shadow: none'"
    :class="{ 'full-play-idle': isFullPlayIdle && showFullPlay, 'is-full-play': showFullPlay }"
    @click.stop="toggleFullPlay"
  >
    <!-- 进度条 -->
    <div class="progress-bar-container">
      <div
        ref="progressRef"
        class="progress-bar"
        @mousedown="handleProgressDragStart($event)"
        @touchstart.prevent="handleProgressTouchStart"
        @touchmove.prevent="handleProgressTouchMove"
        @touchend="handleProgressTouchEnd"
        @click.stop="handleProgressClick"
      >
        <div class="progress-background"></div>
        <!-- 无感过渡预告区间标记 -->
        <div
          v-if="crossfadeMarkVisible"
          class="crossfade-mark"
          :style="{ left: crossfadeMarkLeft + '%', width: crossfadeMarkWidth + '%' }"
        ></div>
        <!-- 过渡进行中的活跃区段标记 -->
        <div
          v-if="crossfadeActiveMarkVisible"
          class="crossfade-active-mark"
          :style="{ left: crossfadeActiveMarkLeft + '%', width: crossfadeActiveMarkWidth + '%' }"
        ></div>
        <!-- 过渡完成后：新歌开头的淡入标记 -->
        <div
          v-if="crossfadeFadeInMarkVisible"
          class="crossfade-fadein-mark"
          :style="{ left: '0%', width: crossfadeFadeInMarkWidth + '%' }"
        ></div>
        <div class="progress-filled" :style="{ transform: `translateY(-50%) scaleX(${progressPercentage / 100})` }"></div>
        <div class="progress-handle" :style="{ left: `${progressPercentage}%` }"></div>
      </div>
    </div>

    <div class="player-content">
      <!-- 左侧：封面和歌曲信息 -->
      <div class="left-section">
        <div v-if="songInfo.songmid" class="album-cover">
          <img :src="player.cover || songCover" :style="{ opacity: coverOpacity, viewTransitionName: songInfo.songmid ? 'player-cover' : 'none' }" :alt="songInfo.name || t('common.unknownAlbum')" />
        </div>

        <div class="song-info">
          <div class="song-name">{{ songInfo.name }}</div>
          <div class="artist-name">{{ songInfo.singer }}</div>
        </div>

        <div class="left-actions">
          <t-tooltip>
            <template #content>{{ isLiked ? t('play.liked') : t('play.like') }}</template>
            <t-button
              class="control-btn"
              variant="text"
              shape="circle"
              :disabled="!songInfo.songmid"
              @click.stop="onToggleLike"
            >
              <heart-icon
                :fill-color="isLiked ? ['#FF7878', '#FF7878'] : ''"
                :stroke-color="isLiked ? [] : [contrastTextColor, contrastTextColor]"
                :stroke-width="isLiked ? 0 : 2"
                size="18"
              />
            </t-button>
          </t-tooltip>
          <t-tooltip :content="t('play.download')">
            <t-button
              class="control-btn"
              variant="text"
              shape="circle"
              :disabled="!songInfo.songmid"
              @click.stop="onDownload"
            >
              <DownloadIcon size="18" />
            </t-button>
          </t-tooltip>
          <Transition name="comment-fade" mode="out-in" appear>
            <div v-if="songInfo.source !== 'local' && showFullPlay" class="comment-btn-wrapper">
              <t-tooltip :content="t('play.comment')">
                <t-button
                  class="control-btn"
                  variant="text"
                  shape="circle"
                  :disabled="!songInfo.songmid"
                  @click.stop="toggleComments"
                >
                  <chat-bubble-1-icon
                    :fill-color="'transparent'"
                    :stroke-color="'currentColor'"
                    :stroke-width="1.5"
                  />
                </t-button>
              </t-tooltip>
            </div>
          </Transition>
        </div>
      </div>

      <!-- 中间：播放控制 -->
      <div class="center-controls">
        <t-button class="control-btn" variant="text" shape="circle" @click.stop="playPrevious">
          <span class="iconfont icon-shangyishou"></span>
        </t-button>
        <button
          class="control-btn play-btn"
          :disabled="isLoadingSong"
          @click.stop="() => !isLoadingSong && handleTogglePlayPause()"
        >
          <transition name="fade" mode="out-in">
            <div v-if="isLoadingSong" key="loading" class="loading-spinner play-loading"></div>
            <span v-else-if="Audio.isPlay" key="play" class="iconfont icon-zanting"></span>
            <span v-else key="pause" class="iconfont icon-bofang"></span>
          </transition>
        </button>
        <t-button class="control-btn" shape="circle" variant="text" @click.stop="playNext">
          <span class="iconfont icon-xiayishou"></span>
        </t-button>
      </div>

      <!-- 右侧：时间和其他控制 -->
      <div class="right-section">
        <div class="time-display">{{ currentTimeFormatted }} / {{ durationFormatted }}</div>

        <!-- 移动端播放控制 -->
        <div class="mobile-play-controls">
          <button
            class="control-btn"
            :disabled="!songInfo.songmid"
            @click.stop="onToggleLike"
          >
            <heart-icon
              :fill-color="isLiked ? ['#FF7878', '#FF7878'] : ''"
              :stroke-color="isLiked ? [] : [contrastTextColor, contrastTextColor]"
              :stroke-width="isLiked ? 0 : 1.5"
              size="18"
            />
          </button>
          <button
            class="control-btn"
            @click.stop="playPrevious"
          >
            <span class="iconfont icon-shangyishou"></span>
          </button>
          <button
            class="control-btn play-btn"
            :disabled="isLoadingSong"
            @click.stop="() => !isLoadingSong && handleTogglePlayPause()"
          >
            <transition name="fade" mode="out-in">
              <div v-if="isLoadingSong" key="loading" class="loading-spinner play-loading"></div>
              <span v-else-if="Audio.isPlay" key="play" class="iconfont icon-zanting"></span>
              <span v-else key="pause" class="iconfont icon-bofang"></span>
            </transition>
          </button>
          <button
            class="control-btn"
            @click.stop="playNext"
          >
            <span class="iconfont icon-xiayishou"></span>
          </button>
          <button
            class="control-btn"
            @click.stop="togglePlaylist"
          >
            <liebiao style="width: 1.2em; height: 1.2em" />
          </button>
        </div>

        <div class="extra-controls">
          <!-- 播放模式按钮 -->
          <t-tooltip>
            <template #content>{{ playModeTip }}</template>
            <t-button
              class="control-btn"
              shape="circle"
              variant="text"
              @click.stop="updatePlayMode"
            >
              <i :class="playModeIconClass + ' ' + 'PlayMode'" style="width: 1.5em"></i>
            </t-button>
          </t-tooltip>

          <!-- 音量控制 -->
          <div
            class="volume-control"
            @mouseenter="showVolumeSlider = true"
            @mouseleave="showVolumeSlider = false"
            @wheel.prevent="handleVolumeWheel"
          >
            <button class="control-btn">
              <shengyin style="width: 1.5em; height: 1.5em" />
            </button>

            <!-- 音量滑块 -->
            <transition name="volume-popup">
              <div v-show="showVolumeSlider" class="volume-slider-container" @click.stop>
                <div class="volume-slider">
                  <div
                    ref="volumeBarRef"
                    class="volume-bar"
                    @click="handleVolumeClick"
                    @mousedown="handleVolumeDragStart"
                  >
                    <div class="volume-background"></div>
                    <div class="volume-filled" :style="{ height: `${volumeValue}%` }"></div>
                    <div class="volume-handle" :style="{ bottom: `${volumeValue}%` }"></div>
                  </div>
                  <div class="volume-value">{{ volumeValue }}%</div>
                </div>
              </div>
            </transition>
          </div>

          <!-- 桌面歌词开关按钮 -->
          <t-tooltip>
            <template #content>
              {{ desktopLyricOpen ? (desktopLyricLocked ? t('play.unlockLyric') : t('play.closeDesktopLyric')) : t('play.openDesktopLyric') }}
            </template>
            <t-button
              class="control-btn lyric-btn"
              shape="circle"
              variant="text"
              :disabled="!songInfo.songmid"
              @click.stop="toggleDesktopLyric"
            >
              <SvgIcon name="lyricOpen" size="18"></SvgIcon>
              <transition name="fade" mode="out-in">
                <template v-if="desktopLyricOpen">
                  <LockOnIcon v-if="desktopLyricLocked" key="lock" class="lyric-lock" size="8" />
                  <CheckIcon v-else key="check" class="lyric-check" size="8" />
                </template>
              </transition>
            </t-button>
          </t-tooltip>

          <!-- 播放列表按钮 -->
          <t-tooltip :content="t('play.playlist')">
            <n-badge :value="list.length" :max="99" color="#bbb">
              <t-button
                class="control-btn"
                shape="circle"
                variant="text"
                @click.stop="togglePlaylist"
              >
                <liebiao style="width: 1.5em; height: 1.5em" />
              </t-button>
            </n-badge>
          </t-tooltip>
        </div>
      </div>
    </div>
  </div>
  <div class="fullbox">
    <FullPlay
      v-model:show-comments="showComments"
      :song-id="songInfo.songmid ? songInfo.songmid.toString() : null"
      :show="showFullPlay"
      :cover-image="player.cover"
      :song-info="songInfo"
      :main-color="maincolor"
      @toggle-fullscreen="toggleFullPlay"
      @idle-change="handleIdleChange"
    />
  </div>

  <!-- 播放列表组件 -->
  <PlaylistDrawer
    ref="playlistDrawerRef"
    :show="showPlaylist"
    :current-song-id="currentSongId"
    :full-screen-mode="showFullPlay"
    @close="closePlaylist"
    @play-song="playSong"
  />

  <!-- 无感过渡提示 -->
  <CrossfadeHint />
</template>

<style lang="scss" scoped>
.fade-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
}

.fade-enter-active {
  transition: opacity var(--motion-duration-instant) var(--motion-ease-standard), transform var(--motion-duration-instant) var(--motion-ease-standard);
}

.fade-leave-to {
  opacity: 0;
  transform: rotate(180deg);
}

.fade-enter-from {
  opacity: 0;
  transform: rotate(-180deg);
}

.comment-btn-wrapper {
  display: inline-flex;
  will-change: opacity, transform;
}

.comment-fade-enter-active,
.comment-fade-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
}
.comment-fade-enter-from,
.comment-fade-leave-to {
  opacity: 0;
  transform: scale(0.9);
}

/* 加载动画 */
.loading-spinner {
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top: 2px solid v-bind(hoverColor);
  border-radius: 50%;
  will-change: transform; animation: spin 1s linear infinite;
  display: inline-block;
  width: 1em;
  height: 1em;
}

/* 播放按钮中的加载动画 */
.play-loading {
  width: 20px !important;
  height: 20px !important;
  margin: 4px;
  border-width: 3px;
  border-color: rgba(255, 255, 255, 0.3);
  border-top-color: v-bind(hoverColor);
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}

@keyframes crossfade-pulse {
  0%,
  100% {
    opacity: 0.65;
    filter: brightness(1);
  }
  50% {
    opacity: 1;
    filter: brightness(1.35);
  }
}

@keyframes crossfade-fadein-glow {
  0% {
    opacity: 0;
    transform: translateY(-50%) scaleX(0.6);
    transform-origin: left center;
  }
  35% {
    opacity: 1;
    transform: translateY(-50%) scaleX(1);
  }
  100% {
    opacity: 0.9;
    transform: translateY(-50%) scaleX(1);
  }
}

/* 加载歌曲过渡动画 - 缩小透明效果 */
.loadSong-enter-active,
.loadSong-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
}

.loadSong-enter-from,
.loadSong-leave-to {
  opacity: 0;
  transform: scale(0.8);
}

.loadSong-enter-to,
.loadSong-leave-from {
  opacity: 1;
  transform: scale(1);
}

.player-container {
  box-shadow: 0px -1px 8px 0px #00000039;
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  transition:
    transform var(--motion-duration-expressive) var(--motion-ease-spring),
    background-color var(--motion-duration-standard) var(--motion-ease-standard);
  background: v-bind(bg);
  // border-top: 1px solid #e5e7eb;
  backdrop-filter: saturate(var(--mobile-glass-saturate, 180%)) blur(var(--glass-blur-panel));
  -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate, 180%)) blur(var(--glass-blur-panel));
  z-index: var(--mobile-player-layer-z, 1000);
  height: var(--play-bottom-height);
  display: flex;
  flex-direction: column;

  &.full-play-idle {
    transform: translateY(100%);
  }
}

/* 进度条样式 */
.progress-bar-container {
  width: 100%;
  --touch-range-height: 20px;
  --play-line-height: 4px;
  height: calc(var(--touch-range-height) + var(--play-line-height)); // 放大可点击区域，但保持视觉细
  position: absolute;
  top: calc(var(--touch-range-height) / 2 * -1);
  cursor: pointer;

  .progress-bar {
    width: 100%;
    height: 100%;
    position: relative;

    // 视觉上的细轨道，垂直居中
    .progress-background,
    .progress-filled {
      position: absolute;
      left: 0;
      right: 0;
      height: var(--play-line-height);
      top: 50%;
      transform: translateY(-50%);
      border-radius: 999px;
    }

    .progress-background {
      background: transparent;
    }

    .progress-filled {
      background: linear-gradient(to right, v-bind(startmaincolor), v-bind(maincolor) 80%);
      transform-origin: left;
    }

    // 无感过渡预告区间：在进度条末尾以斜纹/半透明条块显示
    .crossfade-mark {
      position: absolute;
      top: 50%;
      transform: translateY(-50%);
      height: var(--play-line-height);
      border-radius: 999px;
      pointer-events: none;
      background: repeating-linear-gradient(
        45deg,
        rgba(255, 255, 255, 0.35),
        rgba(255, 255, 255, 0.35) 3px,
        rgba(255, 255, 255, 0.1) 3px,
        rgba(255, 255, 255, 0.1) 6px
      );
      box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.18);
      opacity: 0.75;
      transition:
        left 0.2s linear,
        width 0.2s linear,
        height 0.2s ease;
    }

    // 过渡进行中的活跃区段：更醒目的脉动高亮
    .crossfade-active-mark {
      position: absolute;
      top: 50%;
      transform: translateY(-50%);
      height: var(--play-line-height);
      border-radius: 999px;
      pointer-events: none;
      background: linear-gradient(
        90deg,
        rgba(255, 255, 255, 0.85),
        v-bind(maincolor) 50%,
        rgba(255, 255, 255, 0.85)
      );
      box-shadow: 0 0 8px rgba(255, 255, 255, 0.6);
      will-change: transform, opacity; animation: crossfade-pulse 1.2s ease-in-out infinite;
    }

    // 过渡完成后：新歌开头的淡入段标记（从左向右渐弱的条带 + 轻微辉光）
    .crossfade-fadein-mark {
      position: absolute;
      top: 50%;
      transform: translateY(-50%);
      height: var(--play-line-height);
      border-radius: 999px;
      pointer-events: none;
      background: linear-gradient(
        90deg,
        rgba(255, 255, 255, 0.7),
        rgba(255, 255, 255, 0.25) 70%,
        rgba(255, 255, 255, 0)
      );
      box-shadow: 0 0 6px rgba(255, 255, 255, 0.4);
      opacity: 0.9;
      animation: crossfade-fadein-glow 2.4s ease-out;
      transition:
        width 0.3s linear,
        opacity 0.8s ease,
        height 0.2s ease;
    }

    .progress-handle {
      position: absolute;
      top: 50%;
      width: 12px;
      height: 12px;
      background: v-bind(hoverColor);
      border-radius: 50%;
      transform: translate(-50%, -50%);
      opacity: 0;

      &:hover,
      &:active,
      &.dragging {
        opacity: 1;
      }
    }

    // 悬停或拖拽时，轻微加粗提升可见性
    &:hover {
      .progress-background,
      .progress-filled,
      .crossfade-mark,
      .crossfade-active-mark,
      .crossfade-fadein-mark {
        height: 6px;
      }
    }
    &:has(.progress-handle.dragging) {
      .progress-background,
      .progress-filled,
      .crossfade-mark,
      .crossfade-active-mark,
      .crossfade-fadein-mark {
        height: 6px;
      }
    }

    &:hover .progress-handle {
      opacity: 1;
    }
  }
}

/* 播放器内容 */
.player-content {
  user-select: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 40px;
  height: calc(100% - 4px);
}

/* 左侧：封面和歌曲信息 */
.left-section {
  display: flex;
  align-items: center;
  min-width: 0;
  flex: 1;
  padding-top: 2px;

  .album-cover {
    width: 50px;
    height: 50px;
    border-radius: 4px;
    overflow: hidden;
    margin-right: 12px;
    flex-shrink: 0;

    img {
      user-select: none;
      width: 100%;
      height: 100%;
      object-fit: cover;
      -webkit-user-drag: none;
      transition: opacity var(--motion-duration-standard) var(--motion-ease-standard);
    }
  }

  .song-info {
    min-width: 0;

    .song-name {
      font-size: 14px;
      font-weight: 700;
      color: v-bind(hoverColor);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      margin-bottom: 4px;
    }

    .artist-name {
      font-size: 12px;
      color: v-bind(contrastTextColor);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }
}

/* 左侧操作按钮 */
.left-actions {
  display: flex;
  align-items: center;
  gap: 3px;
  margin-left: 12px;

  .control-btn {
    background: transparent;
    border: none;
    color: v-bind(contrastTextColor);
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    justify-content: center;

    .iconfont {
      font-size: 18px;
    }

    &:hover {
      color: v-bind(hoverColor);
    }

    &:disabled {
      cursor: not-allowed;
      opacity: 0.6;
    }
  }
}

/* 中间：播放控制 */
.center-controls {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  flex: 1;

  .control-btn {
    background: transparent;
    border: none;
    color: v-bind(contrastTextColor);
    cursor: pointer;
    padding: 5px;
    display: flex;
    align-items: center;
    justify-content: center;

    span {
      font-size: 28px;
    }

    &:hover {
      color: v-bind(hoverColor);
    }

    &.play-btn {
      width: 42px;
      height: 42px;
      background-color: v-bind(playbg);
      transition: background-color var(--motion-duration-quick) var(--motion-ease-standard);

      border-radius: 50%;

      span {
        font-size: 28px;
        font-weight: 800;
        color: v-bind(hoverColor);
      }

      .play-icon {
        width: 24px;
        height: 24px;
      }

      &:hover {
        background-color: v-bind(playbghover);
        color: v-bind(contrastTextColor);
      }
    }
  }
}

/* 右侧：时间和其他控制 */
.right-section {
  display: flex;
  align-items: center;
  gap: 16px;
  flex: 1;
  justify-content: flex-end;

  .time-display {
    font-size: 12px;
    line-height: 12px;
    color: v-bind(contrastTextColor);
    white-space: nowrap;
  }

  .extra-controls {
    display: flex;
    align-items: center;
    gap: 12px;

    .control-btn {
      background: transparent;
      border: none;
      color: v-bind(contrastTextColor);
      cursor: pointer;
      padding: 4px;
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;

      .iconfont {
        font-size: 18px;
      }

      &:hover {
        color: v-bind(hoverColor);
      }

      &.lyric-btn .lyric-check,
      &.lyric-btn .lyric-lock {
        position: absolute;
        right: -1px;
        bottom: -1px;
        background: #fff;
        border-radius: 50%;
        box-shadow: 0 0 0 2px #fff;
        color: v-bind(maincolor);
      }
    }
  }
}

/* 音量控制 */
.volume-control {
  position: relative;
}

.volume-slider-container {
  position: absolute;
  bottom: calc(100% + 10px);
  /* 向上偏移，留出间距 */
  right: -10px;
  /* 位置微调 */
  background: v-bind(contrastTextColor);
  /* 毛玻璃背景 */
  backdrop-filter: blur(var(--glass-blur-panel));
  border-radius: 8px;
  padding: 15px 10px;
  width: 40px;
  height: 150px;
  z-index: 1000;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  align-items: center;
  transform-origin: bottom center;
  /* 设置变换原点，使弹出效果更自然 */
}

.volume-slider {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: space-between;
  height: 100%;
  width: 100%;
  gap: 8px;
}

.volume-value {
  font-size: 12px;
  color: v-bind(maincolor);
  margin-top: 8px;
}

.volume-bar {
  width: 4px;
  height: 100px;
  position: relative;
  cursor: pointer;
}

.volume-background {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: #ffffff71;
  border-radius: 2px;
}

.volume-filled {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  background: v-bind(maincolor);
  border-radius: 2px;
}

.volume-handle {
  position: absolute;
  left: 50%;
  width: 12px;
  height: 12px;
  background: v-bind(maincolor);
  border-radius: 50%;
  transform: translate(-50%, 50%);
  opacity: 1;
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard);
}

// .volume-bar:hover .volume-handle {
//   opacity: 1;
// }

/* 音量条弹出过渡 */
.volume-popup-enter-active,
.volume-popup-leave-active {
  transition:
    opacity 0.2s cubic-bezier(0.8, 0, 0.8, 0.43),
    transform 0.2s cubic-bezier(0.8, 0, 0.8, 0.43);
}

.volume-popup-enter-from,
.volume-popup-leave-to {
  opacity: 0;
  transform: translateY(10px) scale(0.95);
}

/* 移动端播放控制（默认隐藏） */
.mobile-play-controls {
  display: none;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .player-container {
    background: v-bind(mobilePlayerBg) !important;
    bottom: var(--mobile-player-bottom);
    height: var(--mobile-player-height);
    border-top: 0.5px solid var(--mobile-glass-border);

    &::after {
      content: '';
      position: absolute;
      top: 100%;
      right: 0;
      left: 0;
      height: var(--mobile-nav-total-height);
      background: v-bind(mobilePlayerBg);
      pointer-events: none;
    }
  }

  .player-container.is-full-play {
    bottom: var(--mobile-safe-bottom);

    &::after {
      display: none;
    }
  }

  .player-content {
    padding: 0 max(10px, calc(var(--mobile-page-gutter) - 6px));
    gap: 0;
  }

  /* 手机端进度条：加大触摸区域和视觉尺寸 */
  .progress-bar-container {
    --touch-range-height: 28px;
    --play-line-height: 4px;
    top: calc(var(--touch-range-height) / 2 * -1);

    .progress-bar {
      .progress-handle {
        width: 12px;
        height: 12px;
        opacity: 1;
      }

      &:active {
        .progress-background,
        .progress-filled,
        .crossfade-mark,
        .crossfade-active-mark,
        .crossfade-fadein-mark {
          height: 8px;
        }
        .progress-handle {
          width: 16px;
          height: 16px;
          opacity: 1;
        }
      }
    }
  }

  /* 左侧：封面 + 歌曲信息，撑满剩余空间 */
  .left-section {
    flex: 1;
    min-width: 0;
    padding-top: 0;

    .album-cover {
      width: 42px;
      height: 42px;
      margin-right: 10px;
      border-radius: 6px;
    }

    .song-info {
      min-width: 0;

      .song-name {
        font-size: 13px;
        margin-bottom: 2px;
      }

      .artist-name {
        font-size: 11px;
      }
    }

    .left-actions {
      display: none;
    }
  }

  /* 中间：隐藏，控制按钮移到右侧 */
  .center-controls {
    display: none;
  }

  /* 右侧：只保留移动端控制 */
  .right-section {
    flex: 0 0 auto;
    gap: 0;
    margin-left: 4px;

    .time-display {
      display: none;
    }

    .extra-controls {
      display: none;
    }
  }

  /* 移动端播放控制组 */
  .mobile-play-controls {
    display: flex;
    align-items: center;
    gap: 0;
    flex-shrink: 0;

    .control-btn {
      background: transparent;
      border: none;
      color: v-bind(contrastTextColor);
      cursor: pointer;
      min-width: var(--mobile-touch-target);
      width: var(--mobile-touch-target);
      height: var(--mobile-touch-target);
      padding: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: var(--mobile-control-radius);
      transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);

      &:active {
        transform: scale(0.94);
      }

      .iconfont {
        font-size: 20px;
      }

      &:hover {
        color: v-bind(hoverColor);
      }

      &:disabled {
        opacity: 0.4;
      }

      &.play-btn {
        width: var(--mobile-touch-target);
        height: var(--mobile-touch-target);
        background-color: v-bind(playbg);
        border-radius: 50%;

        span {
          font-size: 22px;
          font-weight: 800;
          color: v-bind(hoverColor);
        }

        &:hover {
          background-color: v-bind(playbghover);
        }
      }
    }
  }
}

@media (max-width: 576px) {
  .left-section .song-info {
    max-width: 140px;
  }
}
</style>
