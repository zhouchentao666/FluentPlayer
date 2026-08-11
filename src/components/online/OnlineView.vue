<script setup lang="ts">
import { ref, onMounted, inject, computed, type Ref } from 'vue'
import type { Song, Playlist as LocalPlaylist } from '../../types'
import type { Playlist } from '@online/lib/playlists'
import type { Album } from '@online/lib/albums'
import type { MusicInfo } from '@online/types/music'
import type { AppSettings, PinnedOnlineItem } from '../../composables/useConfig'
import { musicInfoToSong } from '@online/player'
import { parsePlaylistLink } from '@online/lib/playlists/openLink'
import { toast } from '../../composables/useToast'
import OnlineSearch from './OnlineSearch.vue'
import OnlineHotPlaylists from './OnlineHotPlaylists.vue'
import OnlineHotAlbums from './OnlineHotAlbums.vue'
import OnlineCharts from './OnlineCharts.vue'
import ComboBox, { type ComboBoxOption } from '../settings/ComboBox.vue'
import { downloadSong, downloadMany } from '@online/lib/download'
import type { OnlineTab } from '../../composables/useConfig'

type OpenTarget = { source: 'wy' | 'kw' | 'kg' | 'tx' | 'mg'; id: string; kind: 'playlist' | 'album' }

const props = defineProps<{
  playlists: LocalPlaylist[]
  currentSong: Song | null
  tab: OnlineTab
}>()
const emit = defineEmits<{
  (e: 'play-songs', songs: Song[], index: number): void
  (e: 'add-to-queue', song: Song): void
  (e: 'add-to-playlist', playlistId: string, songs: Song[]): void
  (e: 'open-detail', target: OpenTarget): void
  (e: 'update:tab', tab: OnlineTab): void
  (e: 'comment', m: MusicInfo): void
  (e: 'open-sources'): void
  (e: 'recognize'): void
}>()

const settings = inject<Ref<AppSettings>>('settings')

const TABS: { id: OnlineTab; label: string }[] = [
  { id: 'playlists', label: '歌单' },
  { id: 'albums', label: '专辑' },
  { id: 'charts', label: '排行榜' },
  { id: 'search', label: '搜索' },
]
const tabTitle = computed(() => TABS.find((t) => t.id === props.tab)?.label ?? '')

const addMenu = ref<{ musics: MusicInfo[]; title: string } | null>(null)

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
  emit('open-detail', {
    source: item.source as 'wy' | 'kw' | 'kg' | 'tx' | 'mg',
    id: item.id,
    kind,
  })
}

// ---- 听歌识曲（改为在 App 中打开独立界面） ----
function openRecognize() {
  emit('recognize')
}

// ---- 固定到侧栏 ----
const pinnedList = () => settings?.value.pinnedOnlinePlaylists ?? []
function isPinned(t: OpenTarget): boolean {
  return pinnedList().some((p) => p.source === t.source && p.id === t.id && p.kind === t.kind)
}
function onTogglePin(item: PinnedOnlineItem) {
  if (!settings) return
  const list = settings.value.pinnedOnlinePlaylists ?? []
  const idx = list.findIndex((p) => p.source === item.source && p.id === item.id && p.kind === item.kind)
  if (idx >= 0) {
    settings.value.pinnedOnlinePlaylists = list.filter((_, i) => i !== idx)
    toast('已取消固定', 'success')
  } else {
    settings.value.pinnedOnlinePlaylists = [...list, item]
    toast('已固定到侧栏', 'success')
  }
}

// ---- 打开外部歌单 / 专辑链接 ----
const linkModal = ref(false)
const linkSource = ref<'wy' | 'kw' | 'kg' | 'tx' | 'mg'>('wy')
const linkKind = ref<'playlist' | 'album'>('playlist')
const linkText = ref('')
const linkLoading = ref(false)
const linkSources: { id: 'wy' | 'kw' | 'kg' | 'tx' | 'mg'; label: string }[] = [
  { id: 'wy', label: '网易云' },
  { id: 'kw', label: '酷我' },
  { id: 'kg', label: '酷狗' },
  { id: 'tx', label: 'QQ' },
  { id: 'mg', label: '咪咕' },
]
const linkSourceOptions = computed<ComboBoxOption[]>(() =>
  linkSources.map(s => ({ value: s.id, label: s.label }))
)
function setLinkSource(value: string) {
  linkSource.value = value as typeof linkSource.value
}
async function openExternalLink() {
  const link = linkText.value.trim()
  if (!link) return
  linkLoading.value = true
  try {
    const id = await parsePlaylistLink(linkSource.value, link)
    linkModal.value = false
    linkText.value = ''
    emit('open-detail', { source: linkSource.value, id, kind: linkKind.value })
  } catch (err) {
    toast((err as Error).message || '链接解析失败', 'error')
  } finally {
    linkLoading.value = false
  }
}

// ---- 自定义音源：移至独立音源管理界面 (OnlineSources.vue) ----
</script>

