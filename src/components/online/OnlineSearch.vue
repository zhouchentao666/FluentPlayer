<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { MusicInfo, SearchResult } from '@online/types/music'
import type { Playlist } from '@online/lib/playlists'
import type { Album } from '@online/lib/albums'
import { searchPlaylists } from '@online/lib/playlists/search'
import { searchAlbums } from '@online/lib/albums/search'
import { getHotSearch } from '@online/lib/hotSearch'
import {
  SINGLE_PLATFORMS,
  BROWSE_PLATFORMS,
  platformName,
} from '@online/platforms'
import OnlinePlatformTabs from './OnlinePlatformTabs.vue'
import OnlineCard from './OnlineCard.vue'
import OnlineSongRow from './OnlineSongRow.vue'

const emit = defineEmits<{
  (e: 'play', musics: MusicInfo[], index: number): void
  (e: 'queue', m: MusicInfo): void
  (e: 'add-playlist', m: MusicInfo): void
  (e: 'download', m: MusicInfo): void
  (e: 'open', item: Playlist | Album, kind: 'playlist' | 'album'): void
}>()

type Tab = 'song' | 'playlist' | 'album'

const tab = ref<Tab>('song')
const platform = ref<string>('wy')
const query = ref('')
const page = ref(1)
const loading = ref(false)
const error = ref('')
const hasSearched = ref(false)

const songs = ref<MusicInfo[]>([])
const cards = ref<(Playlist | Album)[]>([])
const allPages = ref(1)

const platformOptions = computed(() =>
  tab.value === 'song' ? SINGLE_PLATFORMS : BROWSE_PLATFORMS
)

const hotKeywords = ref<{ keyword: string; rank: number }[]>([])

const sourceForHot = computed(() => platform.value as 'wy' | 'kw' | 'kg' | 'tx' | 'mg')

async function loadHot() {
  try {
    hotKeywords.value = await getHotSearch(sourceForHot.value)
  } catch {
    hotKeywords.value = []
  }
}

watch(platform, () => {
  loadHot()
  if (hasSearched.value && query.value.trim()) runSearch(1)
})

watch(tab, () => {
  // txdesk only supports single-track search
  if (tab.value !== 'song' && platform.value === 'txdesk') {
    platform.value = 'tx'
  }
  cards.value = []
  songs.value = []
  hasSearched.value = false
  loadHot()
})

function runSearch(p = 1) {
  const q = query.value.trim()
  if (!q) return
  loading.value = true
  error.value = ''
  page.value = p
  hasSearched.value = true

  const source = platform.value as 'wy' | 'kw' | 'kg' | 'tx' | 'mg'

  if (tab.value === 'song') {
    const pdef = SINGLE_PLATFORMS.find((x) => x.id === platform.value)!
    pdef
      .search(q, p, 30)
      .then((res: SearchResult) => {
        songs.value = res.list
        allPages.value = res.allPage || 1
      })
      .catch((e: unknown) => {
        error.value = (e as Error).message || '搜索失败'
        songs.value = []
      })
      .finally(() => (loading.value = false))
  } else if (tab.value === 'playlist') {
    searchPlaylists(source, q, p, 30)
      .then((res) => {
        cards.value = res
        allPages.value = 1
      })
      .catch((e: unknown) => {
        error.value = (e as Error).message || '搜索失败'
        cards.value = []
      })
      .finally(() => (loading.value = false))
  } else {
    searchAlbums(source, q, p, 30)
      .then((res) => {
        cards.value = res
        allPages.value = 1
      })
      .catch((e: unknown) => {
        error.value = (e as Error).message || '搜索失败'
        cards.value = []
      })
      .finally(() => (loading.value = false))
  }
}

function onHot(kw: string) {
  query.value = kw
  runSearch(1)
}

function onPlaySong(m: MusicInfo) {
  const idx = songs.value.findIndex((x) => x.id === m.id)
  emit('play', songs.value, idx >= 0 ? idx : 0)
}

function onCardOpen(item: Playlist | Album) {
  emit('open', item, tab.value === 'album' ? 'album' : 'playlist')
}
</script>

