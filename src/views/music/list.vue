<script setup lang="ts">
import { ref, onMounted, computed, onBeforeUnmount, onActivated, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { musicSdk, type MusicItem } from '@/services/musicSdk'
import { playSong } from '@/utils/audio/globaPlayList'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { downloadSingleSong } from '@/utils/audio/download'
import { fillMissingSongCovers } from '@/utils/songCover'
import { MessagePlugin } from 'tdesign-vue-next'
import { EllipsisIcon, SearchIcon, MapAimingIcon } from 'tdesign-icons-vue-next'
import AddToPlaylistDialog from '@/components/Playlist/AddToPlaylistDialog.vue'
import SongVirtualList from '@/components/Music/SongVirtualList.vue'
import { createLiquidGlassConfirm } from '@/utils/liquidGlassConfirm'

const route = useRoute()
const router = useRouter()
const playStatus = useGlobalPlayStatusStore()
const localUserStore = LocalUserDetailStore()
const { t } = useI18n()

const handleBack = () => {
  if (window.history.length > 1) {
    router.back()
  } else {
    router.push('/home')
  }
}

const isLocalPlaylist = computed(() => route.query.type === 'local')
const isLeaderboard = computed(() => route.query.isLeaderboard === 'true')
const playlistId = computed(() => route.params.id as string)

const playlistInfo = ref({
  id: '',
  title: (route.query.title as string) || t('music.list.title'),
  author: (route.query.author as string) || '',
  cover: ((route.query.cover as string) && (route.query.cover as string) !== 'default-cover') ? (route.query.cover as string) : '/default-cover.png',
  total: 0,
  source: (route.query.source as string) || '',
  desc: (route.query.description as string) || ''
})

const songs = ref<MusicItem[]>([])
const loading = ref(false)
const loadingMore = ref(false)
const hasMore = ref(true)
const currentPage = ref(1)
const totalCount = ref(0)

const songListRef = ref<InstanceType<typeof SongVirtualList> | null>(null)

const currentSongId = computed(() => localUserStore.userInfo?.lastPlaySongId)

// 搜索
const searchQuery = ref('')
const searchFocused = ref(false)
const displaySongs = computed(() => {
  const q = (searchQuery.value || '').trim().toLowerCase()
  if (!q) return songs.value
  const includes = (s?: string) => !!s && s.toLowerCase().includes(q)
  return songs.value.filter(s => includes(s.name) || includes(s.singer) || includes(s.albumName))
})

// 背景图片加载状态
const bgImageLoaded = ref(false)
const bgImageFromRoute = ref(false)

// 紧凑头部
const isHeaderCompact = ref(false)

// 封面编辑（本地歌单）
const fileInputRef = ref<HTMLInputElement | null>(null)

// 添加到歌单
const showAddToPlaylist = ref(false)
const songsToAdd = ref<any[]>([])

// 多选模式
const multiSelect = ref(false)

// 定位当前播放
const showLocateBtn = ref(false)
let locateBtnTimer: ReturnType<typeof setTimeout> | null = null

const currentPlayingSong = computed(() => {
  if (!currentSongId.value) return null
  return displaySongs.value.find(s => String(s.songmid) === String(currentSongId.value))
})

const hasCurrentPlaying = computed(() => !!currentPlayingSong.value)

function triggerLocateBtn() {
  if (!hasCurrentPlaying.value) { showLocateBtn.value = false; return }
  showLocateBtn.value = true
  clearLocateTimer()
  locateBtnTimer = setTimeout(() => { showLocateBtn.value = false }, 3000)
}

function clearLocateTimer() {
  if (locateBtnTimer) { clearTimeout(locateBtnTimer); locateBtnTimer = null }
}

function locateCurrentSong() {
  if (!currentPlayingSong.value) return
  clearLocateTimer()
  songListRef.value?.scrollToSong(currentPlayingSong.value.songmid, currentPlayingSong.value.source)
  showLocateBtn.value = false
}

// 获取歌曲
const fetchSongs = async (reset = false) => {
  if (reset) {
    if (loading.value) return
    currentPage.value = 1; songs.value = []; hasMore.value = true
    loading.value = true
  } else {
    if (loadingMore.value || !hasMore.value) return
    loadingMore.value = true
  }

  try {
    if (isLocalPlaylist.value) {
      playlistInfo.value.id = playlistId.value
      const pl = localUserStore.playlists.find(p => p.id === playlistId.value)
      if (pl) {
        const cover = pl.coverImgUrl && pl.coverImgUrl !== 'default-cover' ? pl.coverImgUrl : ''
        if (cover) playlistInfo.value.cover = cover
        if (pl.description) playlistInfo.value.desc = pl.description
      }
      const rows = await localUserStore.getSongsForPlaylist(playlistId.value)
      const parsed = (rows || []).map(r => {
        try { return JSON.parse(r.data) } catch {
          return { songmid: r.songmid, name: r.name, singer: r.singer, albumName: r.albumName, img: r.img, source: 'local', interval: '' }
        }
      })
      songs.value = parsed
      totalCount.value = parsed.length
      playlistInfo.value.total = parsed.length
      const desc = (route.query.description as string) || ''
      if (desc) playlistInfo.value.desc = desc
      hasMore.value = false
      return
    }

    const source = playlistInfo.value.source || undefined
    const res = isLeaderboard.value
      ? await musicSdk.getLeaderboardDetail(playlistId.value, currentPage.value)
      : await musicSdk.getPlaylistDetail(playlistId.value, currentPage.value, source)
    const newSongs = res?.list || []
    songs.value = reset ? newSongs : [...songs.value, ...newSongs]
    totalCount.value = res?.total || 0

    void fillMissingSongCovers(newSongs, {
      onBatchComplete: () => { songs.value = [...songs.value] }
    })
    if (res?.info) {
      playlistInfo.value.desc = res.info.desc || ''
    }
    hasMore.value = songs.value.length < (res?.total || 0)
    currentPage.value += 1
  } catch (e) { console.error('获取歌单详情失败:', e) }
  finally {
    loading.value = false
    loadingMore.value = false
  }
}

const handlePlay = (song: MusicItem) => {
  playStatus.updatePlayerInfo(song as any)
  playSong(song as any)
}

const handlePlayAll = async () => {
  if (songs.value.length === 0) return
  const confirmed = await createLiquidGlassConfirm({
    title: t('music.list.playPlaylist'),
    body: t('music.list.replaceConfirm', { title: playlistInfo.value.title, count: songs.value.length }),
    confirmText: t('music.list.confirmReplace'),
    cancelText: t('common.cancel'),
    icon: 'play'
  })
  if (!confirmed) return

  const sourceSongs = songListRef.value?.sortedSongs ?? displaySongs.value
  localUserStore.replaceSongList(sourceSongs.map(s => ({
    songmid: s.songmid, name: s.name, singer: s.singer,
    albumName: s.albumName, img: s.img, source: s.source,
    url: '', interval: s.interval
  })) as any)
  playSong(sourceSongs[0] as any)
  playStatus.updatePlayerInfo(sourceSongs[0] as any)
}

const handleShufflePlay = async () => {
  if (songs.value.length === 0) return
  const confirmed = await createLiquidGlassConfirm({
    title: t('music.list.shufflePlaylist'),
    body: t('music.list.shuffleConfirm', { title: playlistInfo.value.title, count: songs.value.length }),
    confirmText: t('music.list.confirmReplace'),
    cancelText: t('common.cancel'),
    icon: 'shuffle'
  })
  if (!confirmed) return

  const sourceSongs = [...(songListRef.value?.sortedSongs ?? songs.value)]
  for (let i = sourceSongs.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[sourceSongs[i], sourceSongs[j]] = [sourceSongs[j], sourceSongs[i]]
  }
  localUserStore.replaceSongList(sourceSongs.map(s => ({
    songmid: s.songmid, name: s.name, singer: s.singer,
    albumName: s.albumName, img: s.img, source: s.source,
    url: '', interval: s.interval
  })) as any)
  playSong(sourceSongs[0] as any)
  playStatus.updatePlayerInfo(sourceSongs[0] as any)
}

const handleDownloadAll = () => {
  if (songs.value.length === 0) return
  MessagePlugin.info(t('music.list.startDownloadCount', { count: songs.value.length }))
  songs.value.forEach(song => downloadSingleSong(song as any))
}

const handleDownloadSong = (song: MusicItem) => {
  downloadSingleSong(song as any)
}

const handleDownloadBatch = (batchSongs: MusicItem[]) => {
  if (!batchSongs.length) return
  MessagePlugin.info(t('music.list.startDownloadCount', { count: batchSongs.length }))
  batchSongs.forEach(s => downloadSingleSong(s as any))
}

const handlePlayBatch = (batchSongs: MusicItem[]) => {
  if (!batchSongs.length) return
  localUserStore.replaceSongList(batchSongs.map(s => ({
    songmid: s.songmid, name: s.name, singer: s.singer,
    albumName: s.albumName, img: s.img, source: s.source,
    url: '', interval: s.interval
  })) as any)
  playSong(batchSongs[0] as any)
  playStatus.updatePlayerInfo(batchSongs[0] as any)
  MessagePlugin.success(t('music.list.replacedPlaylist', { count: batchSongs.length }))
}

const handleRemoveBatch = async (batchSongs: MusicItem[]) => {
  if (!isLocalPlaylist.value || !batchSongs.length) return
  const mids = batchSongs.map(s => String(s.songmid))
  const ok = await localUserStore.removeSongsFromPlaylist(playlistInfo.value.id, mids)
  if (ok) {
    const removeSet = new Set(mids)
    songs.value = songs.value.filter(s => !removeSet.has(String(s.songmid)))
    playlistInfo.value.total = songs.value.length
    MessagePlugin.success(t('music.list.removedSongs', { count: mids.length }))
    multiSelect.value = false
  } else {
    MessagePlugin.error(t('music.list.batchRemoveFailed'))
  }
}

const handleRemoveFromPlaylist = async (song: MusicItem) => {
  if (!isLocalPlaylist.value) return
  try {
    const ok = await localUserStore.removeSongFromPlaylist(playlistInfo.value.id, String(song.songmid))
    if (ok) {
      songs.value = songs.value.filter(s => s.songmid !== song.songmid)
      playlistInfo.value.total = songs.value.length
      MessagePlugin.success(t('music.list.removedFromPlaylist', { name: song.name }))
    } else {
      MessagePlugin.error(t('music.list.removeFailed'))
    }
  } catch {
    MessagePlugin.error(t('music.list.removeFailed'))
  }
}

const handleAddToPlaylist = (song: MusicItem) => {
  songsToAdd.value = [song as any]
  showAddToPlaylist.value = true
}

const handleAddToSongListBatch = (batchSongs: MusicItem[], _playlist: any) => {
  if (!batchSongs.length) return
  songsToAdd.value = batchSongs as any[]
  showAddToPlaylist.value = true
}

const handleCoverClick = () => {
  if (!isLocalPlaylist.value) return
  fileInputRef.value?.click()
}

const handleFileSelect = async (event: Event) => {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return
  if (!file.type.startsWith('image/')) {
    MessagePlugin.error(t('music.list.selectImage'))
    return
  }
  if (file.size > 5 * 1024 * 1024) {
    MessagePlugin.error(t('music.list.imageTooLarge'))
    return
  }
  try {
    const reader = new FileReader()
    reader.onload = async (e) => {
      const base64Data = e.target?.result as string
      const ok = await localUserStore.updatePlaylistCover(playlistInfo.value.id, base64Data)
      if (ok) {
        playlistInfo.value.cover = base64Data
        MessagePlugin.success(t('music.list.coverUpdated'))
      } else {
        MessagePlugin.error(t('music.list.coverUpdateFailed'))
      }
    }
    reader.readAsDataURL(file)
  } catch {
    MessagePlugin.error(t('music.list.imageProcessFailed'))
  }
  target.value = ''
}

const handleScroll = (event: Event) => {
  const target = event.target as HTMLElement
  const { scrollTop, scrollHeight, clientHeight } = target

  isHeaderCompact.value = scrollTop > 100

  if (scrollHeight - scrollTop - clientHeight < 100 && !loading.value && !loadingMore.value && hasMore.value && !isLocalPlaylist.value) {
    fetchSongs()
  }

  triggerLocateBtn()
}

// 更多操作
const moreActions = computed(() => {
  const items: any[] = [
    { content: multiSelect.value ? t('common.cancelBatch') : t('common.batchSelect'), value: 'toggleMultiSelect' },
    { content: t('common.downloadAll'), value: 'downloadAll' }
  ]
  return items
})

const handleMoreAction = (value: string) => {
  if (value === 'downloadAll') handleDownloadAll()
  else if (value === 'toggleMultiSelect') multiSelect.value = !multiSelect.value
}

const handleExitMultiSelect = () => { multiSelect.value = false }

watch(playlistId, () => fetchSongs(true))

watch(hasCurrentPlaying, (v) => {
  if (v) triggerLocateBtn()
  else { showLocateBtn.value = false; clearLocateTimer() }
})

onMounted(() => {
  const rawCover = (route.query.cover as string) || ''
  const coverUrl = rawCover && rawCover !== 'default-cover' ? rawCover : ''
  if (coverUrl) {
    playlistInfo.value.cover = coverUrl
    bgImageFromRoute.value = true
    const img = new Image()
    img.onload = () => { bgImageLoaded.value = true }
    img.onerror = () => { bgImageLoaded.value = true }
    img.src = coverUrl
  } else {
    bgImageLoaded.value = true
  }

  fetchSongs(true)
})

onActivated(() => {
  if (songs.value.length === 0) fetchSongs(true)
})

onBeforeUnmount(() => {
  clearLocateTimer()
})
</script>

<template>
  <div class="list-container">
    <input
      ref="fileInputRef"
      type="file"
      accept="image/*"
      style="display: none"
      @change="handleFileSelect"
    />

    <!-- 手机端返回按钮 -->
    <button class="mobile-back-btn" type="button" :aria-label="t('common.back')" @click="handleBack">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>

    <!-- 固定头部 -->
    <div class="fixed-header" :class="{ compact: isHeaderCompact }">
      <div
        class="playlist-header"
        :class="{ compact: isHeaderCompact, 'bg-loaded': bgImageLoaded }"
        :style="{ '--header-cover': `url(${playlistInfo.cover})` }"
      >
        <div
          class="playlist-cover"
          :class="{ clickable: isLocalPlaylist }"
          @click="handleCoverClick"
        >
          <img :src="playlistInfo.cover" :alt="playlistInfo.title" />
          <div v-if="isLocalPlaylist" class="cover-overlay">
            <svg class="edit-icon" viewBox="0 0 24 24" fill="currentColor">
              <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z" />
            </svg>
            <span>{{ t('music.list.editCover') }}</span>
          </div>
        </div>

        <div class="playlist-details">
          <h1 class="playlist-title">{{ playlistInfo.title }}</h1>
          <div class="playlist-meta" :class="{ collapsed: isHeaderCompact }">
            <p class="playlist-desc">
              {{ playlistInfo.desc || 'By ' + playlistInfo.source }}
            </p>
            <p class="playlist-stats">{{ t('common.songCount', { count: playlistInfo.total || songs.length }) }}</p>
          </div>
        </div>

        <div class="playlist-actions" :class="{ compact: isHeaderCompact }">
            <t-button
              theme="primary"
              size="medium"
              :disabled="songs.length === 0 || loading"
              class="play-btn"
              :aria-label="t('common.playAll')"
              @click="handlePlayAll"
            >
              <template #icon>
                <svg class="play-icon" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M8 5v14l11-7z" />
                </svg>
              </template>
              {{ t('common.playAll') }}
            </t-button>

            <t-button
              variant="outline"
              size="medium"
              :disabled="songs.length === 0 || loading"
              class="shuffle-btn"
              :aria-label="t('common.shufflePlay')"
              @click="handleShufflePlay"
            >
              <template #icon>
                <svg class="shuffle-icon" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.46 20 9.5V4h-5.5zm.33 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z" />
                </svg>
              </template>
              {{ t('common.shufflePlay') }}
            </t-button>

            <t-dropdown
              :max-column-width="160"
              :options="moreActions"
              trigger="click"
              @click="(data: any) => handleMoreAction(data.value)"
            >
              <t-button theme="default" class="action-btn-more" aria-label="更多操作">
                <template #icon>
                  <EllipsisIcon :stroke-width="1.5" />
                </template>
              </t-button>
            </t-dropdown>

            <div class="playlist-search" :class="{ focused: searchFocused || !!searchQuery }">
              <t-input
                v-model="searchQuery"
                :aria-label="t('common.searchPlaylist')"
                clearable
                @focus="searchFocused = true"
                @blur="searchFocused = !!searchQuery"
              >
                <template #prefix-icon>
                  <SearchIcon size="16px" />
                </template>
              </t-input>
            </div>
        </div>
      </div>
    </div>

    <!-- 歌曲列表 -->
    <div class="scrollable-content">
      <div v-if="loading" class="loading-container">
        <div class="loading-content">
          <div class="loading-spinner"></div>
          <p>{{ t('common.loading') }}</p>
        </div>
      </div>

      <div v-else class="song-list-wrapper">
        <SongVirtualList
          ref="songListRef"
          :songs="displaySongs"
          :current-song-id="currentSongId"
          :is-playing="false"
          :show-index="true"
          :show-album="true"
          :show-duration="true"
          :is-local-playlist="isLocalPlaylist"
          :enable-sort="isLocalPlaylist"
          :playlist-id="playlistInfo.id"
          :multi-select="multiSelect"
          :enable-download="true"
          @play="handlePlay"
          @add-to-playlist="handleAddToPlaylist"
          @download="handleDownloadSong"
          @download-batch="handleDownloadBatch"
          @play-batch="handlePlayBatch"
          @remove-from-playlist="handleRemoveFromPlaylist"
          @remove-batch="handleRemoveBatch"
          @exit-multi-select="handleExitMultiSelect"
          @scroll="handleScroll"
        />

        <div v-if="loadingMore" class="loading-more">
          <div class="loading-spinner small"></div>
        </div>

        <div v-if="!loading && displaySongs.length === 0" class="empty-state">
          <p>{{ searchQuery ? t('music.list.noMatchSongs') : t('music.list.noSongs') }}</p>
        </div>

        <!-- 定位当前播放按钮 -->
        <transition name="locate-fade">
          <div
            v-if="hasCurrentPlaying && showLocateBtn"
            class="locate-current-btn"
            :title="t('music.list.locatePlaying')"
            @click="locateCurrentSong"
          >
            <MapAimingIcon size="20" />
          </div>
        </transition>
      </div>
    </div>

    <AddToPlaylistDialog
      v-model:visible="showAddToPlaylist"
      :songs="songsToAdd"
    />
  </div>
</template>

<style lang="scss" scoped>
.list-container {
  width: 100%;
  max-width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 20px;
  overflow: hidden;
  box-sizing: border-box;
  min-width: 0;
}

.fixed-header {
  flex-shrink: 0;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  margin-bottom: 20px;
}

.playlist-header {
  display: grid;
  grid-template-columns: minmax(96px, 192px) minmax(0, 1fr) minmax(240px, 420px);
  grid-template-areas: 'cover details actions';
  align-items: center;
  gap: 1.5rem;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
  padding: 1.5rem;
  height: 240px;
  background: var(--td-bg-color-container);
  border-radius: 0.75rem;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  z-index: 1;
  overflow: hidden;

  &::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-image: var(--header-cover);
    background-size: cover;
    background-position: top center;
    background-repeat: no-repeat;
    z-index: -1;
    border-radius: inherit;
    filter: blur(10px);
    transform: scale(1.1);
    -webkit-mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.8) 0%, rgba(0, 0, 0, 0.05) 60%, rgba(0, 0, 0, 0) 70%);
    mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.8) 0%, rgba(0, 0, 0, 0.05) 60%, rgba(0, 0, 0, 0) 70%);
    transition: opacity 0.8s ease-out;
    opacity: 0;

    @media (prefers-color-scheme: dark) {
      filter: blur(10px) grayscale(0.5) brightness(0.6);
    }

    :root[data-theme='dark'] & {
      filter: blur(10px) grayscale(0.5) brightness(0.6);
    }
  }

  &.bg-loaded::before {
    opacity: 1;
  }

  &.compact {
    height: 120px;
    padding: 1rem;
    gap: 1rem;

    .playlist-title {
      font-size: 25px;
      margin: 0 0 0.25rem 0;
    }
  }
}

