import { ref, toRaw } from 'vue'
import { ControlAudioStore } from '@/store/ControlAudio'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { useSettingsStore } from '@/store/Settings'
import { playSetting } from '@/store/playSetting'
import {
  PlayMode,
  type PlayerSnapshotPayload,
  type PlayerSnapshotResult,
  type SongList,
  type UnsubscribeFunction
} from '@/types/audio'
import { MessagePlugin } from 'tdesign-vue-next'
import { calculateBestQuality, compareQuality, normalizeTypes } from '@/utils/quality'
import { musicSdk } from '@/services/musicSdk'
import PluginRunner from '@/utils/plugin/PluginRunner'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { platform } from '@tauri-apps/plugin-os'
import i18n from '@/locales'

export { isLoadingSong } from './loadingState'
import { isLoadingSong } from './loadingState'
export const playMode = ref<PlayMode>(PlayMode.LIST)

let _playIndex = -1
let currentPlayRequestId = 0
let playbackRequestSeq = 0
let playbackAttemptId = 0
let handledShortDurationAttemptId = 0
let shortDurationHandling = false
let shortDurationSongKey = ''
let currentPlaybackUrl = ''
let currentResolvedSong: SongList | null = null
let shortDurationUnsubscribers: UnsubscribeFunction[] = []

// ===== 无缝换曲预加载状态 =====
let prefetchRequestId = 0
let preloadedSong: SongList | null = null
let preloadedReady = false
let unlistenPreload: UnlistenFn | null = null
let prefetchTimer: ReturnType<typeof setTimeout> | null = null
let isAndroidPlatform: boolean | null = null
let androidNativeQueueSongByUrl = new Map<string, SongList>()
let androidSyncingPlaybackState = false

// ===== Android 通知栏/锁屏媒体按钮 =====
// Rust 端 nativePlayerCommand 已直接控制播放器，前端只负责 UI 状态同步
let _mediaButtonUnlisten: UnlistenFn | null = null

export async function setupMediaButtonListener() {
  if (_mediaButtonUnlisten) return
  _mediaButtonUnlisten = await listen<string>('media-button-command', (event) => {
    const cmd = event.payload
    const audio = ControlAudioStore()
    switch (cmd) {
      case 'play':
      case 'resume':
        audio.Audio.isPlay = true
        break
      case 'pause':
      case 'stop':
        audio.Audio.isPlay = false
        break
      case 'next':
        playNext()
        break
      case 'prev':
        playPrevious()
        break
    }
  })
}

/** 预加载提前量（秒）：剩余时长低于此值时开始预加载 */
const PREFETCH_LEAD_TIME = 30
const MIN_PLAYABLE_DURATION = 20
const SHORT_DURATION_SKIP_SOURCE_KEY = '__short_duration_skip_source__'

/**
 * 计算歌曲的最佳音质（用于 URL 解析和缓存键）
 */
function getSongQuality(song: SongList): string {
  if (song.source === 'local') return 'local'
  const localUserStore = LocalUserDetailStore()
  let quality =
    (localUserStore.userInfo.sourceQualityMap || {})[song.source || ''] ||
    (localUserStore.userSource.quality as string)
  return calculateBestQuality(song.types, quality) || '128k'
}

/**
 * 构造缓存键：{source}_{songmid}_{quality}
 */
function buildCacheKey(song: SongList): string | undefined {
  if (song.source === 'local' || !song.songmid) return undefined
  const quality = getSongQuality(song)
  return `${song.source}_${song.songmid}_${quality}`
}

/**
 * 通过音乐插件解析歌曲真实播放 URL
 */
