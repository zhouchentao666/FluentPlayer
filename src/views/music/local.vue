<script setup lang="ts">
import { ref, shallowRef, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { useVirtualList } from '@vueuse/core'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { playSong } from '@/utils/audio/globaPlayList'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { useSettingsStore } from '@/store/Settings'
import { useRouter } from 'vue-router'
import { MessagePlugin } from 'tdesign-vue-next'
import {
  RefreshIcon,
  EllipsisIcon,
  PlayCircleIcon,
  SearchIcon,
  FolderIcon
} from 'tdesign-icons-vue-next'
import AddToPlaylistDialog from '@/components/Playlist/AddToPlaylistDialog.vue'

const { t } = useI18n()
const router = useRouter()
const localUserStore = LocalUserDetailStore()
const playStatus = useGlobalPlayStatusStore()
const settingsStore = useSettingsStore()

const loading = ref(false)
const scanning = ref(false)
const tracks = shallowRef<any[]>([])
const searchQuery = ref('')
const coverCache = ref<Record<string, string>>({})

// 目录管理
const scanDirs = ref<string[]>([])
const showDirModal = ref(false)

// 多选
const selectedIds = ref<Set<string>>(new Set())
const multiSelect = ref(false)
const showAddToPlaylist = ref(false)
const songsToAdd = ref<any[]>([])

// 移动端
const isMobile = ref(window.innerWidth <= 768)
let longPressTimer: ReturnType<typeof setTimeout> | null = null

const handleTouchStart = (track: any) => {
  if (!isMobile.value) return
  longPressTimer = setTimeout(() => {
    if (!multiSelect.value) multiSelect.value = true
    toggleSelect(track.songmid)
    if (navigator.vibrate) navigator.vibrate(30)
  }, 500)
}

const handleTouchCancel = () => {
  if (longPressTimer) { clearTimeout(longPressTimer); longPressTimer = null }
}

const showMobileSearch = ref(false)
const toggleMobileSearch = () => {
  showMobileSearch.value = !showMobileSearch.value
  if (!showMobileSearch.value) searchQuery.value = ''
}

// 右键菜单
const contextMenuVisible = ref(false)
const contextMenuPos = ref({ top: 0, left: 0 })
const contextMenuTrack = ref<any | null>(null)

// 当前播放
const currentSongId = computed(() => localUserStore.userInfo?.lastPlaySongId)

const displayTracks = computed(() => {
  const q = (searchQuery.value || '').trim().toLowerCase()
  if (!q) return tracks.value
  const includes = (s?: string) => !!s && s.toLowerCase().includes(q)
  return tracks.value.filter(t => includes(t.name) || includes(t.singer) || includes(t.albumName))
})

const hasSelection = computed(() => selectedIds.value.size > 0)
const isAllSelected = computed(() =>
  displayTracks.value.length > 0 && displayTracks.value.every(t => selectedIds.value.has(t.songmid))
)
const { list: virtualTracks, containerProps, wrapperProps } = useVirtualList(displayTracks, {
  itemHeight: 60,
  overscan: 8
})

watch(searchQuery, () => {
  containerProps.ref.value?.scrollTo({ top: 0 })
})

// 加载歌曲
const fetchTracks = async () => {
  loading.value = true
  try {
    const res = await (window as any).api?.localMusic?.getList?.()
    if (res?.success) {
      tracks.value = res.data || []
      const ids = tracks.value.slice(0, 50).map((t: any) => t.songmid)
      if (ids.length) loadCovers(ids)
    }
  } catch (e) { console.error('获取本地音乐失败:', e) }
  finally { loading.value = false }
}

// 加载封面
const loadCovers = async (ids: string[]) => {
  try {
    const res = await (window as any).api?.localMusic?.getCoversBase64?.(ids)
    if (res?.success && res.data) coverCache.value = { ...coverCache.value, ...res.data }
  } catch {}
}

// 目录管理
const fetchDirs = async () => {
  try {
    const res = await (window as any).api?.localMusic?.getDirs?.()
    if (res?.success && Array.isArray(res.data)) scanDirs.value = res.data
  } catch {}
}

const selectDirs = async () => {
  try {
    const res = await (window as any).api?.localMusic?.selectDirs?.()
    const dirs = res?.success ? (res.data || []) : (Array.isArray(res) ? res : [])
    if (Array.isArray(dirs) && dirs.length > 0) {
      scanDirs.value = Array.from(new Set([...scanDirs.value, ...dirs]))
    }
  } catch {}
}

const removeDir = (d: string) => {
  scanDirs.value = scanDirs.value.filter(x => x !== d)
}

const saveDirs = async () => {
  try {
    await (window as any).api?.localMusic?.setDirs?.(scanDirs.value)
    showDirModal.value = false
    MessagePlugin.success(t('music.local.dirSaved'))
  } catch {
    MessagePlugin.error(t('music.local.dirSaveFailed'))
  }
}

// 扫描
const scanLibrary = async () => {
  if (scanDirs.value.length === 0) {
    MessagePlugin.warning(t('music.local.selectScanDir'))
    return
  }
  scanning.value = true
  try {
    const scanRes = await (window as any).api?.localMusic?.scan?.(scanDirs.value, !!settingsStore.settings.skipHiddenFiles)
    if (scanRes?.success) {
      const data = scanRes.data
      const parts = [t('music.local.scanComplete', { scanned: data.scanned, added: data.added })]
      if (data.updated) parts.push(t('music.local.scanUpdated', { updated: data.updated }))
      if (data.errors) parts.push(t('music.local.scanErrors', { errors: data.errors }))
      MessagePlugin.success(parts.join('，'))
      await fetchTracks()
    }
  } catch (e) { console.error('扫描失败:', e); MessagePlugin.error(t('music.local.scanFailed')) }
  finally { scanning.value = false }
}

// 清空
const clearScan = async () => {
  try {
    const res = await (window as any).api?.localMusic?.clearIndex?.()
    if (res?.success) {
      tracks.value = []
      MessagePlugin.success(t('music.local.indexCleared'))
    } else {
      MessagePlugin.error(t('music.local.clearFailed'))
    }
  } catch {
    MessagePlugin.error(t('music.local.clearFailed'))
  }
}

// 播放
const handlePlay = (track: any) => {
  const song = {
    songmid: track.songmid, name: track.name, singer: track.singer,
    albumName: track.albumName, img: coverCache.value[track.songmid] || '',
    source: 'local', url: track.url || '', interval: track.interval,
    path: track.path, hasCover: !!track.hasCover
  }
  playStatus.updatePlayerInfo(song)
  playSong(song)
  localUserStore.addSongToFirst(song)
}

// 播放全部
const playAll = () => {
  if (displayTracks.value.length === 0) return
  const songList = displayTracks.value.map(track => ({
    songmid: track.songmid, name: track.name, singer: track.singer,
    albumName: track.albumName, img: '',
    source: 'local', url: track.url || '', interval: track.interval,
    path: track.path, hasCover: !!track.hasCover
  }))
  localUserStore.replaceSongList(songList as any)
  playSong(songList[0] as any)
  playStatus.updatePlayerInfo(songList[0] as any)
  MessagePlugin.success(t('music.local.playingLocal', { count: songList.length }))
}

// 添加全部到播放列表
const addAllToPlaylist = () => {
  if (displayTracks.value.length === 0) return
  displayTracks.value.forEach(track => {
    const song = {
      songmid: track.songmid, name: track.name, singer: track.singer,
      albumName: track.albumName, img: '',
      source: 'local', url: track.url || '', interval: track.interval,
      hasCover: !!track.hasCover
    }
    localUserStore.addSong(song)
  })
  MessagePlugin.success(t('music.local.addedAllToPlaylist'))
}

// 选择
const toggleSelect = (songmid: string) => {
  const s = new Set(selectedIds.value)
  if (s.has(songmid)) s.delete(songmid)
  else s.add(songmid)
  selectedIds.value = s
}

const toggleSelectAll = () => {
  if (isAllSelected.value) {
    selectedIds.value = new Set()
  } else {
    selectedIds.value = new Set(displayTracks.value.map(t => t.songmid))
  }
}

const clearSelection = () => { selectedIds.value = new Set() }

const batchPlay = () => {
  const selected = displayTracks.value.filter(t => selectedIds.value.has(t.songmid))
  if (selected.length === 0) return
  const songList = selected.map(track => ({
    songmid: track.songmid, name: track.name, singer: track.singer,
    albumName: track.albumName, img: '',
    source: 'local', url: track.url || '', interval: track.interval,
    hasCover: !!track.hasCover
  }))
  localUserStore.replaceSongList(songList as any)
  playSong(songList[0] as any)
  playStatus.updatePlayerInfo(songList[0] as any)
}

const batchAddToPlaylist = () => {
  const selected = displayTracks.value.filter(t => selectedIds.value.has(t.songmid))
  if (selected.length === 0) return
  songsToAdd.value = selected.map(track => ({
    songmid: track.songmid, name: track.name, singer: track.singer,
    albumName: track.albumName, img: '',
    source: 'local', url: track.url || '', interval: track.interval,
    hasCover: !!track.hasCover
  }))
  showAddToPlaylist.value = true
}

// 右键菜单
const handleContextMenu = (e: MouseEvent, track: any) => {
  e.preventDefault()
  e.stopPropagation()
  contextMenuTrack.value = track
  contextMenuPos.value = { top: e.clientY, left: e.clientX }
  contextMenuVisible.value = true
}

const closeContextMenu = () => {
  contextMenuVisible.value = false
  contextMenuTrack.value = null
}

const handleMenuAction = (action: string) => {
  const track = contextMenuTrack.value
  if (!track) return
  closeContextMenu()
  if (action === 'play') handlePlay(track)
  else if (action === 'addToEnd') {
    const song = {
      songmid: track.songmid, name: track.name, singer: track.singer,
      albumName: track.albumName, img: '',
      source: 'local', url: track.url || '', interval: track.interval,
      hasCover: !!track.hasCover
    }
    localUserStore.addSong(song)
    MessagePlugin.success(t('music.local.addedToPlaylist'))
  } else if (action === 'addToList') {
    songsToAdd.value = [{
      songmid: track.songmid, name: track.name, singer: track.singer,
      albumName: track.albumName, img: '',
      source: 'local', url: track.url || '', interval: track.interval,
      hasCover: !!track.hasCover
    }]
    showAddToPlaylist.value = true
  } else if (action === 'editTags') {
    router.push({ name: 'local-tag-editor', query: { songmid: track.songmid } })
  }
}

// 编辑标签
const openTagEditor = (track: any) => {
  router.push({ name: 'local-tag-editor', query: { songmid: track.songmid } })
}

// 格式化
const formatDuration = (sec: number) => {
  if (!sec || !isFinite(sec)) return '--:--'
  const m = Math.floor(sec / 60), s = Math.floor(sec % 60)
  return `${m}:${s.toString().padStart(2, '0')}`
}

const formatSize = (bytes: number) => {
  if (!bytes) return ''
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1048576).toFixed(1) + ' MB'
}

