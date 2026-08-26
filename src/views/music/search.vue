<script setup lang="ts">
import { ref, reactive, watch, onActivated, onMounted, onBeforeUnmount, computed } from 'vue'
import { searchValue as useSearchStore } from '@/store/search'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { type MusicItem } from '@/services/musicSdk'
import { useSearchSongs } from '@/composables/useSearchSongs'
import { useSearchPlaylists, type PlaylistCardItem } from '@/composables/useSearchPlaylists'
import { useSourceAccess } from '@/composables/useSourceAccess'
import { unescapeHtml } from '@/utils/search/normalize'
import SkeletonLoader from '@/components/SkeletonLoader.vue'
import { playSong } from '@/utils/audio/globaPlayList'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { useRouter } from 'vue-router'

interface SourceOption {
  key: string
  name: string
}

const { t } = useI18n()
const searchStore = useSearchStore()
const router = useRouter()
const localUserStore = LocalUserDetailStore()
const { getSourceName, getSourceOptions, isSourceEnabled } = useSourceAccess()

const songs = reactive(useSearchSongs())
const playlists = reactive(useSearchPlaylists())

type SearchTab = 'songs' | 'playlists'

const keyword = ref('')
const activeTab = ref<SearchTab>('songs')
const selectedSource = ref<string>('all')
const initialSearchDone = ref(false)

const setActiveTab = (tab: SearchTab) => {
  if (activeTab.value === tab) return
  activeTab.value = tab
}

const sourceOptions = computed<SourceOption[]>(() => {
  return getSourceOptions({ excludeSubsonic: true })
})

const sourceOrderMap = computed(() => new Map(sourceOptions.value.map((s, i) => [s.key, i])))

const getSourceNameLocal = (source: string) => {
  return getSourceName(source)
}

const sourceKeys = () => sourceOptions.value.map(s => s.key)

const searchActiveTab = async () => {
  const query = keyword.value.trim()
  if (!query) return
  if (activeTab.value === 'songs') {
    if (selectedSource.value === 'all') {
      await songs.fetchAggregate(query)
    } else {
      await songs.fetchPage(query, selectedSource.value, true)
    }
  } else {
    await playlists.fetch(query, selectedSource.value, sourceKeys(), sourceOrderMap.value, true)
  }
}

const onSourceChange = async (src: string) => {
  if (src !== 'all' && !sourceOptions.value.some(item => item.key === src)) return
  if (selectedSource.value === src) return
  selectedSource.value = src
  resetResults()
  if (keyword.value.trim()) await searchActiveTab()
}

function resetResults() {
  songs.reset()
  playlists.reset()
}

const handlePlay = (song: MusicItem) => {
  const playStatus = useGlobalPlayStatusStore()
  playStatus.updatePlayerInfo(song as any)
  playSong(song as any)
}

const handleScroll = (event: Event) => {
  if (activeTab.value !== 'songs' || songScrollFrame !== null) return
  const target = event.target as HTMLElement
  songScrollFrame = requestAnimationFrame(() => {
    songScrollFrame = null
    if (!isNearBottom(target)) return
    if (selectedSource.value === 'all') {
      if (!songs.aggregateLoading && songs.aggregateHasMore) songs.fetchAggregateNextPage(keyword.value)
    } else if (!songs.loading && songs.hasMore) {
      songs.fetchPage(keyword.value, selectedSource.value, false)
    }
  })
}

const routerToPlaylist = (playlist: PlaylistCardItem) => {
  router.push({
    name: 'list',
    params: { id: playlist.id },
    query: { title: playlist.title, source: playlist.source, cover: playlist.cover },
  })
}

const onPlaylistScroll = (event: Event) => {
  if (selectedSource.value === 'all' || playlistScrollFrame !== null) return
  const target = event.target as HTMLElement
  playlistScrollFrame = requestAnimationFrame(() => {
    playlistScrollFrame = null
    if (isNearBottom(target) && playlists.results.length < playlists.total && !playlists.loading) {
      playlists.fetch(keyword.value, selectedSource.value, sourceKeys(), sourceOrderMap.value, false)
    }
  })
}

