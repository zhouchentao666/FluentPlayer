<script setup lang="ts">
import { ref, onMounted, computed, onActivated } from 'vue'
import { useRoute } from 'vue-router'
import { musicSdk, type MusicItem, type SingerInfo, type SingerAlbumListResult } from '@/services/musicSdk'
import { playSong } from '@/utils/audio/globaPlayList'
import { fillMissingSongCovers } from '@/utils/songCover'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import SongVirtualList from '@/components/Music/SongVirtualList.vue'
import { createLiquidGlassConfirm } from '@/utils/liquidGlassConfirm'

const { t } = useI18n()
const route = useRoute()
const playStatus = useGlobalPlayStatusStore()
const localUserStore = LocalUserDetailStore()

const singerId = computed(() => route.params.id as string)
const singerSource = computed(() => (route.query.source as string) || '')

const singerInfo = ref<SingerInfo | null>(null)
const songs = ref<MusicItem[]>([])
const albums = ref<SingerAlbumListResult['list']>([])
const loading = ref(false)
const loadingMore = ref(false)
const hasMore = ref(true)
const currentPage = ref(1)
const totalCount = ref(0)

const songListRef = ref<InstanceType<typeof SongVirtualList> | null>(null)

const currentSongId = computed(() => localUserStore.userInfo?.lastPlaySongId)

const bgImageLoaded = ref(false)
const isHeaderCompact = ref(false)
const activeTab = ref<'songs' | 'albums'>('songs')

async function fetchSingerInfo() {
  try {
    singerInfo.value = await musicSdk.getSingerInfo(singerId.value, singerSource.value)
  } catch (e) {
    console.error('获取歌手信息失败:', e)
  }
}

async function fetchSongs(reset = false) {
  if (loading.value) return
  if (reset) { currentPage.value = 1; songs.value = []; hasMore.value = true }
  if (!hasMore.value) return

  loading.value = true
  try {
    const res = await musicSdk.getSingerSongList(singerId.value, currentPage.value, 30, singerSource.value)
    const newSongs = res?.list || []
    songs.value = reset ? newSongs : [...songs.value, ...newSongs]
    totalCount.value = res?.total || 0
    hasMore.value = songs.value.length < (res?.total || 0)
    currentPage.value += 1

    void fillMissingSongCovers(newSongs, {
      onBatchComplete: () => { songs.value = [...songs.value] }
    })
  } catch (e) {
    console.error('获取歌手歌曲失败:', e)
  } finally {
    loading.value = false
  }
}

async function fetchAlbums() {
  if (albums.value.length > 0) return
  loading.value = true
  try {
    const res = await musicSdk.getSingerAlbumList(singerId.value, 1, 30, singerSource.value)
    albums.value = res?.list || []
  } catch (e) {
    console.error('获取歌手专辑失败:', e)
  } finally {
    loading.value = false
  }
}

function handlePlay(song: MusicItem) {
  playStatus.updatePlayerInfo(song as any)
  playSong(song as any)
}

async function handlePlayAll() {
  if (songs.value.length === 0) return
  const confirmed = await createLiquidGlassConfirm({
    title: t('common.playAll'),
    body: t('music.singer.replaceConfirm', { name: singerInfo.value?.info.name || '', count: songs.value.length }),
    confirmText: t('music.list.confirmReplace'),
    cancelText: t('common.cancel'),
    icon: 'play'
  })
  if (!confirmed) return

  const sourceSongs = songListRef.value?.sortedSongs ?? songs.value
  localUserStore.replaceSongList(sourceSongs.map(s => ({
    songmid: s.songmid, name: s.name, singer: s.singer,
    albumName: s.albumName, img: s.img, source: s.source,
    url: '', interval: s.interval
  })) as any)
  playSong(sourceSongs[0] as any)
  playStatus.updatePlayerInfo(sourceSongs[0] as any)
}

function switchTab(tab: 'songs' | 'albums') {
  activeTab.value = tab
  if (tab === 'albums') fetchAlbums()
}

function goBack() {
  window.history.back()
}

onMounted(() => {
  fetchSingerInfo()
  fetchSongs(true)
})

onActivated(() => {
  if (!singerInfo.value && singerId.value) {
    fetchSingerInfo()
    fetchSongs(true)
  }
})
</script>

