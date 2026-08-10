<script lang="ts" setup>
import { computed, ref } from 'vue'
import type { Playlist, Song } from '../types'
import type { SortMode } from '../composables/usePlaylistView'
import { OpenInExplorer, OpenSongEditor } from '@bridge/app'
import { downloadSong } from '@online/lib/download'
import WinUICheckBox from './settings/WinUICheckBox.vue'
import {
  displayTitle,
  displayArtist,
  displayAlbum,
  displayDuration,
} from '../composables/usePlaylistDisplay'

const props = defineProps<{
  songs: Song[]
  playlists: Playlist[]
  currentSong: Song | null
  selectedIds: Set<string>
  batchMode: boolean
  playlistId: string
  sortMode: SortMode
  searchQuery?: string
}>()

const emit = defineEmits<{
  play: [song: Song]
  addToQueue: [song: Song]
  toggle: [id: string]
  remove: [id: string]
  reorder: [songs: Song[]]
  addToPlaylist: [playlistId: string, song: Song]
  comment: [song: Song]
}>()

const dragOverIndex = ref<number | null>(null)
const menuVisible = ref(false)
const menuX = ref(0)
const menuY = ref(0)
const contextSong = ref<Song | null>(null)
const submenuOpen = ref(false)

function isPlaying(song: Song) {
  return props.currentSong?.id === song.id
}

/** 在线歌曲没有本地文件，能做的是「下载」而不是「编辑 / 在资源管理器打开」。 */
function isOnline(song: Song) {
  return !!song.online
}

const downloading = ref<Set<string>>(new Set())

async function download(song: Song) {
  if (!song.online || downloading.value.has(song.id)) return
  downloading.value = new Set(downloading.value).add(song.id)
  try {
    await downloadSong(song.online)
  } finally {
    const next = new Set(downloading.value)
    next.delete(song.id)
    downloading.value = next
  }
}

function isCustomSort() {
  return props.sortMode === 'custom' && !props.searchQuery?.trim()
}

function dragData(song: Song) {
  return JSON.stringify({ songId: song.id, sourcePlaylistId: props.playlistId })
}

function onDragStart(e: DragEvent, song: Song) {
  if (!e.dataTransfer) return
  e.dataTransfer.effectAllowed = 'move'
  // 同时设置 text/plain 兜底：自定义 MIME 在部分浏览器 dragover 阶段不可读
  e.dataTransfer.setData('application/sugarplayer-song', dragData(song))
  e.dataTransfer.setData('text/plain', song.id)
}

function onDragOver(e: DragEvent, index: number) {
  if (!isCustomSort()) return
  // 必须 preventDefault，否则浏览器显示禁止(圆形)光标且不允许 drop
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  dragOverIndex.value = index
}

function onDragLeave() {
  dragOverIndex.value = null
}

/** 列表容器兜底：拖到空白处也允许放置，避免禁止光标。 */
function onListDragOver(e: DragEvent) {
  if (!isCustomSort()) return
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
}

function onListDrop(e: DragEvent) {
  if (!isCustomSort()) return
  // 若已落到具体 song-item / trailing-drop-zone 上，其自身 handler 会处理；
  // 这里仅在落到空白区域时，当作拖到末尾处理。
  if (e.target !== e.currentTarget) return
  e.preventDefault()
  const raw = e.dataTransfer?.getData('application/sugarplayer-song')
  if (!raw) return
  const { songId, sourcePlaylistId } = JSON.parse(raw) as { songId: string; sourcePlaylistId: string }
  if (sourcePlaylistId !== props.playlistId) return
  const next = [...props.songs]
  const fromIndex = next.findIndex(s => s.id === songId)
  if (fromIndex < 0) return
  const [moved] = next.splice(fromIndex, 1)
  next.push(moved)
  emit('reorder', next)
}