export async function getSongRealUrl(song: SongList): Promise<string> {
  try {
    if (song.source === 'local') {
      let path = (song as any).path
      if (!path) {
        const res = await (window as any).api?.localMusic?.getUrlById?.(song.songmid)
        const data = res?.success ? res.data : null
        path = data?.path
      }
      if (!path) throw new Error(i18n.global.t('play.localNoPath'))
      return path
    }
    if (song.url && typeof song.url === 'string') {
      return song.url
    }
    const localUserStore = LocalUserDetailStore()
    let quality =
      (localUserStore.userInfo.sourceQualityMap || {})[song.source || ''] ||
      (localUserStore.userSource.quality as string)
    quality = calculateBestQuality(song.types, quality) || '128k'

    const pluginQualities = localUserStore.userInfo.supportedSources?.[song.source || '']?.qualitys
    if (pluginQualities?.length && !pluginQualities.includes(quality)) {
      const songTypes = normalizeTypes(song.types)
      const available = (songTypes.length ? songTypes : pluginQualities)
        .filter(t => pluginQualities.includes(t))
      if (available.length) {
        quality = [...available].sort(compareQuality)[0]
      }
    }

    if (song.source === 'subsonic') {
      const rawUrl = await musicSdk.getMusicUrl(toRaw(song) as any, quality)
      if (!rawUrl || typeof rawUrl !== 'string') {
        throw new Error(i18n.global.t('play.cannotGetUrl'))
      }
      return rawUrl
    }

    const pluginId = localUserStore.userSource.pluginId
    if (!pluginId) {
      throw new Error(i18n.global.t('play.noSourcePlugin'))
    }

    let rawUrl: string | null = null

    try {
      rawUrl = await PluginRunner.getMusicUrl(
        pluginId, song.source || 'kw', toRaw(song) as any, quality
      )
    } catch (e: any) {
      throw new Error(i18n.global.t('play.pluginParseFailed', { message: e?.message || e }))
    }

    if (!rawUrl || typeof rawUrl !== 'string') {
      throw new Error(i18n.global.t('play.cannotGetUrl'))
    }

    return rawUrl
  } catch (error: any) {
    throw new Error(i18n.global.t('play.getUrlFailed', { message: error.message || '' }))
  }
}

// ===== 自动换源 =====

interface FindMusicCandidate {
  songmid: string | number
  name: string
  singer: string
  albumName?: string
  source?: string
  interval?: string
  types?: any
  [key: string]: any
}

/**
 * 通过 Rust 后端跨源搜索匹配候选歌曲，逐个尝试获取 URL
 */
export async function autoSwitchSource(song: SongList): Promise<{ url: string; song: SongList } | null> {
  if (song.source === 'local') return null

  const skippedSources = getSkippedShortDurationSources(song)
  let candidates: FindMusicCandidate[] = []
  try {
    const result: any = await invoke('service_music_find_music', {
      name: song.name,
      singer: song.singer,
      albumName: song.albumName || '',
      interval: song.interval || '',
      source: song.source || '',
    })
    if (Array.isArray(result)) {
      candidates = result
    } else if (result?.data && Array.isArray(result.data)) {
      candidates = result.data
    }
  } catch {
    return null
  }

  if (candidates.length === 0) return null

  for (const candidate of candidates) {
    try {
      const candidateSong = withShortDurationSkipSources({
        ...candidate,
        source: candidate.source,
      } as SongList, skippedSources)
      if (skippedSources.has(getSourceKey(candidateSong))) continue
      const url = await getSongRealUrl(candidateSong)
      if (url && typeof url === 'string') {
        return { url, song: candidateSong }
      }
    } catch {
    }
  }

  return null
}


function getSongKey(song: SongList | null): string {
  if (!song) return ''
  return `${song.source || ''}:${String(song.songmid)}`
}

function getSourceKey(song: SongList): string {
  return `${song.source || ''}:${String(song.songmid)}`
}

function getSkippedShortDurationSources(song: SongList): Set<string> {
  return (song as any)[SHORT_DURATION_SKIP_SOURCE_KEY] instanceof Set
    ? (song as any)[SHORT_DURATION_SKIP_SOURCE_KEY]
    : new Set<string>()
}

