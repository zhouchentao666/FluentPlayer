<script lang="ts" setup>
import { computed } from 'vue'
const props = defineProps<{
  coverUrl: string
  songPath: string
  isExpanded: boolean
  transition: 'fade' | 'slide-left' | 'slide-both'
}>()

// slide-both: 交替方向（左右滑入滑出）；slide-left: 固定从左侧滑入滑出
let slideDir = 1
let lastPath = ''
function beforeEnter() {
  if (props.songPath !== lastPath) {
    slideDir = -slideDir
    lastPath = props.songPath
  }
}
const transitionName = computed(() => props.transition === 'fade' ? 'cover-fade'
  : props.transition === 'slide-left' ? 'cover-slide-left'
  : 'cover-slide-both')
const isSlide = computed(() => props.transition !== 'fade')
</script>

<template>
  <Transition
    :name="transitionName"
    @before-enter="beforeEnter"
  >
    <img
      v-if="coverUrl"
      :key="`thumb:${songPath}:${coverUrl}`"
      :src="coverUrl"
      class="cover-img"
      :class="isSlide ? 'slide-img' : (isExpanded ? 'expanded-img' : 'collapsed-img')"
      :style="isSlide ? { '--slide-dir': slideDir } : undefined"
      draggable="false"
      decoding="async"
    />
  </Transition>
  <div v-if="!coverUrl" class="cover-placeholder">
    <svg xmlns="http://www.w3.org/2000/svg" class="h-32 w-32" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
    </svg>
  </div>
</template>

<style scoped>
.cover-img {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  user-select: none;
  z-index: 10;
}

.expanded-img {
  transform: scale(1);
  filter: blur(0);
}

.collapsed-img {
  transform: scale(1.25);
  filter: blur(0);
}

.slide-img {
  transform: none;
}

/* 淡入淡出 */
.cover-fade-enter-active,
.cover-fade-leave-active {
  transition: opacity 500ms ease-out;
}
.cover-fade-enter-from,
.cover-fade-leave-to {
  opacity: 0;
}

/* 左边滑入滑出：新封面从左侧滑入，旧封面向左滑出 */
.cover-slide-left-enter-active,
.cover-slide-left-leave-active {
  transition: transform 500ms cubic-bezier(0.22, 1, 0.36, 1), opacity 400ms ease-out;
}
.cover-slide-left-enter-from {
  transform: translateX(-100%);
  opacity: 0;
}
.cover-slide-left-leave-to {
  transform: translateX(-100%);
  opacity: 0;
}

/* 左右交替滑入滑出：新封面从一侧滑入，旧封面向另一侧滑出 */
.cover-slide-both-enter-active,
.cover-slide-both-leave-active {
  transition: transform 500ms cubic-bezier(0.22, 1, 0.36, 1), opacity 400ms ease-out;
}
.cover-slide-both-enter-from {
  transform: translateX(calc(var(--slide-dir, 1) * 100%));
  opacity: 0;
}
.cover-slide-both-leave-to {
  transform: translateX(calc(var(--slide-dir, 1) * -100%));
  opacity: 0;
}

.cover-placeholder {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  background: rgba(255, 255, 255, 0.05);
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.1);
  z-index: 0;
}

.h-32 { height: 128px; }
.w-32 { width: 128px; }
</style>
