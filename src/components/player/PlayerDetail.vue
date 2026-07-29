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
    :class="{ visible: props.show }"
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
        @seek="emit('seek', $event)"
      />
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
</style>
