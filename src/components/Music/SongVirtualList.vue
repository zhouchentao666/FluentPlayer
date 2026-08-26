<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, toRaw } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import {
  DownloadIcon,
  DeleteIcon,
  HeartIcon,
  ArrowUpIcon,
  ArrowDownIcon,
  TimeIcon,
  SwapIcon
} from 'tdesign-icons-vue-next'
import { MessagePlugin } from 'tdesign-vue-next'
import type { MusicItem } from '@/services/musicSdk'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { useSourceAccess } from '@/composables/useSourceAccess'
import { getQualityDisplayName, compareQuality } from '@/utils/quality'
import { useRouter } from 'vue-router'

interface Props {
  songs: MusicItem[]
  currentSongId?: string | number | null
  isPlaying?: boolean
  showIndex?: boolean
  showAlbum?: boolean
  showDuration?: boolean
  isLocalPlaylist?: boolean
  enableSort?: boolean
  playlistId?: string
  multiSelect?: boolean
  enableDownload?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  currentSongId: null,
  isPlaying: false,
  showIndex: true,
  showAlbum: true,
  showDuration: true,
  isLocalPlaylist: false,
  enableSort: false,
  playlistId: '',
  multiSelect: false,
  enableDownload: true
})

const emit = defineEmits([
  'play',
  'addToPlaylist',
  'download',
  'downloadBatch',
  'playBatch',
  'scroll',
  'removeFromPlaylist',
  'removeBatch',
  'exitMultiSelect',
  'addToSongListBatch',
  'moveToPosition'
])

const router = useRouter()
const { t } = useI18n()
const { getSourceName } = useSourceAccess()

function handleSingerClick(song: MusicItem) {
  if (!song.singerId || !song.source || song.source === 'local') return
  router.push({ name: 'singer', params: { id: song.singerId }, query: { source: song.source } })
}
const localUserStore = LocalUserDetailStore()

const getLocalQualityLabel = (song: MusicItem): string => {
  if (song.source !== 'local') return ''
  if ((song as any).sampleRate && (song as any).sampleRate > 48000) return 'Hi-Res'
  if (song.hash) {
    const ext = song.hash.split('.').pop()?.toLowerCase()
    if (ext === 'flac' || ext === 'wav' || ext === 'ape' || ext === 'dsd' || ext === 'dff') return t('music.songList.flacLossless')
  }
  return ''
}

const getFilteredQualityLabel = (song: MusicItem): string => {
  if (!song.types?.length) return ''
  const pluginQualities = localUserStore.userInfo.supportedSources?.[song.source]?.qualitys
  if (!pluginQualities?.length) return getQualityDisplayName(song.types[0])
  const intersection = song.types.filter(t => pluginQualities.includes(t))
  if (!intersection.length) return ''
  intersection.sort(compareQuality)
  return getQualityDisplayName(intersection[0])
}

// --- Sorting (must be before virtualizerOptions which reads sortedSongs) ---
type SortType =
  | 'default'
  | 'title_asc' | 'title_desc'
  | 'artist_asc' | 'artist_desc'
  | 'album_asc' | 'album_desc'
  | 'duration_asc' | 'duration_desc'

const sortType = ref<SortType>('default')
const hoveredHeader = ref<'title' | 'album' | 'duration' | null>(null)

const isTitleSortActive = computed(() =>
  ['title_asc', 'title_desc', 'artist_asc', 'artist_desc'].includes(sortType.value)
)
const isAlbumSortActive = computed(() =>
  ['album_asc', 'album_desc'].includes(sortType.value)
)
const isDurationSortActive = computed(() =>
  ['duration_asc', 'duration_desc'].includes(sortType.value)
)

const parseDuration = (interval: string | number): number => {
  if (typeof interval === 'number') return interval
  if (!interval) return 0
  if (typeof interval === 'string' && interval.includes(':')) {
    const parts = interval.split(':')
    return parseInt(parts[0]) * 60 + parseInt(parts[1])
  }
  return parseInt(String(interval)) || 0
}

const handleSort = (column: 'title' | 'album' | 'duration') => {
  if (column === 'title') {
    if (sortType.value === 'title_asc') sortType.value = 'title_desc'
    else if (sortType.value === 'title_desc') sortType.value = 'artist_asc'
    else if (sortType.value === 'artist_asc') sortType.value = 'artist_desc'
    else if (sortType.value === 'artist_desc') sortType.value = 'default'
    else sortType.value = 'title_asc'
  } else if (column === 'album') {
    if (sortType.value === 'album_asc') sortType.value = 'album_desc'
    else if (sortType.value === 'album_desc') sortType.value = 'default'
    else sortType.value = 'album_asc'
  } else if (column === 'duration') {
    if (sortType.value === 'duration_asc') sortType.value = 'duration_desc'
    else if (sortType.value === 'duration_desc') sortType.value = 'default'
    else sortType.value = 'duration_asc'
  }
}

