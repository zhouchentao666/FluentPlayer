import { ref, watch, type Ref } from 'vue'
import { SaveConfig, LoadConfig } from '@bridge/app'
import { type Playlist } from '../types'
import type { SortMode, SortOrder } from './usePlaylistView'
import type { LocalSongMetadata } from './useLocalMetadata'

export type WindowEffect = 'none' | 'acrylic' | 'custom-image' | 'song-color'
export type FullScreenBackground = 'static' | 'dynamic'
export type CoverTransition = 'fade' | 'slide-left' | 'slide-both'
export type HotkeyAction = 'togglePlay' | 'prevSong' | 'nextSong' | 'volumeUp' | 'volumeDown' | 'mute' | 'togglePlayerDetail'
export type DesktopLyricPosition = 'left' | 'center' | 'right' | 'both'
/** 在线音乐的四个独立子标签页。 */
export type OnlineTab = 'playlists' | 'albums' | 'charts' | 'search'
/** 在线音乐音质档位（与 @online/lib/quality 的 Quality 保持一致）。 */
export type AudioQuality = '128k' | '320k' | 'flac' | 'flac24bit'

export const AUDIO_QUALITY_OPTIONS: { value: AudioQuality; label: string }[] = [
  { value: '128k', label: '标准 128K' },
  { value: '320k', label: '高品 320K' },
  { value: 'flac', label: '无损 FLAC' },
  { value: 'flac24bit', label: 'Hi-Res 24bit' },
]

function parseQuality(raw: unknown, fallback: AudioQuality): AudioQuality {
  return raw === '128k' || raw === '320k' || raw === 'flac' || raw === 'flac24bit' ? raw : fallback
}

export const HOTKEY_ACTIONS: { value: HotkeyAction; label: string }[] = [
  { value: 'togglePlay', label: '播放/暂停' },
  { value: 'prevSong', label: '上一首' },
  { value: 'nextSong', label: '下一首' },
  { value: 'volumeUp', label: '增大音量' },
  { value: 'volumeDown', label: '减小音量' },
  { value: 'mute', label: '静音' },
  { value: 'togglePlayerDetail', label: '全屏播放器' },
]

export const DEFAULT_HOTKEYS: Record<HotkeyAction, string> = {
  togglePlay: ' ',
  prevSong: 'ArrowLeft',
  nextSong: 'ArrowRight',
  volumeUp: 'ArrowUp',
  volumeDown: 'ArrowDown',
  mute: 'm',
  togglePlayerDetail: 'i',
}

export interface PlaylistSort {
  mode: SortMode
  order: SortOrder
}

export interface DesktopLyricConfig {
  enabled: boolean
  fontSize: number
  mainColor: string
  unplayedColor: string
  shadowColor: string
  fontWeight: number
  position: DesktopLyricPosition
  alwaysShowPlayInfo: boolean
  animation: boolean
  showYrc: boolean
  showTran: boolean
  isDoubleLine: boolean
  textBackgroundMask: boolean
  backgroundMaskColor: string
  fontFamily: string
  x: number
  y: number
  width: number
  height: number
  isLock: boolean
}

export const DEFAULT_DESKTOP_LYRIC: DesktopLyricConfig = {
  enabled: false,
  fontSize: 30,
  mainColor: '#73BCFC',
  unplayedColor: 'rgba(255, 255, 255, 0.5)',
  shadowColor: 'rgba(255, 255, 255, 0.5)',
  fontWeight: 600,
  position: 'center',
  alwaysShowPlayInfo: false,
  animation: true,
  showYrc: true,
  showTran: false,
  isDoubleLine: true,
  textBackgroundMask: false,
  backgroundMaskColor: 'rgba(0,0,0,0.2)',
  fontFamily: 'PingFangSC-Semibold, system-ui, -apple-system, sans-serif',
  x: 0,
  y: 0,
  width: 800,
  height: 180,
  isLock: false,
}

