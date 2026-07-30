<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, watch, provide } from 'vue'
import { Events } from '@bridge/runtime'
import {
  LoadConfig,
  ApplyAutoStart,
  EnableTray,
  SetTraySongInfo,
  SetCloseToTray,
  ShowMainWindow,
  CloseDesktopLyric,
  SetDesktopLyricIgnoreMouseEvents,
} from '@bridge/app'
import TitleBar from './components/TitleBar.vue'
import Sidebar from './components/Sidebar.vue'
import Settings from './components/Settings.vue'
import PlaylistView from './components/PlaylistView.vue'
import PlayerFooter from './components/PlayerFooter.vue'
import PlayerDetail from './components/player/PlayerDetail.vue'
import PlayQueue from './components/player/PlayQueue.vue'
import { useAudioPlayer } from './composables/useAudioPlayer'
import { usePlaylists } from './composables/usePlaylists'
import { useConfig, type AppSettings, type ConfigPlayback, DEFAULT_HOTKEYS, DEFAULT_DESKTOP_LYRIC } from './composables/useConfig'
import { useLyrics } from './composables/useLyrics'
import { useWindowEffect } from './composables/useWindowEffect'
import { useSession } from './composables/useSession'
import { useDesktopLyricBridge } from './composables/useDesktopLyricBridge'
import { useDesktopLyric } from './composables/useDesktopLyric'
import type { PlayMode } from './components/player/PlayerControls.vue'
import type { Song } from './types'
import type { SortMode, SortOrder } from './composables/usePlaylistView'
import { localMetadata, type LocalSongMetadata } from './composables/useLocalMetadata'

const view = ref<'main' | 'settings'>('main')
const isLoading = ref(true)
const audioRef = ref<HTMLAudioElement | null>(null)
const showPlayerDetail = ref(false)
const showQueue = ref(false)
const playMode = ref<PlayMode>('sequential')
const settings = ref<AppSettings>({
  theme: 'system',
  accentColor: '#0078d4',
  autoplay: false,
  savePlaylistAndSong: true,
  windowEffect: 'acrylic',
  customImagePath: '',
  customImageOpacity: 35,
  customImageBlur: 20,
  songColorOpacity: 45,
  songColorBlur: 30,
  fullScreenBackground: 'dynamic',
  coverTransition: 'fade',
  immersivePlayerBar: false,
  lyricFontSize: 36,
  lyricFontFamily: '',
  lyricAlignPosition: 0.5,
  lyricFontSizeAdaptive: false,
  lyricBlur: true,
  lyricSpring: true,
  lyricFlowSpeed: 2,
  lyricFps: 30,
  hotkeys: { ...DEFAULT_HOTKEYS },
  autoStart: false,
  trayEnabled: false,
  closeToTray: false,
  desktopLyric: { ...DEFAULT_DESKTOP_LYRIC },
  selectedPlaylistId: '',
  playlistSorts: {},
  localMetadata: {},
})

const playbackState = ref<ConfigPlayback>({
  playlistId: '',
  songIndex: -1,
  time: 0,
})

const { playlists, selectedId, updatePlaylists, updatePlaylist, selectPlaylist, addMusicFiles, addMusicFolder, refreshFolder, rewatchFolders, addSongs, replaceSongs } = usePlaylists()
const currentPlaylist = computed(() => playlists.value.find(p => p.id === selectedId.value))
const currentPlaylistSort = computed(() => currentPlaylist.value ? settings.value.playlistSorts[currentPlaylist.value.id] : undefined)

const { save, load } = useConfig(playlists, settings, playbackState, isLoading)

// 提供 settings 给子组件使用（使用 computed 保持响应性）
provide('settings', settings)

const audio = useAudioPlayer({
  audioRef,
  onEnded: playNext,
})

const lyrics = useLyrics(audio.currentSong)
const { appStyle, layerStyle } = useWindowEffect(settings, audio.coverUrl)

const { dispose: disposeBridge } = useDesktopLyricBridge({
  currentSong: audio.currentSong,
  lyrics: lyrics.lyrics,
  isPlaying: audio.isPlaying,
  currentTime: audio.currentTime,
  handlers: {
    onPrev: playPrev,
    onNext: playNext,
    onToggle: handleTogglePlay,
    onShowMain: () => ShowMainWindow().catch(() => {}),
    onClose: () => {
      CloseDesktopLyric().catch(() => {})
      settings.value.desktopLyric.enabled = false
    },
    onLockChange: (locked: boolean) => {
      settings.value.desktopLyric.isLock = locked
      SetDesktopLyricIgnoreMouseEvents(locked).catch(() => {})
    },
  },
})