const songKey = (song: MusicItem, index: number) => {
  const source = song.source || selectedSource.value
  const id = song.songmid || song.hash || `${song.name}-${song.singer}-${song.albumName}`
  return `${source}-${id}-${index}`
}

const playlistKey = (playlist: PlaylistCardItem) => `${playlist.source || selectedSource.value}-${playlist.id}`

let songScrollFrame: number | null = null
let playlistScrollFrame: number | null = null

const isNearBottom = (target: HTMLElement) => {
  const { scrollTop, scrollHeight, clientHeight } = target
  return scrollHeight - scrollTop - clientHeight < 100
}

onMounted(async () => {
  const val = searchStore.getValue.trim()
  if (val === '') { router.push({ name: 'find' }); return }
  initialSearchDone.value = true
  keyword.value = val
  resetResults()
  await searchActiveTab()
})

onActivated(async () => {
  if (!initialSearchDone.value) return
  const val = searchStore.getValue.trim()
  if (val === '') { router.push({ name: 'find' }); return }
  if (val !== keyword.value.trim()) {
    keyword.value = val
    resetResults()
    await searchActiveTab()
  }
})

onBeforeUnmount(() => {
  if (songScrollFrame !== null) cancelAnimationFrame(songScrollFrame)
  if (playlistScrollFrame !== null) cancelAnimationFrame(playlistScrollFrame)
})

watch(
  [() => searchStore.getValue, () => searchStore.getFocus],
  async ([val, focus]) => {
    const nextKeyword = val.trim()
    if (focus) return
    if (nextKeyword === '') { router.push({ name: 'find' }); return }
    if (nextKeyword === keyword.value.trim()) return
    keyword.value = nextKeyword
    resetResults()
    await searchActiveTab()
  }
)

watch(sourceOptions, async (options, previousOptions = []) => {
  if (selectedSource.value !== 'all' && !options.some(item => item.key === selectedSource.value)) {
    selectedSource.value = 'all'
    resetResults()
    if (keyword.value.trim()) await searchActiveTab()
    return
  }

  if (selectedSource.value !== 'all') return

  const previousKeys = previousOptions.map(item => item.key).join('|')
  const nextKeys = options.map(item => item.key).join('|')
  if (previousKeys === nextKeys) return

  playlists.reset()
  if (activeTab.value === 'playlists' && keyword.value.trim()) {
    await playlists.fetch(keyword.value, selectedSource.value, sourceKeys(), sourceOrderMap.value, true)
  }
})

watch(activeTab, async (val) => {
  if (!keyword.value.trim()) return
  if (val === 'songs') {
    if (selectedSource.value === 'all') {
      if (!songs.aggregateSearched && !songs.aggregateLoading) await songs.fetchAggregate(keyword.value)
    } else {
      if (songs.results.length === 0) await songs.fetchPage(keyword.value, selectedSource.value, true)
    }
  } else {
    if (!playlists.searched && !playlists.loading) {
      await playlists.fetch(keyword.value, selectedSource.value, sourceKeys(), sourceOrderMap.value, true)
    }
  }
})
</script>

