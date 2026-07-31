<script setup lang="ts">
import type { MusicInfo } from '@online/types/music'

defineProps<{
  music: MusicInfo
  index: number
  current?: boolean
  playing?: boolean
  showAlbum?: boolean
}>()

const emit = defineEmits<{
  (e: 'play', m: MusicInfo): void
  (e: 'add-queue', m: MusicInfo): void
  (e: 'add-playlist', m: MusicInfo): void
  (e: 'download', m: MusicInfo): void
}>()
</script>

<template>
  <div class="song-row" :class="{ current }" @dblclick="emit('play', music)">
    <div class="idx">
      <span v-if="!current" class="num">{{ index + 1 }}</span>
      <svg v-else-if="playing" class="eq" viewBox="0 0 16 16" width="14" height="14">
        <rect x="1" y="6" width="3" height="8" rx="1" class="b"><animate attributeName="height" values="4;12;4" dur="0.9s" repeatCount="indefinite"/><animate attributeName="y" values="10;4;10" dur="0.9s" repeatCount="indefinite"/></rect>
        <rect x="6" y="3" width="3" height="11" rx="1" class="b"><animate attributeName="height" values="11;4;11" dur="0.9s" repeatCount="indefinite"/><animate attributeName="y" values="3;10;3" dur="0.9s" repeatCount="indefinite"/></rect>
        <rect x="11" y="6" width="3" height="8" rx="1" class="b"><animate attributeName="height" values="6;13;6" dur="0.9s" repeatCount="indefinite"/><animate attributeName="y" values="8;1;8" dur="0.9s" repeatCount="indefinite"/></rect>
      </svg>
      <svg v-else viewBox="0 0 16 16" width="13" height="13" class="play-glyph"><path d="M4 2.5v11l9-5.5z" fill="currentColor"/></svg>
    </div>

    <div class="title-cell">
      <span class="title" :title="music.name">{{ music.name }}</span>
      <span class="artist" :title="music.singer">{{ music.singer }}</span>
    </div>

    <div v-if="showAlbum" class="album" :title="music.albumName">{{ music.albumName }}</div>

    <div class="time">{{ music.interval }}</div>

    <div class="actions">
      <button class="row-btn" title="播放" @click="emit('play', music)">
        <svg viewBox="0 0 16 16" width="14" height="14"><path d="M4 2.5v11l9-5.5z" fill="currentColor"/></svg>
      </button>
      <button class="row-btn" title="加入播放队列" @click="emit('add-queue', music)">
        <svg viewBox="0 0 16 16" width="14" height="14"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
      </button>
      <button class="row-btn" title="收藏到歌单" @click="emit('add-playlist', music)">
        <svg viewBox="0 0 16 16" width="14" height="14"><path d="M8 13.5S2.5 9.8 2.5 6.2A2.7 2.7 0 018 4.6a2.7 2.7 0 015.5 1.6C13.5 9.8 8 13.5 8 13.5z" fill="none" stroke="currentColor" stroke-width="1.4"/></svg>
      </button>
      <button class="row-btn" title="下载" @click="emit('download', music)">
        <svg viewBox="0 0 16 16" width="14" height="14"><path d="M8 2v7m0 0L5.5 6.5M8 9l2.5-2.5M3 12.5h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" fill="none"/></svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.song-row {
  display: grid;
  grid-template-columns: 36px 1fr auto auto;
  align-items: center;
  gap: 12px;
  padding: 0 12px;
  height: 46px;
  border-radius: 8px;
  cursor: default;
}
.song-row:hover {
  background: var(--fluent-bg-hover);
}
.song-row.current {
  background: var(--fluent-bg-active);
}
.idx {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--fluent-text-secondary);
  font-size: 13px;
}
.idx .eq .b {
  fill: var(--fluent-accent);
}
.play-glyph {
  color: var(--fluent-text);
}
.title-cell {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.title {
  font-size: 14px;
  color: var(--fluent-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.current .title {
  color: var(--fluent-accent);
}
.artist {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.album {
  max-width: 200px;
  font-size: 12px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.time {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  font-variant-numeric: tabular-nums;
}
.actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.12s ease;
}
.song-row:hover .actions {
  opacity: 1;
}
.row-btn {
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  border-radius: 6px;
  cursor: pointer;
}
.row-btn:hover {
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
}
</style>