const SORT_CLEAN_RE = /[^\w\s一-龥]/g
const cleanStr = (s: string | undefined) => (s || '').replace(SORT_CLEAN_RE, '').trim()
const charPriority = (s: string) => {
  if (!s) return 4
  const f = s.charAt(0)
  if (f >= '0' && f <= '9') return 0
  if (f >= 'A' && f <= 'Z') return 1
  if (f >= 'a' && f <= 'z') return 2
  return 3
}
const customCompare = (a: string | undefined, b: string | undefined, asc: boolean) => {
  const c1 = cleanStr(a); const c2 = cleanStr(b)
  const p1 = charPriority(c1); const p2 = charPriority(c2)
  if (p1 !== p2) return asc ? p1 - p2 : p2 - p1
  return asc ? c1.localeCompare(c2, 'zh-CN') : c2.localeCompare(c1, 'zh-CN')
}

const sortedSongs = computed(() => {
  if (!props.enableSort || sortType.value === 'default') return props.songs
  const list = [...props.songs]
  list.sort((a, b) => {
    switch (sortType.value) {
      case 'title_asc': return customCompare(a.name, b.name, true)
      case 'title_desc': return customCompare(a.name, b.name, false)
      case 'artist_asc': return customCompare(a.singer, b.singer, true)
      case 'artist_desc': return customCompare(a.singer, b.singer, false)
      case 'album_asc': return customCompare(a.albumName, b.albumName, true)
      case 'album_desc': return customCompare(a.albumName, b.albumName, false)
      case 'duration_asc': return parseDuration(a.interval) - parseDuration(b.interval)
      case 'duration_desc': return parseDuration(b.interval) - parseDuration(a.interval)
      default: return 0
    }
  })
  return list
})

// --- Virtual scrolling ---
const scrollContainerRef = ref<HTMLElement>()
const itemHeight = 64

const virtualizerOptions = computed(() => ({
  count: sortedSongs.value.length,
  getScrollElement: () => scrollContainerRef.value ?? null,
  estimateSize: () => itemHeight,
  overscan: 10
}))

const virtualizer = useVirtualizer(virtualizerOptions)

const virtualItems = computed(() => virtualizer.value.getVirtualItems())
const totalSize = computed(() => virtualizer.value.getTotalSize())

// 将虚拟行与歌曲数据绑定，避免模板中重复调用 getSong
const virtualRows = computed(() =>
  virtualItems.value.map(vi => ({
    ...vi,
    song: sortedSongs.value[vi.index],
    songId: String(sortedSongs.value[vi.index]?.songmid ?? ''),
    isLiked: likedSet.value.has(sortedSongs.value[vi.index]?.songmid),
    qualityLabel: getFilteredQualityLabel(sortedSongs.value[vi.index]),
    localQualityLabel: getLocalQualityLabel(sortedSongs.value[vi.index])
  }))
)
const hasScroll = computed(() =>
  !!(scrollContainerRef.value && scrollContainerRef.value.scrollHeight > scrollContainerRef.value.clientHeight)
)

// --- Multi-select ---
const isMultiSelect = computed(() => props.multiSelect)
const selectedSet = ref<Set<string | number>>(new Set())
const selectedSongs = computed(() =>
  props.songs.filter(s => selectedSet.value.has(s.songmid))
)
const selectedCount = computed(() => selectedSongs.value.length)
const selectedNonLocalCount = computed(() =>
  selectedSongs.value.filter(s => s.source !== 'local').length
)
const isAllSelected = computed(() =>
  selectedSet.value.size > 0 && selectedSet.value.size === props.songs.length
)

const toggleSelect = (song: MusicItem) => {
  if (selectedSet.value.has(song.songmid)) selectedSet.value.delete(song.songmid)
  else selectedSet.value.add(song.songmid)
}
const toggleSelectAll = () => {
  if (isAllSelected.value) selectedSet.value.clear()
  else selectedSet.value = new Set(props.songs.map(s => s.songmid))
}
const playSelected = () => {
  if (selectedSongs.value.length === 0) return
  emit('playBatch', selectedSongs.value)
}
const downloadSelected = () => {
  const nonLocal = selectedSongs.value.filter(s => s.source !== 'local')
  if (nonLocal.length === 0) { MessagePlugin.warning(t('music.songList.noDownloadableSongs')); return }
  emit('downloadBatch', nonLocal)
}
const removeSelected = () => {
  if (selectedSongs.value.length === 0) return
  emit('removeBatch', selectedSongs.value)
  const removeIds = new Set(selectedSongs.value.map(s => s.songmid))
  selectedSet.value = new Set([...selectedSet.value].filter(id => !removeIds.has(id)))
}

