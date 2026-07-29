<script lang="ts" setup>
import { ref, computed } from 'vue'

const props = defineProps<{
  playbackRate: number
}>()

const emit = defineEmits<{
  (e: 'set-playback-rate', rate: number): void
}>()

const showPopup = ref(false)
const isDragging = ref(false)
let hideTimer: ReturnType<typeof setTimeout> | null = null

const presets = [0.5, 1, 1.25, 1.5, 2]

const displayRate = computed(() => `${props.playbackRate.toFixed(2)}x`)

function handleInput(e: Event) {
  const value = parseFloat((e.target as HTMLInputElement).value)
  emit('set-playback-rate', value)
}

function setRate(rate: number) {
  emit('set-playback-rate', rate)
}

function handleEnter() {
  if (hideTimer) clearTimeout(hideTimer)
  showPopup.value = true
}

function handleLeave() {
  hideTimer = setTimeout(() => {
    if (!isDragging.value) showPopup.value = false
  }, 300)
}

function startDrag() {
  isDragging.value = true
}

function stopDrag() {
  isDragging.value = false
  if (!showPopup.value) return
  hideTimer = setTimeout(() => {
    showPopup.value = false
  }, 300)
}
</script>

<template>
  <div
    class="rate-wrap"
    @mouseenter="handleEnter"
    @mouseleave="handleLeave"
  >
    <div v-if="showPopup || isDragging" class="rate-popup">
      <div class="rate-panel">
        <div class="rate-value">{{ displayRate }}</div>
        <input
          type="range"
          class="rate-slider"
          min="0.25"
          max="16"
          step="0.05"
          :value="playbackRate"
          @input="handleInput"
          @pointerdown="startDrag"
          @pointerup="stopDrag"
        />
        <div class="rate-presets">
          <button
            v-for="rate in presets"
            :key="rate"
            class="preset-btn"
            :class="{ active: Math.abs(playbackRate - rate) < 0.01 }"
            @click="setRate(rate)"
          >
            {{ rate }}x
          </button>
        </div>
      </div>
    </div>
    <button class="control-btn rate-btn" title="倍速播放">
      {{ displayRate }}
    </button>
  </div>
</template>

<style scoped>
.rate-wrap {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.rate-popup {
  position: absolute;
  bottom: 100%;
  right: 0;
  padding-bottom: 8px;
  z-index: 20;
}

.rate-panel {
  width: 220px;
  padding: 14px 16px;
  border-radius: 12px;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
  backdrop-filter: blur(16px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.rate-value {
  font-size: 14px;
  font-weight: 600;
  text-align: center;
  color: var(--fluent-text);
  font-variant-numeric: tabular-nums;
}

.rate-slider {
  width: 100%;
  height: 4px;
  border-radius: 2px;
  background: var(--fluent-bg-active);
  outline: none;
  -webkit-appearance: none;
  appearance: none;
  cursor: pointer;
}

.rate-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--fluent-accent);
  border: 2px solid #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
}

.rate-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--fluent-accent);
  border: 2px solid #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
}

.rate-presets {
  display: flex;
  justify-content: space-between;
  gap: 6px;
}

.preset-btn {
  flex: 1;
  height: 26px;
  border-radius: 6px;
  border: none;
  background: var(--fluent-bg-active);
  color: var(--fluent-text);
  font-size: 11px;
  cursor: pointer;
  transition: background 0.18s ease, color 0.18s ease;
}

.preset-btn:hover {
  background: var(--fluent-bg-hover);
}

.preset-btn.active {
  background: var(--fluent-accent);
  color: #fff;
}

.rate-btn {
  min-width: 46px;
  height: 28px;
  padding: 0 8px;
  border-radius: 14px;
  font-size: 11px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.control-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
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
