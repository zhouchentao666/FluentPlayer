<script lang="ts" setup>
import { computed, ref, watch, onMounted } from 'vue'
import PlayerDetailBackground from './PlayerDetailBackground.vue'
import PlayerDetailLeft from './PlayerDetailLeft.vue'
import PlayerDetailTopChrome from './PlayerDetailTopChrome.vue'
import PlayerDetailLyrics from './PlayerDetailLyrics.vue'
import { usePlayerDetail } from '../../composables/usePlayerDetail'
import type { Song } from '../../types'
import type { LyricLine } from '../../composables/useLyrics'

const props = defineProps<{
  show: boolean
  currentSong: Song | null
  coverUrl: string | null
  isPlaying: boolean
  lyrics: LyricLine[]
  hasLyrics: boolean
  currentTime: number
  backgroundMode?: 'static' | 'dynamic'
  immersivePlayerBar?: boolean
  coverTransition?: 'fade' | 'slide-left' | 'slide-both'
  hideLyrics?: boolean
  fullScreenStyle?: 'classic' | 'am'
}>()

const emit = defineEmits<{
  close: []
  seek: [time: number]
}>()

const {
  isTopChromeVisible,
  isMaximised,
  isFullscreen,
  isAlwaysOnTop,
  showTopChrome,
  handleTopChromeLeave,
  runStaggerEnter,
  runStaggerLeave,
  updateMaximizeState,
  staggerStyle,
  minimize,
  toggleMaximize,
  toggleFullscreen,
  toggleAlwaysOnTop,
  closeApp,
} = usePlayerDetail(computed(() => props.immersivePlayerBar ?? false))

const showLyrics = ref(false)
const positionLyrics = ref(false)

const handleClose = () => emit('close')

onMounted(() => {
  updateMaximizeState()
})

function toggleLyrics() {
  if (!props.hasLyrics) return
  showLyrics.value = !showLyrics.value
  positionLyrics.value = showLyrics.value
}

watch(() => props.hasLyrics, (hasLyrics) => {
  if (props.show) {
    showLyrics.value = hasLyrics
    positionLyrics.value = hasLyrics
  }
})

// 纯 CSS 驱动：show 切换直接改 isExpanded，CSS transition 自动从当前插值位置继续
// 天然支持多次打断，无需 JS FLIP 逻辑
watch(() => props.show, (visible) => {
  if (visible) {
    showLyrics.value = props.hasLyrics
    positionLyrics.value = props.hasLyrics
    isTopChromeVisible.value = true
    showTopChrome()
    runStaggerEnter()
  } else {
    showLyrics.value = false
    positionLyrics.value = false
    isTopChromeVisible.value = false
    runStaggerLeave()
  }
})
</script>