.playlist-cover {
  grid-area: cover;
  width: 100%;
  max-width: 192px;
  height: auto;
  aspect-ratio: 1 / 1;
  border-radius: 0.5rem;
  overflow: hidden;
  flex-shrink: 0;
  position: relative;
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.2s ease;
  }

  &.clickable {
    cursor: pointer;

    &:hover {
      .cover-overlay { opacity: 1; }
      img { transform: scale(1.05); }
    }
  }

  .cover-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.2s ease;
    color: white;
    font-size: 12px;
    text-align: center;
    padding: 8px;

    .edit-icon {
      width: 24px;
      height: 24px;
      margin-bottom: 4px;
    }

    span {
      font-weight: 500;
      line-height: 1.2;
    }
  }
}

.playlist-details {
  grid-area: details;
  min-width: 0;
  font-family: lyricfont, Arial, Helvetica, sans-serif;

  .playlist-title {
    line-height: 1em;
    font-size: 34px;
    font-weight: 800;
    color: var(--td-text-color-primary);
    margin: 0 0 0.5rem;
    transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
}

.playlist-meta {
  overflow: hidden;
  max-height: 80px;
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);

  &.collapsed {
    max-height: 0;
    opacity: 0;
    margin: 0;
  }
}

.playlist-desc {
  font-size: 1rem;
  color: var(--td-text-color-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  text-overflow: ellipsis;
  overflow: hidden;
  margin: 0 0 0.5rem;
  transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease, box-shadow 0.3s ease, opacity 0.3s ease, transform 0.3s ease;
  transform: translateY(0);
}

.playlist-stats {
  font-size: 0.875rem;
  color: var(--td-text-color-placeholder);
  margin: 0;
  padding: 5px 0 0 0;
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.playlist-actions {
  grid-area: actions;
  display: flex;
  align-items: center;
  min-width: 0;
  max-width: 100%;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 0;
  margin-left: auto;
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);

  &.compact {
    margin-top: 0.5rem;
    gap: 0.5rem;

    .play-btn, .shuffle-btn {
      min-width: 100px;
      padding: 6px 12px;
      font-size: 0.875rem;
    }

    .play-icon, .shuffle-icon {
      width: 14px;
      height: 14px;
    }
  }
}

.play-btn, .shuffle-btn {
  min-width: 120px;
  padding: 6px 9px;
  border-radius: 8px;
  height: 36px;
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.play-icon, .shuffle-icon {
  width: 16px;
  height: 16px;
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.action-btn-more {
  width: 36px;
  height: 36px;
  padding: 6px;
  border-radius: 8px;
}

.playlist-search {
  flex: 1 1 160px;
  min-width: 0;
  max-width: 260px;
  margin-left: 0;
  overflow: hidden;
  transition: flex-basis 0.2s ease, max-width 0.2s ease, opacity 0.2s ease;

  &.focused {
    flex-basis: 240px;
    max-width: 320px;
  }

  :deep(.t-input) {
    height: 36px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--td-bg-color-container) 84%, transparent);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }

  :deep(.t-input__inner) {
    min-width: 0;
    font-size: 0.875rem;
  }
}

.scrollable-content {
  background: var(--td-bg-color-container);
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.loading-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
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

  &.small {
    width: 24px;
    height: 24px;
    border-width: 2px;
  }
}

.loading-content p {
  font-size: 14px;
  color: var(--td-text-color-secondary);
  margin: 0;
}

.song-list-wrapper {
  height: 100%;
  display: flex;
  flex-direction: column;
  position: relative;
}

.loading-more { display: flex; justify-content: center; padding: 20px; }

.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 200px;
  color: var(--td-text-color-placeholder);
}