<template>
  <div class="singer-page">
    <!-- Header -->
    <div class="singer-header" :class="{ compact: isHeaderCompact }">
      <div class="header-bg">
        <img
          v-if="singerInfo?.info.avatar"
          :src="singerInfo.info.avatar"
          :class="{ loaded: bgImageLoaded }"
          @load="bgImageLoaded = true"
        />
        <div class="bg-overlay" />
      </div>

      <div class="header-content">
        <div class="back-btn" @click="goBack">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <path d="M15.41 7.41L14 6L8 12L14 18L15.41 16.59L10.83 12L15.41 7.41Z" fill="currentColor" />
          </svg>
        </div>

        <div class="singer-meta">
          <div class="singer-cover">
            <img v-if="singerInfo?.info.avatar" :src="singerInfo.info.avatar" />
            <div v-else class="cover-placeholder" />
          </div>
          <div class="singer-info">
            <h1 class="singer-name">{{ singerInfo?.info.name || t('music.singer.loading') }}</h1>
            <div class="singer-stats" v-if="singerInfo">
              <span>{{ t('music.singer.songCount', { count: singerInfo.count.music }) }}</span>
              <span class="dot">&middot;</span>
              <span>{{ t('music.singer.albumCount', { count: singerInfo.count.album }) }}</span>
            </div>
            <p class="singer-desc" v-if="singerInfo?.info.desc">{{ singerInfo.info.desc }}</p>
          </div>
        </div>

        <div class="header-actions" v-if="songs.length > 0">
          <button class="play-all-btn" @click="handlePlayAll">{{ t('music.singer.playAll') }}</button>
        </div>
      </div>
    </div>

    <!-- Tabs -->
    <div class="singer-tabs">
      <div
        class="tab" :class="{ active: activeTab === 'songs' }"
        @click="switchTab('songs')"
      >
        {{ t('music.singer.tabSongs') }}({{ totalCount }})
      </div>
      <div
        class="tab" :class="{ active: activeTab === 'albums' }"
        @click="switchTab('albums')"
      >
        {{ t('music.singer.tabAlbums') }}({{ singerInfo?.count.album || 0 }})
      </div>
    </div>

    <!-- Song List -->
    <div v-show="activeTab === 'songs'" class="singer-content">
      <SongVirtualList
        ref="songListRef"
        :songs="songs"
        :currentSongId="currentSongId"
        :showAlbum="true"
        :showDuration="true"
        @play="handlePlay"
      />
      <div v-if="loading" class="loading-more">{{ t('common.loading') }}</div>
      <div v-if="!hasMore && songs.length > 0" class="no-more">{{ t('music.singer.noMore') }}</div>
    </div>

    <!-- Album List -->
    <div v-show="activeTab === 'albums'" class="singer-content">
      <div v-if="loading" class="loading-more">{{ t('common.loading') }}</div>
      <div v-else-if="albums.length === 0" class="empty-state">{{ t('music.singer.noAlbums') }}</div>
      <div v-else class="album-grid">
        <div
          v-for="album in albums"
          :key="String(album.id)"
          class="album-card"
          @click="$router.push({ name: 'list', params: { id: String(album.id) }, query: { title: album.info.name, cover: album.info.img, source: singerSource, type: 'album' } })"
        >
          <div class="album-cover">
            <img v-if="album.info.img" :src="album.info.img" loading="lazy" />
            <div v-else class="cover-placeholder" />
          </div>
          <div class="album-title">{{ album.info.name }}</div>
          <div class="album-meta">{{ t('music.songList.songsUnit', { count: album.count }) }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.singer-page {
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
}

.singer-header {
  position: relative;
  padding: 60px 24px 20px;
  min-height: 220px;
}

.header-bg {
  position: absolute;
  inset: 0;
  overflow: hidden;
  z-index: 0;
}

.header-bg img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  filter: blur(12px) brightness(0.4);
  transform: scale(1.2);
  opacity: 0;
  transition: opacity 0.5s;
}

.header-bg img.loaded {
  opacity: 1;
}

.bg-overlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(transparent 0%, var(--td-bg-color-container) 100%);
}

.header-content {
  position: relative;
  z-index: 1;
}

.back-btn {
  position: absolute;
  top: -40px;
  left: 0;
  cursor: pointer;
  color: var(--td-text-color-primary);
  opacity: 0.8;
  transition: opacity 0.2s;
  padding: 4px;
}

