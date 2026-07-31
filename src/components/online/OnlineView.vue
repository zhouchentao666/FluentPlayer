<script setup lang="ts">
import { ref, onMounted } from 'vue'
import type { Song, Playlist as LocalPlaylist } from '../../types'
import type { Playlist } from '@online/lib/playlists'
import type { Album } from '@online/lib/albums'
import type { MusicInfo } from '@online/types/music'
import { musicInfoToSong } from '@online/player'
import { useOnlineSources } from '@online/store'
import { toast } from '../../composables/useToast'
import OnlineSearch from './OnlineSearch.vue'
import OnlineHotPlaylists from './OnlineHotPlaylists.vue'
import OnlineHotAlbums from './OnlineHotAlbums.vue'
import OnlineDetail from './OnlineDetail.vue'
import { downloadSong, downloadMany } from '@online/lib/download'

const props = defineProps<{ playlists: LocalPlaylist[]; currentSong: Song | null }>()
const emit = defineEmits<{
  (e: 'play-songs', songs: Song[], index: number): void
  (e: 'add-to-queue', song: Song): void
  (e: 'add-to-playlist', playlistId: string, songs: Song[]): void
}>()

type Tab = 'search' | 'playlists' | 'albums' | 'sources'
const tab = ref<Tab>('search')

const detail = ref<{ source: 'wy' | 'kw' | 'kg' | 'tx' | 'mg'; id: string; kind: 'playlist' | 'album' } | null>(null)
const addMenu = ref<{ musics: MusicInfo[]; title: string } | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)

function onPlay(musics: MusicInfo[], index: number) {
  emit('play-songs', musics.map(musicInfoToSong), index)
}
function onQueue(m: MusicInfo) {
  emit('add-to-queue', musicInfoToSong(m))
}
function openAddMenu(musics: MusicInfo[], title: string) {
  addMenu.value = { musics, title }
}
function onAddPlaylist(m: MusicInfo) {
  openAddMenu([m], '收藏到歌单')
}
function onAddAll(musics: MusicInfo[]) {
  openAddMenu(musics, `收藏 ${musics.length} 首到歌单`)
}
function confirmAdd(plId: string) {
  if (!addMenu.value) return
  emit('add-to-playlist', plId, addMenu.value.musics.map(musicInfoToSong))
  addMenu.value = null
  toast('已添加到歌单', 'success')
}
function onDownload(m: MusicInfo) {
  // 真正的离线文件下载：解析直链并保存为本地文件
  downloadSong(m)
}
function onDownloadAll(musics: MusicInfo[]) {
  // 批量离线下载到所选文件夹
  downloadMany(musics)
}
function onOpen(item: Playlist | Album, kind: 'playlist' | 'album') {
  detail.value = { source: item.source as 'wy' | 'kw' | 'kg' | 'tx' | 'mg', id: item.id, kind }
}

// ---- 自定义音源 ----
const { state, importScript, importScriptFromUrl, removeScript, toggleEnabled, initOnlineSources } =
  useOnlineSources()
const importUrl = ref('')

onMounted(() => {
  initOnlineSources().catch(() => {})
})

async function onPickFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  try {
    const text = await file.text()
    await importScript(text)
    toast('音源已导入', 'success')
  } catch (err) {
    toast((err as Error).message, 'error')
  }
  input.value = ''
}
async function onImportUrl() {
  const u = importUrl.value.trim()
  if (!u) return
  try {
    await importScriptFromUrl(u)
    toast('音源已导入', 'success')
    importUrl.value = ''
  } catch (err) {
    toast((err as Error).message, 'error')
  }
}
function fmtPlatforms(s?: Record<string, unknown>): string {
  if (!s) return '通用'
  const keys = Object.keys(s)
  return keys.length ? keys.join(' / ') : '通用'
}
</script>