// 更多操作下拉
const moreActions = computed(() => [
  { content: t('common.playAll'), value: 'playAll' },
  { content: t('music.local.addAllToPlaylist'), value: 'addAll' },
  { content: multiSelect.value ? t('common.cancelBatch') : t('common.batchSelect'), value: 'toggleMulti' },
  { content: t('common.clearAll'), value: 'clear' }
])

const handleMoreAction = (value: string) => {
  if (value === 'playAll') playAll()
  else if (value === 'addAll') addAllToPlaylist()
  else if (value === 'toggleMulti') { multiSelect.value = !multiSelect.value; if (!multiSelect.value) clearSelection() }
  else if (value === 'clear') clearScan()
}

const handleGlobalClick = () => { if (contextMenuVisible.value) closeContextMenu() }

// 生命周期
onMounted(async () => {
  await fetchDirs()
  await fetchTracks()
  document.addEventListener('click', handleGlobalClick)
  window.addEventListener('resize', () => { isMobile.value = window.innerWidth <= 768 })
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleGlobalClick)
  window.removeEventListener('resize', () => { isMobile.value = window.innerWidth <= 768 })
})
</script>

<template>
  <div class="local-container">
    <!-- 头部 -->
    <div class="local-header">
      <div class="left-container">
        <h2 class="title">
          {{ t('music.local.title') }}<span style="font-size: 12px; color: var(--td-text-color-placeholder)">{{ t('music.local.totalSongs', { count: tracks.length }) }}</span>
        </h2>
      </div>
      <div class="right-container">
        <t-button
          shape="round"
          theme="primary"
          variant="base"
          class="select-dir-trigger"
          @click="showDirModal = true"
        >
          <template #icon><FolderIcon size="16px" /></template>
          {{ t('music.local.selectDir') }}
        </t-button>
      </div>
    </div>

    <!-- 控制栏 -->
    <div class="controls">
      <t-button theme="primary" class="local-btn play-all" @click="playAll" :disabled="tracks.length === 0">
        <template #icon><i class="iconfont icon-bofang"></i></template>
        {{ t('common.playAll') }}
      </t-button>
      <t-button theme="default" class="local-btn scan" :loading="scanning" @click="scanLibrary">
        <template #icon><RefreshIcon :stroke-width="1.5" /></template>
      </t-button>
      <t-dropdown
        trigger="hover"
        :options="moreActions"
        placement="bottom-left"
        @click="(data: any) => handleMoreAction(data.value)"
      >
        <t-button theme="default" class="local-btn more">
          <template #icon><EllipsisIcon :stroke-width="1.5" /></template>
        </t-button>
      </t-dropdown>
      <t-button
        theme="default"
        class="local-btn search-toggle mobile-only"
        :class="{ active: showMobileSearch }"
        @click="toggleMobileSearch"
      >
        <template #icon><SearchIcon size="18px" /></template>
      </t-button>
      <div class="desktop-search" style="margin-left: auto; display: flex; align-items: center">
        <t-input
          v-model="searchQuery"
          clearable
          :placeholder="t('music.local.searchPlaceholder')"
          style="width: 260px"
        >
          <template #prefix-icon><SearchIcon size="16px" /></template>
        </t-input>
      </div>
    </div>

    <!-- 移动端搜索栏 -->
    <Transition name="search-slide">
      <div v-if="showMobileSearch" class="mobile-search-bar">
        <t-input
          v-model="searchQuery"
          clearable
          :placeholder="t('music.local.searchPlaceholder')"
          autofocus
        >
          <template #prefix-icon><SearchIcon size="16px" /></template>
        </t-input>
      </div>
    </Transition>

    <!-- 批量操作栏 -->
    <div v-if="hasSelection" class="batch-toolbar">
      <span class="batch-info">{{ t('music.local.selectedCount', { count: selectedIds.size }) }}</span>
      <t-button size="small" @click="batchPlay">{{ t('music.local.playSelected') }}</t-button>
      <t-button size="small" @click="batchAddToPlaylist">{{ t('common.addToPlaylist') }}</t-button>
      <t-button size="small" variant="text" @click="clearSelection">{{ t('music.local.cancelSelect') }}</t-button>
    </div>

    <!-- 歌曲列表 -->
    <div v-if="loading" class="loading-container">
      <div class="loading-content">
        <div class="loading-spinner"></div>
        <p>{{ t('common.loading') }}</p>
      </div>
    </div>

    <div v-else-if="displayTracks.length > 0" class="list">
      <div class="list-header">
        <span class="col-check">
          <t-checkbox :checked="isAllSelected" @change="toggleSelectAll" />
        </span>
        <span class="col-cover"></span>
        <div class="col-info">
          <span class="col-name">{{ t('music.local.colSong') }}</span>
          <span class="col-singer">{{ t('music.local.colArtist') }}</span>
        </div>
        <span class="col-album">{{ t('music.local.colAlbum') }}</span>
        <span class="col-duration">{{ t('music.local.colDuration') }}</span>
        <span class="col-size">{{ t('music.local.colSize') }}</span>
        <span class="col-actions-header">{{ t('music.local.colActions') }}</span>
      </div>
      <div v-bind="containerProps" class="list-body">
        <div v-bind="wrapperProps">
          <div
            v-for="{ data: track } in virtualTracks"
            :key="track.songmid"
            class="row"
            :class="{ 'is-selected': selectedIds.has(track.songmid), 'multi-select-active': multiSelect }"
            @click="multiSelect ? toggleSelect(track.songmid) : handlePlay(track)"
            @contextmenu="handleContextMenu($event, track)"
            @touchstart.passive="handleTouchStart(track)"
            @touchend="handleTouchCancel"
            @touchmove.passive="handleTouchCancel"
          >
            <div class="col-check" @click.stop>
              <t-checkbox :checked="selectedIds.has(track.songmid)" @change="toggleSelect(track.songmid)" />
            </div>
            <div class="col-cover">
              <img
                v-if="coverCache[track.songmid]"
                :src="coverCache[track.songmid]"
                class="track-cover"
                loading="lazy"
              />
              <img v-else src="/default-cover.png" class="track-cover" loading="lazy" />
            </div>
            <div class="col-info">
              <span class="name-text" :class="{ playing: track.songmid === currentSongId }">
                <span v-if="track.songmid === currentSongId" class="playing-eq"><i></i><i></i><i></i></span>
                {{ track.name }}
              </span>
              <span class="col-singer">{{ track.singer || t('common.unknown') }}</span>
            </div>
            <span class="col-album">{{ track.albumName || t('common.unknownAlbum') }}</span>
            <span class="col-duration">{{ formatDuration(track.duration || track.interval) }}</span>
            <span class="col-size">{{ formatSize(track.size) }}</span>
            <div class="col-actions" @click.stop>
              <t-button variant="text" size="small" @click="openTagEditor(track)">{{ t('music.local.edit') }}</t-button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="empty">
      <div class="empty-card" :class="{ 'is-search-empty': !!searchQuery }">
        <div class="empty-visual">
          <FolderIcon v-if="!searchQuery" size="44px" :stroke-width="1.5" />
          <SearchIcon v-else size="44px" :stroke-width="1.5" />
        </div>
        <div class="empty-copy">
          <h3>{{ searchQuery ? t('music.local.noMatchMusic') : t('music.local.emptyTitle') }}</h3>
          <p>{{ searchQuery ? t('music.local.emptySearchHint') : t('music.local.noMusicDir') }}</p>
        </div>
        <div v-if="!searchQuery" class="empty-actions">
          <t-button theme="primary" size="large" class="empty-action" @click="showDirModal = true">
            <template #icon><FolderIcon /></template>
            {{ t('music.local.selectDir') }}
          </t-button>
          <span class="empty-tip">{{ t('music.local.emptyScanTip') }}</span>
        </div>
      </div>
    </div>

    <!-- 目录管理弹窗 -->
    <t-dialog
      v-model:visible="showDirModal"
      :header="t('music.local.selectDirDialog')"
      placement="center"
      width="500px"
      :footer="false"
    >
      <div class="dir-modal-content">
        <div class="dir-hint">{{ t('music.local.selectDirTip') }}</div>
        <div v-if="scanDirs.length === 0" class="dir-empty">
          <div class="dir-empty-icon"><FolderIcon size="28px" :stroke-width="1.5" /></div>
          <div>
            <strong>{{ t('music.local.dirEmptyTitle') }}</strong>
            <p>{{ t('music.local.dirEmptyHint') }}</p>
          </div>
        </div>
        <div v-for="d in scanDirs" :key="d" class="dir-row">
          <span class="dir-path">{{ d }}</span>
          <t-button size="small" variant="text" theme="danger" @click="removeDir(d)">{{ t('common.delete') }}</t-button>
        </div>
        <div class="dir-actions">
          <t-button block variant="outline" @click="selectDirs">{{ t('common.addFolder') }}</t-button>
          <t-button block theme="primary" @click="saveDirs">{{ t('common.confirm') }}</t-button>
        </div>
      </div>
    </t-dialog>

    <!-- 右键菜单 -->
    <Teleport to="body">
      <div
        v-if="contextMenuVisible"
        class="context-menu"
        :style="{ top: contextMenuPos.top + 'px', left: contextMenuPos.left + 'px' }"
        @click.stop
      >
        <div class="menu-item" @click="handleMenuAction('play')">
          <PlayCircleIcon size="14px" /> {{ t('music.local.contextPlay') }}
        </div>
        <div class="menu-item" @click="handleMenuAction('addToEnd')">
          <i class="iconfont icon-zengjia" style="font-size:14px"></i> {{ t('music.local.contextJoinPlaylist') }}
        </div>
        <div class="menu-item" @click="handleMenuAction('addToList')">
          <FolderIcon size="14px" /> {{ t('music.local.contextAddToLocal') }}
        </div>
        <div class="menu-separator"></div>
        <div class="menu-item" @click="handleMenuAction('editTags')">
          <i class="iconfont icon-bianji" style="font-size:14px"></i> {{ t('music.local.contextEditTag') }}
        </div>
      </div>
    </Teleport>

    <AddToPlaylistDialog
      v-model:visible="showAddToPlaylist"
      :songs="songsToAdd"
    />
  </div>