export interface AppSettings {
  theme: 'system' | 'light' | 'dark'
  accentColor: string
  autoplay: boolean
  savePlaylistAndSong: boolean
  /** 在线播放首选音质（获取不到时自动向下降级）。 */
  playQuality: AudioQuality
  /** 在线下载首选音质（获取不到时自动向下降级）。 */
  downloadQuality: AudioQuality
  /** 是否把播放信息同步到系统媒体控制中心（SMTC / MediaSession）。 */
  systemMediaControl: boolean
  windowEffect: WindowEffect
  customImagePath: string
  customImageOpacity: number
  customImageBlur: number
  songColorOpacity: number
  songColorBlur: number
  fullScreenBackground: FullScreenBackground
  coverTransition: CoverTransition
  immersivePlayerBar: boolean
  lyricFontSize: number
  lyricFontFamily: string
  lyricAlignPosition: number
  lyricFontSizeAdaptive: boolean
  lyricBlur: boolean
  lyricSpring: boolean
  lyricFlowSpeed: number
  lyricFps: number
  hotkeys: Partial<Record<HotkeyAction, string>>
  autoStart: boolean
  trayEnabled: boolean
  closeToTray: boolean
  desktopLyric: DesktopLyricConfig
  selectedPlaylistId: string
  playlistSorts: Record<string, PlaylistSort>
  localMetadata: Record<string, LocalSongMetadata>
  pinnedOnlinePlaylists: PinnedOnlineItem[]
  /** 窗口失焦时是否保持材质（模糊）效果。 */
  keepMaterialOnBlur: boolean
  /** 在线下载保存的本地文件夹（空字符串表示使用系统音乐文件夹）。 */
  downloadFolder: string
  /** 下载时默认不弹出“选择保存位置”对话框，直接下载到 downloadFolder。 */
  downloadWithoutDialog: boolean
  /** 在线播放默认音质。 */
  onlineQuality: AudioQuality
}

export interface ConfigPlayback {
  playlistId: string
  songIndex: number
  time: number
}

// 固定到侧栏的在线歌单 / 专辑（打开时实时拉取，而非本地快照）
export interface PinnedOnlineItem {
  source: 'wy' | 'kw' | 'kg' | 'tx' | 'mg'
  id: string
  kind: 'playlist' | 'album'
  name: string
  img: string | null
}

function parseDesktopLyricConfig(raw: unknown): DesktopLyricConfig {
  const cfg = { ...DEFAULT_DESKTOP_LYRIC }
  if (!raw || typeof raw !== 'object') return cfg
  const src = raw as Partial<DesktopLyricConfig>
  if (typeof src.enabled === 'boolean') cfg.enabled = src.enabled
  if (typeof src.fontSize === 'number' && src.fontSize > 0) cfg.fontSize = src.fontSize
  if (typeof src.mainColor === 'string' && src.mainColor) cfg.mainColor = src.mainColor
  if (typeof src.unplayedColor === 'string' && src.unplayedColor) cfg.unplayedColor = src.unplayedColor
  if (typeof src.shadowColor === 'string' && src.shadowColor) cfg.shadowColor = src.shadowColor
  if (typeof src.fontWeight === 'number' && src.fontWeight > 0) cfg.fontWeight = src.fontWeight
  if (src.position === 'left' || src.position === 'center' || src.position === 'right' || src.position === 'both') {
    cfg.position = src.position
  }
  if (typeof src.alwaysShowPlayInfo === 'boolean') cfg.alwaysShowPlayInfo = src.alwaysShowPlayInfo
  if (typeof src.animation === 'boolean') cfg.animation = src.animation
  if (typeof src.showYrc === 'boolean') cfg.showYrc = src.showYrc
  if (typeof src.showTran === 'boolean') cfg.showTran = src.showTran
  if (typeof src.isDoubleLine === 'boolean') cfg.isDoubleLine = src.isDoubleLine
  if (typeof src.textBackgroundMask === 'boolean') cfg.textBackgroundMask = src.textBackgroundMask
  if (typeof src.backgroundMaskColor === 'string' && src.backgroundMaskColor) cfg.backgroundMaskColor = src.backgroundMaskColor
  if (typeof src.fontFamily === 'string' && src.fontFamily) cfg.fontFamily = src.fontFamily
  if (typeof src.x === 'number') cfg.x = src.x
  if (typeof src.y === 'number') cfg.y = src.y
  if (typeof src.width === 'number' && src.width > 0) cfg.width = src.width
  if (typeof src.height === 'number' && src.height > 0) cfg.height = src.height
  if (typeof src.isLock === 'boolean') cfg.isLock = src.isLock
  return cfg
}

