<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
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
const hotLoading = ref(false)
const hotError = ref('')

const sourceForHot = computed(() => platform.value as 'wy' | 'kw' | 'kg' | 'tx' | 'mg')

async function loadHot() {
  const requested = sourceForHot.value
  hotLoading.value = true
  hotError.value = ''
  try {
    const list = await getHotSearch(requested)
    if (requested !== sourceForHot.value) return
    hotKeywords.value = list
  } catch (e) {
    if (requested !== sourceForHot.value) return
    hotKeywords.value = []
    hotError.value = (e as Error).message || '热搜加载失败'
  } finally {
    if (requested === sourceForHot.value) hotLoading.value = false
  }
}

// 之前只在 platform / tab 变化时才拉热搜，首次进入搜索页永远是空的
onMounted(loadHot)

// ---- 历史搜索记录 ----
const HISTORY_KEY = 'fluentplayer-online:searchHistory'
const HISTORY_MAX = 20
const history = ref<string[]>(loadHistory())

function loadHistory(): string[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY)
    const arr = raw ? JSON.parse(raw) : []
    return Array.isArray(arr) ? arr.filter((x) => typeof x === 'string').slice(0, HISTORY_MAX) : []
  } catch {
    return []
  }
}
function saveHistory() {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(history.value))
  } catch {
    /* 隐私模式下写入可能失败，忽略 */
  }
}
function pushHistory(q: string) {
  history.value = [q, ...history.value.filter((x) => x !== q)].slice(0, HISTORY_MAX)
  saveHistory()
}
function removeHistory(q: string) {
  history.value = history.value.filter((x) => x !== q)
  saveHistory()
}
function clearHistory() {
  history.value = []
  saveHistory()
}
function backToDiscover() {
  hasSearched.value = false
  query.value = ''
  songs.value = []
  cards.value = []
  error.value = ''
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
  pushHistory(q)

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

    <div v-if="!hasSearched" class="discover">
      <div v-if="history.length" class="hot-section">
        <div class="hot-title">
          <span>历史搜索</span>
          <button class="link-btn" @click="clearHistory">清空</button>
        </div>
        <div class="history-cloud">
          <span v-for="h in history" :key="h" class="history-chip" @click="onHot(h)">
            <span class="history-text">{{ h }}</span>
            <button class="history-del" title="删除该记录" @click.stop="removeHistory(h)">
              <svg viewBox="0 0 12 12" width="10" height="10"><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round"/></svg>
            </button>
          </span>
        </div>
      </div>

      <div class="hot-section">
        <div class="hot-title">
          <span>热门搜索 · {{ platformName(sourceForHot) }}</span>
          <button class="link-btn" :disabled="hotLoading" @click="loadHot">刷新</button>
        </div>
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
          <span v-if="hotLoading" class="hot-empty">热搜加载中…</span>
          <span v-else-if="hotError" class="hot-empty error">{{ hotError }}</span>
          <span v-else-if="!hotKeywords.length" class="hot-empty">暂无热搜</span>
        </div>
      </div>
    </div>

    <div v-else class="results">
      <button class="link-btn back-btn" @click="backToDiscover">← 返回热搜 / 历史</button>
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
.discover {
  display: flex;
  flex-direction: column;
  gap: 22px;
  margin-top: 8px;
}
.hot-section {
  margin-top: 0;
}
.hot-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 13px;
  color: var(--fluent-text-secondary);
  margin-bottom: 12px;
}
.link-btn {
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  font-size: 12px;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 6px;
}
.link-btn:hover:not(:disabled) {
  color: var(--fluent-accent);
  background: var(--fluent-bg-hover);
}
.link-btn:disabled {
  opacity: 0.5;
  cursor: default;
}
.back-btn {
  margin-bottom: 10px;
}
.history-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.history-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 5px 8px 5px 12px;
  border-radius: 999px;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
  font-size: 12.5px;
  color: var(--fluent-text);
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
}
.history-chip:hover {
  background: var(--fluent-bg-hover);
  border-color: var(--fluent-accent);
}
.history-del {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: var(--fluent-text-secondary);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s ease, background 0.15s ease;
}
.history-chip:hover .history-del {
  opacity: 1;
}
.history-del:hover {
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
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
.hot-empty.error {
  color: #f87171;
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