function onDrop(e: DragEvent, targetIndex: number) {
  if (!isCustomSort()) return
  e.preventDefault()
  dragOverIndex.value = null

  const raw = e.dataTransfer?.getData('application/sugarplayer-song')
  if (!raw) return
  const { songId, sourcePlaylistId } = JSON.parse(raw) as { songId: string; sourcePlaylistId: string }
  if (sourcePlaylistId !== props.playlistId) return

  const next = [...props.songs]
  const fromIndex = next.findIndex(s => s.id === songId)
  if (fromIndex < 0) return
  if (fromIndex === targetIndex) return
  const [moved] = next.splice(fromIndex, 1)
  next.splice(targetIndex, 0, moved)
  emit('reorder', next)
}

function onTrailingDrop(e: DragEvent) {
  if (!isCustomSort()) return
  e.preventDefault()
  dragOverIndex.value = null

  const raw = e.dataTransfer?.getData('application/sugarplayer-song')
  if (!raw) return
  const { songId, sourcePlaylistId } = JSON.parse(raw) as { songId: string; sourcePlaylistId: string }
  if (sourcePlaylistId !== props.playlistId) return

  const next = [...props.songs]
  const fromIndex = next.findIndex(s => s.id === songId)
  if (fromIndex < 0) return
  const [moved] = next.splice(fromIndex, 1)
  next.push(moved)
  emit('reorder', next)
}

function openContextMenu(e: MouseEvent, song: Song) {
  e.preventDefault()
  contextSong.value = song
  menuVisible.value = true
  submenuOpen.value = false
  menuX.value = e.clientX
  menuY.value = e.clientY
}

function closeMenu() {
  menuVisible.value = false
  submenuOpen.value = false
  contextSong.value = null
}

function openEditor() {
  if (contextSong.value) OpenSongEditor(contextSong.value.path)
  closeMenu()
}

function openExplorer() {
  if (contextSong.value) OpenInExplorer(contextSong.value.path)
  closeMenu()
}

function removeFromPlaylist() {
  if (contextSong.value) emit('remove', contextSong.value.id)
  closeMenu()
}

function downloadFromMenu() {
  const song = contextSong.value
  closeMenu()
  if (song) void download(song)
}

function playFromMenu() {
  const song = contextSong.value
  closeMenu()
  if (song) emit('play', song)
}

function queueFromMenu() {
  const song = contextSong.value
  closeMenu()
  if (song) emit('addToQueue', song)
}

function addToPlaylist(playlistId: string) {
  if (contextSong.value) emit('addToPlaylist', playlistId, contextSong.value)
  closeMenu()
}

function commentFromMenu() {
  const song = contextSong.value
  closeMenu()
  if (song) emit('comment', song)
}

const otherPlaylists = computed(() => props.playlists.filter(p => p.id !== props.playlistId))
</script>

