<script lang="ts" setup>
import { type Song } from '../types'
import { ref, computed, inject, type Ref } from 'vue'
import ProgressBar from './player/ProgressBar.vue'
import SongInfo from './player/SongInfo.vue'
import PlayerControls, { type PlayMode } from './player/PlayerControls.vue'
import VolumeControl from './player/VolumeControl.vue'
import PlaybackRateControl from './player/PlaybackRateControl.vue'
import AudioVisualizer from './player/AudioVisualizer.vue'
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

// 音频可视化（注入 App 提供的 AnalyserNode 与 settings）
const audioAnalyser = inject<Ref<AnalyserNode | null> | null>('audioAnalyser', null)
const settings = inject<any>('settings', null)
const visualizerOn = computed(() => Boolean(settings?.value?.audioVisualizer))

const qualityLevels = computed<Quality[]>(() => {
  const src = props.currentSong?.online?.source
  return src ? qualityLevelsFor(src) : []
})
const qualityShort = computed(() => {
  const q = activeQuality.value ?? '128k'
  return QUALITY_SHORT[q as Quality] || q
})

const qualityOpen = ref(false)
const qualityMenuRef = ref<HTMLElement | null>(null)

function toggleQuality() {
  qualityOpen.value = !qualityOpen.value
}

function closeQuality() {
  qualityOpen.value = false
}

function selectQuality(q: Quality) {
  emit('change-quality', q)
  qualityOpen.value = false
}

function formatDuration(seconds: number): string {
  if (!seconds || seconds < 0) return '0:00'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

// 点击外部关闭音质菜单
function handleClickOutside(e: MouseEvent) {
  if (qualityOpen.value && qualityMenuRef.value && !qualityMenuRef.value.contains(e.target as Node)) {
    closeQuality()
  }
}

if (typeof window !== 'undefined') {
  window.addEventListener('click', handleClickOutside)
}
</script>

<template>
  <footer
    class="player-footer"
    :class="{ 'detail-mode': showDetail, immersive: immersive }"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <div v-if="visualizerOn" class="footer-visualizer">
      <AudioVisualizer
        v-if="audioAnalyser"
        :analyser="audioAnalyser.value"
        :playing="isPlaying"
        :bars="56"
        height="32px"
      />
      <AudioVisualizer v-else :playing="false" :bars="56" height="32px" />
    </div>

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
        <div v-if="isOnline" class="quality-wrap" ref="qualityMenuRef">
          <button class="side-btn quality-btn" :class="{ active: qualityOpen }" title="音质" @click.stop="toggleQuality">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 12h3l2-7 4 14 3-9 2 4h4" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            <span class="quality-tag">{{ qualityShort }}</span>
          </button>
          <Transition name="flyout">
            <div v-if="qualityOpen" class="quality-pop">
              <div class="quality-pop-title">音质</div>
              <button
                v-for="q in qualityLevels"
                :key="q"
                class="quality-item"
                :class="{ active: q === activeQuality }"
                @click="selectQuality(q)"
              >
                {{ QUALITY_SHORT[q] || q }}
                <span v-if="q === activeQuality" class="quality-check">✓</span>
              </button>
            </div>
          </Transition>
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

/* 底部播放条音频可视化：作为背景层覆盖整条 footer，不拦截交互 */
.footer-visualizer {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: -1;
  pointer-events: none;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.5;
  mask-image: linear-gradient(to right, transparent, black 12%, black 88%, transparent);
  -webkit-mask-image: linear-gradient(to right, transparent, black 12%, black 88%, transparent);
}
.player-footer.detail-mode .footer-visualizer {
  opacity: 0.85;
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
  position: relative;
  display: inline-flex;
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

.quality-tag {
  margin-left: 4px;
  font-size: 11px;
  letter-spacing: 0.2px;
}

.quality-pop {
  position: absolute;
  right: 0;
  bottom: calc(100% + 8px);
  z-index: 200;
  min-width: 120px;
  padding: 4px;
  box-sizing: border-box;
  border: 1px solid var(--fluent-border);
  border-radius: 8px;
  background: var(--fluent-bg-card);
  backdrop-filter: blur(30px) saturate(160%);
  -webkit-backdrop-filter: blur(30px) saturate(160%);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.28);
  transform-origin: bottom right;
  animation: quality-pop-in 0.16s cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes quality-pop-in {
  from {
    opacity: 0;
    transform: scale(0.94) translateY(6px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.flyout-enter-active,
.flyout-leave-active {
  transition: opacity 0.16s ease, transform 0.16s cubic-bezier(0.16, 1, 0.3, 1);
}

.flyout-enter-from,
.flyout-leave-to {
  opacity: 0;
  transform: scale(0.94) translateY(6px);
}

.quality-pop-title {
  padding: 6px 12px 4px;
  font-size: 11px;
  font-weight: 600;
  color: var(--fluent-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}

.quality-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 32px;
  padding: 5px 11px 5px 12px;
  border-radius: 5px;
  color: var(--fluent-text);
  font-size: 13px;
  line-height: 20px;
  cursor: pointer;
  white-space: nowrap;
  user-select: none;
  background: transparent;
  border: none;
  width: 100%;
  text-align: left;
  transition: color 0.12s ease, background-color 0.1s ease;
}

.quality-item:hover {
  color: var(--fluent-accent);
  background: var(--fluent-bg-hover);
}

.quality-item.active {
  color: var(--fluent-accent);
  font-weight: 600;
  background: var(--fluent-bg-active);
}

.quality-check {
  margin-left: auto;
  font-size: 12px;
  color: var(--fluent-accent);
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