// --- Like / favorites ---
const likedSet = ref<Set<string | number>>(new Set())
const loadFavorites = async () => {
  try {
    const favId = await localUserStore.getFavoritesId()
    if (!favId) { likedSet.value = new Set(); return }
    const rows = await localUserStore.getSongsForPlaylist(favId)
    likedSet.value = new Set((rows || []).map(r => r.songmid))
  } catch { /* ignore */ }
}
const isLiked = (song: MusicItem) => likedSet.value.has(song.songmid)
const onToggleLike = async (song: MusicItem) => {
  try {
    let favId = await localUserStore.getFavoritesId()
    if (!favId) {
      const pl = await localUserStore.createPlaylist(t('music.songList.myFavoriteMusic'), t('music.songList.collectedSongs'))
      if (!pl) return
      await localUserStore.setFavoritesId(pl.id)
      favId = pl.id
    }
    if (isLiked(song)) {
      await localUserStore.removeSongFromPlaylist(favId, String(song.songmid))
      likedSet.value.delete(song.songmid)
    } else {
      await localUserStore.addSongsToPlaylist(favId, [toRaw(song) as any])
      likedSet.value.add(song.songmid)
    }
  } catch (e) {
    console.error('Toggle like failed:', e)
    MessagePlugin.error(t('common.failed'))
  }
}

// --- Song click (click = play) ---
const handleSongClick = (song: MusicItem) => {
  if (isMultiSelect.value) { toggleSelect(song); return }
  emit('play', song)
}

// --- Context menu ---
const contextMenuVisible = ref(false)
const contextMenuPos = ref({ top: 0, left: 0 })
const contextMenuSong = ref<MusicItem | null>(null)

const handleContextMenu = (e: MouseEvent, song: MusicItem) => {
  e.preventDefault(); e.stopPropagation()
  contextMenuSong.value = song
  contextMenuPos.value = { top: e.clientY, left: e.clientX }
  contextMenuVisible.value = true
}
const closeContextMenu = () => { contextMenuVisible.value = false; contextMenuSong.value = null }
const handleMenuAction = (action: string) => {
  const song = contextMenuSong.value
  closeContextMenu()
  if (!song) return
  if (action === 'play') emit('play', song)
  else if (action === 'download') emit('download', song)
  else if (action === 'favorite') onToggleLike(song)
  else if (action === 'addToPlaylist') emit('addToPlaylist', song)
  else if (action === 'remove') emit('removeFromPlaylist', song)
  else if (action === 'moveToPosition') emit('moveToPosition', song)
}

// --- Format ---
const formatDuration = (duration: string | number | undefined) => {
  if (!duration) return '--:--'
  if (typeof duration === 'string' && duration.includes(':')) return duration
  const sec = typeof duration === 'number' ? duration : parseInt(duration)
  if (isNaN(sec)) return '--:--'
  return `${Math.floor(sec / 60)}:${(sec % 60).toString().padStart(2, '0')}`
}

// --- Expose ---
const resetSort = () => { sortType.value = 'default' }
const scrollToSong = (songmid: string | number, source?: string) => {
  const idx = sortedSongs.value.findIndex(
    s => String(s.songmid) === String(songmid) && (!source || s.source === source)
  )
  if (idx === -1 || !scrollContainerRef.value) return
  virtualizer.value.scrollToIndex(idx, { align: 'center' })
}

defineExpose({ scrollToSong, sortedSongs, sortType, resetSort })

// --- Lifecycle ---
const handleGlobalClick = () => { if (contextMenuVisible.value) closeContextMenu() }

onMounted(async () => {
  document.addEventListener('click', handleGlobalClick)
  await loadFavorites()
  window.addEventListener('playlist-updated', loadFavorites)
})

onUnmounted(() => {
  closeContextMenu()
  document.removeEventListener('click', handleGlobalClick)
  window.removeEventListener('playlist-updated', loadFavorites)
})

watch(() => props.multiSelect, (val) => { if (!val) selectedSet.value.clear() })
watch(() => props.songs, (newSongs) => {
  const ids = new Set(newSongs.map(s => s.songmid))
  selectedSet.value = new Set([...selectedSet.value].filter(id => ids.has(id)))
})
</script>