<template>
  <div class="online-view">
    <div class="topbar">
      <div class="tabs">
        <button :class="{ active: tab === 'search' }" @click="tab = 'search'">搜索</button>
        <button :class="{ active: tab === 'playlists' }" @click="tab = 'playlists'">排行榜</button>
        <button :class="{ active: tab === 'albums' }" @click="tab = 'albums'">新碟</button>
        <button :class="{ active: tab === 'sources' }" @click="tab = 'sources'">音源</button>
      </div>
    </div>

    <div class="body">
      <OnlineSearch
        v-if="tab === 'search'"
        @play="onPlay"
        @queue="onQueue"
        @add-playlist="onAddPlaylist"
        @download="onDownload"
        @open="onOpen"
      />
      <OnlineHotPlaylists v-else-if="tab === 'playlists'" @open="(item) => onOpen(item, 'playlist')" />
      <OnlineHotAlbums v-else-if="tab === 'albums'" @open="(item) => onOpen(item, 'album')" />

      <div v-else class="sources-view">
        <div class="sources-head">
          <h2 class="title">自定义音源</h2>
          <div class="import-row">
            <button class="ghost" @click="fileInput?.click()">导入脚本文件</button>
            <input
              ref="fileInput"
              type="file"
              accept=".js,.txt"
              style="display: none"
              @change="onPickFile"
            />
            <input v-model="importUrl" class="url-input" placeholder="粘贴音源脚本链接…" />
            <button class="ghost" @click="onImportUrl">从链接导入</button>
          </div>
        </div>

        <div v-if="state.error" class="src-error">{{ state.error }}</div>

        <div v-if="state.scripts.length === 0" class="state">
          暂无自定义音源。导入 LX 格式音源脚本后，即可在播放 / 搜索中自动使用。
        </div>

        <div v-else class="src-list">
          <div v-for="s in state.scripts" :key="s.id" class="src-item">
            <div class="src-main">
              <div class="src-name">{{ s.name }}</div>
              <div class="src-meta">
                <span v-if="s.author">作者：{{ s.author }}</span>
                <span v-if="s.version">版本：{{ s.version }}</span>
                <span>平台：{{ fmtPlatforms(s.sources) }}</span>
              </div>
            </div>
            <div class="src-actions">
              <label class="switch">
                <input type="checkbox" :checked="s.enabled" @change="toggleEnabled(s.id)" />
                <span class="slider" />
              </label>
              <button class="del" title="删除" @click="removeScript(s.id)">删除</button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 详情覆盖层 -->
    <div v-if="detail" class="overlay">
      <button class="back" @click="detail = null">
        <svg viewBox="0 0 16 16" width="16" height="16"><path d="M10 3L5 8l5 5" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
        返回
      </button>
      <OnlineDetail
        :source="detail.source"
        :id="detail.id"
        :kind="detail.kind"
        :current-song="currentSong"
        @play="onPlay"
        @queue="onQueue"
        @add-playlist="onAddPlaylist"
        @download="onDownload"
        @add-all="onAddAll"
        @download-all="onDownloadAll"
      />
    </div>

    <!-- 收藏到歌单菜单 -->
    <div v-if="addMenu" class="modal-mask" @click.self="addMenu = null">
      <div class="add-menu">
        <div class="add-menu-title">{{ addMenu.title }}</div>
        <div class="add-menu-list">
          <button
            v-for="pl in playlists"
            :key="pl.id"
            class="add-menu-item"
            @click="confirmAdd(pl.id)"
          >
            {{ pl.name }}
            <span class="cnt">{{ pl.songs.length }}</span>
          </button>
          <div v-if="playlists.length === 0" class="state small">暂无歌单</div>
        </div>
        <button class="cancel" @click="addMenu = null">取消</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.online-view {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.topbar {
  display: flex;
  align-items: center;
  padding: 14px 28px 6px;
  flex-shrink: 0;
}
.tabs {
  display: inline-flex;
  gap: 4px;
}
.tabs button {
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  padding: 8px 18px;
  border-radius: 10px;
  font-size: 15px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}
.tabs button.active {
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
  font-weight: 600;
}
.body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.overlay {
  position: absolute;
  inset: 0;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(20px) saturate(160%);
  z-index: 20;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.back {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  align-self: flex-start;
  margin: 14px 22px 0;
  border: none;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  padding: 8px 14px;
  border-radius: 10px;
  cursor: pointer;
  font-size: 13px;
}
.back:hover {
  background: var(--fluent-bg-hover);
}

/* 音源管理 */
.sources-view {
  padding: 18px 28px 28px;
  height: 100%;
  overflow-y: auto;
}
.sources-head {
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin-bottom: 18px;
}
.title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fluent-text);
  margin: 0;
}
.import-row {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  align-items: center;
}
.ghost {
  height: 36px;
  padding: 0 16px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 13px;
  cursor: pointer;
}
.ghost:hover {
  background: var(--fluent-bg-hover);
}
.url-input {
  flex: 1;
  min-width: 220px;
  height: 36px;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid var(--fluent-input-border);
  background: var(--fluent-input-bg);
  color: var(--fluent-text);
  outline: none;
  font-size: 13px;
}
.url-input:focus {
  border-color: var(--fluent-accent);
}
.src-error {
  color: #f87171;
  font-size: 13px;
  margin-bottom: 12px;
}
.src-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.src-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  border-radius: 12px;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
}
.src-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--fluent-text);
}
.src-meta {
  display: flex;
  gap: 14px;
  flex-wrap: wrap;
  margin-top: 4px;
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.src-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}
.switch {
  position: relative;
  display: inline-block;
  width: 42px;
  height: 24px;
}
.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}
.slider {
  position: absolute;
  inset: 0;
  background: var(--fluent-bg-active);
  border-radius: 999px;
  cursor: pointer;
  transition: background 0.18s ease;
}
.slider::before {
  content: '';
  position: absolute;
  width: 18px;
  height: 18px;
  left: 3px;
  top: 3px;
  background: #fff;
  border-radius: 50%;
  transition: transform 0.18s ease;
}
.switch input:checked + .slider {
  background: var(--fluent-accent);
}
.switch input:checked + .slider::before {
  transform: translateX(18px);
}
.del {
  border: 1px solid var(--fluent-border);
  background: transparent;
  color: var(--fluent-text-secondary);
  padding: 6px 12px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
}
.del:hover {
  color: #f87171;
  border-color: #f87171;
}
.state {
  color: var(--fluent-text-secondary);
  padding: 30px 0;
  text-align: center;
  font-size: 14px;
}
.state.small {
  padding: 12px 0;
}

/* 收藏菜单 */
.modal-mask {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 30;
}
.add-menu {
  width: 360px;
  max-width: 90%;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(24px) saturate(160%);
  border: 1px solid var(--fluent-border);
  border-radius: 16px;
  padding: 18px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
}
.add-menu-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--fluent-text);
  margin-bottom: 12px;
}
.add-menu-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 320px;
  overflow-y: auto;
}
.add-menu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border: none;
  border-radius: 10px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 14px;
  cursor: pointer;
  text-align: left;
}
.add-menu-item:hover {
  background: var(--fluent-bg-hover);
}
.cnt {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.cancel {
  width: 100%;
  margin-top: 14px;
  height: 36px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: transparent;
  color: var(--fluent-text-secondary);
  cursor: pointer;
}
</style>