function withShortDurationSkipSources(song: SongList, skippedSources: Set<string>): SongList {
  const nextSong = { ...toRaw(song) } as SongList
  Object.defineProperty(nextSong, SHORT_DURATION_SKIP_SOURCE_KEY, {
    value: skippedSources,
    enumerable: false,
    configurable: true,
  })
  return nextSong
}

function withSkippedShortDurationSource(song: SongList, sourceKey: string): SongList {
  const skipped = new Set(getSkippedShortDurationSources(song))
  skipped.add(sourceKey)
  const nextSong = { ...toRaw(song) } as SongList
  Object.defineProperty(nextSong, SHORT_DURATION_SKIP_SOURCE_KEY, {
    value: skipped,
    enumerable: false,
    configurable: true,
  })
  return nextSong
}

function resetPlaybackAttempt(song: SongList, url: string) {
  playbackAttemptId++
  handledShortDurationAttemptId = 0
  shortDurationHandling = false
  shortDurationSongKey = getSongKey(song)
  currentPlaybackUrl = url
  currentResolvedSong = song
}

function resetPlaybackTracking() {
  playbackAttemptId++
  handledShortDurationAttemptId = 0
  shortDurationHandling = false
  shortDurationSongKey = ''
  currentPlaybackUrl = ''
  currentResolvedSong = null
}


function shouldHandleShortDuration(): boolean {
  const audio = ControlAudioStore()
  return (
    audio.Audio.duration > 0 &&
    audio.Audio.duration < MIN_PLAYABLE_DURATION &&
    !!currentResolvedSong &&
    !!currentPlaybackUrl &&
    audio.Audio.url === currentPlaybackUrl &&
    getSongKey(currentResolvedSong) === shortDurationSongKey &&
    handledShortDurationAttemptId !== playbackAttemptId &&
    !shortDurationHandling
  )
}

function stopPlaybackForUnplayable() {
  const audio = ControlAudioStore()
  invoke('player__pause')
  audio.Audio.isPlay = false
  isLoadingSong.value = false
  resetPlaybackTracking()
}

function playNextAfterUnplayable(): SongList | null {
  const store = LocalUserDetailStore()
  if (store.list.length === 0) {
    stopPlaybackForUnplayable()
    return null
  }

  if (store.list.length === 1 || (playMode.value === PlayMode.LIST && _playIndex >= store.list.length - 1)) {
    stopPlaybackForUnplayable()
    return null
  }

  if (playMode.value === PlayMode.RANDOM) {
    let nextIndex = _playIndex
    while (nextIndex === _playIndex) {
      nextIndex = Math.floor(Math.random() * store.list.length)
    }
    _playIndex = nextIndex
  } else {
    _playIndex = (_playIndex + 1) % store.list.length
  }

  const song = store.list[_playIndex]
  if (song) playSong(song)
  return song
}

async function playResolvedSource(song: SongList, url: string) {
  const requestId = ++playbackRequestSeq
  currentPlayRequestId = requestId
  const audio = ControlAudioStore()

  try {
    isLoadingSong.value = true
    resetPlaybackAttempt(song, url)
    useGlobalPlayStatusStore().updatePlaybackSongInfo(toRaw(song) as any)
    const cacheKey = buildCacheKey(song)
    const result: any = await invoke('player__play', { url, cacheKey })
    if (currentPlayRequestId !== requestId) return

    if (result && !result.success) {
      throw new Error(result.error || i18n.global.t('play.playerStartFailed'))
    }

    await invoke('player__set_volume', { volume: audio.Audio.volume })
    scheduleNextPrefetch()
  } catch {
    if (currentPlayRequestId !== requestId) return
    isLoadingSong.value = false
    resetPlaybackTracking()
    playNextAfterUnplayable()
  }
}