<template>
  <div class="song-virtual-list">
    <!-- 表头 -->
    <div class="list-header-container">
      <Transition name="header-fade" mode="out-in">
        <div
          v-if="!isMultiSelect"
          class="list-header"
          :style="{ marginRight: hasScroll ? '10px' : '0' }"
        >
          <div v-if="showIndex" class="col-index">#</div>
          <div
            class="col-title"
            :class="{ sortable: enableSort }"
            @mouseenter="enableSort && (hoveredHeader = 'title')"
            @mouseleave="hoveredHeader = null"
            @click="enableSort && handleSort('title')"
          >
            {{ t('music.songList.title') }}
            <div
              v-if="enableSort"
              v-show="hoveredHeader === 'title' || isTitleSortActive"
              class="sort-icon-container"
            >
              <span v-if="sortType === 'title_asc'" class="sort-icon active">
                <ArrowUpIcon class="sort-icon-arrow" /> {{ t('music.songList.titleAsc') }}
              </span>
              <span v-else-if="sortType === 'title_desc'" class="sort-icon active">
                <ArrowDownIcon class="sort-icon-arrow" /> {{ t('music.songList.titleDesc') }}
              </span>
              <span v-else-if="sortType === 'artist_asc'" class="sort-icon active">
                <ArrowUpIcon class="sort-icon-arrow" /> {{ t('music.songList.artistAsc') }}
              </span>
              <span v-else-if="sortType === 'artist_desc'" class="sort-icon active">
                <ArrowDownIcon class="sort-icon-arrow" /> {{ t('music.songList.artistDesc') }}
              </span>
              <span v-else class="sort-icon default">
                <TimeIcon class="sort-icon-arrow" /> {{ t('music.songList.defaultSort') }}
              </span>
            </div>
          </div>
          <div
            v-if="showAlbum"
            class="col-album"
            :class="{ sortable: enableSort }"
            @mouseenter="enableSort && (hoveredHeader = 'album')"
            @mouseleave="hoveredHeader = null"
            @click="enableSort && handleSort('album')"
          >
            {{ t('music.songList.album') }}
            <div v-if="enableSort" v-show="hoveredHeader === 'album' || isAlbumSortActive" class="sort-icon-container">
              <span v-if="sortType === 'album_asc'" class="sort-icon active"><ArrowUpIcon /></span>
              <span v-else-if="sortType === 'album_desc'" class="sort-icon active"><ArrowDownIcon /></span>
              <span v-else class="sort-icon default"><TimeIcon class="sort-icon-arrow" /> {{ t('music.songList.defaultSort') }}</span>
            </div>
          </div>
          <div class="col-like">{{ t('music.songList.liked') }}</div>
          <div
            v-if="showDuration"
            class="col-duration"
            :class="{ sortable: enableSort }"
            @mouseenter="enableSort && (hoveredHeader = 'duration')"
            @mouseleave="hoveredHeader = null"
            @click="enableSort && handleSort('duration')"
          >
            {{ t('music.songList.duration') }}
            <div v-if="enableSort" v-show="hoveredHeader === 'duration' || isDurationSortActive" class="sort-icon-container">
              <span v-if="sortType === 'duration_asc'" class="sort-icon active"><ArrowUpIcon /></span>
              <span v-else-if="sortType === 'duration_desc'" class="sort-icon active"><ArrowDownIcon /></span>
              <span v-else class="sort-icon default"><TimeIcon /></span>
            </div>
          </div>
        </div>

        <!-- 多选表头 -->
        <div v-else class="list-header multi">
          <div class="multi-left">
            <div class="select-all">
              <t-checkbox
                :checked="isAllSelected"
                @change="toggleSelectAll"
              >
                {{ t('music.songList.selectAll') }}
              </t-checkbox>
            </div>
            <t-button
              class="square-btn"
              theme="primary"
              size="small"
              :disabled="selectedCount === 0"
              @click="playSelected"
            >
              <template #icon>
                <i class="iconfont icon-bofang"></i>
              </template>
            </t-button>
            <t-button
              v-if="enableDownload"
              class="action-btn-compact"
              theme="default"
              size="small"
              :disabled="selectedNonLocalCount === 0"
              @click="downloadSelected"
            >
              {{ t('music.songList.batchDownload') }}
            </t-button>
          </div>
          <div class="multi-right">
            <span class="selected-info">{{ t('music.songList.selectedCount', { count: selectedCount }) }}</span>
            <t-button
              v-if="isLocalPlaylist"
              class="action-btn-compact"
              theme="danger"
              size="small"
              variant="outline"
              :disabled="selectedCount === 0"
              @click="removeSelected"
            >
              <template #icon><DeleteIcon /></template>
            </t-button>
            <t-button
              class="action-btn-compact"
              theme="default"
              size="small"
              variant="outline"
              @click="emit('exitMultiSelect')"
            >
              {{ t('music.songList.done') }}
            </t-button>
          </div>
        </div>
      </Transition>
    </div>

    <!-- 虚拟滚动区域 -->
    <div
      ref="scrollContainerRef"
      class="virtual-scroll-container"
      @scroll.passive="(e) => emit('scroll', e)"
    >
      <div :style="{ height: totalSize + 'px', position: 'relative', width: '100%' }">
        <div
          :style="{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            transform: `translateY(${virtualItems.length > 0 ? virtualItems[0].start : 0}px)`,
            willChange: 'transform'
          }"
        >
          <div
            v-for="row in virtualRows"
            :key="String(row.key)"
            :data-index="row.index"
            class="song-item"
            :class="{
              'is-playing': row.songId === String(currentSongId)
            }"
            @contextmenu="handleContextMenu($event, row.song!)"
          >
            <!-- 序号列 -->
            <div v-if="showIndex" class="col-index">
              <template v-if="!isMultiSelect">
                <span class="track-number">
                  {{ String(row.index + 1).padStart(2, '0') }}
                </span>
                <button
                  class="play-btn-overlay"
                  :title="t('music.songList.play')"
                  @click.stop="emit('play', row.song!)"
                >
                  <i class="iconfont icon-bofang"></i>
                </button>
              </template>
              <t-checkbox
                v-else
                class="select-checkbox"
                :class="{ 'always-show': isMultiSelect }"
                :checked="selectedSet.has(row.songId)"
                @change="(checked: boolean) => {
                  if (checked) selectedSet.add(row.song!.songmid)
                  else selectedSet.delete(row.song!.songmid)
                }"
              />
            </div>

            <!-- 歌曲信息 -->
            <div class="col-title" @click="handleSongClick(row.song!)">
              <div v-if="row.song?.img" class="song-cover">
                <img :src="row.song.img" alt="" loading="lazy" />
              </div>
              <div class="song-info">
                <div class="song-title" :title="row.song?.name">
                  {{ row.song?.name }}
                </div>
                <div class="song-artist" :title="row.song?.singer">
                  <span
                    v-if="row.song?.types?.length && row.qualityLabel"
                    class="quality-tag"
                  >
                    {{ row.qualityLabel }}
                  </span>
                  <span
                    v-else-if="row.localQualityLabel"
                    class="quality-tag"
                  >
                    {{ row.localQualityLabel }}
                  </span>
                  <span
                    v-if="row.song?.source && row.song?.source !== 'local'"
                    class="source-tag"
                  >
                    {{ getSourceName(row.song.source) }}
                  </span>
                  <span
                    v-if="row.song?.singerId && row.song?.source !== 'local'"
                    class="singer-link"
                    @click.stop="handleSingerClick(row.song!)"
                  >{{ row.song?.singer }}</span>
                  <template v-else>{{ row.song?.singer }}</template>
                </div>
              </div>
            </div>

            <!-- 专辑 -->
            <div v-if="showAlbum" class="col-album">
              <span class="album-name" :title="row.song?.albumName">
                {{ row.song?.albumName || '-' }}
              </span>
            </div>

            <!-- 喜欢 -->
            <div class="col-like">
              <button
                class="like-btn"
                :title="t('music.songList.liked')"
                @click.stop="onToggleLike(row.song!)"
              >
                <HeartIcon
                  :fill-color="row.isLiked ? 'var(--td-error-color)' : ''"
                  :stroke-color="row.isLiked ? [] : ['currentColor']"
                  :stroke-width="row.isLiked ? 0 : 2"
                  size="18"
                />
              </button>
            </div>

            <!-- 时长 / hover操作 — CSS-driven via .is-hovered -->
            <div v-if="showDuration" class="col-duration">
              <div class="duration-wrapper">
                <span class="duration">
                  {{ formatDuration(row.song?.interval) }}
                </span>
                <div class="action-buttons">
                  <button
                    v-if="enableDownload && row.song?.source !== 'local'"
                    class="action-btn-small"
                    :title="t('music.songList.download')"
                    @click.stop="emit('download', row.song!)"
                  >
                    <DownloadIcon size="16" />
                  </button>
                  <button
                    class="action-btn-small"
                    :title="t('music.songList.addToPlaylist')"
                    @click.stop="emit('addToPlaylist', row.song!)"
                  >
                    <i class="iconfont icon-zengjia"></i>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 右键菜单 -->
    <Teleport v-if="contextMenuVisible" to="body">
      <div
        class="song-context-menu"
        :style="{ top: contextMenuPos.top + 'px', left: contextMenuPos.left + 'px' }"
        @click.stop
      >
        <div class="ctx-item" @click="handleMenuAction('play')">
          <i class="iconfont icon-bofang"></i> {{ t('music.songList.play') }}
        </div>
        <div class="ctx-item" @click="handleMenuAction('addToPlaylist')">
          <i class="iconfont icon-zengjia"></i> {{ t('music.songList.addToPlaylist') }}
        </div>
        <div class="ctx-item" @click="handleMenuAction('favorite')">
          <HeartIcon size="14" style="margin-right: 6px" /> {{ isLiked(contextMenuSong!) ? t('music.songList.cancelLike') : t('music.songList.liked') }}
        </div>
        <template v-if="enableDownload && contextMenuSong?.source !== 'local'">
          <div class="ctx-separator"></div>
          <div class="ctx-item" @click="handleMenuAction('download')">
            <DownloadIcon size="14" style="margin-right: 6px" /> {{ t('music.songList.download') }}
          </div>
        </template>
        <template v-if="isLocalPlaylist">
          <div class="ctx-separator"></div>
          <div class="ctx-item" @click="handleMenuAction('moveToPosition')">
            <SwapIcon size="14" style="margin-right: 6px" /> {{ t('music.songList.moveToPosition') }}
          </div>
          <div class="ctx-item danger" @click="handleMenuAction('remove')">
            <DeleteIcon size="14" style="margin-right: 6px" /> {{ t('music.songList.removeFromPlaylist') }}
          </div>
        </template>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.song-virtual-list {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
}

