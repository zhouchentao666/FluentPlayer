<template>
  <div class="skeleton-container" :class="type">
    <!-- 歌曲列表骨架 -->
    <template v-if="type === 'song-list'">
      <div v-for="(w, i) in widths" :key="i" class="skeleton-song-item">
        <div class="sk-index skeleton-block" />
        <div class="sk-info">
          <div class="sk-title skeleton-block" :style="{ width: w.title + '%' }" />
          <div class="sk-sub skeleton-block" :style="{ width: w.sub + '%' }" />
        </div>
        <div class="sk-duration skeleton-block" />
      </div>
    </template>

    <!-- 歌单卡片网格骨架 -->
    <template v-else-if="type === 'playlist-grid'">
      <div v-for="i in rows" :key="i" class="skeleton-playlist-card">
        <div class="sk-cover skeleton-block" />
        <div class="sk-card-info">
          <div class="sk-card-title skeleton-block" />
          <div class="sk-card-desc skeleton-block" />
        </div>
      </div>
    </template>

    <!-- 通用块骨架 -->
    <template v-else>
      <div v-for="i in rows" :key="i" class="skeleton-row">
        <div class="skeleton-block" :style="{ width: '100%', height: height + 'px' }" />
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  type?: 'song-list' | 'playlist-grid' | 'block'
  rows?: number
  height?: number
}>(), {
  type: 'block',
  rows: 8,
  height: 48
})

const widths = computed(() =>
  Array.from({ length: props.rows || 8 }, (_, i) => ({
    title: 60 + ((i * 17 + 5) % 30),
    sub: 40 + ((i * 13 + 7) % 30),
  }))
)
</script>

<style scoped>
.skeleton-container {
  width: 100%;
  transition: opacity 0.3s ease;
}

.skeleton-container.playlist-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  padding: 0;
}

.skeleton-block {
  background: linear-gradient(
    90deg,
    var(--td-bg-color-component) 25%,
    var(--td-bg-color-container) 50%,
    var(--td-bg-color-component) 75%
  );
  background-size: 200% 100%;
  will-change: background-position; animation: shimmer 1.4s ease infinite;
  will-change: background-position;
  border-radius: 6px;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

/* Song list skeleton */
.skeleton-song-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  gap: 12px;
}

.sk-index { width: 28px; height: 12px; flex-shrink: 0; }

.sk-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.sk-title { height: 14px; }
.sk-sub { height: 12px; }
.sk-duration { width: 40px; height: 12px; flex-shrink: 0; }

/* Playlist grid skeleton */
.playlist-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  width: 100%;
}

.skeleton-playlist-card {
  border-radius: 12px;
  overflow: hidden;
  background: var(--td-bg-color-container);
}

.sk-cover {
  width: 100%;
  aspect-ratio: 1;
}

.sk-card-info {
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sk-card-title { height: 14px; width: 70%; }
.sk-card-desc { height: 12px; width: 90%; }

/* block skeleton */
.skeleton-row {
  padding: 6px 0;
}
</style>
