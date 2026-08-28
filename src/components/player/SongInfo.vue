<script lang="ts" setup>
import { type Song } from '../../types'
import { displayTitle, displayArtist } from '../../composables/usePlaylistDisplay'

const props = defineProps<{
  song: Song | null
  coverUrl: string | null
  showDetail?: boolean
}>()

const emit = defineEmits<{
  click: []
}>()

function songTitle(song: Song | null): string {
  return song ? displayTitle(song) : '未播放'
}

function songArtist(song: Song | null): string {
  return song ? displayArtist(song) : '未知艺术家'
}

function handleClick() {
  if (props.showDetail) {
    emit('click')
  }
}
</script>

<template>
  <div class="song-info-wrap">
    <!-- 空锚点：封面图由 PlayerDetailLeft 折叠态渲染并定位到此区域
         纯 CSS 平移动画无需 first rect，此处仅保留点击触发与占位 -->
    <div
      class="cover-placeholder"
      data-footer-cover
      @click="emit('click')"
    ></div>
    <div
      class="song-info"
      :class="{ 'detail-mode': showDetail }"
      @click="handleClick"
    >
      <div class="song-title" :class="{ 'detail-mode': showDetail }">{{ songTitle(song) }}</div>
      <div class="song-artist" :class="{ 'detail-mode': showDetail }">{{ songArtist(song) }}</div>
    </div>
  </div>
</template>

<style scoped>
.song-info-wrap {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

/* 占位锚点：与封面折叠态尺寸一致（44x44），透明，仅接收点击 */
.cover-placeholder {
  width: 44px;
  height: 44px;
  border-radius: 8px;
  flex-shrink: 0;
  cursor: pointer;
  background: transparent;
}

.song-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  cursor: default;
  transition: transform 500ms cubic-bezier(0.22, 1, 0.36, 1);
}

.song-info.detail-mode {
  transform: translateX(-60px);
  cursor: pointer;
}

.song-title {
  font-size: 13px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.song-artist {
  font-size: 11px;
  color: var(--fluent-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: color 500ms ease;
}

.song-artist.detail-mode {
  color: rgba(255, 255, 255, 0.7);
}
</style>
