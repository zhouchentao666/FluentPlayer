<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick } from 'vue'

import type { MusicInfo } from '@online/types/music'
import type { Song, Playlist as LocalPlaylist } from '../../types'
import type { PinnedOnlineItem } from '../../composables/useConfig'
import type { SortMode } from '../../composables/usePlaylistView'
import { getPlaylistDetail } from '@online/lib/playlists'
import { getAlbumDetail } from '@online/lib/albums'
import { musicInfoToSong } from '@online/player'
import PlaylistViewList from '../PlaylistViewList.vue'
import PlaylistBatchBar from '../PlaylistBatchBar.vue'
import ComboBox, { type ComboBoxOption } from '../settings/ComboBox.vue'
import { toast } from '../../composables/useToast'

const props = defineProps<{
  source: 'wy' | 'kw' | 'kg' | 'tx' | 'mg'
  id: string
  kind: 'playlist' | 'album'
  currentSong: Song | null
  playlists: LocalPlaylist[]
  pinned?: boolean
}>()

const emit = defineEmits<{
  (e: 'play', songs: Song[], index: number): void
  (e: 'queue', song: Song): void
  (e: 'add-playlist', playlistId: string, song: Song): void
  (e: 'download', song: Song): void
  (e: 'comment', song: Song): void
  (e: 'add-all', playlistId: string, songs: Song[]): void
  (e: 'download-all', songs: Song[]): void
  (e: 'toggle-pin', item: PinnedOnlineItem): void
  (e: 'back'): void
}>()

interface DetailInfo {
  name: string
  img: string | null
  author?: string
}
const info = ref<DetailInfo | null>(null)
const list = ref<MusicInfo[]>([])
const page = ref(1)
const loading = ref(false)
const loadingMore = ref(false)
const coverFailed = ref(false)
/** 自动加载全部：每 1s 翻一页 */
const autoLoading = ref(false)
let autoTimer: ReturnType<typeof setTimeout> | null = null

const currentId = computed(() => props.currentSong?.id ?? '')
const cover = computed(() => (coverFailed.value ? null : info.value?.img ?? null))
/** 与本地歌单共用统一的歌曲列表组件 */
const fullList = computed(() => list.value.map(musicInfoToSong))

// ---- 搜索 / 排序 / 批量 ----
type SortOrder = 'asc' | 'desc'

const SORT_LABELS: Partial<Record<SortMode, string>> = {
  custom: '默认',
  title: '标题',
  artist: '艺术家',
  album: '专辑',
  duration: '时长',
  year: '年份',
}
const sortOptions = computed<ComboBoxOption[]>(() =>
  (Object.keys(SORT_LABELS) as SortMode[]).map((m) => ({ value: m, label: SORT_LABELS[m] ?? m })),
)
const orderOptions: ComboBoxOption[] = [
  { value: 'asc', label: '升序' },
  { value: 'desc', label: '降序' },
]

const searchQuery = ref('')
const sortMode = ref<SortMode>('custom')
const sortOrder = ref<SortOrder>('asc')
const batchMode = ref(false)
const selectedIds = ref<Set<string>>(new Set())

// 工具栏图标展开状态
const showSearch = ref(false)
const showSort = ref(false)
const searchInput = ref<HTMLInputElement | null>(null)
function toggleSearch() {
  showSearch.value = !showSearch.value
  if (showSearch.value) {
    showSort.value = false
    nextTick(() => searchInput.value?.focus())
  }
}

function sortKey(song: Song, mode: SortMode): string | number {
  const m = song.metadata
  switch (mode) {
    case 'title':
      return (m?.title || song.title || '').toLowerCase()
    case 'artist':
      return (m?.artist || '').toLowerCase()
    case 'album':
      return (m?.album || '').toLowerCase()
    case 'duration':
      return m?.duration ?? 0
    case 'year':
      return m?.year?.toString().toLowerCase() ?? ''
    default:
      return 0
  }
}

