<script lang="ts" setup>
const props = defineProps<{
  isVisible: boolean
  isTopChromeVisible: boolean
  isMaximised: boolean
  isFullscreen: boolean
  isAlwaysOnTop: boolean
  staggerStyle: (phase: number, translateDir?: 'Y' | 'X', distance?: number) => Record<string, string | number>
}>()

const emit = defineEmits<{
  close: []
  minimize: []
  toggleMaximize: []
  toggleFullscreen: []
  toggleAlwaysOnTop: []
  closeApp: []
  showTopChrome: []
  topChromeLeave: []
}>()

const handleClose = () => emit('close')
const minimize = () => emit('minimize')
const toggleMaximize = () => emit('toggleMaximize')
const toggleFullscreen = () => emit('toggleFullscreen')
const toggleAlwaysOnTop = () => emit('toggleAlwaysOnTop')
const closeApp = () => emit('closeApp')
const showTopChrome = () => emit('showTopChrome')
const topChromeLeave = () => emit('topChromeLeave')
</script>

<template>
  <div
    class="top-chrome"
    :style="staggerStyle(1, 'Y', -10)"
    @mouseenter="showTopChrome"
    @mousemove="showTopChrome"
    @mouseleave="topChromeLeave"
  >
    <div
      class="chrome-hitbox"
      :class="{ 'pe-auto': isVisible, 'pe-none': !isVisible }"
    ></div>

    <div
      class="chrome-inner"
      :class="{
        'translate-y-0 opacity-100': isTopChromeVisible,
        '-translate-y-3 opacity-0': !isTopChromeVisible,
        'pe-auto': isVisible,
        'pe-none': !isVisible,
      }"
      @dblclick="toggleMaximize"
    >
      <!-- 拖动区域：覆盖整个顶部 -->
      <div class="drag-region" data-tauri-drag-region></div>

      <div class="chrome-left" @dblclick.stop>
        <button title="收起详情页" class="win-btn" @click="handleClose">
          <svg xmlns="http://www.w3.org/2000/svg" class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>
        <button :title="props.isAlwaysOnTop ? '取消置顶' : '置顶'" class="win-btn" :class="{ active: props.isAlwaysOnTop }" @click="toggleAlwaysOnTop">
          <svg xmlns="http://www.w3.org/2000/svg" class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 17v5" />
            <path d="M9 10.76V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v6.76l2 4.24H7l2-4.24Z" />
          </svg>
        </button>
      </div>

      <div class="chrome-right" @dblclick.stop>
        <button class="win-btn" :title="props.isFullscreen ? '退出全屏 (F11)' : '全屏 (F11)'" @click="toggleFullscreen">
          <svg v-if="props.isFullscreen" xmlns="http://www.w3.org/2000/svg" class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M8 3v3a2 2 0 0 1-2 2H3M21 8h-3a2 2 0 0 1-2-2V3M3 16h3a2 2 0 0 1 2 2v3M16 21v-3a2 2 0 0 1 2-2h3" />
          </svg>
          <svg v-else xmlns="http://www.w3.org/2000/svg" class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 8V5a2 2 0 0 1 2-2h3M21 8V5a2 2 0 0 0-2-2h-3M3 16v3a2 2 0 0 0 2 2h3M21 16v3a2 2 0 0 1-2 2h-3" />
          </svg>
        </button>
        <button class="win-btn" title="最小化" @click="minimize">
          <svg xmlns="http://www.w3.org/2000/svg" class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M5 12h14" />
          </svg>
        </button>
        <button class="win-btn" :title="props.isMaximised ? '还原' : '最大化'" @click="toggleMaximize">
          <svg v-if="props.isMaximised" xmlns="http://www.w3.org/2000/svg" class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="6" y="6" width="14" height="14" rx="2" ry="2" />
            <rect x="3" y="3" width="14" height="14" rx="2" ry="2" />
          </svg>
          <svg v-else xmlns="http://www.w3.org/2000/svg" class="win-icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          </svg>
        </button>
        <button class="win-btn win-close" title="关闭" @click="closeApp">
          <svg xmlns="http://www.w3.org/2000/svg" class="win-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.top-chrome {
  position: relative;
  z-index: 60;
  height: 96px;
}

.chrome-hitbox {
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  height: 96px;
}

.chrome-inner {
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 0 0 24px;
  transition: all 500ms ease-out;
}

/* 拖动区域：覆盖整个 chrome-inner */
.drag-region {
  position: absolute;
  inset: 0;
  z-index: 0;
}

.chrome-left {
  position: relative;
  z-index: 10;
  display: flex;
  height: 100%;
  align-items: stretch;
}

.chrome-right {
  position: relative;
  z-index: 10;
  display: flex;
  height: 100%;
  align-items: stretch;
  justify-content: flex-end;
}

/* Windows 原生风格窗口控制按钮 */
.win-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 46px;
  height: 100%;
  background: transparent;
  border: none;
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: background 120ms ease, color 120ms ease;
}

.win-btn:hover {
  background: rgba(255, 255, 255, 0.12);
  color: white;
}

.win-btn:active {
  background: rgba(255, 255, 255, 0.06);
}

.win-btn.active {
  color: white;
  background: rgba(255, 255, 255, 0.18);
}

.win-btn.active:hover {
  background: rgba(255, 255, 255, 0.25);
}

.win-close:hover {
  background: #e81123;
  color: white;
}

.win-close:active {
  background: #f1707a;
  color: white;
}

.win-icon {
  width: 16px;
  height: 16px;
}

.win-icon-sm {
  width: 12px;
  height: 12px;
}

.pe-auto {
  pointer-events: auto;
}

.pe-none {
  pointer-events: none;
}

.translate-y-0 {
  transform: translateY(0);
}

.-translate-y-3 {
  transform: translateY(-12px);
}

.opacity-100 {
  opacity: 1;
}

.opacity-0 {
  opacity: 0;
}
</style>
