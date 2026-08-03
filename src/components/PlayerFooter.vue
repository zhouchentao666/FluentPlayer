<script lang="ts" setup>
import { type Song } from '../types'
import { ref, computed } from 'vue'
import ProgressBar from './player/ProgressBar.vue'
import SongInfo from './player/SongInfo.vue'
import PlayerControls, { type PlayMode } from './player/PlayerControls.vue'
import VolumeControl from './player/VolumeControl.vue'
import PlaybackRateControl from './player/PlaybackRateControl.vue'
import ComboBox from './settings/ComboBox.vue'
import { activeQuality } from '@online/player'
import { qualityLevelsFor, QUALITY_SHORT } from '@online/lib/quality'
import type { Quality } from '@online/types/music'

const props = defineProps<{
  currentSong: Song | null
  coverUrl: string | null
  isPlaying: boolean
  loading?: boolean
  currentTime: number
  duration: number
  volume: number
  playbackRate: number
  showDetail?: boolean
  playMode: PlayMode
  immersive?: boolean
  desktopLyricEnabled?: boolean
}>()

const isHovered = ref(false)

const emit = defineEmits<{
  (e: 'toggle-play'): void
  (e: 'prev'): void
  (e: 'next'): void
  (e: 'seek', time: number): void
  (e: 'set-volume', volume: number): void
  (e: 'set-playback-rate', rate: number): void
  (e: 'open-detail'): void
  (e: 'cycle-mode'): void
  (e: 'toggle-queue'): void
  (e: 'toggle-desktop-lyric'): void
  (e: 'comment'): void
  (e: 'change-quality', q: Quality): void
}>()

// 在线音乐才显示音质 / 评论
const isOnline = computed(() => !!props.currentSong?.online)

const qualityLevels = computed<Quality[]>(() => {
  const src = props.currentSong?.online?.source
  return src ? qualityLevelsFor(src) : []
})

const qualityOptions = computed(() =>
  qualityLevels.value.map(q => ({ value: q, label: QUALITY_SHORT[q] || q }))
)

function selectQuality(q: string) {
  emit('change-quality', q as Quality)
}

function formatDuration(seconds: number): string {
  if (!seconds || seconds < 0) return '0:00'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}
</script>

<template>
  <footer
    class="player-footer"
    :class="{ 'detail-mode': showDetail, immersive: immersive }"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <ProgressBar
      :current-time="currentTime"
      :duration="duration"
      @seek="time => emit('seek', time)"
    />

    <div class="footer-content">
      <div class="section left" @click="emit('open-detail')">
        <SongInfo
          :song="currentSong"
          :cover-url="coverUrl"
          :show-detail="showDetail"
        />
      </div>

      <div class="section center" :class="{ faded: immersive && !isHovered }">
        <PlayerControls
          :is-playing="isPlaying"
          :loading="loading"
          :play-mode="playMode"
          @toggle-play="emit('toggle-play')"
          @prev="emit('prev')"
          @next="emit('next')"
          @cycle-mode="emit('cycle-mode')"
          @toggle-queue="emit('toggle-queue')"
        />
      </div>

      <div class="section right" :class="{ faded: immersive && !isHovered }">
        <span class="time-label">{{ formatDuration(currentTime) }} / {{ formatDuration(duration || 0) }}</span>

        <!-- 音质切换（仅在线音乐） -->
        <div v-if="isOnline" class="quality-wrap">
          <ComboBox
            width="80px"
            aria-label="音质"
            :options="qualityOptions"
            :model-value="activeQuality ?? '128k'"
            @update:model-value="selectQuality"
          />
        </div>

        <!-- 评论（仅在线音乐） -->
        <button
          v-if="isOnline"
          class="side-btn comment-btn"
          title="评论"
          @click="emit('comment')"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" />
          </svg>
        </button>

        <button
          class="side-btn lyric-btn"
          :class="{ active: desktopLyricEnabled }"
          title="桌面歌词"
          @click="emit('toggle-desktop-lyric')"
        >
          <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M4 6h12v2H4V6zm0 5h16v2H4v-2zm0 5h10v2H4v-2z" />
          </svg>
        </button>
        <VolumeControl :volume="volume" @set-volume="v => emit('set-volume', v)" />
        <PlaybackRateControl :playback-rate="playbackRate" @set-playback-rate="r => emit('set-playback-rate', r)" />
      </div>
    </div>
  </footer>
</template>

<style scoped>
.player-footer {
  height: 72px;
  flex-shrink: 0;
  position: relative;
  z-index: 60;
  display: flex;
  flex-direction: column;
  color: var(--fluent-text);
  background: var(--fluent-bg-player);
  border-top: 1px solid var(--fluent-border);
  backdrop-filter: none;
  user-select: none;
  transition: color 500ms ease, background-color 500ms ease, border-color 500ms ease;
}

.player-footer.detail-mode {
  color: white;
  background: transparent;
  border-top-color: transparent;
  backdrop-filter: none;
}

.footer-content {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  gap: 16px;
}

.section {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.section.left {
  flex: 1;
  cursor: pointer;
}

.section.center {
  flex: 0 0 auto;
  justify-content: center;
}

.section.right {
  flex: 1;
  justify-content: flex-end;
}

.player-footer.detail-mode .time-label {
  color: rgba(255, 255, 255, 0.9);
}

.player-footer.detail-mode :deep(.side-btn) {
  color: rgba(255, 255, 255, 0.9);
}

.player-footer.detail-mode :deep(.side-btn:hover) {
  color: #fff;
  background: rgba(255, 255, 255, 0.1);
}

.time-label {
  font-size: 11px;
  color: var(--fluent-text-secondary);
  font-variant-numeric: tabular-nums;
  transition: color 500ms ease;
}

.lyric-btn {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: inherit;
  cursor: pointer;
  transition: background 0.18s ease, transform 0.1s ease, color 0.18s ease;
}

.lyric-btn:hover {
  background: var(--fluent-bg-hover);
}

.lyric-btn.active {
  color: var(--fluent-accent);
}

.lyric-btn svg {
  width: 22px;
  height: 22px;
}

.quality-wrap {
  display: inline-flex;
  align-items: center;
}

/* 播放栏中的 ComboBox 样式微调 */
.quality-wrap :deep(.win-combo) {
  min-height: 28px;
  padding: 3px 8px 3px 10px;
  font-size: 12px;
  font-weight: 600;
  border-radius: 14px;
}

.quality-wrap :deep(.win-combo-text) {
  font-variant-numeric: tabular-nums;
}

.comment-btn {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: inherit;
  cursor: pointer;
  transition: background 0.18s ease, transform 0.1s ease, color 0.18s ease;
}

.comment-btn:hover {
  background: var(--fluent-bg-hover);
}

.comment-btn.active {
  color: var(--fluent-accent);
}

.comment-btn svg {
  width: 20px;
  height: 20px;
}

.player-footer.detail-mode .lyric-btn {
  color: rgba(255, 255, 255, 0.9);
}

.player-footer.detail-mode .lyric-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.1);
}

.section.center,
.section.right {
  transition: opacity 300ms ease;
}

.player-footer.immersive .section.center.faded,
.player-footer.immersive .section.right.faded {
  opacity: 0;
  pointer-events: none;
}
</style>