<template>
  <div class="online-search">
    <div class="search-bar">
      <input
        v-model="query"
        class="search-input"
        type="text"
        placeholder="搜索歌曲、歌手、歌单或专辑…"
        @keyup.enter="runSearch(1)"
      />
      <button class="search-go" @click="runSearch(1)">搜索</button>
    </div>

    <div class="search-controls">
      <OnlinePlatformTabs v-model="platform" :options="platformOptions" />
      <div class="type-tabs">
        <button :class="{ active: tab === 'song' }" @click="tab = 'song'">单曲</button>
        <button :class="{ active: tab === 'playlist' }" @click="tab = 'playlist'">歌单</button>
        <button :class="{ active: tab === 'album' }" @click="tab = 'album'">专辑</button>
      </div>
    </div>

    <div v-if="!hasSearched" class="hot-section">
      <div class="hot-title">热门搜索 · {{ platformName(sourceForHot) }}</div>
      <div class="hot-cloud">
        <button
          v-for="kw in hotKeywords"
          :key="kw.keyword"
          class="hot-keyword"
          :style="{ fontSize: 13 + Math.min(kw.rank, 10) * 1.2 + 'px' }"
          @click="onHot(kw.keyword)"
        >
          {{ kw.keyword }}
        </button>
        <span v-if="!hotKeywords.length && !loading" class="hot-empty">暂无热搜</span>
      </div>
    </div>

    <div v-else class="results">
      <div v-if="loading" class="state">加载中…</div>
      <div v-else-if="error" class="state error">{{ error }}</div>
      <div v-else-if="tab === 'song' && songs.length === 0" class="state">未找到相关歌曲</div>
      <div v-else-if="tab !== 'song' && cards.length === 0" class="state">未找到相关内容</div>

      <template v-else>
        <div v-if="tab === 'song'" class="song-list">
          <OnlineSongRow
            v-for="(m, i) in songs"
            :key="m.id"
            :music="m"
            :index="i"
            @play="onPlaySong"
            @add-queue="emit('queue', $event)"
            @add-playlist="emit('add-playlist', $event)"
            @download="emit('download', $event)"
          />
        </div>

        <div v-else class="card-grid">
          <OnlineCard
            v-for="c in cards"
            :key="c.id"
            :item="c"
            :kind="tab === 'album' ? 'album' : 'playlist'"
            @open="onCardOpen"
          />
        </div>

        <div v-if="allPages > 1" class="pager">
          <button :disabled="page <= 1" @click="runSearch(page - 1)">上一页</button>
          <span>{{ page }} / {{ allPages }}</span>
          <button :disabled="page >= allPages" @click="runSearch(page + 1)">下一页</button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.online-search {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 18px 28px 28px;
  height: 100%;
  overflow-y: auto;
}
.search-bar {
  display: flex;
  gap: 10px;
  max-width: 560px;
}
.search-input {
  flex: 1;
  height: 38px;
  padding: 0 14px;
  border-radius: 10px;
  border: 1px solid var(--fluent-input-border);
  background: var(--fluent-input-bg);
  color: var(--fluent-text);
  font-size: 14px;
  outline: none;
}
.search-input:focus {
  border-color: var(--fluent-accent);
}
.search-go {
  height: 38px;
  padding: 0 20px;
  border: none;
  border-radius: 10px;
  background: var(--fluent-accent);
  color: #fff;
  font-size: 14px;
  cursor: pointer;
}
.search-controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.type-tabs {
  display: inline-flex;
  gap: 4px;
}
.type-tabs button {
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
}
.type-tabs button.active {
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
}
.hot-section {
  margin-top: 8px;
}
.hot-title {
  font-size: 13px;
  color: var(--fluent-text-secondary);
  margin-bottom: 12px;
}
.hot-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 14px;
}
.hot-keyword {
  border: none;
  background: transparent;
  color: var(--fluent-text);
  cursor: pointer;
  opacity: 0.92;
  font-weight: 600;
}
.hot-keyword:hover {
  color: var(--fluent-accent);
}
.hot-empty {
  color: var(--fluent-text-secondary);
  font-size: 13px;
}
.results {
  flex: 1;
  min-height: 0;
}
.state {
  color: var(--fluent-text-secondary);
  text-align: center;
  padding: 40px 0;
}
.state.error {
  color: #f87171;
}
.song-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 14px;
}
.pager {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  margin-top: 18px;
  color: var(--fluent-text-secondary);
  font-size: 13px;
}
.pager button {
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  padding: 6px 14px;
  border-radius: 8px;
  cursor: pointer;
}
.pager button:disabled {
  opacity: 0.4;
  cursor: default;
}
</style>