/* --- Header --- */
.list-header-container {
  background-color: var(--song-list-header-bg, var(--td-bg-color-container));
  border-bottom: 1px solid var(--song-list-header-border, var(--td-border-level-1-color));
  flex-shrink: 0;
}

.list-header {
  display: grid;
  grid-template-columns: 50px 1fr 200px 60px 80px;
  padding: 0 10px;
  font-size: 12px;
  color: var(--song-list-header-text, var(--td-text-color-placeholder));
  height: 40px;
  overflow: hidden;
  box-sizing: border-box;
  align-items: center;

  .sortable {
    cursor: pointer;
    user-select: none;
    &:hover { color: var(--song-list-title-hover, var(--td-text-color-primary)); }
  }

  .sort-icon-container {
    display: inline-flex;
    align-items: center;
    margin-left: 6px;

    .sort-icon {
      display: inline-flex;
      align-items: center;
      gap: 2px;
      font-size: 12px;
      color: var(--song-list-header-text, var(--td-text-color-placeholder));

      &.active { color: var(--song-list-title-hover, var(--td-brand-color)); }
      :deep(.t-icon) { font-size: 14px; }
    }
  }

  .col-index { text-align: center; display: flex; align-items: center; justify-content: center; }
  .col-title { padding-left: 10px; display: flex; align-items: center; }
  .col-album { display: flex; align-items: center; padding: 0 10px; overflow: hidden; }
  .col-like { text-align: center; display: flex; align-items: center; justify-content: center; }
  .col-duration { text-align: center; display: flex; align-items: center; justify-content: center; }
}

