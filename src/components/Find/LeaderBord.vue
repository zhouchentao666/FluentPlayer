<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { musicSdk } from '@/services/musicSdk'
import { useSourceAccess } from '@/composables/useSourceAccess'
import LeaderBordCard from './LeaderBordCard.vue'

const { t } = useI18n()
const boards = ref<any[]>([])
const loading = ref(true)
const router = useRouter()
const localUserStore = LocalUserDetailStore()
const { enabledSourceKeys } = useSourceAccess()

const currentSource = computed(() => localUserStore.userSource.source)
let boardsCache: { source: string; data: any[] } | null = null

const fetchBoards = async (force = false) => {
  if (enabledSourceKeys.value.size === 0) {
    boards.value = []
    loading.value = false
    return
  }
  if (!force && boardsCache && boardsCache.source === currentSource.value) {
    boards.value = boardsCache.data
    loading.value = false
    return
  }
  loading.value = true
  try {
    const res = await musicSdk.getLeaderboards()
    const list = Array.isArray(res?.list) ? res.list : Array.isArray(res) ? res : []
    boards.value = list
    boardsCache = { source: currentSource.value || '', data: list }
  } catch (e) {
    console.error('获取排行榜失败:', e)
    boards.value = []
  } finally {
    loading.value = false
  }
}

const handleCardClick = (board: any) => {
  router.push({
    name: 'list',
    params: { id: board.board_id || board.id },
    query: {
      title: board.name,
      cover: board.pic || board.img || '',
      source: board.source || currentSource.value,
      isLeaderboard: 'true'
    }
  })
}

watch(() => localUserStore.userSource.source, () => fetchBoards(true))
onMounted(() => fetchBoards())
</script>

<template>
  <div class="leaderboard-container">
    <div class="section-header">
      <div class="title">{{ t('music.leaderboard.title') }}</div>
    </div>

    <div v-if="loading" class="board-grid">
      <div v-for="n in 24" :key="n" class="skeleton-card">
        <div class="skeleton-block"></div>
      </div>
    </div>

    <div v-else-if="boards.length > 0" class="board-grid">
      <LeaderBordCard
        v-for="board in boards"
        :key="board.id || board.board_id"
        :data="board"
        @click="handleCardClick"
      />
    </div>

    <div v-else class="empty-state">
      <div class="empty-text">{{ t('music.leaderboard.noData') }}</div>
      <t-button variant="text" @click="fetchBoards(true)">{{ t('music.leaderboard.retry') }}</t-button>
    </div>
  </div>
</template>

<style scoped>
.leaderboard-container {
  min-height: 100%;
  padding: 0 2rem 1rem;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 1rem;
}

.section-header .title {
  font-size: 20px;
  font-weight: 700;
  color: var(--td-text-color-primary);
}

.board-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 24px;
}

.skeleton-card {
  width: 100%;
  aspect-ratio: 1;
  border-radius: 12px;
  overflow: hidden;
  background: var(--td-bg-color-secondarycontainer);
}

.skeleton-block {
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, rgba(0, 0, 0, 0.06) 25%, rgba(0, 0, 0, 0.12) 37%, rgba(0, 0, 0, 0.06) 63%);
  background-size: 400% 100%;
  will-change: background-position; animation: shimmer 1.4s ease infinite;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 300px;
  color: var(--td-text-color-secondary);
}

.empty-text {
  margin-bottom: 12px;
}

@keyframes shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

@media (max-width: 768px) {
  .leaderboard-container {
    padding: 0 0.75rem 1rem;
  }

  .board-grid {
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
  }

  .section-header .title {
    font-size: 17px;
  }
}
</style>