.back-btn:hover {
  opacity: 1;
}

.singer-meta {
  display: flex;
  gap: 16px;
  align-items: flex-end;
}

.singer-cover {
  width: 100px;
  height: 100px;
  border-radius: 50%;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--td-bg-color-secondarycontainer);
}

.singer-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.cover-placeholder {
  width: 100%;
  height: 100%;
  background: var(--td-bg-color-secondarycontainer);
}

.singer-info {
  flex: 1;
  min-width: 0;
}

.singer-name {
  font-size: 22px;
  font-weight: 700;
  margin: 0 0 4px;
  color: var(--td-text-color-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.singer-stats {
  font-size: 13px;
  color: var(--td-text-color-secondary);
  margin-bottom: 4px;
}

.singer-stats .dot {
  margin: 0 6px;
}

.singer-desc {
  font-size: 12px;
  color: var(--td-text-color-placeholder);
  margin: 4px 0 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.header-actions {
  margin-top: 16px;
}

.play-all-btn {
  padding: 8px 24px;
  border-radius: 20px;
  border: none;
  background: var(--td-brand-color);
  color: #fff;
  font-size: 14px;
  cursor: pointer;
  transition: opacity 0.2s;
}

.play-all-btn:hover {
  opacity: 0.85;
}

.singer-tabs {
  display: flex;
  padding: 0 24px;
  gap: 24px;
  border-bottom: 1px solid var(--td-border-level-1-color);
  position: sticky;
  top: 0;
  background: var(--td-bg-color-container);
  z-index: 10;
}

.tab {
  padding: 10px 0;
  font-size: 14px;
  color: var(--td-text-color-secondary);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
}

.tab.active {
  color: var(--td-brand-color);
  border-bottom-color: var(--td-brand-color);
}

.tab:hover {
  color: var(--td-text-color-primary);
}

.singer-content {
  padding: 0;
}

.loading-more,
.no-more,
.empty-state {
  text-align: center;
  padding: 24px;
  color: var(--td-text-color-placeholder);
  font-size: 13px;
}

.album-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 16px;
  padding: 16px 24px;
}

.album-card {
  cursor: pointer;
  transition: transform 0.2s;
}

.album-card:hover {
  transform: translateY(-2px);
}

.album-cover {
  width: 100%;
  aspect-ratio: 1;
  border-radius: 8px;
  overflow: hidden;
  background: var(--td-bg-color-secondarycontainer);
}

.album-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.album-title {
  font-size: 13px;
  margin-top: 6px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--td-text-color-primary);
}

.album-meta {
  font-size: 12px;
  color: var(--td-text-color-placeholder);
  margin-top: 2px;
}

@media (max-width: 768px) {
  .singer-page {
    -webkit-overflow-scrolling: touch;
  }

  .singer-header {
    min-height: auto;
    padding: calc(var(--mobile-safe-top) + var(--mobile-page-top-gutter) + var(--mobile-touch-target)) var(--mobile-page-gutter) var(--mobile-page-top-gutter);
  }

  .back-btn {
    top: calc(-1 * var(--mobile-touch-target));
    left: 0;
    width: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.18);
    color: #fff;
    touch-action: manipulation;
  }

  .singer-meta {
    align-items: center;
    flex-direction: column;
    text-align: center;
    gap: 0.75rem;
  }

  .singer-cover {
    width: 112px;
    height: 112px;
  }

  .singer-name {
    font-size: clamp(2rem, 9vw, 2.6rem);
    line-height: 1.1;
    letter-spacing: -0.04em;
    white-space: normal;
  }

  .singer-desc {
    -webkit-line-clamp: 3;
    font-size: 0.85rem;
  }

  .header-actions {
    display: flex;
    justify-content: center;
  }

  .play-all-btn {
    min-height: var(--mobile-touch-target);
    padding: 0 24px;
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .singer-tabs {
    padding: 0 var(--mobile-page-gutter);
    gap: 0.75rem;
    background: var(--mobile-glass-bg-strong);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
  }

  .tab {
    min-height: var(--mobile-touch-target);
    padding: 0;
    display: flex;
    align-items: center;
    font-size: 15px;
    touch-action: manipulation;
  }

  .album-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    padding: 16px var(--mobile-page-gutter);
  }

  .album-cover {
    border-radius: var(--mobile-card-radius-small);
  }
}
</style>