.list-header.multi {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 48px;
}

.multi-left, .multi-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.select-all {
  display: inline-flex;
  align-items: center;
  padding-left: 15px;
  gap: 6px;
  font-size: 12px;
  color: var(--song-list-header-text, var(--td-text-color-placeholder));
}

.selected-info {
  font-size: 12px;
  color: var(--song-list-header-text, var(--td-text-color-placeholder));
}

.square-btn {
  width: 32px; height: 32px; padding: 0;
  border-radius: 8px;
  display: inline-flex; align-items: center; justify-content: center;
}

.action-btn-compact {
  border-radius: 8px;
  height: 32px;
  padding: 8px 12px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

/* --- Virtual scroll container --- */
.virtual-scroll-container {
  background: var(--song-list-content-bg, var(--td-bg-color-container));
  overflow-y: auto;
  position: relative;
  flex: 1;
  min-height: 0;
  -webkit-overflow-scrolling: touch;
}

/* --- Song items --- */
.song-item {
  display: grid;
  grid-template-columns: 50px 1fr 200px 60px 80px;
  padding: 8px 10px;
  border-bottom: 1px solid var(--song-list-item-border, var(--td-border-level-1-color));
  cursor: pointer;
  transition: background-color 0.15s ease;
  height: 64px;
  box-sizing: border-box;
  background-color: transparent;
  contain: layout style paint;

  &:hover,
  &.is-hovered {
    background-color: var(--song-list-item-hover, var(--td-bg-color-component-hover));

    .col-title .song-info .song-title {
      color: var(--song-list-title-hover, var(--td-brand-color));
    }

    .col-duration .duration-wrapper {
      .duration { opacity: 0; }
      .action-buttons { opacity: 1; pointer-events: auto; }
    }
  }

  &.is-playing {
    background: var(--song-list-item-playing, var(--td-brand-color-light));

    .col-title .song-info .song-title {
      color: var(--song-list-btn-hover, var(--td-brand-color));
    }
  }

  /* Index column */
  .col-index {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    position: relative;

    .track-number {
      font-size: 14px;
      color: var(--song-list-track-number, var(--td-text-color-placeholder));
      font-variant-numeric: tabular-nums;
      width: 100%;
      text-align: center;
      position: absolute;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%) scale(1);
      transition: opacity 0.2s ease, transform 0.2s ease;
      opacity: 1;
    }

    .play-btn-overlay {
      background: none;
      border: none;
      cursor: pointer;
      color: var(--song-list-btn-hover, var(--td-brand-color));
      font-size: 16px;
      padding: 8px;
      border-radius: 50%;
      width: 32px;
      height: 32px;
      display: flex;
      align-items: center;
      justify-content: center;
      position: absolute;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%) scale(0.8);
      transition: opacity 0.2s ease, transform 0.2s ease;
      opacity: 0;
      pointer-events: none;

      &:hover {
        background: var(--song-list-btn-bg-hover, var(--td-brand-color-focus));
      }
    }

    .select-checkbox {
      position: absolute;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%) scale(0.8);
      transition: opacity 0.2s ease, transform 0.2s ease;
      opacity: 0;
      pointer-events: none;

      &.always-show {
        opacity: 1;
        transform: translate(-50%, -50%) scale(1);
        pointer-events: auto;
      }
    }
  }

  &:hover .col-index {
    .track-number {
      opacity: 0;
      transform: translate(-50%, -50%) scale(0.8);
    }
    .play-btn-overlay {
      opacity: 1;
      transform: translate(-50%, -50%) scale(1);
      pointer-events: auto;
    }
    .select-checkbox {
      opacity: 1;
      transform: translate(-50%, -50%) scale(1);
      pointer-events: auto;
    }
  }

  /* Title column */
  .col-title {
    display: flex;
    align-items: center;
    padding-left: 10px;
    min-width: 0;
    overflow: hidden;

    .song-cover {
      width: 40px;
      height: 40px;
      margin-right: 10px;
      border-radius: 4px;
      overflow: hidden;
      flex-shrink: 0;

      img {
        width: 100%;
        height: 100%;
        object-fit: cover;
      }
    }

    .song-info {
      min-width: 0;
      flex: 1;
      display: flex;
      flex-direction: column;
      justify-content: center;
      overflow: hidden;

      .song-title {
        font-size: 14px;
        color: var(--song-list-title-color, var(--td-text-color-primary));
        margin-bottom: 4px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        line-height: 1.2;
        transition: color 0.2s ease;
        font-weight: 500;
      }

      .song-artist {
        font-size: 12px;
        color: var(--song-list-artist-color, var(--td-text-color-secondary));
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        line-height: 1.2;
        display: flex;
        align-items: center;
        gap: 4px;

        .singer-link {
          cursor: pointer;
          &:hover { color: var(--td-brand-color); }
        }

        .source-tag {
          background: var(--song-list-source-bg, var(--td-brand-color-1));
          color: var(--song-list-source-color, var(--td-brand-color));
          padding: 2px 6px;
          border-radius: 4px;
          font-size: 10px;
          line-height: 1;
        }

        .quality-tag {
          background: var(--song-list-quality-bg, rgba(255, 165, 0, 0.15));
          color: var(--song-list-quality-color, #e6a817);
          padding: 2px 6px;
          border-radius: 4px;
          font-size: 10px;
          line-height: 1;
          font-weight: 500;
        }
      }
    }
  }

  /* Album column */
  .col-album {
    display: flex;
    align-items: center;
    padding: 0 10px;
    overflow: hidden;

    .album-name {
      font-size: 12px;
      color: var(--song-list-album-color, var(--td-text-color-secondary));
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      width: 100%;
      transition: color 0.2s ease;
    }
  }

  /* Like column */
  .col-like {
    display: flex;
    align-items: center;
    justify-content: center;

    .like-btn {
      background: none;
      border: none;
      cursor: pointer;
      color: var(--song-list-btn-color, var(--td-text-color-placeholder));
      padding: 8px;
      border-radius: 50%;
      transition: color 0.15s ease, background-color 0.15s ease;
      width: 32px;
      height: 32px;
      display: flex;
      align-items: center;
      justify-content: center;

      &:hover {
        color: var(--song-list-btn-hover, var(--td-error-color));
        background: var(--song-list-btn-bg-hover, var(--td-error-color-light));
      }
    }
  }

  /* Duration column */
  .col-duration {
    display: flex;
    align-items: center;
    justify-content: center;

    .duration-wrapper {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 100%;
      position: relative;

      .duration {
        font-size: 12px;
        color: var(--song-list-duration-color, var(--td-text-color-placeholder));
        font-variant-numeric: tabular-nums;
        min-width: 35px;
        text-align: center;
        transition: opacity 0.15s ease;
      }

      .action-buttons {
        display: flex;
        gap: 2px;
        justify-content: center;
        align-items: center;
        position: absolute;
        opacity: 0;
        pointer-events: none;
        transition: opacity 0.15s ease;

        .action-btn-small {
          background: none;
          border: none;
          cursor: pointer;
          color: var(--song-list-btn-color, var(--td-text-color-secondary));
          padding: 6px;
          border-radius: 50%;
          transition: color 0.15s ease, background-color 0.15s ease;
          width: 28px;
          height: 28px;
          display: flex;
          align-items: center;
          justify-content: center;

          &:hover {
            color: var(--song-list-btn-hover, var(--td-brand-color));
            background: var(--song-list-btn-bg-hover, var(--td-brand-color-focus));
          }

          i { display: block; line-height: 1; font-size: 14px; }
        }
      }
    }
  }
}

