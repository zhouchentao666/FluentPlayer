<script lang="ts" setup>
import { toRaw, computed, ref, watch } from 'vue'
import type { Playlist, Song } from '../types'
import { usePlaylistView, type SortMode, type SortOrder } from '../composables/usePlaylistView'
import PlaylistViewToolbar from './PlaylistViewToolbar.vue'
import PlaylistViewList from './PlaylistViewList.vue'
import PlaylistBatchBar from './PlaylistBatchBar.vue'

const props = defineProps<{
  playlist: Playlist
  playlists: Playlist[]
  currentSong: Song | null
  initialSort?: { mode: SortMode; order: SortOrder }
}>()

const emit = defineEmits<{
  (e: 'update:playlist', playlist: Playlist): void
  (e: 'add-music-files'): void
  (e: 'add-music-folder'): void
  (e: 'play-song', index: number): void
  (e: 'play-all'): void
  (e: 'add-to-queue', song: Song): void
  (e: 'add-to-playlist', playlistId: string, songs: Song[]): void
  (e: 'replace-to-playlist', playlistId: string, songs: Song[]): void
  (e: 'update-sort', payload: { playlistId: string; mode: SortMode; order: SortOrder }): void
}>()

const {
  searchQuery,
  sortMode,
  sortOrder,
  batchMode,
  selectedIds,
  displaySongs,
  selectedSongs,
  allSelected,
  sortLabels,
  toggleSelection,
  selectAll,
  clearSelection,
  exitBatchMode,
} = usePlaylistView(computed(() => props.playlist))

const skipSortEmit = ref(false)

watch(() => props.initialSort, (saved) => {
  if (saved && (saved.mode !== sortMode.value || saved.order !== sortOrder.value)) {
    skipSortEmit.value = true
    sortMode.value = saved.mode
    sortOrder.value = saved.order
  }
}, { immediate: true })

watch([sortMode, sortOrder], () => {
  if (skipSortEmit.value) {
    skipSortEmit.value = false
    return
  }
  emit('update-sort', { playlistId: props.playlist.id, mode: sortMode.value, order: sortOrder.value })
})

function updateSongs(songs: Song[]) {
  emit('update:playlist', { ...props.playlist, songs })
}

function removeSong(id: string) {
  const songs = props.playlist.songs.filter(song => song.id !== id)
  updateSongs(songs)
}

function removeSelected() {
  const ids = new Set(selectedIds.value)
  const songs = props.playlist.songs.filter(song => !ids.has(song.id))
  updateSongs(songs)
  clearSelection()
}

function playSong(song: Song) {
  const index = props.playlist.songs.findIndex(s => s.id === song.id)
  if (index >= 0) emit('play-song', index)
}

function handleSelectAll() {
  if (allSelected.value) clearSelection()
  else selectAll()
}

function handleAddToPlaylist(playlistId: string) {
  emit('add-to-playlist', playlistId, toRaw(selectedSongs.value))
  exitBatchMode()
}

function handleReplaceToPlaylist(playlistId: string) {
  emit('replace-to-playlist', playlistId, toRaw(selectedSongs.value))
  exitBatchMode()
}

function handleAddSingleToPlaylist(playlistId: string, song: Song) {
  emit('add-to-playlist', playlistId, [song])
}

function handleReorder(songs: Song[]) {
  updateSongs(songs)
}

function handleAddToQueue(song: Song) {
  emit('add-to-queue', song)
}
</script>

<template>
  <div class="playlist-view">
    <header class="playlist-header">
      <div class="playlist-title">
        <h1>{{ props.playlist.name }}</h1>
        <span class="song-count">{{ props.playlist.songs.length }} 首歌曲</span>
      </div>
      <div class="playlist-actions">
        <button class="action-button play-all-btn" :disabled="props.playlist.songs.length === 0" @click="emit('play-all')">
          <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
          <span>播放全部</span>
        </button>
        <button class="action-button" @click="emit('add-music-files')">
          <span class="icon">+</span>
          <span>添加音乐</span>
        </button>
        <button class="action-button" @click="emit('add-music-folder')">
          <span class="icon">📁</span>
          <span>文件夹</span>
        </button>
        <PlaylistViewToolbar
          v-model:search-query="searchQuery"
          v-model:sort-mode="sortMode"
          v-model:sort-order="sortOrder"
          :batch-mode="batchMode"
          :sort-labels="sortLabels"
          @toggle-batch="batchMode = !batchMode"
        />
      </div>
    </header>

    <PlaylistViewList
      :songs="displaySongs"
      :playlists="props.playlists"
      :current-song="props.currentSong"
      :selected-ids="selectedIds"
      :batch-mode="batchMode"
      :playlist-id="props.playlist.id"
      :sort-mode="sortMode"
      :search-query="searchQuery"
      @play="playSong"
      @add-to-queue="handleAddToQueue"
      @toggle="toggleSelection"
      @remove="removeSong"
      @reorder="handleReorder"
      @add-to-playlist="handleAddSingleToPlaylist"
    />

    <PlaylistBatchBar
      v-if="batchMode"
      :selected-songs="selectedSongs"
      :playlists="props.playlists"
      :current-playlist-id="props.playlist.id"
      @remove="removeSelected"
      @add-to-playlist="handleAddToPlaylist"
      @replace-to-playlist="handleReplaceToPlaylist"
      @select-all="handleSelectAll"
      @clear-selection="clearSelection"
      @close="exitBatchMode"
    />
  </div>
</template>

<style scoped>
.playlist-view {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;
  color: var(--fluent-text);
  overflow: hidden;
}

.playlist-header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 20px 28px;
  border-bottom: 1px solid var(--fluent-border);
}

.playlist-title {
  min-width: 0;
}

.playlist-title h1 {
  margin: 0 0 4px;
  font-size: 22px;
  font-weight: 600;
  word-break: break-word;
}

.song-count {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}

.playlist-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
}

.action-button {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border: none;
  border-radius: 8px;
  background: var(--fluent-bg-hover);
  color: inherit;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.18s ease;
}

.action-button:hover {
  background: var(--fluent-bg-active);
}

.play-all-btn {
  background: var(--fluent-accent);
  color: #fff;
}

.play-all-btn:hover {
  filter: brightness(1.08);
  background: var(--fluent-accent);
}

.play-all-btn:disabled {
  opacity: 0.5;
  cursor: default;
  filter: none;
}

.play-all-btn svg {
  width: 14px;
  height: 14px;
}

.action-button .icon {
  font-size: 14px;
}
</style>
