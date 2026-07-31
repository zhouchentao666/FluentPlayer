<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import type { MusicInfo } from '@online/types/music'
import type { Song } from '../../types'
import type { PinnedOnlineItem } from '../../composables/useConfig'
import { getPlaylistDetail } from '@online/lib/playlists'
import { getAlbumDetail } from '@online/lib/albums'
import OnlineSongRow from './OnlineSongRow.vue'

const props = defineProps<{
  source: 'wy' | 'kw' | 'kg' | 'tx' | 'mg'
  id: string
  kind: 'playlist' | 'album'
  currentSong: Song | null
  pinned?: boolean
}>()

const emit = defineEmits<{
  (e: 'play', musics: MusicInfo[], index: number): void
  (e: 'queue', m: MusicInfo): void
  (e: 'add-playlist', m: MusicInfo): void
  (e: 'download', m: MusicInfo): void
  (e: 'add-all', musics: MusicInfo[]): void
  (e: 'download-all', musics: MusicInfo[]): void
  (e: 'toggle-pin', item: PinnedOnlineItem): void
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
const noMore = ref(false)
const loadedIds = new Set<string>()

const currentId = computed(() => props.currentSong?.online?.id ?? '')
const cover = computed(() => (coverFailed.value ? null : info.value?.img ?? null))

async function load(p: number, append = false) {
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
      loadedIds.clear()
      for (const m of res.list) loadedIds.add(m.id)
      noMore.value = res.list.length === 0
    } else {
      const fresh = res.list.filter((m) => !loadedIds.has(m.id))
      for (const m of fresh) loadedIds.add(m.id)
      list.value = [...list.value, ...fresh]
      noMore.value = fresh.length === 0
    }
    page.value = p
  } finally {
    loading.value = false
    loadingMore.value = false
  }
}

function onPlay(m: MusicInfo) {
  const idx = list.value.findIndex((x) => x.id === m.id)
  emit('play', list.value, idx >= 0 ? idx : 0)
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

onMounted(() => load(1))
</script>

<template>
  <div class="detail">
    <div v-if="loading" class="state">加载中…</div>
    <template v-else-if="info">
      <div class="header">
        <div class="cover-wrap">
          <img v-if="cover" :src="cover" class="cover" alt="" @error="coverFailed = true" />
          <div v-else class="cover placeholder">{{ (info.name || '?').charAt(0) }}</div>
        </div>
        <div class="info">
          <div class="kind-tag">{{ kind === 'album' ? '专辑' : '歌单' }}</div>
          <h1 class="name">{{ info.name }}</h1>
          <div v-if="info.author" class="author">{{ info.author }}</div>
          <div class="count">{{ list.length }} 首</div>
          <div class="buttons">
            <button class="play-all" @click="emit('play', list, 0)">
              <svg viewBox="0 0 16 16" width="14" height="14"><path d="M4 2.5v11l9-5.5z" fill="currentColor"/></svg>
              播放全部
            </button>
            <button class="ghost" @click="emit('add-all', list)">收藏歌单</button>
            <button class="ghost" @click="emit('download-all', list)">下载全部</button>
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
          </div>
        </div>
      </div>

      <div class="song-list">
        <OnlineSongRow
          v-for="(m, i) in list"
          :key="m.id"
          :music="m"
          :index="i"
          :current="m.id === currentId"
          @play="onPlay"
          @add-queue="emit('queue', $event)"
          @add-playlist="emit('add-playlist', $event)"
          @download="emit('download', $event)"
        />
      </div>

      <button
        v-if="!noMore"
        class="load-more"
        :disabled="loadingMore"
        @click="load(page + 1, true)"
      >
        {{ loadingMore ? '加载中…' : '加载更多' }}
      </button>
      <div v-else class="state">没有更多了</div>
    </template>
  </div>
</template>

<style scoped>
.detail {
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 22px 28px 28px;
  height: 100%;
  overflow-y: auto;
}
.state {
  color: var(--fluent-text-secondary);
  text-align: center;
  padding: 40px 0;
}
.header {
  display: flex;
  gap: 22px;
}
.cover-wrap {
  width: 180px;
  height: 180px;
  flex-shrink: 0;
  border-radius: 14px;
  overflow: hidden;
  background: var(--fluent-bg-card);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
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
  font-size: 56px;
  font-weight: 700;
  color: var(--fluent-text-secondary);
  background: linear-gradient(135deg, var(--fluent-bg-active), var(--fluent-bg-card));
}
.info {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}
.kind-tag {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.name {
  font-size: 26px;
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
  gap: 12px;
  margin-top: 8px;
}
.play-all {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 38px;
  padding: 0 22px;
  border: none;
  border-radius: 10px;
  background: var(--fluent-accent);
  color: #fff;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
}
.ghost {
  height: 38px;
  padding: 0 18px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 14px;
  cursor: pointer;
}
.ghost:hover {
  background: var(--fluent-bg-hover);
}
.song-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.load-more {
  align-self: center;
  margin-top: 8px;
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  padding: 8px 22px;
  border-radius: 10px;
  cursor: pointer;
}
.load-more:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
