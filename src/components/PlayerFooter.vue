<script lang="ts" setup>
import { ref } from 'vue'
import { type Song } from '../types'
import { QUALITY_LADDER, QUALITY_SHORT, type Quality } from '@online/player'
import ProgressBar from './player/ProgressBar.vue'
import SongInfo from './player/SongInfo.vue'
import PlayerControls, { type PlayMode } from './player/PlayerControls.vue'
import VolumeControl from './player/VolumeControl.vue'
import PlaybackRateControl from './player/PlaybackRateControl.vue'

defineProps<{
  currentSong: Song | null
  coverUrl: string | null
  isPlaying: boolean
  currentTime: number
  duration: number
  volume: number
  playbackRate: number
  showDetail?: boolean
  playMode: PlayMode
  immersive?: boolean
  desktopLyricEnabled?: boolean
  quality?: Quality | null
  availableQualities?: Quality[]
}>()

const isHovered = ref(false)
const qualityPop = ref(false)

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
  (e: 'quality-change', q: Quality): void
}>()

function formatDuration(seconds: number): string {
  if (!seconds || seconds < 0) return '0:00'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

/** 按音质阶梯从低到高排列可切换选项。 */
function qualityOptions(available?: Quality[]): Quality[] {
  const set = new Set(available ?? [])
  return QUALITY_LADDER.filter((q) => set.has(q))
}
function pickQuality(q: Quality) {
  qualityPop.value = false
  emit('quality-change', q)
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
          :play-mode="playMode"
          @toggle-play="emit('toggle-play')"
          @prev="emit('prev')"
          @next="emit('next')"
          @cycle-mode="emit('cycle-mode')"
          @toggle-queue="emit('toggle-queue')"
        />
      </div>

      <div class="section right" :class="{ faded: immersive && !isHovered }">
        <button
          v-if="currentSong?.online && (availableQualities?.length ?? 0) > 0"
          class="quality-btn"
          :class="{ active: qualityPop }"
          @click="qualityPop = !qualityPop"
        >
          {{ quality ? QUALITY_SHORT[quality] : '' }}
          <div v-if="qualityPop" class="quality-pop" @click.stop>
            <div
              v-for="q in qualityOptions(availableQualities)"
              :key="q"
              class="quality-item"
              :class="{ active: q === quality }"
              @click="pickQuality(q)"
            >
              {{ QUALITY_SHORT[q] }}
            </div>
          </div>
        </button>
        <span class="time-label">{{ formatDuration(currentTime) }} / {{ formatDuration(duration || 0) }}</span>
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

.quality-btn {
  min-width: 40px;
  height: 28px;
  padding: 0 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--fluent-border);
  border-radius: 14px;
  background: transparent;
  color: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
}

.quality-btn:hover {
  background: var(--fluent-bg-hover);
}

.quality-btn.active {
  color: var(--fluent-accent);
  border-color: var(--fluent-accent);
}

.player-footer.detail-mode .quality-btn {
  color: rgba(255, 255, 255, 0.9);
}

.player-footer.detail-mode .quality-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.1);
}

.player-footer.detail-mode .quality-btn.active {
  color: #fff;
  border-color: rgba(255, 255, 255, 0.6);
}

.quality-pop {
  z-index: 200;
  min-width: 96px;
  padding: 6px;
  background: var(--fluent-bg-glass);
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
  backdrop-filter: blur(20px) saturate(140%);
  -webkit-backdrop-filter: blur(20px) saturate(140%);
}

.quality-item {
  padding: 7px 12px;
  border-radius: 6px;
  font-size: 13px;
  color: var(--fluent-text);
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s ease;
}

.quality-item:hover {
  background: var(--fluent-bg-hover);
}

.quality-item.active {
  color: var(--fluent-accent);
  font-weight: 600;
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