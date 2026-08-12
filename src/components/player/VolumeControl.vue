<script lang="ts" setup>
import { ref } from 'vue'

const props = defineProps<{
  volume: number
}>()

const emit = defineEmits<{
  (e: 'set-volume', volume: number): void
}>()

const barRef = ref<HTMLElement | null>(null)
const isDragging = ref(false)
const showSlider = ref(false)
let hideTimer: ReturnType<typeof setTimeout> | null = null

function updateFromEvent(e: PointerEvent) {
  if (!barRef.value) return
  const rect = barRef.value.getBoundingClientRect()
  const distance = rect.bottom - e.clientY
  const percent = Math.max(0, Math.min(1, distance / rect.height))
  emit('set-volume', Math.round(percent * 100))
}

function startDrag(e: PointerEvent) {
  if (e.button !== 0) return
  e.preventDefault()
  ;(e.currentTarget as HTMLElement | null)?.setPointerCapture?.(e.pointerId)
  isDragging.value = true
  updateFromEvent(e)
}

function stopDrag() {
  isDragging.value = false
}

function onMove(e: PointerEvent) {
  if (isDragging.value) {
    e.preventDefault()
    updateFromEvent(e)
  }
}

function onUp() {
  stopDrag()
}

function handleEnter() {
  if (hideTimer) clearTimeout(hideTimer)
  showSlider.value = true
}

function handleLeave() {
  hideTimer = setTimeout(() => {
    if (!isDragging.value) showSlider.value = false
  }, 300)
}
</script>

<template>
  <div
    class="volume-wrap"
    @mouseenter="handleEnter"
    @mouseleave="handleLeave"
  >
    <div v-if="showSlider || isDragging" class="volume-popup">
      <div
        ref="barRef"
        class="volume-bar"
        @pointerdown="startDrag"
        @pointermove="onMove"
        @pointerup="onUp"
        @pointerleave="onUp"
      >
        <div class="volume-fill" :style="{ height: volume + '%' }"></div>
        <div class="volume-thumb" :style="{ bottom: `calc(${volume}% - 6px)` }"></div>
      </div>
    </div>
    <button class="control-btn volume-btn" title="音量">
      <svg v-if="volume === 0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><line x1="23" y1="9" x2="17" y2="15"></line><line x1="17" y1="9" x2="23" y2="15"></line></svg>
      <svg v-else-if="volume < 40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon></svg>
      <svg v-else-if="volume < 75" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path></svg>
      <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon><path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path><path d="M19.07 4.93a10 10 0 0 1 0 14.14"></path></svg>
    </button>
  </div>
</template>

<style scoped>
.volume-wrap {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.volume-popup {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  padding-bottom: 8px;
  z-index: 20;
}

.volume-bar {
  width: 28px;
  height: 110px;
  border-radius: 14px;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
  backdrop-filter: blur(16px);
  position: relative;
  cursor: pointer;
  overflow: visible;
}

.volume-fill {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  border-radius: 0 0 14px 14px;
  background: var(--fluent-accent);
}

.volume-thumb {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
}

.volume-btn svg {
  width: 20px;
  height: 20px;
}

.control-btn {
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
  transition: background 0.18s ease, transform 0.1s ease;
}

.control-btn:hover {
  background: var(--fluent-bg-hover);
}

.control-btn:active {
  transform: scale(0.95);
}
</style>