<template>
  <div class="online-view">
    <div class="topbar">
      <div class="topbar-title">{{ tabTitle }}</div>
      <div class="topbar-actions">
        <button class="open-link-btn" title="打开外部歌单 / 专辑链接" @click="linkModal = true">
          <svg viewBox="0 0 16 16" width="15" height="15"><path d="M6.5 9.5l3-3M7 4h4v4M9.5 6.5L12 4M4 12V6h3" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
          打开链接
        </button>
        <button class="open-link-btn" title="自定义音源管理" @click="emit('open-sources')">
          <svg viewBox="0 0 16 16" width="15" height="15"><path d="M8 2v12M3.5 5.5v5M12.5 5.5v5" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/></svg>
          音源
        </button>
      </div>
    </div>

    <div class="body">
      <Transition name="view-flip" mode="out-in">
        <OnlineSearch
          v-if="tab === 'search'"
          key="search"
          @play="onPlay"
          @queue="onQueue"
          @add-playlist="onAddPlaylist"
          @download="onDownload"
          @comment="(m) => emit('comment', m)"
          @open="onOpen"
          @recognize="openRecognize"
        />
        <OnlineHotPlaylists v-else-if="tab === 'playlists'" key="playlists" @open="(item) => onOpen(item, 'playlist')" />
        <OnlineHotAlbums v-else-if="tab === 'albums'" key="albums" @open="(item) => onOpen(item, 'album')" />
        <OnlineCharts
          v-else-if="tab === 'charts'"
          key="charts"
          @play="onPlay"
          @queue="onQueue"
          @add-playlist="onAddPlaylist"
          @download="onDownload"
          @add-all="onAddAll"
          @download-all="onDownloadAll"
          @comment="(m) => emit('comment', m)"
        />
      </Transition>
    </div>

    <!-- 打开外部歌单 / 专辑链接 -->
    <div v-if="linkModal" class="modal-mask" @click.self="linkModal = false">
      <div class="link-modal">
        <div class="link-title">打开外部歌单 / 专辑</div>
        <div class="link-hint">粘贴分享链接（如网易云歌单 / 专辑链接），选择对应平台后打开。</div>
        <div class="link-row">
          <ComboBox
            class="link-select"
            width="110px"
            aria-label="选择平台"
            :options="linkSourceOptions"
            :model-value="linkSource"
            @update:model-value="setLinkSource"
          />
          <div class="link-kind">
            <button :class="{ active: linkKind === 'playlist' }" @click="linkKind = 'playlist'">歌单</button>
            <button :class="{ active: linkKind === 'album' }" @click="linkKind = 'album'">专辑</button>
          </div>
          <input
            v-model="linkText"
            class="link-input"
            placeholder="https://music.163.com/playlist?id=..."
            @keyup.enter="openExternalLink"
          />
        </div>
        <div class="link-actions">
          <button class="cancel" @click="linkModal = false">取消</button>
          <button class="confirm" :disabled="linkLoading" @click="openExternalLink">
            {{ linkLoading ? '解析中…' : '打开' }}
          </button>
        </div>
      </div>
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
  justify-content: space-between;
  padding: 14px 28px 6px;
  flex-shrink: 0;
}
.topbar-actions {
  display: inline-flex;
  gap: 8px;
  flex-shrink: 0;
}
.open-link-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 34px;
  padding: 0 14px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 13px;
  cursor: pointer;
  flex-shrink: 0;
}
.open-link-btn:hover {
  background: var(--fluent-bg-hover);
}
.topbar-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--fluent-text);
}
.body {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
/*
 * 在线子标签切换：与主界面 view-flip 动画保持一致
 */
.view-flip-enter-active {
  position: absolute;
  inset: 0;
  transition: transform 320ms cubic-bezier(0.22, 1, 0.36, 1);
}
.view-flip-leave-active {
  position: absolute;
  inset: 0;
  transition: opacity 320ms cubic-bezier(0.22, 1, 0.36, 1);
}
.view-flip-enter-from {
  transform: translateY(56px);
}
.view-flip-leave-to {
  opacity: 0;
}

/* 音源管理 */
.sources-modal {
  width: 620px;
  max-width: 92%;
  max-height: 78%;
  display: flex;
  flex-direction: column;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(24px) saturate(160%);
  border: 1px solid var(--fluent-border);
  border-radius: 16px;
  padding: 8px 18px 18px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
}
.sources-modal > .cancel {
  align-self: flex-end;
  margin-top: 4px;
}
.sources-view {
  padding: 18px 10px 8px;
  flex: 1;
  min-height: 0;
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

/* 打开外部链接弹窗 */
.link-modal {
  width: 440px;
  max-width: 92%;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(24px) saturate(160%);
  border: 1px solid var(--fluent-border);
  border-radius: 16px;
  padding: 20px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
}
.link-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--fluent-text);
  margin-bottom: 8px;
}
.link-hint {
  font-size: 12.5px;
  color: var(--fluent-text-secondary);
  line-height: 1.5;
  margin-bottom: 14px;
}
.link-row {
  display: flex;
  gap: 10px;
}
.link-select {
  flex-shrink: 0;
}
.link-select :deep(.win-combo) {
  min-height: 38px;
  border-radius: 10px;
}
.link-kind {
  display: inline-flex;
  height: 38px;
  border-radius: 10px;
  border: 1px solid var(--fluent-input-border);
  background: var(--fluent-input-bg);
  overflow: hidden;
  flex-shrink: 0;
}
.link-kind button {
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  font-size: 13px;
  padding: 0 12px;
  cursor: pointer;
}
.link-kind button.active {
  background: var(--fluent-accent);
  color: #fff;
}
.link-input {
  flex: 1;
  min-width: 0;
  height: 38px;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid var(--fluent-input-border);
  background: var(--fluent-input-bg);
  color: var(--fluent-text);
  outline: none;
  font-size: 13px;
}
.link-input:focus,

.link-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 16px;
}
.link-actions .cancel {
  width: auto;
  margin: 0;
  padding: 0 18px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
}
.link-actions .confirm {
  height: 36px;
  padding: 0 20px;
  border: none;
  border-radius: 10px;
  background: var(--fluent-accent);
  color: #fff;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
}
.link-actions .confirm:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
