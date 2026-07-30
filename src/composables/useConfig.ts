import { ref, watch, type Ref } from 'vue'
import { SaveConfig, LoadConfig } from '@bridge/app'
import { type Playlist } from '../types'
import type { SortMode, SortOrder } from './usePlaylistView'
import type { LocalSongMetadata } from './useLocalMetadata'

export type WindowEffect = 'none' | 'acrylic' | 'custom-image' | 'song-color'
export type FullScreenBackground = 'static' | 'dynamic'
export type CoverTransition = 'fade' | 'slide-left' | 'slide-both'
export type LyricAlignAnchor = 'top' | 'bottom' | 'center'
export type HotkeyAction = 'togglePlay' | 'prevSong' | 'nextSong' | 'volumeUp' | 'volumeDown' | 'mute' | 'togglePlayerDetail'
export type DesktopLyricPosition = 'left' | 'center' | 'right' | 'both'

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
  saveWindowPosition: boolean
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
  lyricAlignAnchor: LyricAlignAnchor
  lyricAlignPosition: number
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
}

export interface ConfigPlayback {
  playlistId: string
  songIndex: number
  time: number
}

export interface ConfigWindow {
  x: number
  y: number
  width: number
  height: number
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
  windowState: Ref<ConfigWindow>,
  isLoading: Ref<boolean>
) {
  function buildConfig() {
    return {
      playlists: playlists.value,
      settings: settings.value,
      playback: playback.value,
      window: windowState.value,
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
          saveWindowPosition: config.settings.saveWindowPosition ?? true,
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
          lyricAlignAnchor: (config.settings.lyricAlignAnchor as LyricAlignAnchor) || 'center',
          lyricAlignPosition: typeof config.settings.lyricAlignPosition === 'number' ? Math.min(1, Math.max(0, config.settings.lyricAlignPosition)) : 0.5,
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
        }
      }
      if (config.playback) {
        playback.value = {
          playlistId: config.playback.playlistId || '',
          songIndex: config.playback.songIndex ?? -1,
          time: config.playback.time ?? 0,
        }
      }
      if (config.window) {
        windowState.value = {
          x: config.window.x ?? 0,
          y: config.window.y ?? 0,
          width: config.window.width ?? 800,
          height: config.window.height ?? 600,
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