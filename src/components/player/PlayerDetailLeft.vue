<script lang="ts" setup>
import { ref, watch, computed } from 'vue'
import { useCoverTilt } from '../../composables/useCoverTilt'
import PlayerDetailCoverImage from './PlayerDetailCoverImage.vue'

const props = defineProps<{
  coverUrl: string | null
  isPlaying: boolean
  isExpanded: boolean
  showLyrics: boolean
  coverTransition: 'fade' | 'slide-left' | 'slide-both'
}>()

const emit = defineEmits<{
  toggleLyrics: []
}>()

const localCoverUrl = ref('')
const detailCoverRef = ref<HTMLElement | null>(null)

const currentSongPath = computed(() => props.coverUrl || '')

watch(() => props.coverUrl, (cover) => {
  localCoverUrl.value = cover || ''
}, { immediate: true })

const {
  shineX,
  shineY,
  isHovering,
  coverTransform,
  shadowTransform,
  handleMouseMove,
  handleMouseEnter,
  handleMouseLeave,
} = useCoverTilt(() => props.isExpanded)

function onMouseMove(e: MouseEvent) {
  handleMouseMove(e, detailCoverRef.value)
}

function toggleLyrics() {
  emit('toggleLyrics')
}

defineExpose({ detailCoverRef })
</script>

<template>
  <Teleport to="body">
    <div class="pointer-events-none">
      <div
        ref="detailCoverRef"
        class="cover-container"
        :class="[
          props.isExpanded ? 'expanded' : 'collapsed',
          props.showLyrics ? 'with-lyrics' : 'center',
        ]"
        @click="toggleLyrics"
        @mousemove="onMouseMove"
        @mouseenter="handleMouseEnter"
        @mouseleave="handleMouseLeave"
      >
        <div class="cover-inner" :style="{ transform: coverTransform }">
          <PlayerDetailCoverImage
            :cover-url="localCoverUrl"
            :song-path="currentSongPath"
            :is-expanded="props.isExpanded"
            :transition="props.coverTransition"
          />

          <div
            v-if="props.isExpanded"
            class="shine-layer"
            :style="{
              background: `radial-gradient(circle at ${shineX}% ${shineY}%, rgba(255,255,255,0.35) 0%, transparent 50%)`,
              opacity: isHovering ? 1 : 0,
            }"
          ></div>
        </div>

        <div
          v-if="props.isExpanded"
          class="cover-shadow"
          :style="{ transform: shadowTransform }"
        ></div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.pointer-events-none {
  pointer-events: none;
}

/* 封面容器：通过 Teleport 到 body，脱离 PlayerDetail stacking context
   z-index 70 盖住 PlayerFooter(60)，折叠态可见
   纯 CSS transition 驱动，天然支持多次打断（CSS 从当前插值位置继续） */
.cover-container {
  position: fixed;
  aspect-ratio: 1;
  will-change: transform, top, left, width;
  pointer-events: auto;
  overflow: visible;
  z-index: 70;
  /* 参考 LyciaMusic：duration-700 + cubic-bezier(0.22,1,0.36,1) */
  transition:
    top 700ms cubic-bezier(0.22, 1, 0.36, 1),
    left 700ms cubic-bezier(0.22, 1, 0.36, 1),
    width 700ms cubic-bezier(0.22, 1, 0.36, 1),
    border-radius 300ms ease,
    opacity 300ms ease;
}

.cover-container.expanded {
  --cover-size: min(clamp(180px, 38vw, 520px), clamp(220px, 45vh, 580px));
  top: calc(45% - var(--cover-size) / 2);
  width: var(--cover-size);
  border-radius: 16px;
  opacity: 1;
}

.cover-container.expanded.center {
  left: calc(50% - var(--cover-size) / 2);
}

.cover-container.expanded.with-lyrics {
  left: calc(28% - var(--cover-size) / 2);
}

/* 折叠态：定位到底栏封面位置（底栏 72px，封面 44px，垂直居中）
   top = 100vh - 72px + (72-44)/2 = 100vh - 58px */
.cover-container.collapsed {
  top: calc(100vh - 58px);
  left: 16px;
  width: 44px;
  border-radius: 8px;
  pointer-events: none;
  opacity: 1;
}

.cover-inner {
  width: 100%;
  height: 100%;
  border-radius: inherit;
  overflow: hidden;
  position: relative;
  isolation: isolate;
  z-index: 20;
  transition: transform 240ms cubic-bezier(0.22, 1, 0.36, 1);
  transform-style: preserve-3d;
}

.shine-layer {
  position: absolute;
  inset: 0;
  z-index: 30;
  pointer-events: none;
  mix-blend-mode: overlay;
  transition: opacity 240ms ease-out;
  border-radius: inherit;
}

.cover-shadow {
  position: absolute;
  top: calc(100% + 8px);
  left: 10%;
  width: 80%;
  height: 14%;
  border-radius: 50%;
  background: radial-gradient(
    ellipse at center,
    rgba(0, 0, 0, 0.5) 0%,
    rgba(0, 0, 0, 0.2) 45%,
    transparent 80%
  );
  filter: blur(12px);
  pointer-events: none;
  z-index: 5;
  opacity: 0.85;
  transition: transform 240ms cubic-bezier(0.22, 1, 0.36, 1), opacity 240ms ease;
}
</style>
