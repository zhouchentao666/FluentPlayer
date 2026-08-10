<script lang="ts" setup>
import { ref } from 'vue'
import { type Playlist } from '../types'
import type { PinnedOnlineItem, OnlineTab } from '../composables/useConfig'
import PlaylistItem from './sidebar/PlaylistItem.vue'
import PlaylistCreateInput from './sidebar/PlaylistCreateInput.vue'

const props = defineProps<{
  playlists: Playlist[]
  selectedId: string
  activeView?: 'main' | 'settings' | 'online' | 'sponsor'
  onlineTab?: OnlineTab
  pinnedOnline?: PinnedOnlineItem[]
}>()

const emit = defineEmits<{
  (e: 'update:playlists', playlists: Playlist[]): void
  (e: 'update:selectedId', id: string): void
  (e: 'open-settings'): void
  (e: 'open-online', tab: OnlineTab): void
  (e: 'select', id: string): void
  (e: 'open-online-item', item: PinnedOnlineItem): void
  (e: 'unpin-online', id: string): void
  (e: 'open-sponsor'): void
  (e: 'drop-songs', payload: { targetPlaylistId: string; sourcePlaylistId: string; songIds: string[] }): void
}>()

/** 在线音乐拆分成四个独立入口，点击直达对应子标签页。 */
const ONLINE_TABS: { id: OnlineTab; label: string; path: string }[] = [
  { id: 'playlists', label: '歌单', path: 'M4 6h10M4 10h10M4 14h6M17 8v7.2a1.8 1.8 0 1 1-1.2-1.7' },
  { id: 'albums', label: '专辑', path: 'M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14zm0 5.6a1.4 1.4 0 1 0 0 2.8 1.4 1.4 0 0 0 0-2.8z' },
  { id: 'charts', label: '排行榜', path: 'M4 16V9m6 7V4m6 12v-5' },
  { id: 'search', label: '搜索', path: 'M9 3a6 6 0 1 0 0 12A6 6 0 0 0 9 3zm4.5 10.5L17 17' },
]

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
      <div class="section-title">在线音乐</div>
      <ul class="online-nav">
        <li v-for="t in ONLINE_TABS" :key="t.id">
          <button
            :class="['nav-btn', 'sub', { active: activeView === 'online' && onlineTab === t.id }]"
            @click="emit('open-online', t.id)"
          >
            <span class="icon">
              <svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                <path :d="t.path" />
              </svg>
            </span>
            <span>{{ t.label }}</span>
          </button>
        </li>
      </ul>
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
        <li
          v-for="item in (pinnedOnline || [])"
          :key="item.source + '-' + item.id + '-' + item.kind"
          class="item pinned-item"
          @click="emit('open-online-item', item)"
        >
          <div class="playlist-info">
            <div class="playlist-icon online">
              <svg viewBox="0 0 24 24" width="16" height="16">
                <path
                  d="M12 3v10.55A4 4 0 1 0 14 17V7h4V3h-6z"
                  fill="currentColor"
                />
              </svg>
            </div>
            <div class="playlist-name">{{ item.name || '未知' }}</div>
          </div>
          <div class="item-actions">
            <button title="取消固定" @click.stop="emit('unpin-online', item.id)">
              <svg viewBox="0 0 24 24" width="15" height="15">
                <path d="M6 6l12 12M18 6L6 18" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" />
              </svg>
            </button>
          </div>
        </li>
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
    </div>
    <div class="bottom">
      <button
        :class="['nav-btn', 'bottom-btn', { active: activeView === 'sponsor' }]"
        @click="emit('open-sponsor')"
      >
        <span class="icon">
          <svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M10 14.5l-4.5-2.5c-1 .3-2 .2-2.5-.5-.5-.7-.3-1.6.4-2.1.6-.4 1.4-.4 2-.1M10 14.5l4.5-2.5c1 .3 2 .2 2.5-.5.5-.7.3-1.6-.4-2.1-.6-.4-1.4-.4-2-.1M10 14.5V4.5M10 9c1.5 0 2.5-1 2.5-2.2S11.4 4.6 10 4.6 7.5 5.6 7.5 6.8 8.5 9 10 9z" />
          </svg>
        </span>
        <span>赞助作者</span>
      </button>
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

.online-nav {
  list-style: none;
  margin: 0 0 18px;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-btn .icon {
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.85;
}

.nav-btn.sub {
  padding: 8px 12px;
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

/* 固定到侧栏的在线歌单 / 专辑（样式与本地歌单 PlaylistItem 一致） */
.item.pinned-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.18s ease;
}
.item.pinned-item:hover {
  background: var(--fluent-bg-hover);
}
.playlist-info {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  min-width: 0;
}
.playlist-icon {
  width: 38px;
  height: 38px;
  border-radius: 8px;
  background: var(--fluent-bg-active);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--fluent-text-secondary);
  flex-shrink: 0;
}
.playlist-icon.online {
  color: var(--fluent-accent, #3b82f6);
}
.playlist-icon svg {
  width: 16px;
  height: 16px;
}
.playlist-name {
  flex: 1;
  font-size: 13px;
  color: var(--fluent-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.item-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.15s ease;
  flex-shrink: 0;
}
.item.pinned-item:hover .item-actions {
  opacity: 1;
}
.item-actions button {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--fluent-text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}
.item-actions button:hover {
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
}

.bottom {
  padding: 10px 12px;
  border-top: 1px solid var(--fluent-border);
}
</style>
