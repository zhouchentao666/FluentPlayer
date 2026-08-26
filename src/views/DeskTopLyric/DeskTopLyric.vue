<script setup lang="ts">
import '@applemusic-like-lyrics/core/style.css'
import { ref, shallowRef, computed, onMounted, onBeforeUnmount, markRaw, type Component } from 'vue'
import { listen, emit, type UnlistenFn } from '@tauri-apps/api/event'
import type { LyricLine } from '@/types/lyric'

const { t } = useI18n()

const LyricPlayerComp = shallowRef<Component | null>(null)
const lyricPlayerError = ref(false)

async function loadLyricPlayer() {
  try {
    const mod = await import('@applemusic-like-lyrics/vue')
    LyricPlayerComp.value = mod.LyricPlayer
  } catch (e) {
    console.error('[DeskTopLyric] Failed to load LyricPlayer:', e)
    lyricPlayerError.value = true
  }
}

const lyricLines = shallowRef<LyricLine[]>([])
const songInfo = ref({ name: '', singer: '' })
const isPlaying = ref(false)
const isLocked = ref(false)
const isHovering = ref(false)

let baseMs = 0
let anchorTick = 0
const playSeekMs = ref(0)
let rafId: number | null = null

const unlisteners: UnlistenFn[] = []

interface StyleOption {
  fontSize?: number
  mainColor?: string
  fontWeight?: number
  fontFamily?: string
  position?: 'left' | 'center' | 'right' | 'both'
  showYrc?: boolean
}

const styleOptions = ref<StyleOption>({})
const fontSize = computed(() => styleOptions.value.fontSize || 30)
const fontWeight = computed(() => styleOptions.value.fontWeight || 800)
const fontFamily = computed(() => styleOptions.value.fontFamily || '')
const resolvedFontFamily = computed(() => fontFamily.value || '-apple-system, BlinkMacSystemFont, sans-serif')
const lyricViewColor = computed(() => styleOptions.value.mainColor || 'rgba(255, 255, 255, 1)')

const lyricPosition = computed(() => styleOptions.value.position || 'center')
const alignAnchor = computed(() => {
  if (lyricPosition.value === 'left') return 'left'
  if (lyricPosition.value === 'right') return 'right'
  return 'center'
})
const alignPosition = computed(() => {
  if (lyricPosition.value === 'left') return 0.2
  if (lyricPosition.value === 'right') return 0.8
  return 0.5
})
const lyricTextAlign = computed(() => {
  if (lyricPosition.value === 'left') return 'left'
  if (lyricPosition.value === 'right') return 'right'
  return 'center'
})

const showYrc = computed(() => styleOptions.value.showYrc !== false)

const safeLyricLines = computed(() => {
  if (showYrc.value) return lyricLines.value

  return lyricLines.value.map((line: any) => {
    const words = Array.isArray(line?.words) ? line.words : []
    const text = words.map((w: any) => w?.word ?? '').join('')
    const startTime = Number(line?.startTime ?? words[0]?.startTime ?? 0)
    const endTime = Number(line?.endTime ?? words[words.length - 1]?.endTime ?? startTime + 1)
    return {
      ...line,
      words: [
        {
          word: text,
          startTime,
          endTime,
          obscene: false,
          romanWord: '',
        },
      ],
      startTime,
      endTime,
    }
  })
})
const currentTimeMs = computed(() => Math.floor(playSeekMs.value))

const placeholderText = computed(() => {
  if (lyricLines.value.length > 0) return ''
  if (songInfo.value.name) return `${songInfo.value.name} - ${songInfo.value.singer}`
  return 'Mio Desktop Lyric'
})

// RAF Clock
function startRafLoop() {
  if (rafId !== null) return
  const tick = () => {
    if (isPlaying.value) {
      playSeekMs.value = baseMs + (performance.now() - anchorTick)
    } else {
      playSeekMs.value = baseMs
    }
    rafId = requestAnimationFrame(tick)
  }
  rafId = requestAnimationFrame(tick)
}

function stopRafLoop() {
  if (rafId !== null) {
    cancelAnimationFrame(rafId)
    rafId = null
  }
}