async function handleShortDurationPlayback() {
  if (!shouldHandleShortDuration()) return

  const attemptId = playbackAttemptId
  handledShortDurationAttemptId = attemptId
  shortDurationHandling = true
  const song = currentResolvedSong
  const sourceKey = song ? getSourceKey(song) : ''

  try {
    invalidatePrefetch()
    await invoke('player__pause')
    if (!song) return

    const songWithSkippedSource = withSkippedShortDurationSource(song, sourceKey)
    const switched = await autoSwitchSource(songWithSkippedSource)
    if (attemptId !== playbackAttemptId) return

    if (switched) {
      await playResolvedSource(withSkippedShortDurationSource(switched.song, sourceKey), switched.url)
      return
    }

    playNextAfterUnplayable()
  } catch {
    if (attemptId === playbackAttemptId) playNextAfterUnplayable()
  } finally {
    if (attemptId === playbackAttemptId) shortDurationHandling = false
  }
}

export function installShortDurationGuard() {
  const audio = ControlAudioStore()
  shortDurationUnsubscribers.forEach((unsubscribe) => unsubscribe())
  shortDurationUnsubscribers = [
    audio.subscribe('play', handleShortDurationPlayback),
    audio.subscribe('timeupdate', handleShortDurationPlayback),
  ]
}

export function uninstallShortDurationGuard() {
  shortDurationUnsubscribers.forEach((unsubscribe) => unsubscribe())
  shortDurationUnsubscribers = []
}

export function getPlayIndex(): number {
  return _playIndex
}

export function getCurrentSong(): SongList | null {
  const store = LocalUserDetailStore()
  if (_playIndex >= 0 && _playIndex < store.list.length) {
    return store.list[_playIndex]
  }
  return null
}

// ===== 计算下一首 =====

function computeNextSong(): SongList | null {
  const store = LocalUserDetailStore()
  if (store.list.length === 0) return null

  if (playMode.value === PlayMode.SINGLE) {
    return store.list[_playIndex] || null
  }
  if (playMode.value === PlayMode.RANDOM) {
    return store.list[Math.floor(Math.random() * store.list.length)]
  }
  if (playMode.value === PlayMode.LIST) {
    if (_playIndex >= store.list.length - 1) return null
    return store.list[_playIndex + 1]
  }
  // SEQUENCE
  const nextIdx = (_playIndex + 1) % store.list.length
  return store.list[nextIdx]
}

// ===== 预加载 =====

function syncSeamlessConfig() {
  const setting = playSetting()
  invoke('player__set_seamless_config', {
    mode: setting.seamlessMode,
    crossfadeDurationMs: setting.crossfadeDuration,
  })
}

async function isAndroid(): Promise<boolean> {
  if (isAndroidPlatform === null) {
    try {
      isAndroidPlatform = platform() === 'android'
    } catch {
      isAndroidPlatform = false
    }
  }
  return isAndroidPlatform
}

async function shouldNativePrefetch(): Promise<boolean> {
  const setting = playSetting()
  if (setting.isSeamlessTransition) return true
  return isAndroid()
}

let cacheConfigSynced = false

async function syncCacheConfig() {
  if (cacheConfigSynced) return
  try {
    const dirs = await invoke<{ cacheDir: string; downloadDir: string }>('get_directories')
    const settingsStore = useSettingsStore()
    const maxSize = settingsStore.settings.cacheSizeLimit || 1073741824
    await invoke('player__set_cache_config', {
      cacheDir: settingsStore.settings.autoCacheMusic !== false ? dirs.cacheDir : null,
      maxSize,
    })
    cacheConfigSynced = true
  } catch (e) {
    console.warn('同步缓存配置失败:', e)
  }
}

async function ensurePreloadListener() {
  if (unlistenPreload) return
  unlistenPreload = await listen('player:preload_ready', () => {
    preloadedReady = true
  })
}