<template>
  <div class="song-list" @click="closeMenu" @dragover="onListDragOver" @drop="onListDrop">
    <div class="list-header">
      <span v-if="props.batchMode" class="col-check"></span>
      <span class="col-index">#</span>
      <span class="col-title">标题</span>
      <span class="col-artist">艺术家</span>
      <span class="col-album">专辑</span>
      <span class="col-duration">时长</span>
      <span class="col-action"></span>
    </div>
    <div
      v-for="(song, index) in props.songs"
      :key="song.id"
      :class="['song-item', { playing: isPlaying(song), selected: props.selectedIds.has(song.id), 'drag-over': dragOverIndex === index }]"
      @click="!props.batchMode && emit('addToQueue', song)"
      @dblclick="emit('play', song)"
      @contextmenu="openContextMenu($event, song)"
      @dragover="onDragOver($event, index)"
      @dragleave="onDragLeave"
      @drop="onDrop($event, index)"
    >
      <WinUICheckBox
        v-if="props.batchMode"
        class="col-check"
        :model-value="props.selectedIds.has(song.id)"
        @click.stop
        @update:model-value="emit('toggle', song.id)"
      />
      <span class="col-index">
        <span class="index-number">{{ index + 1 }}</span>
        <span
          v-if="isCustomSort()"
          class="drag-handle"
          draggable="true"
          @dragstart="onDragStart($event, song)"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="8" y1="6" x2="21" y2="6" />
            <line x1="8" y1="12" x2="21" y2="12" />
            <line x1="8" y1="18" x2="21" y2="18" />
          </svg>
        </span>
      </span>
      <div class="col-title">
        <div class="primary-text">{{ displayTitle(song) }}</div>
        <div class="secondary-text">{{ displayArtist(song) }}</div>
      </div>
      <span class="col-artist secondary-text">{{ displayArtist(song) }}</span>
      <span class="col-album secondary-text">{{ displayAlbum(song) }}</span>
      <span class="col-duration secondary-text">{{ displayDuration(song) }}</span>
      <div class="col-action actions">
        <button
          v-if="isOnline(song)"
          class="action-icon"
          :class="{ busy: downloading.has(song.id) }"
          :title="downloading.has(song.id) ? '下载中…' : '下载'"
          :disabled="downloading.has(song.id)"
          @click.stop="download(song)"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 3v12" />
            <path d="M7 10l5 5 5-5" />
            <path d="M4 20h16" />
          </svg>
        </button>
        <button
          v-if="isOnline(song)"
          class="action-icon"
          title="评论"
          @click.stop="emit('comment', song)"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" />
          </svg>
        </button>
        <button
          v-else
          class="action-icon"
          title="编辑"
          @click.stop="OpenSongEditor(song.path)"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
          </svg>
        </button>
        <button
          class="action-icon remove"
          title="移除"
          @click.stop="emit('remove', song.id)"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 7h16" />
            <path d="M10 11v6M14 11v6" />
            <path d="M6 7l1 13h10l1-13" />
            <path d="M9 7V4h6v3" />
          </svg>
        </button>
      </div>
    </div>
    <div
      v-if="isCustomSort()"
      class="trailing-drop-zone"
      :class="{ 'drag-over': dragOverIndex === props.songs.length }"
      @dragover="onDragOver($event, props.songs.length)"
      @dragleave="onDragLeave"
      @drop="onTrailingDrop"
    ></div>
    <div v-if="props.songs.length === 0" class="empty-state">
      暂无歌曲
    </div>

    <div
      v-if="menuVisible"
      class="context-menu"
      :style="{ left: menuX + 'px', top: menuY + 'px' }"
      @click.stop
    >
      <div class="menu-item" @click="playFromMenu">播放</div>
      <div class="menu-item" @click="queueFromMenu">添加到播放队列</div>
      <div class="menu-divider"></div>
      <template v-if="contextSong && isOnline(contextSong)">
        <div
          class="menu-item"
          :class="{ disabled: downloading.has(contextSong.id) }"
          @click="downloadFromMenu"
        >
          {{ downloading.has(contextSong.id) ? '下载中…' : '下载' }}
        </div>
        <div class="menu-item" @click="commentFromMenu">评论</div>
      </template>
      <template v-else>
        <div class="menu-item" @click="openEditor">编辑</div>
        <div class="menu-item" @click="openExplorer">在文件资源管理器打开</div>
      </template>
      <div class="menu-item" @click="removeFromPlaylist">从歌单移除</div>
      <div
        class="menu-item has-submenu"
        @mouseenter="submenuOpen = true"
        @mouseleave="submenuOpen = false"
      >
        <span>添加到</span>
        <span class="arrow">›</span>
        <div v-if="submenuOpen" class="submenu">
          <template v-if="otherPlaylists.length > 0">
            <div
              v-for="playlist in otherPlaylists"
              :key="playlist.id"
              class="menu-item"
              @click="addToPlaylist(playlist.id)"
            >
              {{ playlist.name }}
            </div>
          </template>
          <div v-else class="menu-item disabled">暂无别的歌单</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.song-list {
  position: relative;
  flex: 1;
  overflow-y: auto;
  padding: 0 16px 8px;
}