<template>
  <div class="search-container">
    <div class="search-header">
      <div class="header-row">
        <h2 class="search-title">{{ t('music.search.searchKeyword') }}"<span class="keyword">{{ keyword }}</span>"</h2>
        <div class="result-info">
          <span v-if="activeTab === 'songs' && selectedSource === 'all' && songs.aggregateSearched">{{ t('music.search.foundAggregateSongs', { count: songs.aggregateResults.length }) }}</span>
          <span v-else-if="activeTab === 'songs' && selectedSource !== 'all'">{{ t('music.search.foundSongs', { count: songs.totalItems }) }}</span>
          <span v-else-if="activeTab === 'playlists' && selectedSource === 'all' && playlists.searched">{{ t('music.search.foundAggregatePlaylists', { count: playlists.total }) }}</span>
          <span v-else-if="activeTab === 'playlists' && selectedSource !== 'all'">{{ t('music.search.foundPlaylists', { count: playlists.total }) }}</span>
        </div>
      </div>

      <div class="tabs-row" role="tablist" :aria-label="t('music.search.title')">
        <button
          id="search-songs-tab"
          type="button"
          role="tab"
          class="search-tab-button"
          :class="{ 'is-active': activeTab === 'songs' }"
          :aria-selected="activeTab === 'songs' ? 'true' : 'false'"
          :data-active="activeTab === 'songs' ? 'true' : 'false'"
          :tabindex="activeTab === 'songs' ? 0 : -1"
          aria-controls="search-songs-panel"
          @click="setActiveTab('songs')"
        >{{ t('music.search.tabSongs') }}</button>
        <button
          id="search-playlists-tab"
          type="button"
          role="tab"
          class="search-tab-button"
          :class="{ 'is-active': activeTab === 'playlists' }"
          :aria-selected="activeTab === 'playlists' ? 'true' : 'false'"
          :data-active="activeTab === 'playlists' ? 'true' : 'false'"
          :tabindex="activeTab === 'playlists' ? 0 : -1"
          aria-controls="search-playlists-panel"
          @click="setActiveTab('playlists')"
        >{{ t('music.search.tabPlaylists') }}</button>
      </div>

      <div v-if="sourceOptions.length > 0" class="source-filter">
        <button
          type="button"
          class="source-chip"
          :class="{ active: selectedSource === 'all' }"
          @click="onSourceChange('all')"
        >{{ t('music.search.allSources') }}</button>
        <button
          v-for="source in sourceOptions"
          :key="source.key"
          type="button"
          class="source-chip"
          :class="{ active: selectedSource === source.key }"
          @click="onSourceChange(source.key)"
        >{{ source.name }}</button>
      </div>
    </div>

    <div class="result-content">
      <div
        id="search-songs-panel"
        v-show="activeTab === 'songs'"
        role="tabpanel"
        aria-labelledby="search-songs-tab"
        class="song-tab"
        :aria-busy="selectedSource === 'all' ? songs.aggregateLoading : songs.loading"
        @scroll="handleScroll"
      >
        <template v-if="selectedSource === 'all'">
          <div v-if="songs.aggregateLoading && songs.aggregateResults.length === 0" class="loading-state" role="status" aria-live="polite">
            <div class="loading-spinner"></div>
            <p>{{ t('music.search.searchingSources') }}</p>
          </div>
          <div v-else-if="songs.aggregateResults.length > 0" class="song-list">
            <div v-for="(song, index) in songs.aggregateResults" :key="songKey(song, index)" class="song-item" @click="handlePlay(song)">
              <span class="song-index">{{ index + 1 }}</span>
              <div class="song-info">
                <span class="song-name">{{ song.name }}</span>
                <span class="song-singer">
                  <span
                    v-if="song.singerId && song.source !== 'local'"
                    class="singer-link"
                    @click.stop="router.push({ name: 'singer', params: { id: song.singerId }, query: { source: song.source } })"
                  >{{ song.singer }}</span>
                  <template v-else>{{ song.singer }}</template>
                  <template v-if="song.albumName"> - {{ song.albumName }}</template>
                </span>
              </div>
              <div class="song-meta">
                <span v-if="song.source" class="source-badge">{{ getSourceNameLocal(song.source) }}</span>
                <span class="song-duration">{{ song.interval || '--:--' }}</span>
              </div>
            </div>
            <div v-if="songs.aggregateLoading" class="load-more-indicator" role="status" aria-live="polite">
              <div class="loading-spinner small"></div>
            </div>
          </div>
          <div v-else-if="songs.aggregateSearched" class="empty-state"><div class="empty-content"><h3>{{ t('music.search.noSongResults') }}</h3><p>{{ t('music.search.tryOther') }}</p></div></div>
        </template>

        <template v-else>
          <div v-if="songs.results.length > 0" class="song-list">
            <div v-for="(song, index) in songs.results" :key="songKey(song, index)" class="song-item" @click="handlePlay(song)">
              <span class="song-index">{{ index + 1 }}</span>
              <div class="song-info">
                <span class="song-name">{{ song.name }}</span>
                <span class="song-singer">
                  <span
                    v-if="song.singerId && song.source !== 'local'"
                    class="singer-link"
                    @click.stop="router.push({ name: 'singer', params: { id: song.singerId }, query: { source: song.source } })"
                  >{{ song.singer }}</span>
                  <template v-else>{{ song.singer }}</template>
                  <template v-if="song.albumName"> - {{ song.albumName }}</template>
                </span>
              </div>
              <span class="song-duration">{{ song.interval || '--:--' }}</span>
            </div>
            <div v-if="songs.loading" class="load-more-indicator" role="status" aria-live="polite">
              <div class="loading-spinner small"></div>
            </div>
          </div>
          <div v-else-if="!songs.loading" class="empty-state"><div class="empty-content"><h3>{{ t('music.search.noSongResults') }}</h3><p>{{ t('music.search.tryOther') }}</p></div></div>
          <SkeletonLoader v-if="songs.loading && songs.results.length === 0" type="song-list" :rows="10" />
        </template>
      </div>

      <div id="search-playlists-panel" v-show="activeTab === 'playlists'" role="tabpanel" aria-labelledby="search-playlists-tab" class="playlist-tab" :aria-busy="playlists.loading">
        <div class="grid-scroll-container" @scroll="onPlaylistScroll">
          <div v-if="playlists.loading && playlists.results.length === 0" class="loading-state" role="status" aria-live="polite">
            <div class="loading-spinner"></div>
            <p>{{ selectedSource === 'all' ? t('music.search.searchingAggregatePlaylists') : t('music.search.searchingPlaylists') }}</p>
          </div>
          <TransitionGroup v-else-if="playlists.results.length > 0" name="grid-fade" tag="div" class="playlist-grid">
            <div v-for="playlist in playlists.results" :key="playlistKey(playlist)" class="playlist-card" @click="routerToPlaylist(playlist)">
              <div class="playlist-cover"><img :src="playlist.cover || '/default-cover.png'" :alt="playlist.title" loading="lazy" /></div>
              <div class="playlist-info">
                <h4 class="playlist-title">{{ unescapeHtml(playlist.title) }}</h4>
                <p class="playlist-desc">{{ playlist.description || t('music.search.featuredPlaylist') }}</p>
              </div>
            </div>
          </TransitionGroup>
          <div v-else-if="playlists.searched && !playlists.loading" class="empty-state"><div class="empty-content"><h3>{{ t('music.search.noPlaylistResults') }}</h3><p>{{ t('music.search.tryOther') }}</p></div></div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.search-container { width: 100%; padding: 20px; height: 100%; display: flex; flex-direction: column; overflow: hidden; box-sizing: border-box; background: var(--search-bg); transition: background-color var(--motion-duration-standard) var(--motion-ease-standard), color var(--motion-duration-standard) var(--motion-ease-standard); }
