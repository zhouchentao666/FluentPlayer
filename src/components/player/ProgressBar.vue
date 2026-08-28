<script lang="ts" setup>
import { ref, computed } from 'vue'

const props = defineProps<{
  currentTime: number
  duration: number
}>()

const emit = defineEmits<{
  (e: 'seek', time: number): void
}>()

const barRef = ref<HTMLElement | null>(null)
const isDragging = ref(false)
const dragTime = ref(0)

const progress = computed(() => {
  if (!props.duration || props.duration <= 0) return 0
  const time = isDragging.value ? dragTime.value : props.currentTime
  return Math.max(0, Math.min(100, (time / props.duration) * 100))
})

const currentStr = computed(() => formatDuration(isDragging.value ? dragTime.value : props.currentTime))
const totalStr = computed(() => formatDuration(props.duration || 0))

function formatDuration(seconds: number): string {
  if (!seconds || seconds < 0) return '0:00'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

function updateFromEvent(e: PointerEvent) {
  if (!barRef.value || !props.duration) return
  const rect = barRef.value.getBoundingClientRect()
  const offsetX = Math.max(0, Math.min(e.clientX - rect.left, rect.width))
  dragTime.value = (offsetX / rect.width) * props.duration
}

function startDrag(e: PointerEvent) {
  if (!props.duration || e.button !== 0) return
  e.preventDefault()
  ;(e.currentTarget as HTMLElement | null)?.setPointerCapture?.(e.pointerId)
  isDragging.value = true
  updateFromEvent(e)
}

function stopDrag(commit = true) {
  if (!isDragging.value) return
  const targetTime = dragTime.value
  isDragging.value = false
  if (commit) emit('seek', targetTime)
}

function onMove(e: PointerEvent) {
  if (isDragging.value) {
    e.preventDefault()
    updateFromEvent(e)
  }
}

function onUp() {
  stopDrag(true)
}

function onCancel() {
  stopDrag(false)
}
</script>

<template>
  <footer
    class="progress-bar"
    @pointermove="onMove"
    @pointerup="onUp"
    @pointercancel="onCancel"
  >
    <div
      ref="barRef"
      class="progress-track-wrap"
      @pointerdown="startDrag"
    >
      <div class="progress-track">
        <div class="progress-fill" :style="{ width: progress + '%' }">
          <div class="progress-thumb"></div>
          <transition name="tooltip">
            <div v-if="isDragging" class="time-tooltip">
              {{ currentStr }} / {{ totalStr }}
              <div class="tooltip-arrow"></div>
            </div>
          </transition>
        </div>
      </div>
    </div>
  </footer>
</template>

<style scoped>
.progress-bar {
  position: absolute;
  top: -8px;
  left: 0;
  right: 0;
  height: 18px;
  cursor: pointer;
  z-index: 10;
  display: flex;
  align-items: center;
}

.progress-track-wrap {
  width: 100%;
  height: 3px;
  border-radius: 999px;
  background: var(--fluent-border);
  overflow: visible;
  transition: height 0.2s ease;
}

.progress-bar:hover .progress-track-wrap,
.progress-bar:active .progress-track-wrap {
  height: 5px;
}

.progress-track {
  width: 100%;
  height: 100%;
}

.progress-fill {
  height: 100%;
  border-radius: 999px;
  background: var(--fluent-accent);
  position: relative;
}

.progress-thumb {
  position: absolute;
  right: -5px;
  top: 50%;
  transform: translateY(-50%);
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
  opacity: 0;
  transition: opacity 0.18s ease;
}

.progress-bar:hover .progress-thumb,
.progress-bar:active .progress-thumb {
  opacity: 1;
}

.time-tooltip {
  position: absolute;
  right: -42px;
  bottom: 10px;
  width: 84px;
  padding: 3px 0;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.85);
  color: #fff;
  font-size: 10px;
  font-weight: 600;
  text-align: center;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  pointer-events: none;
}

.tooltip-arrow {
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  border: 4px solid transparent;
  border-top-color: rgba(0, 0, 0, 0.85);
}

.tooltip-enter-active,
.tooltip-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.tooltip-enter-from,
.tooltip-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