.locate-current-btn {
  position: absolute;
  bottom: 30px;
  right: 30px;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: var(--td-bg-color-container);
  color: var(--td-text-color-primary);
  border: 1px solid var(--td-border-level-2-color);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease, box-shadow 0.3s ease, opacity 0.3s ease, transform 0.3s ease;
  z-index: 10;

  &:hover {
    transform: scale(1.05);
    background: var(--td-bg-color-secondarycontainer);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.15);
  }

  &:active {
    transform: scale(0.95);
  }
}

.locate-fade-enter-active,
.locate-fade-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}

.locate-fade-enter-from,
.locate-fade-leave-to {
  opacity: 0;
  transform: scale(0.92);
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* 手机端返回按钮（默认隐藏） */
.mobile-back-btn {
  display: none;
}

@media (max-width: 1120px) and (min-width: 769px) {
  .playlist-header {
    grid-template-columns: minmax(96px, 160px) minmax(0, 1fr);
    grid-template-areas:
      'cover details'
      'cover actions';
    height: auto;
    min-height: 220px;
  }

  .playlist-actions {
    justify-content: flex-start;
    margin-left: 0;
  }

  .playlist-search {
    max-width: 320px;
  }
}

@media (max-width: 768px) {
  .mobile-back-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.2);
    backdrop-filter: blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: blur(var(--mobile-glass-blur));
    border: 0.5px solid rgba(255, 255, 255, 0.18);
    color: #fff;
    cursor: pointer;
    position: fixed;
    top: calc(var(--mobile-safe-top) + 8px);
    left: var(--mobile-page-gutter);
    z-index: 20;
    touch-action: manipulation;
    transition: background-color 0.2s ease, opacity 0.2s ease, transform 0.2s ease;

    svg {
      width: 22px;
      height: 22px;
    }

    &:active {
      transform: scale(0.94);
    }
  }

  .list-container {
    padding: 0 var(--mobile-page-gutter) 0;
  }

  .fixed-header {
    position: sticky;
    top: 0;
    z-index: 12;
    flex-shrink: 0;
    width: calc(100% + var(--mobile-page-gutter) * 2);
    max-width: none;
    margin: 0 calc(var(--mobile-page-gutter) * -1) 10px;
  }

  .playlist-header {
    display: grid;
    grid-template-columns: 84px minmax(0, 1fr);
    grid-template-areas:
      'cover details'
      'actions actions';
    align-items: center;
    gap: 12px;
    min-height: 0;
    height: auto;
    padding: 12px;
    padding-top: calc(var(--mobile-touch-target) + 10px);
    position: relative;
    border-radius: 0 0 var(--mobile-card-radius) var(--mobile-card-radius);
    box-shadow: 0 14px 34px rgba(15, 23, 42, 0.12);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(14px);
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(14px);
    transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;

    &::before {
      filter: blur(10px) saturate(1.2);
      transform: scale(1.08);
      -webkit-mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.72) 0%, rgba(0, 0, 0, 0.12) 72%, rgba(0, 0, 0, 0) 100%);
      mask-image: linear-gradient(to bottom, rgba(0, 0, 0, 0.72) 0%, rgba(0, 0, 0, 0.12) 72%, rgba(0, 0, 0, 0) 100%);
    }

    &.compact {
      grid-template-columns: 52px minmax(0, 1fr) auto;
      grid-template-areas: 'cover details actions';
      gap: 8px;
      padding: 8px;
      padding-left: calc(var(--mobile-touch-target) + 18px);
      border-radius: calc(var(--mobile-card-radius-small) + 2px);
      box-shadow: 0 10px 24px rgba(15, 23, 42, 0.12);
      transform: translateY(0);

      .playlist-cover {
        width: 52px;
        height: 52px;
        border-radius: 12px;
      }

      .playlist-title {
        font-size: 1rem;
        line-height: 1.15;
        -webkit-line-clamp: 1;
        margin-bottom: 0.2rem;
      }

      .playlist-meta {
        max-height: 18px;
        opacity: 1;
      }

      .playlist-desc {
        display: none;
      }

      .playlist-stats {
        padding: 0;
        font-size: 0.75rem;
      }

      .playlist-actions {
        width: auto;
        margin-top: 0;
        flex-wrap: nowrap;
        justify-content: flex-end;
        gap: 4px;
      }

      .play-btn,
      .shuffle-btn {
        flex: 0 0 var(--mobile-touch-target);
        min-width: var(--mobile-touch-target);
        width: var(--mobile-touch-target);
        padding: 0;
        font-size: 0;
      }

      .playlist-search {
        display: none;
      }
    }
  }

  .playlist-cover {
    grid-area: cover;
    width: 84px;
    height: 84px;
    border-radius: var(--mobile-card-radius-small);
    box-shadow: 0 10px 22px rgba(15, 23, 42, 0.16);
  }

  .playlist-details {
    grid-area: details;
    width: 100%;
    min-width: 0;
    text-align: left;

    .playlist-title {
      font-size: clamp(1.2rem, 5.5vw, 1.55rem);
      line-height: 1.12;
      white-space: normal;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
      margin: 0 0 0.35rem;
    }
  }

  .playlist-meta {
    max-height: 48px;
  }

  .playlist-desc {
    font-size: 0.8rem;
    line-height: 1.35;
    -webkit-line-clamp: 1;
    margin: 0 0 0.25rem;
  }

  .playlist-stats {
    font-size: 0.75rem;
    padding: 0;
  }

  .playlist-actions {
    grid-area: actions;
    width: 100%;
    max-width: none;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-top: 0;
    margin-left: 0;

    .play-btn,
    .shuffle-btn {
      min-width: 0;
      flex: 1 1 calc(50% - 4px);
      min-height: var(--mobile-touch-target);
      height: var(--mobile-touch-target);
      border-radius: var(--mobile-control-radius);
      font-size: 0.875rem;
      touch-action: manipulation;
    }

    .action-btn-more {
      flex: 0 0 var(--mobile-touch-target);
      width: var(--mobile-touch-target);
      height: var(--mobile-touch-target);
      border-radius: var(--mobile-control-radius);
      touch-action: manipulation;
    }
  }

  .playlist-search {
    flex: 1 1 calc(100% - var(--mobile-touch-target) - 8px);
    width: auto;
    max-width: none;
    min-width: min(180px, 100%);
    margin-left: 0;

    &.focused {
      flex-basis: calc(100% - var(--mobile-touch-target) - 8px);
      max-width: 100%;
    }

    :deep(.t-input) {
      height: var(--mobile-touch-target);
      min-height: var(--mobile-touch-target);
      overflow: hidden;
      border-radius: 999px !important;
      --td-radius-default: 999px;
      background: color-mix(in srgb, var(--td-bg-color-container) 88%, transparent);
    }

    :deep(.t-input__wrap) {
      border-radius: 999px !important;
    }

    :deep(.t-input__inner) {
      border-radius: 999px !important;
    }
  }

  .scrollable-content {
    width: calc(100% + var(--mobile-page-gutter) * 2);
    max-width: none;
    margin-left: calc(var(--mobile-page-gutter) * -1);
    margin-right: calc(var(--mobile-page-gutter) * -1);
    border-radius: 0;
  }

  .locate-current-btn {
    bottom: 16px;
    right: 16px;
    width: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    touch-action: manipulation;
  }
}
</style>