.list-header,
.song-item {
  display: grid;
  grid-template-columns: 36px 2fr 1.2fr 1.2fr 64px 48px;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
}

.list-header:has(.col-check),
.song-item:has(.col-check) {
  grid-template-columns: 32px 36px 2fr 1.2fr 1.2fr 64px 48px;
}

.list-header {
  position: sticky;
  top: 0;
  font-size: 11px;
  font-weight: 600;
  color: var(--fluent-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(10px);
  z-index: 1;
}

.song-item {
  border-radius: 8px;
  cursor: default;
  transition: background 0.18s ease;
}

.song-item:hover {
  background: var(--fluent-bg-hover);
}

.song-item.selected {
  background: var(--fluent-bg-active);
}

.song-item.drag-over {
  background: var(--fluent-bg-active);
  outline: 1px dashed var(--fluent-accent);
  outline-offset: -2px;
}

.song-item.playing {
  background: var(--fluent-accent);
  color: #fff;
}

.song-item.playing .secondary-text {
  color: rgba(255, 255, 255, 0.75);
}

.song-item.playing .col-index {
  color: #fff;
}

.col-check {
  display: flex;
  align-items: center;
  justify-content: center;
}

.col-index {
  position: relative;
  text-align: center;
  font-size: 12px;
  color: var(--fluent-text-secondary);
}

.index-number {
  display: inline;
}

.drag-handle {
  position: absolute;
  inset: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: grab;
  color: var(--fluent-text-secondary);
  opacity: 0;
  /* 关键：拖拽时鼠标可能离开歌单项 hover，手柄保持可见，避免拖拽中断显示禁止光标 */
  user-select: none;
  -webkit-user-drag: element;
}
.drag-handle svg {
  pointer-events: none;
}

.song-item:hover .drag-handle {
  opacity: 1;
}

/* 拖拽过程中保持手柄可见（:active 在 dragstart 后仍生效） */
.drag-handle:active {
  cursor: grabbing;
  opacity: 1;
}

.col-title {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.primary-text {
  font-size: 13px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.secondary-text {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.col-artist,
.col-album,
.col-duration {
  min-width: 0;
}

.actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
}

.action-icon {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: inherit;
  font-size: 11px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.18s ease, background 0.18s ease;
}

.song-item:hover .action-icon {
  opacity: 1;
}

.action-icon:hover {
  background: var(--fluent-bg-active);
}

.action-icon.remove:hover {
  background: var(--fluent-close-hover);
  color: white;
}

.action-icon.busy {
  opacity: 0.55;
  cursor: default;
}

.menu-divider {
  height: 1px;
  margin: 4px 6px;
  background: var(--fluent-border);
}

.trailing-drop-zone {
  height: 12px;
  margin: 2px 12px;
  border-radius: 4px;
  transition: background 0.15s ease;
}

.trailing-drop-zone.drag-over {
  background: var(--fluent-bg-active);
  outline: 1px dashed var(--fluent-accent);
  outline-offset: -1px;
}

.empty-state {
  padding: 40px 20px;
  text-align: center;
  color: var(--fluent-text-secondary);
  font-size: 13px;
}

.context-menu {
  position: fixed;
  min-width: 160px;
  padding: 6px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: var(--fluent-bg-card);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  z-index: 100;
}

.menu-item {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 10px;
  border-radius: 6px;
  font-size: 13px;
  color: var(--fluent-text);
  cursor: pointer;
  transition: background 0.15s ease;
}

.menu-item:hover {
  background: var(--fluent-bg-hover);
}

.menu-item.disabled {
  color: var(--fluent-text-secondary);
  cursor: default;
}

.menu-item.disabled:hover {
  background: transparent;
}

.arrow {
  margin-left: 8px;
  font-size: 14px;
}

.submenu {
  position: absolute;
  left: calc(100% + 4px);
  top: 0;
  min-width: 140px;
  padding: 6px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: var(--fluent-bg-card);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
}
</style>