.search-header { flex-shrink: 0; margin-bottom: 12px; }
.header-row { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 8px; }
.search-title { font-size: 24px; font-weight: normal; color: var(--search-title-color); margin: 0; border-left: 4px solid var(--td-brand-color); padding-left: 8px; }
.keyword { color: var(--search-keyword-color); }
.result-info { font-size: 12px; color: var(--search-info-color); }
.tabs-row { display: inline-flex; align-items: center; gap: 20px; margin-bottom: 4px; border-bottom: 1px solid color-mix(in srgb, var(--td-text-color-primary) 12%, transparent); }
.search-tab-button { position: relative; border: none; background: transparent; color: color-mix(in srgb, var(--td-text-color-primary) 68%, transparent); padding: 8px 2px; font: inherit; font-size: 14px; line-height: 1.4; cursor: pointer; transition: color var(--motion-duration-standard) var(--motion-ease-standard); }
.search-tab-button::after { content: ''; position: absolute; left: 0; right: 0; bottom: -1px; height: 2px; border-radius: 999px; background: var(--td-brand-color); opacity: 0; transform: scaleX(0); transition: opacity var(--motion-duration-standard) var(--motion-ease-standard), transform var(--motion-duration-standard) var(--motion-ease-standard); }
.search-tab-button:hover { color: var(--td-text-color-primary); }
.search-tab-button.is-active,
.search-tab-button[data-active='true'],
.search-tab-button[aria-selected='true'] { color: var(--td-brand-color); font-weight: 500; }
.search-tab-button.is-active::after,
.search-tab-button[data-active='true']::after,
.search-tab-button[aria-selected='true']::after { opacity: 1; transform: scaleX(1); }
.search-tab-button:focus-visible { outline: 2px solid var(--td-brand-color); outline-offset: 2px; border-radius: 4px; }
.source-filter { display: flex; gap: 8px; overflow-x: auto; padding: 8px 0; scrollbar-width: none; -ms-overflow-style: none; }
.source-filter::-webkit-scrollbar { display: none; }
.source-chip {
  flex-shrink: 0; padding: 4px 14px; border-radius: 16px; border: 1px solid var(--td-border-level-2-color);
  background: var(--search-content-bg); color: var(--td-text-color-secondary); font-size: 13px;
  cursor: pointer; transition: background-color var(--motion-duration-standard) var(--motion-ease-standard), border-color var(--motion-duration-standard) var(--motion-ease-standard), color var(--motion-duration-standard) var(--motion-ease-standard); line-height: 1.4;
}
.source-chip:hover { border-color: var(--td-brand-color); color: var(--td-brand-color); }
.source-chip.active { background: var(--td-brand-color); color: var(--td-text-color-anti); border-color: var(--td-brand-color); }