// IPC handlers
function handleLyricChange(lines: LyricLine[]) {
  const normalized = (lines || []).map((l: any) => ({
    ...l,
    translatedLyric: l.translatedLyric ?? '',
    romanLyric: l.romanLyric ?? '',
    isBG: l.isBG ?? false,
    isDuet: l.isDuet ?? false,
    words: (l.words || []).map((w: any) => ({
      ...w,
      word: w?.word ?? '',
      startTime: Number(w?.startTime ?? 0),
      endTime: Number(w?.endTime ?? 0),
      obscene: w?.obscene ?? false,
      romanWord: w?.romanWord ?? '',
    })),
    startTime: Number(l?.startTime ?? 0),
    endTime: Number(l?.endTime ?? 0),
  }))
  lyricLines.value = markRaw(normalized)
}

function handleSongChange(data: { name: string; singer: string }) {
  songInfo.value = {
    name: data?.name || '',
    singer: data?.singer || ''
  }
}

function handleProgress(data: { currentMs: number; timestamp: number }) {
  if (typeof data?.currentMs === 'number') {
    const newBase = Math.floor(data.currentMs)
    const drift = Math.abs(newBase - playSeekMs.value)
    if (drift > 300) {
      baseMs = newBase
      anchorTick = performance.now()
    }
  }
}

function handlePlayState(playing: boolean) {
  const wasPlaying = isPlaying.value
  isPlaying.value = !!playing
  if (isPlaying.value && !wasPlaying) {
    baseMs = playSeekMs.value
    anchorTick = performance.now()
  } else if (!isPlaying.value && wasPlaying) {
    baseMs = playSeekMs.value
    anchorTick = performance.now()
  }
}

// Controls
function emitControl(name: string, value?: boolean) {
  emit('desktop-lyric-control', { name, value }).catch(() => {})
}

function onPrev() { emitControl('playPrev') }
function onToggle() { emitControl('toggle') }
function onNext() { emitControl('playNext') }

function onToggleLock() {
  const next = !isLocked.value
  isLocked.value = next
  emitControl('lock', next)
}

function onClose() { emitControl('close') }

// Lifecycle
onMounted(async () => {
  loadLyricPlayer()

  try {
    const saved = await (window as any).electron?.ipcRenderer?.invoke('get-desktop-lyric-option')
    if (saved) styleOptions.value = saved
  } catch {}

  unlisteners.push(
    await listen<StyleOption>('desktop-lyric-style-change', (event) => {
      styleOptions.value = event.payload
    })
  )

  unlisteners.push(
    await listen<LyricLine[]>('desktop-lyric-change', (event) => {
      handleLyricChange(event.payload)
    })
  )

  unlisteners.push(
    await listen<{ name: string; singer: string }>('desktop-song-change', (event) => {
      handleSongChange(event.payload)
    })
  )

  unlisteners.push(
    await listen<{ currentMs: number; timestamp: number }>('desktop-lyric-progress', (event) => {
      handleProgress(event.payload)
    })
  )

  unlisteners.push(
    await listen<boolean>('desktop-lyric-play-state', (event) => {
      handlePlayState(event.payload)
    })
  )

  // Listen for force-unlock from system tray
  unlisteners.push(
    await listen<boolean>('desktop-lyric-force-unlock', () => {
      isLocked.value = false
    })
  )

  startRafLoop()
  emit('desktop-lyric-ready', {}).catch(() => {})
})

onBeforeUnmount(() => {
  stopRafLoop()
  unlisteners.forEach((fn) => {
    try { fn() } catch {}
  })
  unlisteners.length = 0
})
</script>

