<script setup lang="ts">
import { Events } from '@bridge/runtime'

const props = defineProps<{
  songName: string
  artistName: string
  isPlaying: boolean
  isLock: boolean
  alwaysShowInfo: boolean
  position: 'left' | 'center' | 'right' | 'both'
}>()

function showMainWindow() {
  Events.Emit('desktop-lyric:control', { action: 'show-main' }).catch(() => {})
}

function control(action: 'prev' | 'next' | 'toggle') {
  Events.Emit('desktop-lyric:control', { action }).catch(() => {})
}

function close() {
  Events.Emit('desktop-lyric:close').catch(() => {})
}

const emit = defineEmits<{
  (e: 'update:lock', value: boolean): void
}>()

function toggleLock() {
  emit('update:lock', !props.isLock)
}
</script>

<template>
  <div class="dl-header">
    <div class="dl-header-left">
      <button class="dl-btn" @pointerdown.stop @click="showMainWindow">
        <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
          <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/>
        </svg>
      </button>
      <span class="dl-song-name">{{ songName }} - {{ artistName }}</span>
    </div>
    <div class="dl-header-center">
      <button class="dl-btn" @pointerdown.stop @click="control('prev')">
        <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
          <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/>
        </svg>
      </button>
      <button class="dl-btn" @pointerdown.stop @click="control('toggle')">
        <svg v-if="isPlaying" viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
          <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>
        </svg>
        <svg v-else viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
          <path d="M8 5v14l11-7z"/>
        </svg>
      </button>
      <button class="dl-btn" @pointerdown.stop @click="control('next')">
        <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
          <path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/>
        </svg>
      </button>
    </div>
    <div class="dl-header-right">
      <button class="dl-btn" @pointerdown.stop :title="isLock ? '解锁' : '锁定'" @click="toggleLock">
        <svg v-if="isLock" viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
          <path d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z"/>
        </svg>
        <svg v-else viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
          <path d="M12 17c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zm6-9h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6h1.9c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm0 12H6V10h12v10z"/>
        </svg>
      </button>
      <button class="dl-btn" @pointerdown.stop title="关闭" @click="close">
        <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
          <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
        </svg>
      </button>
    </div>
    <div
      v-if="alwaysShowInfo"
      :class="['dl-play-title', position]"
    >
      <span class="name">{{ songName }}</span>
      <span class="artist">{{ artistName }}</span>
    </div>
  </div>
</template>

<style scoped>
.dl-header {
  position: relative;
  margin-bottom: 12px;
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 12px;
  cursor: default;
}
.dl-header-left,
.dl-header-center,
.dl-header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: center;
}
.dl-header-left {
  justify-content: flex-start;
  min-width: 0;
}
.dl-header-right {
  justify-content: flex-end;
}
.dl-song-name {
  font-size: 1em;
  text-align: left;
  flex: 1 1 auto;
  line-height: 36px;
  padding: 0 8px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: opacity 0.3s;
}
.dl-btn {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: #fff;
  cursor: pointer;
  transition: background-color 0.2s, transform 0.15s;
  /* Wails v3: 按钮区域禁止拖拽 */
  
  pointer-events: auto;
}
.dl-btn:hover {
  background-color: rgba(255, 255, 255, 0.3);
}
.dl-btn:active {
  transform: scale(0.96);
}
.dl-song-name,
.dl-btn {
  opacity: 0;
}
.dl-play-title {
  position: absolute;
  padding: 0 12px;
  width: 100%;
  text-align: left;
  transition: opacity 0.3s;
  pointer-events: none;
  z-index: 0;
  top: 0;
  left: 0;
}
.dl-play-title span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.8);
  padding: 0 4px;
}
.dl-play-title .artist {
  font-size: 12px;
  opacity: 0.6;
}
.dl-play-title.center,
.dl-play-title.both {
  text-align: center;
}
.dl-play-title.right {
  text-align: right;
}
</style>
