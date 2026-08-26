<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { playSong } from '@/utils/audio/globaPlayList'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { storeToRefs } from 'pinia'

const { t } = useI18n()
const localUserStore = LocalUserDetailStore()
const playStatus = useGlobalPlayStatusStore()
const { list } = storeToRefs(localUserStore)

const recentList = computed(() => list.value.slice().reverse().slice(0, 200))

const handlePlay = (song: any) => {
  playStatus.updatePlayerInfo(song)
  playSong(song)
}

const formatDuration = (interval?: string) => interval || '--:--'
</script>

<template>
  <div class="recent-container">
    <div class="recent-header">
      <h2>{{ t('music.recent.title') }} <span class="count">({{ list.length }} {{ t('common.unitSongs') }})</span></h2>
    </div>

    <div v-if="recentList.length > 0" class="song-list">
      <div v-for="(song, index) in recentList" :key="song.songmid" class="song-row" @click="handlePlay(song)">
        <span class="song-index">{{ index + 1 }}</span>
        <div class="song-info">
          <span class="song-name">{{ song.name }}</span>
          <span class="song-singer">{{ song.singer }}</span>
        </div>
        <span class="song-duration">{{ formatDuration(song.interval) }}</span>
      </div>
    </div>
    <div v-else class="empty-state">
      <p>{{ t('music.recent.empty') }}</p>
    </div>
  </div>
</template>

<style scoped>
.recent-container { width: 100%; height: 100%; display: flex; flex-direction: column; overflow: hidden; padding: 20px; box-sizing: border-box; }
.recent-header { flex-shrink: 0; margin-bottom: 16px; }
.recent-header h2 { font-size: 20px; font-weight: 600; color: var(--td-text-color-primary); margin: 0; }
.count { font-size: 14px; font-weight: 400; color: var(--td-text-color-secondary); }
.song-list { flex: 1; overflow-y: auto; }
.song-row { display: flex; align-items: center; min-height: 44px; padding: 10px 12px; cursor: pointer; transition: background-color var(--motion-duration-instant) var(--motion-ease-standard); border-radius: 6px; box-sizing: border-box; }
.song-row:hover { background: var(--td-bg-color-component-hover); }
.song-index { width: 32px; font-size: 12px; color: var(--td-text-color-secondary); flex-shrink: 0; }
.song-info { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.song-name { font-size: 14px; color: var(--td-text-color-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.song-singer { font-size: 12px; color: var(--td-text-color-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.song-duration { font-size: 12px; color: var(--td-text-color-secondary); flex-shrink: 0; margin-left: 12px; }
.empty-state { display: flex; align-items: center; justify-content: center; min-height: 300px; }
.empty-state p { color: var(--td-text-color-secondary); }

@media (max-width: 768px) {
  .recent-container {
    padding: var(--mobile-page-top-gutter) var(--mobile-page-gutter) 0;
  }

  .recent-header {
    margin-bottom: 1rem;
  }

  .recent-header h2 {
    font-size: clamp(2rem, 9vw, 2.6rem);
    line-height: 1.1;
    letter-spacing: -0.04em;
  }

  .count {
    display: block;
    padding-top: 0.35rem;
    font-size: 1rem;
  }

  .song-list {
    min-height: 0;
    -webkit-overflow-scrolling: touch;
  }

  .song-row {
    min-height: 56px;
    padding: 8px 10px;
    border-radius: var(--mobile-card-radius-small);
    touch-action: manipulation;
  }

  .song-index {
    width: 28px;
  }

  .song-name {
    font-size: 15px;
  }

  .song-duration {
    margin-left: 8px;
  }
}
</style>
