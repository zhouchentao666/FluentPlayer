<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'

/** 在线歌单不支持批量选择，传入空集合即可。 */
const emptySet = new Set<string>()
import type { MusicInfo } from '@online/types/music'
import type { Song, Playlist as LocalPlaylist } from '../../types'
import type { PinnedOnlineItem } from '../../composables/useConfig'
import { getPlaylistDetail } from '@online/lib/playlists'
import { getAlbumDetail } from '@online/lib/albums'
import { musicInfoToSong } from '@online/player'
import PlaylistViewList from '../PlaylistViewList.vue'
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

const currentId = computed(() => props.currentSong?.id ?? '')
const cover = computed(() => (coverFailed.value ? null : info.value?.img ?? null))
/** 与本地歌单共用统一的歌曲列表组件 */
const songList = computed(() => list.value.map(musicInfoToSong))

// 「收藏歌单」：选择本地歌单后整张收藏
const collectMenu = ref<Song[] | null>(null)
function openCollectMenu() {
  collectMenu.value = songList.value
}
function confirmCollect(plId: string) {
  if (!collectMenu.value) return
  emit('add-all', plId, collectMenu.value)
  collectMenu.value = null
  toast('已添加到歌单', 'success')
}

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
    } else {
      list.value = [...list.value, ...res.list]
    }
    page.value = p
  } finally {
    loading.value = false
    loadingMore.value = false
  }
}

function onPlaySong(song: Song) {
  const idx = songList.value.findIndex((x) => x.id === song.id)
  emit('play', songList.value, idx >= 0 ? idx : 0)
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
          <div class="count">{{ list.length }} 首</div>
          <div class="buttons">
            <button class="play-all" @click="emit('play', songList, 0)">
              <svg viewBox="0 0 16 16" width="14" height="14"><path d="M4 2.5v11l9-5.5z" fill="currentColor"/></svg>
              播放全部
            </button>
            <button class="ghost" @click="openCollectMenu">收藏歌单</button>
            <button class="ghost" @click="emit('download-all', songList)">下载全部</button>
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

      <PlaylistViewList
        :songs="songList"
        :playlists="playlists"
        :current-song="currentSong"
        :playlist-id="`online-${source}-${id}`"
        :sort-mode="'custom'"
        :batch-mode="false"
        :selected-ids="emptySet"
        @play="onPlaySong"
        @add-to-queue="(s) => emit('queue', s)"
        @add-to-playlist="(pid, s) => emit('add-playlist', pid, s)"
      />

      <button class="load-more" :disabled="loadingMore" @click="load(page + 1, true)">
        {{ loadingMore ? '加载中…' : '加载更多' }}
      </button>

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