<template>
  <div
    class="player-detail"
    :class="{ visible: props.show, am: props.fullScreenStyle === 'am' && props.show }"
  >
    <div class="player-inner">
      <div
        class="bg-wrapper"
        :class="{ visible: props.show }"
      >
        <PlayerDetailBackground
          :cover-url="props.coverUrl"
          :active="props.show"
          :background-mode="props.backgroundMode ?? 'static'"
          :has-lyrics="props.hasLyrics"
        />
        <div class="bg-fallback"></div>
      </div>

      <PlayerDetailTopChrome
        :is-visible="props.show"
        :is-top-chrome-visible="isTopChromeVisible"
        :is-maximised="isMaximised"
        :is-fullscreen="isFullscreen"
        :is-always-on-top="isAlwaysOnTop"
        :stagger-style="(phase, dir, dist) => staggerStyle(props.show, phase, dir, dist)"
        @close="handleClose"
        @minimize="minimize"
        @toggle-maximize="toggleMaximize"
        @toggle-fullscreen="toggleFullscreen"
        @toggle-always-on-top="toggleAlwaysOnTop"
        @close-app="closeApp"
        @show-top-chrome="showTopChrome"
        @top-chrome-leave="handleTopChromeLeave"
      />

      <!-- AM（Apple Music）风格：左侧大封面 + 右侧逐字歌词 -->
      <div
        v-if="props.fullScreenStyle === 'am' && props.show"
        class="am-body"
      >
        <div class="am-cover">
          <div class="am-cover-art">
            <img
              v-if="props.coverUrl"
              :src="props.coverUrl"
              :alt="props.currentSong?.title"
              draggable="false"
            />
          </div>
          <div class="am-meta">
            <div class="am-title" :title="props.currentSong?.title">{{ props.currentSong?.title }}</div>
            <div class="am-artist" :title="props.currentSong?.online?.singer ?? props.currentSong?.metadata?.artist">{{ props.currentSong?.online?.singer ?? props.currentSong?.metadata?.artist }}</div>
          </div>
        </div>
        <PlayerDetailLyrics
          class="am-lyrics"
          :lyrics="props.lyrics"
          :current-time="props.currentTime"
          :show="showLyrics && !props.hideLyrics"
          :is-playing="props.isPlaying"
          :is-fullscreen="isFullscreen"
          @seek="emit('seek', $event)"
        />
      </div>

      <!-- 经典风格：顶部封面 + 右侧歌词（原有布局） -->
      <template v-else>
        <PlayerDetailLeft
          :cover-url="props.coverUrl"
          :is-playing="props.isPlaying"
          :is-expanded="props.show"
          :show-lyrics="positionLyrics"
          :cover-transition="props.coverTransition ?? 'fade'"
          @toggle-lyrics="toggleLyrics"
        />

        <PlayerDetailLyrics
          :lyrics="props.lyrics"
          :current-time="props.currentTime"
          :show="showLyrics && !props.hideLyrics"
          :is-playing="props.isPlaying"
          :is-fullscreen="isFullscreen"
          @seek="emit('seek', $event)"
        />
      </template>
    </div>
  </div>
</template>

<style scoped>
/* z-index 50 低于 PlayerFooter(60)，保证底部播放栏始终可见
   折叠态不用 visibility:hidden，保证封面图始终可见（定位到底栏位置）
   展开态 footer detail-mode 背景透明，PlayerDetail 背景在 footer 下方透出 */
.player-detail {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
  font-family: sans-serif;
  user-select: none;
  color: white;
  pointer-events: none;
}

.player-detail.visible {
  pointer-events: auto;
}

.player-inner {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100%;
}

.bg-wrapper {
  position: absolute;
  inset: 0;
  opacity: 0;
  transform: translateY(100%);
  transition: opacity 600ms cubic-bezier(0.22, 1, 0.36, 1), transform 600ms cubic-bezier(0.22, 1, 0.36, 1);
}

.bg-wrapper.visible {
  opacity: 1;
  transform: translateY(0);
}

.bg-fallback {
  position: absolute;
  inset: 0;
  z-index: -1;
  background: #0a0a0a;
}

/* ---- AM（Apple Music）风格：左封面 + 右逐字歌词 ---- */
.am-body {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 48px;
  padding: 96px 64px 140px;
  box-sizing: border-box;
}

.am-cover {
  flex: 0 0 auto;
  width: 42vh;
  max-width: 46%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 28px;
}

.am-cover-art {
  width: 42vh;
  height: 42vh;
  max-width: 100%;
  border-radius: 18px;
  overflow: hidden;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.45);
  background: rgba(255, 255, 255, 0.06);
}

.am-cover-art img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.am-meta {
  width: 100%;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.am-title {
  font-size: 22px;
  font-weight: 600;
  color: #fff;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.am-artist {
  font-size: 15px;
  color: rgba(255, 255, 255, 0.7);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* AM 模式下歌词面板占右侧剩余空间（覆盖 .lyrics-panel 默认的 45% 右侧定位） */
.player-detail.am :deep(.lyrics-panel) {
  width: 46%;
  padding-left: 0;
  padding-right: 40px;
}

.player-detail.am :deep(.lyric-player) {
  max-width: 100%;
}
</style>