const { toggle: toggleDesktopLyric, openIfEnabled, dispose: disposeLyric } = useDesktopLyric({ settings })

const lyricTime = computed(() => Math.floor(audio.currentTime.value * 1000))

function pickRandomIndex(current: number, count: number): number {
  if (count <= 1) return 0
  let nextIndex = current
  do { nextIndex = Math.floor(Math.random() * count) } while (nextIndex === current)
  return nextIndex
}

function playNext() {
  const count = audio.queue.value.length
  if (count === 0) return
  if (audio.index.value < 0) {
    audio.playQueueAt(0)
    return
  }
  const current = audio.index.value
  if (playMode.value === 'stop') return

  let nextIndex = current
  if (playMode.value === 'shuffle') {
    nextIndex = pickRandomIndex(current, count)
  } else if (playMode.value === 'single') {
    nextIndex = current
  } else if (playMode.value === 'reverse') {
    nextIndex = current - 1
    if (nextIndex < 0) nextIndex = count - 1
  } else {
    nextIndex = current + 1
    if (nextIndex >= count) nextIndex = 0
  }
  audio.playQueueAt(nextIndex)
}

function playPrev() {
  const count = audio.queue.value.length
  if (count === 0) return
  if (audio.index.value < 0) {
    audio.playQueueAt(0)
    return
  }
  const current = audio.index.value
  if (playMode.value === 'stop') return

  let prevIndex = current
  if (playMode.value === 'shuffle') {
    prevIndex = pickRandomIndex(current, count)
  } else if (playMode.value === 'single') {
    prevIndex = current
  } else if (playMode.value === 'reverse') {
    prevIndex = current + 1
    if (prevIndex >= count) prevIndex = 0
  } else {
    prevIndex = current - 1
    if (prevIndex < 0) prevIndex = count - 1
  }
  audio.playQueueAt(prevIndex)
}

function playSong(playlistId: string, index: number, autoPlay = true) {
  const playlist = playlists.value.find(p => p.id === playlistId)
  if (!playlist || index < 0 || index >= playlist.songs.length) return
  audio.playSongs(playlist.songs, index, playlistId, autoPlay)
}

function playCurrentSong(index: number) {
  if (!currentPlaylist.value) return
  playSong(currentPlaylist.value.id, index)
}

// 播放全部：用整个歌单替换播放列表并从第一首开始
function playAllCurrent() {
  if (!currentPlaylist.value) return
  audio.playSongs(currentPlaylist.value.songs, 0, currentPlaylist.value.id)
}

function handleTogglePlay() {
  if (!audio.currentSong.value && currentPlaylist.value?.songs.length) {
    playSong(currentPlaylist.value.id, 0)
    return
  }
  audio.togglePlay()
}

function updateSettings(newSettings: AppSettings) {
  settings.value = { ...newSettings }
}

const { handleClose, restoreSession } = useSession(settings, playbackState, save, playlists, audio, selectPlaylist)

function handleTrayExit() {
  handleClose(true)
}

function buildTraySongLabel(song: Song | null): string {
  if (!song) return '未在播放'
  const artist = song.metadata?.artist || '未知艺术家'
  return `${song.title} - ${artist}`
}

function syncTraySongInfo() {
  if (!settings.value.trayEnabled) return
  SetTraySongInfo(buildTraySongLabel(audio.currentSong.value)).catch(() => {})
}

function onSelectPlaylist(id: string) {
  selectPlaylist(id)
  settings.value.selectedPlaylistId = id
  view.value = 'main'
}

watch(selectedId, (id) => {
  if (id) settings.value.selectedPlaylistId = id
})

function handleDropSongs(payload: { targetPlaylistId: string; sourcePlaylistId: string; songIds: string[] }) {
  const source = playlists.value.find(p => p.id === payload.sourcePlaylistId)
  if (!source) return
  const songs = payload.songIds
    .map(id => source.songs.find(s => s.id === id))
    .filter((s): s is Song => Boolean(s))
  if (songs.length) addSongs(payload.targetPlaylistId, songs)
}