</template>

<style scoped>
.local-container {
  padding: 0 2rem;
  padding-top: 1rem;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

/* 头部 */
.local-header {
  margin-bottom: 1rem;
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
}

.local-header .title {
  border-left: 8px solid var(--td-brand-color-3);
  padding-left: 12px;
  border-radius: 8px;
  line-height: 1.5em;
  font-size: 28px;
  font-weight: 900;
  color: var(--td-text-color-primary);
  margin: 0;
}

.local-header .title span {
  padding-left: 8px;
  font-size: 18px;
}

/* 头部选择目录入口 */
.select-dir-trigger {
  min-height: 40px;
  font-weight: 700;
  box-shadow: 0 8px 18px color-mix(in srgb, var(--td-brand-color) 18%, transparent);
}
/* 控制栏 */
.controls {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}

.local-btn {
  padding: 6px 9px;
  border-radius: 8px;
  height: 36px;
}

/* 批量操作栏 */
.batch-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  margin-bottom: 12px;
  background: var(--td-brand-color-light);
  border-radius: 8px;
  flex-shrink: 0;
}

.batch-info {
  font-size: 13px;
  color: var(--td-brand-color);
  font-weight: 500;
  margin-right: 4px;
}

/* 加载状态 */
.loading-container {
  display: flex;
  justify-content: center;
  align-items: center;
  flex: 1;
}

