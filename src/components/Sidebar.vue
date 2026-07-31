<script lang="ts" setup>
import { ref } from 'vue'
import { type Playlist } from '../types'
import type { PinnedOnlineItem } from '../composables/useConfig'
import PlaylistItem from './sidebar/PlaylistItem.vue'
import PlaylistCreateInput from './sidebar/PlaylistCreateInput.vue'

const props = defineProps<{
  playlists: Playlist[]
  selectedId: string
  activeView?: 'main' | 'settings' | 'online'
  pinnedOnline?: PinnedOnlineItem[]
}>()

const emit = defineEmits<{
  (e: 'update:playlists', playlists: Playlist[]): void
  (e: 'update:selectedId', id: string): void
  (e: 'open-settings'): void
  (e: 'open-online'): void
  (e: 'select', id: string): void
  (e: 'open-online-item', item: PinnedOnlineItem): void
  (e: 'unpin-online', id: string): void
  (e: 'drop-songs', payload: { targetPlaylistId: string; sourcePlaylistId: string; songIds: string[] }): void
}>()

const sourceLabels: Record<string, string> = { wy: '网易云', kw: '酷我', kg: '酷狗', tx: 'QQ', mg: '咪咕' }
function sourceLabel(s: string) {
  return sourceLabels[s] ?? s
}

const isCreating = ref(false)

function updatePlaylists(updated: Playlist[]) {
  emit('update:playlists', updated)
}

function updatePlaylist(updated: Playlist) {
  updatePlaylists(props.playlists.map(p => (p.id === updated.id ? updated : p)))
}

function onSelect(id: string) {
  emit('update:selectedId', id)
  emit('select', id)
}

function onRename(id: string, name: string) {
  const playlist = props.playlists.find(p => p.id === id)
  if (playlist) updatePlaylist({ ...playlist, name })
}

function onDelete(id: string) {
  if (id === 'favorites') return
  const filtered = props.playlists.filter(p => p.id !== id)
  updatePlaylists(filtered)
  if (props.selectedId === id) {
    const nextId = filtered[0]?.id || ''
    emit('update:selectedId', nextId)
    emit('select', nextId)
  }
}

function startCreate() {
  isCreating.value = true
}

function confirmCreate(name: string) {
  isCreating.value = false
  if (!name) return
  const playlist: Playlist = {
    id: Date.now().toString(),
    name,
    songs: [],
    folders: [],
  }
  updatePlaylists([...props.playlists, playlist])
  onSelect(playlist.id)
}

function cancelCreate() {
  isCreating.value = false
}

function onDropSongs(playlistId: string, payload: { sourcePlaylistId: string; songIds: string[] }) {
  emit('drop-songs', { targetPlaylistId: playlistId, ...payload })
}
</script>

<template>
  <aside class="sidebar">
    <div class="section">
      <button
        :class="['nav-btn', { active: activeView === 'online' }]"
        @click="emit('open-online')"
      >
        <span class="icon">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6">
            <circle cx="12" cy="12" r="9" />
            <path d="M3 12h18M12 3c2.6 2.6 2.6 15.4 0 18M12 3c-2.6 2.6-2.6 15.4 0 18" />
          </svg>
        </span>
        <span>在线音乐</span>
      </button>
      <div class="section-title">歌单</div>
      <ul class="playlist-list">
        <PlaylistItem
          v-for="playlist in playlists"
          :key="playlist.id"
          :playlist="playlist"
          :selected="selectedId === playlist.id"
          @select="onSelect"
          @rename="onRename"
          @delete="onDelete"
          @drop-songs="payload => onDropSongs(playlist.id, payload)"
        />
      </ul>
      <PlaylistCreateInput
        v-if="isCreating"
        @confirm="confirmCreate"
        @cancel="cancelCreate"
      />
      <button v-else class="create-btn" @click="startCreate">
        <span class="icon">+</span>
        <span>新建歌单</span>
      </button>

      <div v-if="pinnedOnline && pinnedOnline.length" class="pinned-block">
        <div class="section-title">在线歌单</div>
        <ul class="pinned-list">
          <li
            v-for="item in pinnedOnline"
            :key="item.source + '-' + item.id + '-' + item.kind"
            class="pinned-item"
            @click="emit('open-online-item', item)"
          >
            <img v-if="item.img" :src="item.img" class="pinned-cover" alt="" />
            <div v-else class="pinned-cover placeholder"></div>
            <div class="pinned-meta">
              <div class="pinned-name">{{ item.name || '未知' }}</div>
              <div class="pinned-badge">{{ sourceLabel(item.source) }} · {{ item.kind === 'album' ? '专辑' : '歌单' }}</div>
            </div>
            <button class="pinned-unpin" title="取消固定" @click.stop="emit('unpin-online', item.id)">×</button>
          </li>
        </ul>
      </div>
    </div>
    <div class="bottom">
      <button
        :class="['settings-btn', { active: activeView === 'settings' }]"
        @click="emit('open-settings')"
      >
        <span class="icon">⚙</span>
        <span>设置</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 220px;
  height: 100%;
  display: flex;
  flex-direction: column;
  color: var(--fluent-text);
  background: var(--fluent-bg-sidebar);
  border-right: 1px solid var(--fluent-border);
  user-select: none;
}

.section {
  flex: 1;
  padding: 16px 12px;
  overflow-y: auto;
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  padding: 0 10px 10px;
  color: var(--fluent-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.6px;
}

.playlist-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.create-btn,
.settings-btn,
.nav-btn {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: inherit;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.18s ease;
}

.nav-btn {
  margin-bottom: 12px;
}

.create-btn:hover,
.settings-btn:hover,
.nav-btn:hover {
  background: var(--fluent-bg-hover);
}

.settings-btn.active,
.nav-btn.active {
  background: var(--fluent-bg-active);
}

.create-btn .icon,
.settings-btn .icon {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  opacity: 0.85;
}

.settings-btn .icon svg {
  width: 16px;
  height: 16px;
}

/* 固定到侧栏的在线歌单 / 专辑 */
.pinned-block {
  margin-top: 18px;
  padding-top: 14px;
  border-top: 1px solid var(--fluent-border);
}
.pinned-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.pinned-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 8px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.18s ease;
}
.pinned-item:hover {
  background: var(--fluent-bg-hover);
}
.pinned-cover {
  width: 40px;
  height: 40px;
  border-radius: 6px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--fluent-bg-card);
}
.pinned-cover.placeholder {
  background: var(--fluent-bg-active);
}
.pinned-meta {
  flex: 1;
  min-width: 0;
}
.pinned-name {
  font-size: 13px;
  color: var(--fluent-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pinned-badge {
  font-size: 11px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pinned-unpin {
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--fluent-text-secondary);
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s ease, background 0.15s ease;
  flex-shrink: 0;
}
.pinned-item:hover .pinned-unpin {
  opacity: 1;
}
.pinned-unpin:hover {
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
}

.bottom {
  padding: 10px 12px;
  border-top: 1px solid var(--fluent-border);
}
</style>