<template>
  <div
    :class="['desktop-lyric', { locked: isLocked, hovered: isHovering }]"
    data-tauri-drag-region
    @mouseenter="isHovering = true"
    @mouseleave="isHovering = false"
  >
    <!-- AMLL LyricPlayer -->
    <component
      :is="LyricPlayerComp"
      v-if="LyricPlayerComp && lyricLines.length > 0"
      :lyric-lines="safeLyricLines"
      :current-time="currentTimeMs"
      :playing="isPlaying"
      :enable-blur="true"
      :enable-spring="false"
      :enable-scale="false"
      :align-anchor="alignAnchor"
      :align-position="alignPosition"
      :style="{ textAlign: lyricTextAlign }"
      class="desktop-lyric-player"
    />

    <!-- Error state -->
    <div v-else-if="lyricPlayerError" class="lyric-placeholder">
      <span>{{ t('play.desktopLyric.loadFailed') }}</span>
    </div>

    <!-- Placeholder when no lyrics -->
    <div v-else class="lyric-placeholder">
      <span>{{ placeholderText }}</span>
    </div>

    <!-- Control bar (outside drag region, shown on hover when not locked) -->
    <Transition name="controls">
      <div v-if="isHovering && !isLocked" class="controls-bar" @pointerdown.stop>
        <div class="song-info" v-if="songInfo.name">
          {{ songInfo.name }} - {{ songInfo.singer }}
        </div>
        <div class="control-buttons">
          <button class="ctrl-btn" :title="t('play.desktopLyric.prev')" @click="onPrev">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/>
            </svg>
          </button>
          <button class="ctrl-btn" :title="isPlaying ? t('play.desktopLyric.pause') : t('play.desktopLyric.play')" @click="onToggle">
            <svg v-if="isPlaying" width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 4h4v16H6zM14 4h4v16h-4z"/>
            </svg>
            <svg v-else width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
              <path d="M8 5v14l11-7z"/>
            </svg>
          </button>
          <button class="ctrl-btn" :title="t('play.desktopLyric.next')" @click="onNext">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/>
            </svg>
          </button>
          <button class="ctrl-btn" :title="isLocked ? t('play.desktopLyric.unlock') : t('play.desktopLyric.lock')" @click="onToggleLock">
            <svg v-if="isLocked" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
              <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
            </svg>
            <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
              <path d="M7 11V7a5 5 0 0 1 9.9-1"/>
            </svg>
          </button>
          <button class="ctrl-btn" :title="t('play.desktopLyric.close')" @click="onClose">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped lang="scss">
.desktop-lyric {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: transparent;
  user-select: none;
  position: relative;
  color: #fff;
  font-family: v-bind(resolvedFontFamily);
  font-weight: v-bind(fontWeight);
  &.locked {
    pointer-events: none;
  }
}

.desktop-lyric-player {
  width: 100%;
  height: 100%;
  pointer-events: none;
  --amll-lyric-view-color: v-bind(lyricViewColor);
  --amll-lp-color: v-bind(lyricViewColor);
  --amll-lyric-player-font-size: v-bind(fontSize + 'px');
  --amll-lp-font-size: v-bind(fontSize + 'px');
  --amll-lyric-player-font-weight: v-bind(fontWeight);
  font-family: v-bind(resolvedFontFamily);

  // Active lyric line only: larger + tracking
  :deep(.FmKaba_active .FmKaba_lyricMainLine),
  :deep([class*='_active'] [class*='lyricMainLine']) {
    font-size: calc(var(--amll-lp-font-size) * 1.2) !important;
    line-height: 1.3 !important;
    letter-spacing: 0.08em !important;
    font-weight: v-bind(fontWeight) !important;
  }

  // Non-active lines: smaller + dimmed
  :deep(.FmKaba_lyricLine:not(.FmKaba_active) .FmKaba_lyricMainLine),
  :deep([class*='lyricLine']:not([class*='_active']) [class*='lyricMainLine']) {
    font-size: calc(var(--amll-lp-font-size) * 0.85) !important;
    opacity: 0.5 !important;
  }

  // Sub lyric line (translation)
  :deep(.FmKaba_lyricSubLine),
  :deep([class*='lyricSubLine']) {
    font-weight: v-bind(fontWeight) !important;
    pointer-events: none;
  }

  // Roman word styling
  :deep([class*='romanWord']) {
    font-size: calc(var(--amll-lp-font-size) * 0.5) !important;
    font-family: v-bind(resolvedFontFamily) !important;
    opacity: 0.8;
  }
}

.lyric-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  font-size: v-bind(fontSize + 'px');
  font-weight: v-bind(fontWeight);
  color: v-bind(lyricViewColor);
  letter-spacing: 0.08em;
  font-family: v-bind(resolvedFontFamily);
  pointer-events: none;
}

/* Controls bar */
.controls-bar {
  position: absolute;
  bottom: 10px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 12px;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(10px);
  padding: 8px 16px;
  border-radius: 20px;
  z-index: 10;
}

.song-info {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.8);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 300px;
}

.control-buttons {
  display: flex;
  gap: 8px;
  align-items: center;
}

.ctrl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  padding: 4px;
  border-radius: 50%;
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;

  &:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.2);
  }

  &:active {
    transform: scale(0.95);
  }
}

/* Controls transition */
.controls-enter-active,
.controls-leave-active {
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
}

.controls-enter-from,
.controls-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(10px);
}
</style>

<style>
html,
body,
#app {
  background-color: transparent !important;
  background: transparent !important;
}

#app {
  border-radius: 0 !important;
  overflow: visible !important;
}
</style>