/** 实际执行预加载（URL 解析 + 下载到 secondary slot） */
async function syncAndroidNativeQueue(startIndex = _playIndex) {
  if (!(await isAndroid())) return
  const store = LocalUserDetailStore()
  if (store.list.length <= 1) return

  const items: Array<{ url: string; cacheKey?: string }> = []
  const songByUrl = new Map<string, SongList>()
  const maxItems = Math.min(store.list.length - 1, 10)
  const seen = new Set<string>()

  for (let offset = 1; offset <= maxItems; offset++) {
    const idx = (startIndex + offset) % store.list.length
    if (playMode.value === PlayMode.LIST && idx <= startIndex) break
    const song = store.list[idx]
    if (!song || song.source === 'local') continue
    const key = getSongKey(song)
    if (seen.has(key)) continue
    seen.add(key)
    try {
      const url = await getSongRealUrl(toRaw(song) as any)
      items.push({ url, cacheKey: buildCacheKey(toRaw(song) as any) })
      songByUrl.set(url, toRaw(song) as SongList)
    } catch {
    }
  }

  androidNativeQueueSongByUrl = songByUrl
  if (items.length) {
    await invoke('player__set_native_queue', { items })
  }
}

export async function syncAndroidCurrentPlaybackState() {
  if (androidSyncingPlaybackState || !(await isAndroid())) return
  androidSyncingPlaybackState = true
  try {
    const result = await invoke<PlayerSnapshotResult>('player__get_state')
    const snapshot = result?.data
    if (!snapshot?.url) return
    const url = snapshot.url

    const song = androidNativeQueueSongByUrl.get(url)
    if (!song) return

    const store = LocalUserDetailStore()
    const audio = ControlAudioStore()
    const globalPlayStatus = useGlobalPlayStatusStore()
    const index = store.list.findIndex((s) => getSongKey(s) === getSongKey(song))
    if (index >= 0) _playIndex = index

    store.userInfo.lastPlaySongId = song.songmid
    globalPlayStatus.player.songInfo = toRaw(song) as any
    globalPlayStatus.updatePlaybackSongInfo(toRaw(song) as any)
    audio.Audio.url = url
    audio.Audio.currentTime = snapshot.position || 0
    audio.Audio.duration = snapshot.duration || 0
    audio.Audio.volume = snapshot.volume ?? audio.Audio.volume
    audio.Audio.isPlay = !!snapshot.isPlaying
    resetPlaybackAttempt(song, url)

    try {
      await invoke('player__update_now_playing', {
        title: song.name || i18n.global.t('common.unknownSong'),
        artist: song.singer || i18n.global.t('common.unknownArtist'),
        album: song.albumName || '',
        duration: snapshot.duration || 0,
        coverUrl: song.img || null
      })
    } catch {}

    scheduleNextPrefetch()
    syncAndroidNativeQueue(_playIndex)
  } finally {
    androidSyncingPlaybackState = false
  }
}

function isPlayerSnapshotResult(
  payload: PlayerSnapshotPayload | PlayerSnapshotResult
): payload is PlayerSnapshotResult {
  return 'data' in payload
}

async function doPrefetchNext() {
  const nextSong = computeNextSong()
  if (!nextSong || nextSong.source === 'local') return

  const requestId = ++prefetchRequestId

  try {
    const url = await getSongRealUrl(toRaw(nextSong) as any)
    if (requestId !== prefetchRequestId) return

    const cacheKey = buildCacheKey(toRaw(nextSong) as any)
    await invoke('player__preload', { url, cacheKey })
    if (requestId !== prefetchRequestId) return
    preloadedSong = nextSong
  } catch {
  }
}

/**
 * 调度下一曲预加载：当剩余时长 > 30s 时延迟触发，否则立即执行。
 * 手动切歌会通过 invalidatePrefetch() 取消待执行的 timer。
 */
