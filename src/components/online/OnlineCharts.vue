<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import type { MusicInfo, OnlineSource } from '@online/types/music'
import { ALL_BOARDS, getBoardSongs } from '@online/lib/charts'
import OnlinePlatformTabs from './OnlinePlatformTabs.vue'
import OnlineSongRow from './OnlineSongRow.vue'

const emit = defineEmits<{
  (e: 'play', musics: MusicInfo[], index: number): void
  (e: 'queue', m: MusicInfo): void
  (e: 'add-playlist', m: MusicInfo): void
  (e: 'download', m: MusicInfo): void
  (e: 'add-all', musics: MusicInfo[]): void
  (e: 'download-all', musics: MusicInfo[]): void
}>()

const platform = ref<OnlineSource>('wy')
const boardId = ref<string>('')
const songs = ref<MusicInfo[]>([])
const loading = ref(false)
const error = ref('')

const boards = computed(() => ALL_BOARDS[platform.value] ?? [])
const currentBoardName = computed(() => boards.value.find((b) => b.id === boardId.value)?.name ?? '')

async function reload() {
  if (!boardId.value) {
    songs.value = []
    return
  }
  loading.value = true
  error.value = ''
  const requested = `${platform.value}:${boardId.value}`
  try {
    const list = await getBoardSongs(platform.value, boardId.value, 1)
    // 平台/榜单可能在请求期间被切换，丢弃过期响应
    if (requested !== `${platform.value}:${boardId.value}`) return
    songs.value = list
  } catch (e: unknown) {
    if (requested !== `${platform.value}:${boardId.value}`) return
    error.value = (e as Error).message || '加载失败'
    songs.value = []
  } finally {
    loading.value = false
  }
}

function pickFirstBoard() {
  boardId.value = boards.value[0]?.id ?? ''
}

onMounted(() => {
  pickFirstBoard()
  reload()
})

watch(platform, () => {
  pickFirstBoard()
  reload()
})
watch(boardId, reload)

function onPlay(m: MusicInfo) {
  const idx = songs.value.findIndex((x) => x.id === m.id)
  emit('play', songs.value, idx >= 0 ? idx : 0)
}
</script>

<template>
  <div class="charts-view">
    <div class="head">
      <h2 class="title">排行榜</h2>
      <OnlinePlatformTabs
        v-model="platform"
        :options="[
          { id: 'wy', name: '网易云' },
          { id: 'tx', name: 'QQ音乐' },
          { id: 'kg', name: '酷狗' },
          { id: 'kw', name: '酷我' },
          { id: 'mg', name: '咪咕' },
        ]"
      />
    </div>

    <div class="chart-body">
      <aside class="board-list">
        <button
          v-for="b in boards"
          :key="b.id"
          class="board"
          :class="{ active: boardId === b.id }"
          @click="boardId = b.id"
        >
          {{ b.name }}
        </button>
      </aside>

      <section class="board-content">
        <div v-if="songs.length && !loading" class="board-head">
          <div class="board-name">{{ currentBoardName }}</div>
          <div class="board-actions">
            <button class="ghost" @click="emit('add-all', songs)">收藏全部</button>
            <button class="ghost" @click="emit('download-all', songs)">下载全部</button>
            <button class="primary" @click="emit('play', songs, 0)">播放全部</button>
          </div>
        </div>

        <div v-if="loading" class="state">加载中…</div>
        <div v-else-if="error" class="state error">{{ error }}</div>
        <div v-else-if="!songs.length" class="state">暂无榜单数据</div>
        <div v-else class="song-list">
          <OnlineSongRow
            v-for="(m, i) in songs"
            :key="m.id"
            :music="m"
            :index="i"
            @play="onPlay"
            @add-queue="emit('queue', $event)"
            @add-playlist="emit('add-playlist', $event)"
            @download="emit('download', $event)"
          />
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.charts-view {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 18px 28px 20px;
  height: 100%;
  overflow: hidden;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  flex-shrink: 0;
}
.title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fluent-text);
  margin: 0;
}
.chart-body {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 18px;
}
.board-list {
  width: 168px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
  padding-right: 4px;
}
.board {
  text-align: left;
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  padding: 9px 12px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}
.board:hover {
  background: var(--fluent-bg-hover);
}
.board.active {
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
  font-weight: 600;
}
.board-content {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
}
.board-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}
.board-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--fluent-text);
}
.board-actions {
  display: flex;
  gap: 8px;
}
.ghost,
.primary {
  height: 32px;
  padding: 0 14px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
}
.ghost {
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
}
.ghost:hover {
  background: var(--fluent-bg-hover);
}
.primary {
  border: none;
  background: var(--fluent-accent);
  color: #fff;
  font-weight: 600;
}
.song-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.state {
  color: var(--fluent-text-secondary);
  text-align: center;
  padding: 40px 0;
}
.state.error {
  color: #f87171;
}
</style>