.loading-content {
  text-align: center;
}

.loading-spinner {
  width: 40px;
  height: 40px;
  border: 4px solid var(--td-bg-color-component-hover);
  border-top: 4px solid var(--td-brand-color);
  border-radius: 50%;
  will-change: transform; animation: spin 1s linear infinite;
  margin: 0 auto 16px;
}

.loading-content p {
  font-size: 14px;
  color: var(--td-text-color-secondary);
  margin: 0;
}

/* 歌曲列表 */
.list {
  margin-top: 0;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 0;
  overflow: hidden;
}

.list-header {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 600;
  color: var(--td-text-color-secondary);
  border-bottom: 1px solid var(--td-border-level-1-color);
  position: sticky;
  top: 0;
  z-index: 2;
  background: var(--td-bg-color-container);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  flex-shrink: 0;
}

.list-body {
  flex: 1;
  overflow-y: auto;
}

.row {
  display: flex;
  align-items: center;
  min-height: 60px;
  padding: 10px 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard);
  box-sizing: border-box;
}

.row:hover {
  background: var(--td-bg-color-component-hover);
}

.row.is-selected {
  background: var(--td-brand-color-light);
}

/* 列宽 */
.col-check {
  width: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.col-cover {
  width: 44px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.track-cover {
  width: 40px;
  height: 40px;
  border-radius: 4px;
  object-fit: cover;
  background: var(--td-bg-color-component-hover);
}

.col-info {
  flex: 3.5;
  min-width: 0;
  display: flex;
  align-items: center;
}

.col-info .col-name {
  flex: 2;
  min-width: 0;
}

.col-info > .name-text {
  flex: 2;
  min-width: 0;
}

.col-info .col-singer {
  flex: 1.5;
  min-width: 0;
}

.col-name {
  flex: 2;
  min-width: 0;
  font-size: 14px;
}

.name-text {
  color: var(--td-text-color-primary);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  display: block;
}

.name-text.playing {
  color: var(--td-brand-color);
}

.col-singer {
  flex: 1.5;
  font-size: 13px;
  color: var(--td-text-color-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.col-album {
  flex: 1.5;
  font-size: 13px;
  color: var(--td-text-color-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.col-duration {
  width: 60px;
  font-size: 12px;
  color: var(--td-text-color-secondary);
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

.col-size {
  width: 80px;
  font-size: 12px;
  color: var(--td-text-color-secondary);
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

.col-actions {
  width: 60px;
  flex-shrink: 0;
}

.col-actions-header {
  width: 60px;
  flex-shrink: 0;
}

/* 空状态 */
.empty {
  padding: 24px;
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--td-text-color-placeholder);
}

.empty-card {
  width: min(100%, 520px);
  padding: 36px 32px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
  text-align: center;
  border: 1px solid color-mix(in srgb, var(--td-brand-color) 16%, var(--td-border-level-1-color));
  border-radius: 24px;
  background:
    radial-gradient(circle at top, color-mix(in srgb, var(--td-brand-color) 12%, transparent), transparent 58%),
    color-mix(in srgb, var(--td-bg-color-container) 88%, transparent);
  box-shadow: 0 18px 48px color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
}

.empty-card.is-search-empty {
  border-color: var(--td-border-level-1-color);
  background: var(--td-bg-color-container);
}

.empty-visual {
  width: 88px;
  height: 88px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 28px;
  color: var(--td-brand-color);
  background: color-mix(in srgb, var(--td-brand-color) 12%, transparent);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--td-brand-color) 18%, transparent);
}

.empty-copy h3 {
  margin: 0 0 8px;
  color: var(--td-text-color-primary);
  font-size: 20px;
  line-height: 1.35;
  font-weight: 800;
}

.empty-copy p {
  margin: 0;
  color: var(--td-text-color-secondary);
  font-size: 14px;
  line-height: 1.7;
  max-width: 360px;
}

.empty-actions {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}

.empty-action {
  min-height: 44px;
  border-radius: 999px;
  padding: 0 28px;
  font-weight: 700;
}

.empty-tip {
  color: var(--td-text-color-placeholder);
  font-size: 12px;
  line-height: 1.5;
}

/* 目录弹窗 */
.dir-modal-content {
  padding: 0;
}

.dir-hint {
  margin-bottom: 10px;
  color: var(--td-text-color-secondary);
  font-size: 12px;
}

.dir-empty {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  border: 1px dashed var(--td-border-level-2-color);
  border-radius: 14px;
  background: var(--td-bg-color-container-hover);
}

.dir-empty-icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border-radius: 16px;
  color: var(--td-brand-color);
  background: color-mix(in srgb, var(--td-brand-color) 12%, transparent);
}

.dir-empty strong {
  display: block;
  color: var(--td-text-color-primary);
  font-size: 14px;
  line-height: 1.4;
  margin-bottom: 4px;
}

.dir-empty p {
  margin: 0;
  color: var(--td-text-color-secondary);
  font-size: 12px;
  line-height: 1.6;
}
.dir-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid var(--td-border-level-1-color);
}

.dir-path {
  font-size: 13px;
  color: var(--td-text-color-primary);
  word-break: break-all;
  flex: 1;
  margin-right: 12px;
}

.dir-actions {
  display: flex;
  gap: 12px;
  margin-top: 20px;
}

.dir-actions .t-button {
  flex: 1;
}

@keyframes spin { to { transform: rotate(360deg); } }

/* 播放中均衡器 */
.playing-eq {
  display: inline-flex;
  align-items: flex-end;
  gap: 2px;
  height: 14px;
  margin-right: 5px;
  vertical-align: middle;
}

.playing-eq i {
  width: 3px;
  border-radius: 1px;
  background: var(--td-brand-color);
  animation: eq-bar 1.2s ease-in-out infinite;
}

.playing-eq i:nth-child(1) { height: 60%; animation-delay: 0s; }
.playing-eq i:nth-child(2) { height: 100%; animation-delay: 0.2s; }
.playing-eq i:nth-child(3) { height: 40%; animation-delay: 0.4s; }

@keyframes eq-bar {
  0%, 100% { transform: scaleY(0.4); }
  50% { transform: scaleY(1); }
}

/* 仅移动端显示 */
.mobile-only { display: none; }

/* 移动端搜索过渡 */
.search-slide-enter-active,
.search-slide-leave-active {
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.search-slide-enter-from,
.search-slide-leave-to {
  opacity: 0;
  max-height: 0;
  margin-bottom: 0;
}

.search-slide-enter-to,
.search-slide-leave-from {
  opacity: 1;
  max-height: 60px;
  margin-bottom: 8px;
}

/* 响应式 */
@media (max-width: 768px) {
  .mobile-only { display: inline-flex; }
  .desktop-search { display: none !important; }

  .local-container {
    padding: var(--mobile-page-top-gutter) var(--mobile-page-gutter) 0;
    overflow: hidden;
  }

  .local-header {
    align-items: flex-start;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .local-header .title {
    border-left: none;
    padding-left: 0;
    font-size: clamp(1.75rem, 8vw, 2.4rem);
    line-height: 1.1;
    letter-spacing: -0.04em;
  }

  .local-header .title span {
    display: block;
    padding: 0.3rem 0 0;
    font-size: 0.875rem !important;
    font-weight: 400;
    letter-spacing: 0;
    opacity: 0.5;
  }

  .right-container :deep(.t-button) {
    min-height: var(--mobile-touch-target);
  }

  /* 控制栏 */
  .controls {
    flex-wrap: nowrap;
    gap: 6px;
    margin-bottom: 8px;
  }

  .controls .play-all {
    flex: 1;
    min-height: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    font-weight: 600;
  }

  .controls .scan,
  .controls .more,
  .controls .search-toggle {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    width: var(--mobile-touch-target);
    border-radius: 50%;
    padding: 0;
    touch-action: manipulation;
    flex-shrink: 0;
  }

  .controls .search-toggle.active {
    background: var(--td-brand-color-light);
    color: var(--td-brand-color);
  }

  /* 移动端搜索栏 */
  .mobile-search-bar {
    margin-bottom: 8px;
  }

  .mobile-search-bar :deep(.t-input) {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
  }

  /* 隐藏表头 */
  .list-header {
    display: none;
  }

  /* 歌曲列表 - 卡片式 */
  .list-body {
    -webkit-overflow-scrolling: touch;
  }

  .row {
    padding: 10px 4px;
    border-radius: 12px;
    min-height: 64px;
    touch-action: manipulation;
    gap: 4px;
    -webkit-tap-highlight-color: transparent;
  }

  .row:active:not(.is-selected) {
    background: var(--td-bg-color-component-hover);
    transform: scale(0.985);
    transition: transform 0.1s ease, background-color 0.1s ease;
  }

  .row.is-selected {
    background: color-mix(in srgb, var(--td-brand-color) 8%, transparent);
  }

  /* 多选模式才显示勾选框 */
  .col-check {
    width: 0;
    overflow: hidden;
    padding: 0;
    opacity: 0;
    transition: width 0.2s ease, opacity 0.2s ease;
  }

  .row.multi-select-active .col-check {
    width: 40px;
    opacity: 1;
  }

  .col-cover {
    width: 52px;
    flex-shrink: 0;
  }

  .track-cover {
    width: 48px;
    height: 48px;
    border-radius: 10px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }

  /* 歌曲信息 - 堆叠布局 */
  .col-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 0 8px;
  }

  .col-info .col-name {
    flex: none;
    min-width: 0;
  }

  .col-info .col-singer {
    flex: none;
    font-size: 13px;
    color: var(--td-text-color-placeholder);
  }

  .name-text {
    font-size: 15px;
  }

  .playing-eq {
    height: 12px;
  }

  .playing-eq i {
    width: 2.5px;
  }

  .col-album,
  .col-size {
    display: none;
  }

  .col-duration {
    width: auto;
    flex-shrink: 0;
    font-size: 12px;
    color: var(--td-text-color-placeholder);
    align-self: center;
    margin-left: auto;
    padding-right: 4px;
  }

  .col-actions,
  .col-actions-header {
    display: none;
  }

  .empty {
    padding: 24px 0;
  }

  .empty-card {
    padding: 28px 20px;
    gap: 18px;
    border-radius: var(--mobile-card-radius, 20px);
  }

  .empty-visual {
    width: 76px;
    height: 76px;
    border-radius: 24px;
  }

  .empty-copy h3 {
    font-size: 18px;
  }

  .empty-copy p {
    font-size: 14px;
  }

  .empty-action {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    padding: 0 28px;
  }

  /* 批量操作栏 */
  .batch-toolbar {
    flex-wrap: wrap;
    border-radius: var(--mobile-card-radius-small);
    gap: 6px;
  }

  .batch-toolbar :deep(.t-button) {
    min-height: 36px;
  }

  /* 目录弹窗适配 */
  .dir-modal-content {
    max-height: 60vh;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }
}
</style>

<style>
/* 右键菜单 (unscoped) */
.context-menu {
  position: fixed;
  z-index: 9999;
  min-width: 156px;
  background: var(--td-bg-color-container);
  border-radius: 10px;
  box-shadow: var(--glass-shadow-control), 0 0 0 1px var(--td-border-level-1-color);
  padding: 4px;
  animation: menuIn var(--motion-duration-quick) var(--motion-ease-out);
}

.context-menu .menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: 13px;
  color: var(--td-text-color-primary);
  border-radius: 6px;
  cursor: pointer;
  transition: background-color var(--motion-duration-instant) var(--motion-ease-standard);
}

.context-menu .menu-item:hover {
  background: var(--td-bg-color-component-hover);
}

.context-menu .menu-separator {
  height: 1px;
  background: var(--td-border-level-1-color);
  margin: 4px 8px;
}

@keyframes menuIn {
  from { opacity: 0; transform: translateY(-4px) scale(0.96); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
</style>