export function useConfig(
  playlists: Ref<Playlist[]>,
  settings: Ref<AppSettings>,
  playback: Ref<ConfigPlayback>,
  isLoading: Ref<boolean>
) {
  function buildConfig() {
    return {
      playlists: playlists.value,
      settings: settings.value,
      playback: playback.value,
    }
  }

  async function save() {
    if (isLoading.value) return
    await SaveConfig(buildConfig())
  }

  async function load() {
    try {
      const config = await LoadConfig()
      if (config.playlists && config.playlists.length > 0) {
        playlists.value = config.playlists as Playlist[]
      }
      if (config.settings) {
        const hasEffect = Boolean(config.settings.windowEffect)
        settings.value = {
          theme: (config.settings.theme as AppSettings['theme']) || 'system',
          accentColor: config.settings.accentColor || '#0078d4',
          autoplay: config.settings.autoplay ?? false,
          savePlaylistAndSong: config.settings.savePlaylistAndSong ?? true,
          playQuality: parseQuality((config.settings as unknown as Record<string, unknown>).playQuality, '320k'),
          downloadQuality: parseQuality((config.settings as unknown as Record<string, unknown>).downloadQuality, 'flac'),
          systemMediaControl: ((config.settings as unknown as Record<string, unknown>).systemMediaControl as boolean) ?? true,
          windowEffect: (config.settings.windowEffect as WindowEffect) || 'acrylic',
          customImagePath: config.settings.customImagePath || '',
          customImageOpacity: hasEffect ? (config.settings.customImageOpacity ?? 35) : 35,
          customImageBlur: hasEffect ? (config.settings.customImageBlur ?? 20) : 20,
          songColorOpacity: hasEffect ? (config.settings.songColorOpacity ?? 45) : 45,
          songColorBlur: hasEffect ? (config.settings.songColorBlur ?? 30) : 30,
          fullScreenBackground: (config.settings.fullScreenBackground as FullScreenBackground) || 'static',
          coverTransition: ((config.settings as unknown as Record<string, unknown>).coverTransition as CoverTransition) || 'fade',
          immersivePlayerBar: config.settings.immersivePlayerBar ?? false,
          lyricFontSize: typeof config.settings.lyricFontSize === 'number' && config.settings.lyricFontSize > 0 ? config.settings.lyricFontSize : 36,
          lyricFontFamily: typeof config.settings.lyricFontFamily === 'string' ? config.settings.lyricFontFamily : '',
          lyricAlignPosition: typeof config.settings.lyricAlignPosition === 'number' ? Math.min(1, Math.max(0, config.settings.lyricAlignPosition)) : 0.5,
          lyricFontSizeAdaptive: config.settings.lyricFontSizeAdaptive === true,
          lyricBlur: config.settings.lyricBlur ?? true,
          lyricSpring: config.settings.lyricSpring ?? true,
          lyricFlowSpeed: typeof config.settings.lyricFlowSpeed === 'number' && config.settings.lyricFlowSpeed > 0 ? config.settings.lyricFlowSpeed : 2,
          lyricFps: typeof config.settings.lyricFps === 'number' && config.settings.lyricFps > 0 ? config.settings.lyricFps : 30,
          hotkeys: ((config.settings as unknown as Record<string, unknown>).hotkeys as Record<string, string>) || { ...DEFAULT_HOTKEYS },
          autoStart: ((config.settings as unknown as Record<string, unknown>).autoStart as boolean) ?? false,
          trayEnabled: ((config.settings as unknown as Record<string, unknown>).trayEnabled as boolean) ?? false,
          closeToTray: ((config.settings as unknown as Record<string, unknown>).closeToTray as boolean) ?? false,
          desktopLyric: parseDesktopLyricConfig((config.settings as unknown as Record<string, unknown>).desktopLyric),
          selectedPlaylistId: (config.settings.selectedPlaylistId as string) ?? '',
          playlistSorts: (config.settings.playlistSorts as Record<string, PlaylistSort>) ?? {},
          localMetadata: (config.settings.localMetadata as Record<string, LocalSongMetadata>) ?? {},
          pinnedOnlinePlaylists: (config.settings.pinnedOnlinePlaylists as PinnedOnlineItem[]) ?? [],
          keepMaterialOnBlur: ((config.settings as unknown as Record<string, unknown>).keepMaterialOnBlur as boolean) ?? false,
          downloadFolder: ((config.settings as unknown as Record<string, unknown>).downloadFolder as string) ?? '',
          downloadWithoutDialog: ((config.settings as unknown as Record<string, unknown>).downloadWithoutDialog as boolean) ?? false,
          onlineQuality: parseQuality((config.settings as unknown as Record<string, unknown>).onlineQuality, '320k'),
        }
      }
      if (config.playback) {
        playback.value = {
          playlistId: config.playback.playlistId || '',
          songIndex: config.playback.songIndex ?? -1,
          time: config.playback.time ?? 0,
        }
      }
    } catch {
      // 首次启动没有配置文件
    } finally {
      isLoading.value = false
    }
  }

  watch(playlists, save, { deep: true })
  watch(settings, save, { deep: true })

  return { save, load }
}