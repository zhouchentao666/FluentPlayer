<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { Events } from '@bridge/runtime'
import { GetDesktopLyricConfig } from '@bridge/app'
import DesktopLyricControls from './components/desktopLyric/DesktopLyricControls.vue'
import DesktopLyricLine from './components/desktopLyric/DesktopLyricLine.vue'
import { useDesktopLyricRenderer } from './composables/useDesktopLyricRenderer'
import { useDesktopLyricWindowControl } from './composables/useDesktopLyricWindowControl'
import { DEFAULT_DESKTOP_LYRIC, type DesktopLyricConfig } from './composables/useConfig'
import type { LyricLine } from '@applemusic-like-lyrics/core'

type LyricData = {
  playName: string
  artistName: string
  playStatus: boolean
  lrcData: LyricLine[]
  yrcData: LyricLine[]
}

const config = ref<DesktopLyricConfig>({ ...DEFAULT_DESKTOP_LYRIC })
const lyricData = reactive<LyricData>({
  playName: '',
  artistName: '',
  playStatus: false,
  lrcData: [],
  yrcData: [],
})

const playSeekMs = ref(0)
let baseMs = 0
let anchorTick = 0
let rafId = 0

function startLoop() {
  if (rafId) return
  anchorTick = performance.now()
  baseMs = playSeekMs.value
  const loop = () => {
    if (!lyricData.playStatus) {
      rafId = 0
      return
    }
    playSeekMs.value = baseMs + (performance.now() - anchorTick)
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

const locked = computed(() => config.value.isLock)
const { cursorStyle, isHovered, setLocked } = useDesktopLyricWindowControl(config, locked)

const { renderLyricLines, hasYrc, getPlainText, getYrcStyle, getScrollStyle, setLineRef, setContentRef } =
  useDesktopLyricRenderer(
    computed(() => lyricData),
    config,
    playSeekMs,
  )

const offSong = Events.On('desktop-lyric:song', (event: any) => {
  const data = event?.data || {}
  lyricData.playName = data.name || ''
  lyricData.artistName = data.artist || ''
})
const offLyrics = Events.On('desktop-lyric:lyrics', (event: any) => {
  const data = event?.data || {}
  lyricData.lrcData = Array.isArray(data.lrcData) ? data.lrcData : []
  lyricData.yrcData = Array.isArray(data.yrcData) ? data.yrcData : []
})
const offPlay = Events.On('desktop-lyric:play', (event: any) => {
  lyricData.playStatus = !!event?.data
  if (lyricData.playStatus) startLoop()
  else stopLoop()
})
const offTime = Events.On('desktop-lyric:time', (event: any) => {
  const ms = event?.data?.currentMs
  if (typeof ms !== 'number') return
  playSeekMs.value = ms
  baseMs = ms
  anchorTick = performance.now()
})
const offConfig = Events.On('desktop-lyric:config', (event: any) => {
  const data = event?.data
  if (!data) return
  config.value = { ...config.value, ...data }
})

async function loadConfig() {
  try {
    const parsed = await GetDesktopLyricConfig()
    config.value = { ...DEFAULT_DESKTOP_LYRIC, ...(parsed as Partial<DesktopLyricConfig>) }
  } catch {}
}

onMounted(() => {
  loadConfig()
  Events.Emit('desktop-lyric:ready').catch(() => {})
})

onBeforeUnmount(() => {
  stopLoop()
  offSong?.()
  offLyrics?.()
  offPlay?.()
  offTime?.()
  offConfig?.()
  Events.Emit('desktop-lyric:closed').catch(() => {})
})
</script>

<template>
  <div
    class="desktop-lyric"
    :class="{
      locked: config.isLock,
      hovered: isHovered,
      'no-animation': !config.animation,
    }"
    :style="{ cursor: cursorStyle, '--mask-bg': config.backgroundMaskColor }"
  >
    <DesktopLyricControls
      :song-name="lyricData.playName"
      :artist-name="lyricData.artistName"
      :is-playing="lyricData.playStatus"
      :is-lock="config.isLock"
      :always-show-info="config.alwaysShowPlayInfo"
      :position="config.position"
      @update:lock="setLocked"
    />
    <div
      class="lyric-container"
      :class="[config.position]"
      :style="{
        fontSize: config.fontSize + 'px',
        fontFamily: config.fontFamily,
        fontWeight: config.fontWeight,
        textShadow: `0 0 4px ${config.shadowColor}`,
      }"
    >
      <DesktopLyricLine
        v-for="(line, index) in renderLyricLines"
        :key="line.key"
        :line="line"
        :config="config"
        :index="index"
        :show-yrc="config.showYrc && hasYrc"
        :yrc-data="lyricData.yrcData"
        :get-yrc-style="getYrcStyle"
        :get-scroll-style="getScrollStyle"
        :get-plain-text="getPlainText"
        :set-line-ref="setLineRef"
        :set-content-ref="setContentRef"
      />
    </div>
  </div>
</template>

<style scoped>
.desktop-lyric {
  display: flex;
  flex-direction: column;
  height: 100vh;
  color: #fff;
  /* 极浅透明底色，阻断系统点击穿透，肉眼几乎看不见 */
  background: rgba(0, 0, 0, 0.005);
  padding: 12px;
  border-radius: 12px;
  overflow: hidden;
  transition: background-color 0.3s;
  cursor: default;
  user-select: none;
}
/* 鼠标悬浮时显示半透明背景 */
.desktop-lyric.hovered:not(.locked) {
  background-color: rgba(0, 0, 0, 0.6);
}
.desktop-lyric.hovered:not(.locked) :deep(.dl-btn),
.desktop-lyric.hovered:not(.locked) :deep(.dl-song-name) {
  opacity: 1;
}
.desktop-lyric.locked {
  cursor: default;
}
.lyric-container {
  flex: 1;
  position: relative;
  padding: 0 8px;
  overflow: hidden;
}
.lyric-container.center :deep(.dl-line) {
  text-align: center;
}
.lyric-container.right :deep(.dl-line) {
  text-align: right;
}
.no-animation :deep(.dl-line) {
  transition: none !important;
}
</style>

<style>
body {
  background-color: transparent !important;
  margin: 0;
}
</style>