export async function scheduleNextPrefetch() {
  if (!(await shouldNativePrefetch())) return

  syncSeamlessConfig()
  ensurePreloadListener()

  preloadedSong = null
  preloadedReady = false

  // 清除上一次的延迟 timer
  if (prefetchTimer !== null) {
    clearTimeout(prefetchTimer)
    prefetchTimer = null
  }

  const audio = ControlAudioStore()
  const duration = audio.Audio.duration

  // duration 有效且剩余时长足够：延迟到接近结束时再预加载
  if (duration > 0 && duration > PREFETCH_LEAD_TIME) {
    const current = audio.Audio.currentTime
    const remainingMs = (duration - current) * 1000
    const delayMs = Math.max(0, remainingMs - PREFETCH_LEAD_TIME * 1000)

    // 已经接近结束（可能 seek 过），立即触发
    if (delayMs === 0) {
      doPrefetchNext()
      return
    }

    prefetchTimer = setTimeout(() => {
      prefetchTimer = null
      doPrefetchNext()
    }, delayMs)
    return
  }

  // 短歌曲或 duration 未知：立即预加载
  doPrefetchNext()
}

export function invalidatePrefetch() {
  if (prefetchTimer !== null) {
    clearTimeout(prefetchTimer)
    prefetchTimer = null
  }
  prefetchRequestId++
  preloadedSong = null
  preloadedReady = false
  invoke('player__clear_secondary').catch(() => {})
}

// ===== 播放控制 =====

/**
 * 播放歌曲：解析 URL → Tauri Rust 原生播放
 */
export async function playSong(song: SongList) {
  // 手动切歌时清除预加载
  invalidatePrefetch()

  const requestId = ++playbackRequestSeq
  currentPlayRequestId = requestId

  const store = LocalUserDetailStore()
  const audio = ControlAudioStore()
  const globalPlayStatus = useGlobalPlayStatusStore()

  try {
    isLoadingSong.value = true

    // 首次播放时同步缓存配置
    await syncCacheConfig()

    const idx = store.list.findIndex((s) => s.songmid === song.songmid)
    if (idx === -1) {
      store.addSongToFirst(song)
      _playIndex = 0
    } else {
      _playIndex = idx
    }

    store.userInfo.lastPlaySongId = song.songmid
    globalPlayStatus.player.songInfo = toRaw(song) as any
    globalPlayStatus.updatePlaybackSongInfo(toRaw(song) as any)

    // 解析真实播放 URL（失败时自动换源）
    let url: string | undefined
    let actualSong = song
    try {
      url = await getSongRealUrl(toRaw(song) as any)
    } catch (originalError) {
      // 原源失败，尝试自动换源
      const switched = await autoSwitchSource(toRaw(song) as any)
      if (switched) {
        url = switched.url
        actualSong = switched.song
        globalPlayStatus.updatePlaybackSongInfo(toRaw(actualSong) as any)
        MessagePlugin.success(i18n.global.t('play.autoSwitchedSource', { source: switched.song.source || i18n.global.t('common.unknown') }))
      }
    }
    if (currentPlayRequestId !== requestId) return

    if (!url || typeof url !== 'string') {
      MessagePlugin.warning(i18n.global.t('play.cannotGetUrl'))
      isLoadingSong.value = false
      resetPlaybackTracking()
      return
    }

    // 调用 Rust 原生播放器
    resetPlaybackAttempt(actualSong, url)
    const cacheKey = buildCacheKey(actualSong)
    const result: any = await invoke('player__play', { url, cacheKey })
    if (currentPlayRequestId !== requestId) return

    if (result && !result.success) {
      throw new Error(result.error || i18n.global.t('play.playerStartFailed'))
    }

    // 更新 macOS 系统媒体控制
    try {
      await invoke('player__update_now_playing', {
        title: song.name || i18n.global.t('common.unknownSong'),
        artist: song.singer || i18n.global.t('common.unknownArtist'),
        album: song.albumName || '',
        duration: 0,
        coverUrl: song.img || null
      })
    } catch {}

    // 设置音量（恢复上次音量）
    await invoke('player__set_volume', { volume: audio.Audio.volume })

    // 播放成功后调度预加载
    scheduleNextPrefetch()
    syncAndroidNativeQueue(_playIndex)
  } catch (error: any) {
    if (currentPlayRequestId !== requestId) return
    isLoadingSong.value = false
    resetPlaybackTracking()
    MessagePlugin.error(error.message || i18n.global.t('play.playFailed'))
  }
}

