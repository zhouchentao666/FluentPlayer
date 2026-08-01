<script lang="ts" setup>
/**
 * WinUI 3 风格滑块（移植自 WinUIonWeb 的 WinSlider）
 * 特性：
 * - 自定义轨道 + WinUI accent 填充 + 圆形拖拽手柄（squish 缩放）
 * - 悬停/按下时手柄放大，轨道高亮
 * - 支持 min / max / step / disabled
 * - 键盘方向键微调（含 Home / End 跳到两端）
 * - 鼠标/触摸拖拽，可点击轨道定位
 * 保持与原生 range 一致接口：v-model / :model-value + @update:model-value
 */
import { computed, ref } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: number
    min?: number
    max?: number
    step?: number
    disabled?: boolean
    ariaLabel?: string
  }>(),
  {
    min: 0,
    max: 100,
    step: 1,
    disabled: false,
    ariaLabel: '',
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
  (e: 'change', value: number): void
}>()

const trackEl = ref<HTMLDivElement | null>(null)
const isHovered = ref(false)
const isDragging = ref(false)
const isPressed = ref(false)

const min = computed(() => props.min)
const max = computed(() => props.max)
const step = computed(() => (props.step <= 0 ? 1 : props.step))

const clamp = (v: number) => Math.min(max.value, Math.max(min.value, v))
const snap = (v: number) => {
  const snapped = Math.round((v - min.value) / step.value) * step.value + min.value
  // 修正浮点误差
  return clamp(Number(snapped.toFixed(6)))
}

const percent = computed(() => {
  const range = max.value - min.value
  if (range <= 0) return 0
  return ((props.modelValue - min.value) / range) * 100
})

function commit(next: number) {
  const v = snap(next)
  if (v !== props.modelValue) {
    emit('update:modelValue', v)
    emit('change', v)
  }
}

function valueFromClientX(clientX: number) {
  const track = trackEl.value
  if (!track) return props.modelValue
  const rect = track.getBoundingClientRect()
  const ratio = (clientX - rect.left) / rect.width
  return min.value + ratio * (max.value - min.value)
}

function onTrackPointerDown(e: PointerEvent) {
  if (props.disabled || e.button !== 0) return
  isDragging.value = true
  isPressed.value = true
  try {
    trackEl.value?.setPointerCapture(e.pointerId)
  } catch {
    /* ignore */
  }
  commit(valueFromClientX(e.clientX))
}

function onTrackPointerMove(e: PointerEvent) {
  if (!isDragging.value || props.disabled) return
  commit(valueFromClientX(e.clientX))
}

function onTrackPointerUp() {
  isDragging.value = false
  isPressed.value = false
}

function onKeyDown(e: KeyboardEvent) {
  if (props.disabled) return
  let next: number | null = null
  switch (e.key) {
    case 'ArrowRight':
    case 'ArrowUp':
      next = props.modelValue + step.value
      break
    case 'ArrowLeft':
    case 'ArrowDown':
      next = props.modelValue - step.value
      break
    case 'PageUp':
      next = props.modelValue + step.value * 10
      break
    case 'PageDown':
      next = props.modelValue - step.value * 10
      break
    case 'Home':
      next = min.value
      break
    case 'End':
      next = max.value
      break
    default:
      return
  }
  e.preventDefault()
  commit(next)
}

function onBlur() {
  isPressed.value = false
}
</script>

<template>
  <div
    ref="trackEl"
    class="win-slider"
    :class="{
      hovered: isHovered,
      dragging: isDragging,
      pressed: isPressed,
      disabled: props.disabled,
    }"
    role="slider"
    tabindex="0"
    :aria-valuemin="min"
    :aria-valuemax="max"
    :aria-valuenow="modelValue"
    :aria-disabled="props.disabled || undefined"
    :aria-label="props.ariaLabel || undefined"
    @pointerdown="onTrackPointerDown"
    @pointermove="onTrackPointerMove"
    @pointerup="onTrackPointerUp"
    @pointercancel="onTrackPointerUp"
    @pointerenter="isHovered = true"
    @pointerleave="isHovered = false"
    @keydown="onKeyDown"
    @blur="onBlur"
  >
    <div class="win-slider-rail"></div>
    <div class="win-slider-fill" :style="{ width: percent + '%' }"></div>
    <div class="win-slider-thumb" :style="{ left: percent + '%' }"></div>
  </div>
</template>

<style scoped>
.win-slider {
  position: relative;
  flex: 1;
  height: 20px;
  display: flex;
  align-items: center;
  cursor: pointer;
  outline: none;
  touch-action: none;
  -webkit-tap-highlight-color: transparent;
}

.win-slider:focus-visible .win-slider-thumb {
  box-shadow:
    0 0 0 2px var(--fluent-text),
    0 1px 3px rgba(0, 0, 0, 0.25);
}

.win-slider.disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

/* 轨道 */
.win-slider-rail {
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  transform: translateY(-50%);
  height: 4px;
  border-radius: 2px;
  background: var(--fluent-bg-active);
}

.win-slider.hovered:not(.disabled) .win-slider-rail {
  background: var(--fluent-border);
}

/* 已填充部分 */
.win-slider-fill {
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  height: 4px;
  border-radius: 2px;
  background: var(--fluent-text);
}

/* 手柄 */
.win-slider-thumb {
  position: absolute;
  top: 50%;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #fff;
  border: 1px solid var(--fluent-text);
  transform: translate(-50%, -50%);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
  transition:
    width 0.12s cubic-bezier(0.16, 1, 0.3, 1),
    height 0.12s cubic-bezier(0.16, 1, 0.3, 1);
}

.win-slider.hovered:not(.disabled) .win-slider-thumb {
  width: 18px;
  height: 18px;
}

.win-slider.pressed:not(.disabled) .win-slider-thumb,
.win-slider.dragging:not(.disabled) .win-slider-thumb {
  width: 18px;
  height: 18px;
}

@media (prefers-reduced-motion: reduce) {
  .win-slider-thumb {
    transition: none;
  }
}
</style>