.result-content { flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden; background: var(--search-content-bg); box-shadow: var(--search-content-shadow); border-radius: 12px; transition: background-color var(--motion-duration-standard) var(--motion-ease-standard), box-shadow var(--motion-duration-standard) var(--motion-ease-standard); }
.song-tab, .playlist-tab { display: flex; flex-direction: column; flex: 1; min-height: 0; overflow-y: auto; }
.song-list { padding: 4px; }
.song-item { display: flex; align-items: center; min-height: 44px; padding: 10px 12px; cursor: pointer; transition: background-color var(--motion-duration-instant) var(--motion-ease-standard); border-radius: 6px; box-sizing: border-box; border-bottom: 1px solid var(--td-border-level-1-color); overflow: hidden; width: 100%; }
.song-item:hover { background: var(--td-bg-color-component-hover); }
.song-item:last-child { border-bottom: none; }
.song-index { width: 32px; font-size: 12px; color: var(--td-text-color-secondary); flex-shrink: 0; }
.song-info { flex: 1; min-width: 0; max-width: 100%; overflow: hidden; display: flex; flex-direction: column; }
.song-name { font-size: 14px; color: var(--td-text-color-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.song-singer { font-size: 12px; color: var(--td-text-color-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.singer-link { cursor: pointer; &:hover { color: var(--td-brand-color); } }
.song-meta { flex-shrink: 1; min-width: 0; display: inline-flex; align-items: center; gap: 12px; margin-left: 12px; overflow: hidden; }
.source-badge { flex-shrink: 1; max-width: 92px; min-width: 0; padding: 2px 8px; border-radius: 999px; background: color-mix(in srgb, var(--td-brand-color) 12%, transparent); color: var(--td-brand-color); font-size: 11px; line-height: 1.5; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; transition: background-color var(--motion-duration-standard) var(--motion-ease-standard), color var(--motion-duration-standard) var(--motion-ease-standard); }
.song-duration { font-size: 12px; color: var(--td-text-color-secondary); flex-shrink: 0; white-space: nowrap; }

.grid-scroll-container { flex: 1; min-height: 0; overflow: auto; padding: 8px; }
.playlist-grid { display: grid; gap: 12px; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); }
.playlist-card { background: var(--td-bg-color-container); border: 1px solid var(--td-border-level-1-color); border-radius: 12px; overflow: hidden; cursor: pointer; transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard); }
.playlist-card:hover { transform: translateY(-2px); border-color: var(--td-brand-color); box-shadow: var(--theme-shadow-hover); }
.playlist-cover { position: relative; aspect-ratio: 1; overflow: hidden; }
.playlist-cover img { width: 100%; height: 100%; object-fit: cover; }
.playlist-info { padding: 12px; }
.playlist-title { font-size: 14px; font-weight: 600; color: var(--td-text-color-primary); margin: 0 0 6px; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.playlist-desc { font-size: 12px; color: var(--td-text-color-secondary); margin: 0; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.empty-state { display: flex; align-items: center; justify-content: center; min-height: 300px; }
.empty-content { text-align: center; }
.empty-content h3 { font-size: 16px; color: var(--search-empty-title); margin: 0 0 8px; font-weight: normal; }
.empty-content p { font-size: 12px; color: var(--search-empty-text); margin: 0; }
.loading-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 40px; }
.loading-spinner { width: 40px; height: 40px; border: 3px solid var(--search-loading-border); border-top-color: var(--search-loading-spinner); border-radius: 50%; will-change: transform; animation: spin 1s linear infinite; margin-bottom: 12px; }
.loading-state p { font-size: 14px; color: var(--search-loading-text); margin: 0; }
@keyframes spin { to { transform: rotate(360deg); } }
.grid-fade-enter-active, .grid-fade-leave-active { transition: background-color var(--motion-duration-standard) var(--motion-ease-standard), border-color var(--motion-duration-standard) var(--motion-ease-standard), color var(--motion-duration-standard) var(--motion-ease-standard), box-shadow var(--motion-duration-standard) var(--motion-ease-standard), opacity var(--motion-duration-standard) var(--motion-ease-standard), transform var(--motion-duration-standard) var(--motion-ease-standard); }
.grid-fade-enter-from, .grid-fade-leave-to { opacity: 0; transform: translateY(6px) scale(0.98); }

.load-more-indicator { display: flex; align-items: center; justify-content: center; padding: 12px; min-height: 44px; }
.loading-spinner.small { width: 20px; height: 20px; border-width: 2px; margin-bottom: 0; }

@media (prefers-reduced-motion: reduce) {
  .loading-spinner { animation: none; }
}

@media (max-width: 768px) {
  .search-container { padding: var(--mobile-page-top-gutter) var(--mobile-page-gutter) 0; max-width: 100vw; box-sizing: border-box; }
  .search-header { margin-bottom: 6px; min-width: 0; max-width: 100%; }
  .header-row { align-items: center; gap: 10px; margin-bottom: 4px; }
  .search-title { max-width: 100%; border-left: none; padding-left: 0; font-size: 18px; font-weight: 600; line-height: 1.3; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .result-info { font-size: 12px; flex-shrink: 0; }
  .tabs-row { gap: 16px; margin-bottom: 0; }
  .search-tab-button { min-height: var(--mobile-touch-target); font-size: 15px; }
  .source-filter { gap: 6px; padding: 6px 0 2px; max-width: 100%; -webkit-overflow-scrolling: touch; }
  .source-chip { padding: 6px 14px; font-size: 12px; min-height: 32px; border-radius: var(--mobile-control-radius); touch-action: manipulation; transition: transform 0.1s ease, opacity 0.1s ease; }
  .source-chip:active { transform: scale(0.95); opacity: 0.8; }
  .result-content { border-radius: var(--mobile-card-radius-small); }
  .song-tab, .playlist-tab, .grid-scroll-container { min-height: 0; -webkit-overflow-scrolling: touch; overflow-x: hidden; }
  .song-list { padding: 2px; }
  .song-item { min-height: 56px; padding: 8px 10px; border-radius: var(--mobile-card-radius-small); touch-action: manipulation; border-bottom: none; max-width: 100%; }
  .song-item:active { background: color-mix(in srgb, var(--td-bg-color-component-hover) 80%, transparent); transform: scale(0.985); }
  .song-index { width: 26px; font-size: 13px; }
  .song-info { flex: 1; min-width: 0; overflow: hidden; }
  .song-name { font-size: 15px; }
  .song-singer { font-size: 12px; }
  .song-meta { flex-shrink: 0; margin-left: auto; gap: 6px; overflow: hidden; padding-left: 8px; }
  .source-badge { max-width: 56px; padding: 1px 6px; font-size: 10px; }
  .song-duration { font-size: 12px; flex-shrink: 0; }
  .grid-scroll-container { padding: 0; }
  .playlist-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .playlist-card { border-radius: var(--mobile-card-radius-small); touch-action: manipulation; transition: transform 0.15s ease; }
  .playlist-card:active { transform: scale(0.97); }
  .playlist-info { padding: 10px; }
}
</style>