export function playNext(): SongList | null {
  const store = LocalUserDetailStore()
  if (store.list.length === 0) return null

  // 列表播放模式：最后一曲后停止播放
  if (playMode.value === PlayMode.LIST && _playIndex >= store.list.length - 1) {
    const audio = ControlAudioStore()
    invoke('player__pause')
    audio.Audio.isPlay = false
    return null
  }

  if (playMode.value === PlayMode.SINGLE) {
    const song = store.list[_playIndex]
    if (song) { playSong(song); return song }
  }
  if (playMode.value === PlayMode.RANDOM) {
    _playIndex = Math.floor(Math.random() * store.list.length)
  } else {
    _playIndex = (_playIndex + 1) % store.list.length
  }
  const song = store.list[_playIndex]
  if (!song) return null

  const setting = playSetting()
  if (setting.isSeamlessTransition && preloadedSong?.songmid === song.songmid && preloadedReady) {
    seamlessNext().then((success) => {
      if (!success) playSong(song)
    })
    return song
  }

  playSong(song)
  return song
}

/**
 * 无缝换曲：尝试使用预加载的 secondary slot 切换
 * @returns true 表示无缝切换成功，false 表示需要回退到普通 playNext
 */
export async function seamlessNext(): Promise<boolean> {
  if (!preloadedSong || !preloadedReady) return false

  const setting = playSetting()

  if (setting.seamlessMode === 'crossfade') {
    return false
  }

  // gapless 模式：前端主动触发即时切换
  try {
    const result: any = await invoke('player__gapless_swap')
    if (result && !result.success) return false
    if (result?.data === false) return false
  } catch {
    return false
  }
  updateSeamlessState()
  return true
}

/**
 * 无缝换曲通用状态更新（crossfade 和 gapless 共用）
 */
function updateSeamlessState() {
  if (!preloadedSong) return
  const store = LocalUserDetailStore()
  const globalPlayStatus = useGlobalPlayStatusStore()
  const song = preloadedSong

  const idx = store.list.findIndex((s) => s.songmid === song.songmid)
  if (idx !== -1) {
    _playIndex = idx
  }
  store.userInfo.lastPlaySongId = song.songmid
  globalPlayStatus.player.songInfo = toRaw(song) as any
  globalPlayStatus.updatePlaybackSongInfo(toRaw(song) as any)

  try {
    invoke('player__update_now_playing', {
      title: song.name || i18n.global.t('common.unknownSong'),
      artist: song.singer || i18n.global.t('common.unknownArtist'),
      album: song.albumName || '',
      duration: 0,
      coverUrl: song.img || null
    })
  } catch {}

  const audio = ControlAudioStore()
  invoke('player__set_volume', { volume: audio.Audio.volume })

  preloadedSong = null
  preloadedReady = false
  scheduleNextPrefetch()
  syncAndroidNativeQueue(_playIndex)
}

/**
 * Rust 自动 crossfade 完成后的前端状态同步
 * Rust 后端自动完成 slot 交换后通知前端，前端使用已有的 preloadedSong 更新 UI
 */