/* --- Header transition --- */
.header-fade-enter-active,
.header-fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.header-fade-enter-from,
.header-fade-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}

.sort-icon-arrow {
  font-size: 12px;
  margin-right: 0.2em;
}

/* --- Responsive --- */
@media (max-width: 768px) {
  .list-header {
    grid-template-columns: var(--mobile-touch-target) minmax(0, 1fr) var(--mobile-touch-target) 52px;
    min-height: var(--mobile-touch-target);
    padding: 0 4px;

    .col-album { display: none; }
    .col-title { padding-left: 4px; }
    .col-duration { font-size: 0; }
  }

  .list-header.multi {
    min-height: var(--mobile-touch-target);
    height: auto;
    padding: 6px 4px;
    gap: 8px;
  }

  .multi-left,
  .multi-right {
    min-width: 0;
    gap: 6px;
  }

  .select-all {
    padding-left: 4px;
  }

  .square-btn,
  .action-btn-compact {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
  }

  .song-item {
    grid-template-columns: var(--mobile-touch-target) minmax(0, 1fr) var(--mobile-touch-target) 52px;
    padding: 8px 4px;
    touch-action: manipulation;

    .col-album { display: none; }

    .col-title {
      padding-left: 4px;
    }

    .col-index {
      .play-btn-overlay {
        width: var(--mobile-touch-target);
        height: var(--mobile-touch-target);
        opacity: 1;
        pointer-events: auto;
        transform: translate(-50%, -50%) scale(1);
      }

      .track-number {
        opacity: 0;
      }
    }

    .col-like {
      .like-btn {
        width: var(--mobile-touch-target);
        height: var(--mobile-touch-target);
      }
    }

    .col-duration {
      .duration-wrapper {
        .duration {
          display: none;
        }

        .action-buttons {
          position: static;
          opacity: 1;
          pointer-events: auto;
          gap: 0;

          .action-btn-small {
            width: 26px;
            height: var(--mobile-touch-target);
            padding: 0;
          }
        }
      }
    }
  }
}
</style>