const displaySongs = computed<Song[]>(() => {
  let result = fullList.value
  const q = searchQuery.value.trim().toLowerCase()
  if (q) {
    result = result.filter((s) =>
      [s.metadata?.title, s.metadata?.artist, s.metadata?.album, s.title]
        .filter(Boolean)
        .join(' ')
        .toLowerCase()
        .includes(q),
    )
  }
  if (sortMode.value !== 'custom') {
    const order = sortOrder.value === 'asc' ? 1 : -1
    result = [...result].sort((a, b) => {
      const av = sortKey(a, sortMode.value)
      const bv = sortKey(b, sortMode.value)
      if (typeof av === 'number' && typeof bv === 'number') return (av - bv) * order
      return String(av).localeCompare(String(bv), undefined, { numeric: true }) * order
    })
  }
  return result
})

const selectedSongs = computed(() =>
  displaySongs.value.filter((s) => selectedIds.value.has(s.id)),
)

function toggleSelection(id: string) {
  const next = new Set(selectedIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedIds.value = next
}
function selectAll() {
  selectedIds.value = new Set(displaySongs.value.map((s) => s.id))
}
function clearSelection() {
  selectedIds.value = new Set()
}
function exitBatch() {
  batchMode.value = false
  clearSelection()
}

// 「收藏歌单」：选择本地歌单后整张收藏
const collectMenu = ref<Song[] | null>(null)
function openCollectMenu() {
  collectMenu.value = fullList.value
}
function confirmCollect(plId: string) {
  if (!collectMenu.value) return
  emit('add-all', plId, collectMenu.value)
  collectMenu.value = null
  toast('已添加到歌单', 'success')
}

async function load(p: number): Promise<number> {
  if (p === 1) loading.value = true
  else loadingMore.value = true
  try {
    const res =
      props.kind === 'album'
        ? await getAlbumDetail(props.source, props.id, p)
        : await getPlaylistDetail(props.source, props.id, p)
    if (p === 1) {
      info.value = res.info as DetailInfo
      list.value = res.list
      coverFailed.value = false
    } else {
      list.value = [...list.value, ...res.list]
    }
    page.value = p
    return res.list.length
  } finally {
    loading.value = false
    loadingMore.value = false
  }
}

/** 每 1s 自动加载下一页，直到本页返回为空或不足一页（一页 30 首）。 */
async function loadNext() {
  if (!autoLoading.value) return
  if (loadingMore.value) {
    // 上一页还在加载，稍后再试，避免并发请求
    autoTimer = setTimeout(loadNext, 1000)
    return
  }
  const n = await load(page.value + 1)
  if (n === 0 || n < 30) {
    autoLoading.value = false
    autoTimer = null
    return
  }
  autoTimer = setTimeout(loadNext, 1000)
}

function stopAuto() {
  autoLoading.value = false
  if (autoTimer) {
    clearTimeout(autoTimer)
    autoTimer = null
  }
}

function onPlaySong(song: Song) {
  const idx = displaySongs.value.findIndex((x) => x.id === song.id)
  emit('play', displaySongs.value, idx >= 0 ? idx : 0)
}

function onTogglePin() {
  emit('toggle-pin', {
    source: props.source,
    id: props.id,
    kind: props.kind,
    name: info.value?.name ?? '',
    img: info.value?.img ?? null,
  })
}

onMounted(async () => {
  const n = await load(1)
  if (n > 0) {
    autoLoading.value = true
    autoTimer = setTimeout(loadNext, 1000)
  }
})
onUnmounted(stopAuto)
</script>

<template>
  <div class="detail">
    <div v-if="loading" class="state">加载中…</div>
    <template v-else-if="info">
      <button class="back" @click="emit('back')">
        <svg viewBox="0 0 16 16" width="16" height="16"><path d="M10 3L5 8l5 5" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
        返回
      </button>
      <div class="header">
        <div class="cover-wrap">
          <img v-if="cover" :src="cover" class="cover" alt="" @error="coverFailed = true" />
          <div v-else class="cover placeholder">{{ (info.name || '?').charAt(0) }}</div>
        </div>
        <div class="info">
          <div class="kind-tag">{{ kind === 'album' ? '专辑' : '歌单' }}</div>
          <h1 class="name">{{ info.name }}</h1>
          <div v-if="info.author" class="author">{{ info.author }}</div>
          <div class="count">{{ fullList.length }} 首</div>
          <div class="buttons">
            <button class="play-all" @click="emit('play', displaySongs, 0)">
              <svg viewBox="0 0 16 16" width="14" height="14"><path d="M4 2.5v11l9-5.5z" fill="currentColor"/></svg>
              播放全部
            </button>
            <button class="ghost" @click="openCollectMenu">收藏歌单</button>
            <button class="ghost" @click="emit('download-all', fullList)">下载全部</button>
            <button class="ghost pin" :class="{ active: pinned }" @click="onTogglePin">
              <svg viewBox="0 0 16 16" width="14" height="14">
                <path
                  d="M5 2.5h6l-0.6 2.2 2.1 2.3-2.9 0.2 1.1 3-2.7-1.8-2.7 1.8 1.1-3-2.9-0.2 2.1-2.3z"
                  :fill="pinned ? 'currentColor' : 'none'"
                  stroke="currentColor"
                  stroke-width="1.1"
                  stroke-linejoin="round"
                />
              </svg>
              {{ pinned ? '已固定' : '固定到侧栏' }}
            </button>

            <!-- 右侧工具图标：点击展开 -->
            <div class="tool-icons">
              <button
                class="tool-icon"
                :class="{ active: showSearch }"
                title="搜索"
                @click="toggleSearch"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="11" cy="11" r="7" />
                  <path d="M21 21l-4.3-4.3" />
                </svg>
              </button>

              <div class="tool-pop" :class="{ open: showSort }">
                <button
                  class="tool-icon"
                  :class="{ active: showSort || sortMode !== 'custom' }"
                  title="排序"
                  @click="showSort = !showSort"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M4 6h12M4 12h9M4 18h6" />
                    <path d="M17 8l3 3 3-3" transform="translate(-3 2)" />
                  </svg>
                </button>
                <div v-if="showSort" class="pop-panel">
                  <ComboBox
                    width="120px"
                    aria-label="排序方式"
                    :options="sortOptions"
                    :model-value="sortMode"
                    @update:model-value="(v) => (sortMode = v as SortMode)"
                  />
                  <button
                    class="order-btn"
                    :title="sortOrder === 'asc' ? '升序' : '降序'"
                    @click="sortOrder = sortOrder === 'asc' ? 'desc' : 'asc'"
                  >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <path v-if="sortOrder === 'asc'" d="M6 15l6 6 6-6M12 3v18" />
                      <path v-else d="M6 9l6-6 6 6M12 21V3" />
                    </svg>
                  </button>
                </div>
              </div>

              <button
                class="tool-icon"
                :class="{ active: batchMode }"
                :title="batchMode ? '退出批量选择' : '批量选择'"
                @click="batchMode ? exitBatch() : (batchMode = true)"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M3 5h18M3 12h18M3 19h18" />
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 搜索展开输入 -->
      <div v-if="showSearch" class="search-row">
        <div class="search-box">
          <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="7" />
            <path d="M21 21l-4.3-4.3" />
          </svg>
          <input
            v-model="searchQuery"
            ref="searchInput"
            class="search-input"
            type="text"
            placeholder="搜索歌曲"
            aria-label="搜索歌曲"
            @keyup.esc="showSearch = false"
          />
          <button v-if="searchQuery" class="search-clear" title="清除" @click="searchQuery = ''">×</button>
        </div>
      </div>

      <PlaylistViewList
        :songs="displaySongs"
        :playlists="playlists"
        :current-song="currentSong"
        :playlist-id="`online-${source}-${id}`"
        :sort-mode="'custom'"
        :batch-mode="batchMode"
        :selected-ids="selectedIds"
        :search-query="searchQuery"
        @play="onPlaySong"
        @add-to-queue="(s) => emit('queue', s)"
        @toggle="toggleSelection"
        @add-to-playlist="(pid, s) => emit('add-playlist', pid, s)"
        @comment="(s) => emit('comment', s)"
      />

      <PlaylistBatchBar
        v-if="batchMode"
        :selected-songs="selectedSongs"
        :playlists="playlists"
        :current-playlist-id="`online-${source}-${id}`"
        :show-download="true"
        :show-remove="false"
        :show-replace="false"
        @select-all="selectAll"
        @clear-selection="clearSelection"
        @download="emit('download-all', selectedSongs)"
        @add-to-playlist="(pid) => emit('add-all', pid, selectedSongs)"
        @close="exitBatch"
      />

      <!-- 收藏整张歌单：选择本地歌单 -->
      <div v-if="collectMenu" class="modal-mask" @click.self="collectMenu = null">
        <div class="add-menu">
          <div class="add-menu-title">收藏到歌单</div>
          <div class="add-menu-list">
            <button
              v-for="pl in playlists"
              :key="pl.id"
              class="add-menu-item"
              @click="confirmCollect(pl.id)"
            >
              {{ pl.name }}
              <span class="cnt">{{ pl.songs.length }}</span>
            </button>
            <div v-if="playlists.length === 0" class="state small">暂无歌单</div>
          </div>
          <button class="cancel" @click="collectMenu = null">取消</button>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.detail {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 16px 22px 24px;
  height: 100%;
  overflow-y: auto;
}
.state {
  color: var(--fluent-text-secondary);
  text-align: center;
  padding: 40px 0;
}
.back {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  align-self: flex-start;
  border: none;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  padding: 8px 14px;
  border-radius: 10px;
  cursor: pointer;
  font-size: 13px;
  flex-shrink: 0;
}
.back:hover {
  background: var(--fluent-bg-hover);
}
/* 收藏歌单弹层 */
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
.state.small {
  padding: 12px 0;
  color: var(--fluent-text-secondary);
  text-align: center;
  font-size: 14px;
}
.header {
  display: flex;
  gap: 16px;
}
.cover-wrap {
  width: 96px;
  height: 96px;
  flex-shrink: 0;
  border-radius: 10px;
  overflow: hidden;
  background: var(--fluent-bg-card);
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.22);
}
.cover {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 30px;
  font-weight: 700;
  color: var(--fluent-text-secondary);
  background: linear-gradient(135deg, var(--fluent-bg-active), var(--fluent-bg-card));
}
.info {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}
.kind-tag {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.name {
  font-size: 18px;
  font-weight: 800;
  color: var(--fluent-text);
  margin: 0;
  word-break: break-word;
}
.author {
  font-size: 14px;
  color: var(--fluent-text-secondary);
}
.count {
  font-size: 13px;
  color: var(--fluent-text-secondary);
}
.buttons {
  display: flex;
  gap: 10px;
  margin-top: 6px;
  flex-wrap: wrap;
}
.play-all {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  padding: 0 20px;
  border: none;
  border-radius: 10px;
  background: var(--fluent-accent);
  color: #fff;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
.ghost {
  height: 34px;
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
.ghost.active {
  border-color: var(--fluent-accent);
  color: var(--fluent-accent);
}
/* 工具图标（搜索 / 排序 / 批量），位于固定按钮右侧 */
.tool-icons {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;
}
.tool-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  border: none;
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
  cursor: pointer;
  transition: background 0.18s ease, color 0.18s ease;
}
.tool-icon:hover {
  background: var(--fluent-bg-active);
}
.tool-icon.active {
  background: var(--fluent-accent);
  color: #fff;
}
.tool-icon svg {
  width: 18px;
  height: 18px;
}
.tool-pop {
  position: relative;
  display: inline-flex;
}
.pop-panel {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border-radius: 12px;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(24px) saturate(160%);
  border: 1px solid var(--fluent-border);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.3);
  z-index: 20;
}
.pop-panel :deep(.win-combo) {
  min-height: 34px;
  border-radius: 8px;
}

/* 搜索展开行 */
.search-row {
  display: flex;
  justify-content: flex-end;
}
.search-box {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 34px;
  padding: 0 10px;
  width: 280px;
  max-width: 100%;
  border-radius: 18px;
  background: var(--fluent-bg-hover);
  border: 1px solid transparent;
  transition: border-color 0.18s ease, background 0.18s ease;
}
.search-box:focus-within {
  border-color: var(--fluent-accent);
  background: var(--fluent-bg-card);
}
.search-icon {
  width: 15px;
  height: 15px;
  color: var(--fluent-text-secondary);
  flex: none;
}
.search-input {
  flex: 1;
  width: 100%;
  border: none;
  outline: none;
  background: transparent;
  color: var(--fluent-text);
  font-size: 13px;
}
.search-input::placeholder {
  color: var(--fluent-text-secondary);
}
.search-clear {
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
  padding: 0 2px;
}
.search-clear:hover {
  color: var(--fluent-text);
}
.order-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  border: none;
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
  cursor: pointer;
  transition: background 0.18s ease;
}
.order-btn:hover {
  background: var(--fluent-bg-active);
}
.order-btn svg {
  width: 16px;
  height: 16px;
}
</style>