export function onCrossfadeSwap(payload?: PlayerSnapshotPayload | PlayerSnapshotResult) {
  const audio = ControlAudioStore()
  const snapshot = payload && isPlayerSnapshotResult(payload) ? payload.data : payload

  if (snapshot) {
    Object.assign(audio.Audio, {
      url: typeof snapshot.url === 'string' ? snapshot.url : audio.Audio.url,
      isPlay: typeof snapshot.isPlaying === 'boolean' ? snapshot.isPlaying : audio.Audio.isPlay,
      currentTime: typeof snapshot.position === 'number' ? snapshot.position : audio.Audio.currentTime,
      duration: typeof snapshot.duration === 'number' ? snapshot.duration : audio.Audio.duration,
      volume: typeof snapshot.volume === 'number' ? snapshot.volume : audio.Audio.volume
    })
  }

  const song = preloadedSong
  if (!song) return

  const store = LocalUserDetailStore()
  const globalPlayStatus = useGlobalPlayStatusStore()

  const idx = store.list.findIndex((s) => s.songmid === song.songmid)
  if (idx !== -1) {
    _playIndex = idx
  }
  store.userInfo.lastPlaySongId = song.songmid
  globalPlayStatus.player.songInfo = toRaw(song) as any
  globalPlayStatus.updatePlaybackSongInfo(toRaw(song) as any)

  try {
    invoke('player__update_now_playing', {
      title: song.name || i18n.global.t('common.unknownSong'),
      artist: song.singer || i18n.global.t('common.unknownArtist'),
      album: song.albumName || '',
      duration: audio.Audio.duration,
      coverUrl: song.img || null
    })
  } catch {}

  preloadedSong = null
  preloadedReady = false
  scheduleNextPrefetch()
  syncAndroidNativeQueue(_playIndex)
}

export function playPrevious(): SongList | null {
  const store = LocalUserDetailStore()
  if (store.list.length === 0) return null
  if (playMode.value === PlayMode.RANDOM) {
    _playIndex = Math.floor(Math.random() * store.list.length)
  } else {
    _playIndex = (_playIndex - 1 + store.list.length) % store.list.length
  }
  const song = store.list[_playIndex]
  if (song) {
    playSong(song)
  }
  return song
}

export function updatePlayMode() {
  const modes = [PlayMode.LIST, PlayMode.SEQUENCE, PlayMode.RANDOM, PlayMode.SINGLE]
  const idx = modes.indexOf(playMode.value)
  playMode.value = modes[(idx + 1) % modes.length]
  // 播放模式变更后重新调度预加载
  invalidatePrefetch()
  scheduleNextPrefetch()
}

export function togglePlayPause() {
  const audio = ControlAudioStore()
  if (audio.Audio.isPlay) {
    invoke('player__pause')
    audio.Audio.isPlay = false
  } else {
    // 没有已加载的音频（重启后），重新播放上一首或列表第一首
    if (!audio.Audio.url) {
      const store = LocalUserDetailStore()
      if (store.list.length > 0) {
        const lastId = store.userInfo.lastPlaySongId
        const idx = lastId ? store.list.findIndex((s: SongList) => s.songmid === lastId) : -1
        const song = idx >= 0 ? store.list[idx] : store.list[0]
        playSong(song)
        return
      }
      return
    }
    invoke('player__resume')
    audio.Audio.isPlay = true
  }
}

export function setVolume(vol: number) {
  const audio = ControlAudioStore()
  audio.setVolume(vol)
  const store = LocalUserDetailStore()
  store.userInfo.volume = vol
}

export function seekTo(time: number) {
  const audio = ControlAudioStore()
  audio.setCurrentTime(time)
  invoke('player__seek', { position: time })
}

// Legacy object export for backward compatibility
export const globalPlaylist = {
  get playIndex() { return _playIndex },
  get playMode() { return playMode.value },
  getCurrentSong,
  playSong,
  playNext,
  playPrev: playPrevious,
  setPlayMode(mode: PlayMode) { playMode.value = mode },
  cyclePlayMode(): PlayMode {
    updatePlayMode()
    return playMode.value
  }
}
