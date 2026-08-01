<script lang="ts" setup>
import { ref, watch } from 'vue'
import { ReadCoverArt } from '@bridge/app'
import WinUICheckBox from './settings/WinUICheckBox.vue'
import type { Song } from '../types'
import {
  displayTitle,
  displayArtist,
  displayDuration,
} from '../composables/usePlaylistDisplay'

const props = defineProps<{
  songs: Song[]
  currentSong: Song | null
  selectedIds: Set<string>
  batchMode: boolean
}>()

const emit = defineEmits<{
  play: [song: Song]
  addToQueue: [song: Song]
  toggle: [id: string]
  remove: [id: string]
}>()

const coverUrls = ref<Map<string, string | null>>(new Map())

async function loadCovers(songs: Song[]) {
  const urls = new Map(coverUrls.value)
  for (const song of songs) {
    if (urls.has(song.id)) continue
    try {
      urls.set(song.id, await ReadCoverArt(song.path))
    } catch {
      urls.set(song.id, null)
    }
  }
  coverUrls.value = urls
}

watch(() => props.songs, loadCovers, { immediate: true, deep: true })

function isPlaying(song: Song) {
  return props.currentSong?.id === song.id
}

function coverStyle(song: Song) {
  const url = coverUrls.value.get(song.id)
  return url ? { backgroundImage: `url(${url})` } : {}
}
</script>

<template>
  <div class="song-grid">
    <div
      v-for="song in props.songs"
      :key="song.id"
      :class="['grid-card', { playing: isPlaying(song), selected: props.selectedIds.has(song.id) }]"
      @click="!props.batchMode && emit('addToQueue', song)"
      @dblclick="emit('play', song)"
    >
      <WinUICheckBox
        v-if="props.batchMode"
        class="card-check"
        :model-value="props.selectedIds.has(song.id)"
        @click.stop
        @update:model-value="emit('toggle', song.id)"
      />
      <button
        class="card-remove"
        title="移除"
        @click.stop="emit('remove', song.id)"
      >
        ✕
      </button>
      <div class="card-cover" :style="coverStyle(song)"></div>
      <div class="card-info">
        <div class="card-title primary-text">{{ displayTitle(song) }}</div>
        <div class="card-meta secondary-text">{{ displayArtist(song) }}</div>
        <div class="card-meta secondary-text">{{ displayDuration(song) }}</div>
      </div>
    </div>
    <div v-if="props.songs.length === 0" class="empty-state">
      暂无歌曲
    </div>
  </div>
</template>

<style scoped>
.song-grid {
  flex: 1;
  overflow-y: auto;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 16px;
  padding: 0 16px 8px;
}

.grid-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
  border-radius: 10px;
  background: var(--fluent-bg-card);
  cursor: default;
  transition: background 0.18s ease, transform 0.18s ease;
}

.grid-card:hover {
  background: var(--fluent-bg-hover);
}

.grid-card.selected {
  background: var(--fluent-bg-active);
}

.grid-card.playing {
  background: var(--fluent-accent);
  color: #fff;
}

.grid-card.playing .secondary-text {
  color: rgba(255, 255, 255, 0.75);
}

.card-cover {
  aspect-ratio: 1;
  border-radius: 8px;
  background: var(--fluent-bg-active) center/cover no-repeat;
}

.card-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.card-title {
  font-size: 13px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-meta {
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-check {
  position: absolute;
  top: 8px;
  left: 8px;
  z-index: 1;
}


.card-remove {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: var(--fluent-bg-glass);
  color: inherit;
  font-size: 10px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.18s ease, background 0.18s ease;
}

.grid-card:hover .card-remove {
  opacity: 1;
}

.card-remove:hover {
  background: var(--fluent-bg-active);
}

.empty-state {
  grid-column: 1 / -1;
  padding: 40px 20px;
  text-align: center;
  color: var(--fluent-text-secondary);
  font-size: 13px;
}
</style>
