<script lang="ts" setup>
import { ref } from 'vue'
import { type Playlist } from '../types'
import PlaylistItem from './sidebar/PlaylistItem.vue'
import PlaylistCreateInput from './sidebar/PlaylistCreateInput.vue'

const props = defineProps<{
  playlists: Playlist[]
  selectedId: string
  activeView?: 'main' | 'settings'
  collapsed?: boolean
  isMobile?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:playlists', playlists: Playlist[]): void
  (e: 'update:selectedId', id: string): void
  (e: 'update:collapsed', collapsed: boolean): void
  (e: 'open-settings'): void
  (e: 'select', id: string): void
  (e: 'drop-songs', payload: { targetPlaylistId: string; sourcePlaylistId: string; songIds: string[] }): void
}>()

function toggleCollapse() {
  emit('update:collapsed', !props.collapsed)
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
  <aside
    class="sidebar"
    :class="{ collapsed: collapsed, mobile: isMobile, open: !collapsed }"
  >
    <div class="sidebar-header">
      <button class="collapse-btn" :title="collapsed ? '展开' : '折叠'" @click="toggleCollapse">
        <svg v-if="collapsed" viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
          <path d="M3 6h18v2H3V6zm0 5h18v2H3v-2zm0 5h18v2H3v-2z" />
        </svg>
        <svg v-else viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
          <path d="M15.41 7.41 14 6l-6 6 6 6 1.41-1.41L10.83 12z" />
        </svg>
      </button>
    </div>
    <div class="section">
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
  <div
    v-if="isMobile"
    class="sidebar-overlay"
    :class="{ show: !collapsed }"
    @click="toggleCollapse"
  ></div>
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
  transition: width 240ms cubic-bezier(0.22, 1, 0.36, 1);
  z-index: 30;
}

/* 折叠态：桌面端显示窄栏（仅图标），移动端变为抽屉（默认移出屏幕） */
.sidebar.collapsed:not(.mobile) {
  width: 64px;
}

.sidebar.collapsed:not(.mobile) .section-title {
  display: none;
}

.sidebar.collapsed:not(.mobile) :deep(.playlist-item .name),
.sidebar.collapsed:not(.mobile) :deep(.playlist-item .actions),
.sidebar.collapsed:not(.mobile) :deep(.playlist-item .spacer) {
  display: none;
}

.sidebar.collapsed:not(.mobile) :deep(.playlist-item) {
  justify-content: center;
  padding: 8px 0;
}

.sidebar.collapsed:not(.mobile) .create-btn span:not(.icon),
.sidebar.collapsed:not(.mobile) .settings-btn span:not(.icon) {
  display: none;
}

.sidebar.collapsed:not(.mobile) .create-btn,
.sidebar.collapsed:not(.mobile) .settings-btn {
  justify-content: center;
}

.sidebar-header {
  height: 44px;
  display: flex;
  align-items: center;
  padding: 0 10px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--fluent-border);
}

.sidebar.collapsed:not(.mobile) .sidebar-header {
  justify-content: center;
  padding: 0;
}

.collapse-btn {
  width: 32px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--fluent-text-secondary);
  cursor: pointer;
  transition: background 0.18s ease, color 0.18s ease;
}

.collapse-btn:hover {
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
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
.settings-btn {
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

.create-btn:hover,
.settings-btn:hover {
  background: var(--fluent-bg-hover);
}

.settings-btn.active {
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

.bottom {
  padding: 10px 12px;
  border-top: 1px solid var(--fluent-border);
}

/* 移动端：侧栏变为抽屉，默认移出屏幕，打开时滑入 */
.sidebar.mobile {
  position: fixed;
  top: 0;
  left: 0;
  bottom: 0;
  width: 78vw;
  max-width: 320px;
  z-index: 210;
  border-right: none;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(28px) saturate(140%);
  -webkit-backdrop-filter: blur(28px) saturate(140%);
  transform: translateX(-100%);
  transition: transform 300ms cubic-bezier(0.22, 1, 0.36, 1);
  will-change: transform;
}

.sidebar.mobile.open {
  transform: translateX(0);
  box-shadow: 0 0 40px rgba(0, 0, 0, 0.5);
}

.sidebar-overlay {
  position: fixed;
  inset: 0;
  z-index: 205;
  background: rgba(0, 0, 0, 0.45);
  opacity: 0;
  pointer-events: none;
  transition: opacity 300ms ease;
}

.sidebar-overlay.show {
  opacity: 1;
  pointer-events: auto;
}
</style>