function handleUpdateSort(payload: { playlistId: string; mode: SortMode; order: SortOrder }) {
  const next = { ...settings.value.playlistSorts }
  next[payload.playlistId] = { mode: payload.mode, order: payload.order }
  settings.value.playlistSorts = next
}

function togglePlayerDetail() {
  showPlayerDetail.value = !showPlayerDetail.value
}

function cyclePlayMode() {
  const modes: PlayMode[] = ['sequential', 'single', 'reverse', 'stop', 'shuffle']
  playMode.value = modes[(modes.indexOf(playMode.value) + 1) % modes.length]
}

function toggleQueue() {
  showQueue.value = !showQueue.value
}

function handleHotkey(e: KeyboardEvent) {
  if (e.repeat) return
  const target = e.target as HTMLElement | null
  if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
    return
  }

  const key = e.key
  const hotkeys = settings.value.hotkeys
  const action = (Object.keys(hotkeys) as Array<keyof typeof hotkeys>).find(a => hotkeys[a] === key)
  if (!action) return

  e.preventDefault()
  switch (action) {
    case 'togglePlay':
      handleTogglePlay()
      break
    case 'prevSong':
      playPrev()
      break
    case 'nextSong':
      playNext()
      break
    case 'volumeUp':
      audio.setVolume(Math.min(100, audio.volume.value + 5))
      break
    case 'volumeDown':
      audio.setVolume(Math.max(0, audio.volume.value - 5))
      break
    case 'mute':
      audio.setVolume(audio.volume.value === 0 ? 100 : 0)
      break
    case 'togglePlayerDetail':
      togglePlayerDetail()
      break
  }
}

let offFolderChanged: (() => void) | null = null
let offMetadataChanged: (() => void) | null = null
let offTrayPrev: (() => void) | null = null
let offTrayNext: (() => void) | null = null
let offTrayExit: (() => void) | null = null
let traySyncId = 0
let traySyncQueue = Promise.resolve()

function syncTraySettings() {
  if (isLoading.value) return

  const syncId = ++traySyncId
  const trayEnabled = settings.value.trayEnabled
  const closeToTray = settings.value.closeToTray

  traySyncQueue = traySyncQueue
    .catch(() => {})
    .then(async () => {
      if (syncId !== traySyncId) return
      await EnableTray(trayEnabled)
      await SetCloseToTray(trayEnabled && closeToTray)
      if (trayEnabled) {
        await SetTraySongInfo(buildTraySongLabel(audio.currentSong.value))
      }
    })
    .catch(() => {})
}

watch(() => settings.value.autoStart, (enabled) => {
  ApplyAutoStart(enabled).catch(() => {})
})

watch(() => settings.value.trayEnabled, syncTraySettings)

watch(() => settings.value.closeToTray, syncTraySettings)

watch(audio.currentSong, () => {
  syncTraySongInfo()
})

onMounted(async () => {
  await load()
  localMetadata.value = settings.value.localMetadata
  if (settings.value.selectedPlaylistId && playlists.value.some(p => p.id === settings.value.selectedPlaylistId)) {
    selectPlaylist(settings.value.selectedPlaylistId)
  }
  await rewatchFolders()
  await restoreSession()

  ApplyAutoStart(settings.value.autoStart).catch(() => {})
  syncTraySettings()
  await openIfEnabled()

  window.addEventListener('keydown', handleHotkey)
  offFolderChanged = Events.On('folder:changed', (event: any) => {
    refreshFolder(event.data)
  })
  offMetadataChanged = Events.On('localmetadata:changed', async () => {
    try {
      const config = await LoadConfig()
      if (config.settings?.localMetadata) {
        const loaded = config.settings.localMetadata as Record<string, LocalSongMetadata>
        settings.value = { ...settings.value, localMetadata: { ...loaded } }
        localMetadata.value = { ...loaded }
      }
    } catch {
      // ignore
    }
  })
  offTrayPrev = Events.On('tray:prev', playPrev)
  offTrayNext = Events.On('tray:next', playNext)
  offTrayExit = Events.On('tray:exit', handleTrayExit)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleHotkey)
  offFolderChanged?.()
  offMetadataChanged?.()
  offTrayPrev?.()
  offTrayNext?.()
  offTrayExit?.()
  disposeBridge()
  disposeLyric()
})
</script>

