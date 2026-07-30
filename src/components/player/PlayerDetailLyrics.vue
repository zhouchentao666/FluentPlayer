<script lang="ts" setup>
import '@applemusic-like-lyrics/core/style.css'
import { computed, inject, toRaw, type Ref } from 'vue'
import { LyricPlayer } from '@applemusic-like-lyrics/vue'
import type { LyricLine, LyricLineMouseEvent } from '@applemusic-like-lyrics/core'
import type { AppSettings } from '../../composables/useConfig'

const props = defineProps<{
  lyrics: LyricLine[]
  currentTime: number
  show: boolean
  isPlaying: boolean
}>()

const emit = defineEmits<{
  seek: [time: number]
}>()

// 全屏歌词相关设置由 App.vue 通过 provide('settings') 下发（提供的是 ref）
const settings = inject<Ref<AppSettings>>('settings')

const lyricStyle = computed(() => ({
  '--amll-lp-font-size': `${settings?.value.lyricFontSize ?? 36}px`,
  fontFamily: settings?.value.lyricFontFamily ? settings.value.lyricFontFamily : 'inherit',
}))

function onLineClick(e: LyricLineMouseEvent) {
  emit('seek', e.line.getLine().startTime / 1000)
}
</script>

<template>
  <div
    class="lyrics-panel"
    :class="{ visible: show }"
  >
    <LyricPlayer
      v-if="lyrics.length > 0"
      class="lyric-player"
      :lyric-lines="toRaw(lyrics)"
      :current-time="currentTime"
      :playing="isPlaying"
      :word-fade-width="0.5"
      :align-anchor="settings?.lyricAlignAnchor ?? 'center'"
      :align-position="settings?.lyricAlignPosition ?? 0.5"
      :enable-blur="settings?.lyricBlur ?? true"
      :enable-spring="settings?.lyricSpring ?? true"
      :style="lyricStyle"
      @line-click="onLineClick"
    />
    <div
      v-else
      class="lyrics-placeholder"
    >
      暂无歌词
    </div>
  </div>
</template>

<style scoped>
.lyrics-panel {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 45%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 120px 64px 140px 32px;
  opacity: 0;
  transform: translateX(30px);
  transition:
    opacity 500ms cubic-bezier(0.22, 1, 0.36, 1),
    transform 500ms cubic-bezier(0.22, 1, 0.36, 1);
  pointer-events: none;
}

.lyrics-panel.visible {
  opacity: 1;
  transform: translateX(0);
  pointer-events: auto;
}

.lyric-player {
  width: 100%;
  max-width: clamp(480px, 42vw, 860px);
  height: 100%;
  --amll-lp-font-size: clamp(18px, 2.6vw, 42px);
}

.lyrics-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  width: 100%;
  max-width: clamp(480px, 42vw, 860px);
  height: 100%;
  font-size: 16px;
  color: rgba(255, 255, 255, 0.45);
}
</style>