<style>
/* Context menu (unscoped, teleported) */
.song-context-menu {
  position: fixed;
  z-index: 9999;
  min-width: 180px;
  background: var(--td-bg-color-container);
  border-radius: 10px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15), 0 0 0 1px var(--td-border-level-1-color);
  padding: 4px;
  animation: ctxMenuIn 0.15s ease;
}

.song-context-menu .ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: 13px;
  color: var(--td-text-color-primary);
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.12s;
}
.song-context-menu .ctx-item:hover {
  background: var(--td-bg-color-component-hover);
}
.song-context-menu .ctx-item .iconfont {
  font-size: 14px;
  color: var(--td-text-color-secondary);
}
.song-context-menu .ctx-item:hover .iconfont {
  color: var(--td-brand-color);
}
.song-context-menu .ctx-item.danger {
  color: var(--td-error-color);
}
.song-context-menu .ctx-item.danger:hover {
  background: var(--td-error-color-light);
}
.song-context-menu .ctx-item.danger .iconfont {
  color: var(--td-error-color);
}
.song-context-menu .ctx-separator {
  height: 1px;
  background: var(--td-border-level-1-color);
  margin: 4px 8px;
}

@keyframes ctxMenuIn {
  from { opacity: 0; transform: translateY(-4px) scale(0.96); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
</style>