<template>
  <div
    class="glass"
    :data-theme="settings.theme"
    :style="appStyle"
  >
    <div
      v-if="layerStyle"
      class="window-bg-layer"
      :style="layerStyle"
    ></div>
    <TitleBar @close="handleClose" />
    <div class="content">
      <Sidebar
        :playlists="playlists"
        :selected-id="selectedId"
        :active-view="view"
        @update:playlists="updatePlaylists"
        @update:selected-id="selectedId = $event"
        @open-settings="view = 'settings'"
        @select="onSelectPlaylist"
        @drop-songs="handleDropSongs"
      />
      <main class="main">
        <Transition name="view-flip">
          <PlaylistView
            v-if="view === 'main' && currentPlaylist"
            :key="'playlist-' + currentPlaylist.id"
            :playlist="currentPlaylist"
            :playlists="playlists"
            :current-song="audio.currentSong.value"
            :initial-sort="currentPlaylistSort"
            @update:playlist="updatePlaylist"
            @add-music-files="currentPlaylist && addMusicFiles(currentPlaylist.id)"
            @add-music-folder="currentPlaylist && addMusicFolder(currentPlaylist.id)"
            @play-song="playCurrentSong"
            @play-all="playAllCurrent"
            @add-to-queue="audio.addToQueue"
            @add-to-playlist="addSongs"
            @replace-to-playlist="replaceSongs"
            @update-sort="handleUpdateSort"
          />
          <Settings
            v-else-if="view === 'settings'"
            :key="'settings'"
            :settings="settings"
            @update:settings="updateSettings"
            @close="view = 'main'"
          />
        </Transition>
      </main>
    </div>
    <PlayerFooter
      :current-song="audio.currentSong.value"
      :cover-url="audio.coverUrl.value"
      :is-playing="audio.isPlaying.value"
      :current-time="audio.currentTime.value"
      :duration="audio.duration.value"
      :volume="audio.volume.value"
      :playback-rate="audio.playbackRate.value"
      :show-detail="showPlayerDetail"
      :play-mode="playMode"
      :immersive="settings.immersivePlayerBar"
      :desktop-lyric-enabled="settings.desktopLyric.enabled"
      @toggle-play="handleTogglePlay"
      @prev="playPrev"
      @next="playNext"
      @seek="audio.seek"
      @set-volume="audio.setVolume"
      @set-playback-rate="audio.setPlaybackRate"
      @open-detail="togglePlayerDetail"
      @cycle-mode="cyclePlayMode"
      @toggle-queue="toggleQueue"
      @toggle-desktop-lyric="toggleDesktopLyric"
    />

    <PlayerDetail
      :show="showPlayerDetail"
      :current-song="audio.currentSong.value"
      :cover-url="audio.coverUrl.value"
      :is-playing="audio.isPlaying.value"
      :lyrics="lyrics.lyrics.value"
      :has-lyrics="lyrics.hasLyrics.value"
      :current-time="lyricTime"
      :background-mode="settings.fullScreenBackground"
      :immersive-player-bar="settings.immersivePlayerBar"
      :cover-transition="settings.coverTransition"
      @close="togglePlayerDetail"
      @seek="audio.seek"
    />
    <PlayQueue
      :show="showQueue"
      :songs="audio.queue.value"
      :current-index="audio.index.value"
      :current-song="audio.currentSong.value"
      @close="showQueue = false"
      @play="audio.playQueueAt"
      @remove="audio.removeFromQueue"
      @clear="audio.clearQueue"
    />
    <audio ref="audioRef" style="display: none;"></audio>
  </div>
</template>

<style scoped>
.glass {
  position: relative;
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  color: var(--fluent-text);
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(40px) saturate(125%);
  -webkit-backdrop-filter: blur(40px) saturate(125%);
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.1);
}

.window-bg-layer {
  position: absolute;
  inset: 0;
  pointer-events: none;
  z-index: -1;
}

.content {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.main {
  flex: 1;
  position: relative;
  overflow: hidden;
}

/* FluentUI 风格界面切换：原界面原地淡化，新界面从偏下方弹出（不淡化） */
.view-flip-enter-active {
  position: absolute;
  inset: 0;
  transition: transform 320ms cubic-bezier(0.22, 1, 0.36, 1);
}

.view-flip-leave-active {
  position: absolute;
  inset: 0;
  transition: opacity 320ms cubic-bezier(0.22, 1, 0.36, 1);
}

.view-flip-enter-from {
  transform: translateY(56px);
}

.view-flip-leave-to {
  opacity: 0;
}
</style>
