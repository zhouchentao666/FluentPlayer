<script lang="ts" setup>
/**
 * WinUI 3 滑块（1:1 还原 WinUIonWeb 的 WinSlider 视觉/交互）
 * 视觉参数对齐 WinSlider：
 *   - 轨道高 4px，圆角 2px，底色 --fluent-bg-active（control-strong-fill）
 *   - 填充 accord --fluent-accent
 *   - thumb 22x22 圆，外层卡片色(dark:#454545 / light:#fff)，内圈 12px accent
 *   - hover 内圈 scale(1.167)，pressed 内圈 scale(0.71)
 * 交互：点击/拖拽定位、键盘方向键/PageUp-Down/Home/End、disabled
 * 接口：v-model / :model-value + @update:model-value + @change
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

const trackRef = ref<HTMLDivElement | null>(null)
const isHovered = ref(false)
const isPressed = ref(false)
const isDragging = ref(false)

const min = computed(() => props.min)
const max = computed(() => Math.max(props.min, props.max))
const step = computed(() => (props.step <= 0 ? 1 : props.step))
const range = computed(() => Math.max(0.0001, max.value - min.value))

const clamp = (v: number) => Math.min(max.value, Math.max(min.value, v))
const snap = (v: number) => {
  const snapped = min.value + Math.round((v - min.value) / step.value) * step.value
  return clamp(Number(snapped.toFixed(4)))
}

const percent = computed(() => {
  const p = ((props.modelValue - min.value) / range.value) * 100
  return Math.max(0, Math.min(100, p))
})

function commit(next: number) {
  const v = snap(next)
  if (v !== props.modelValue) {
    emit('update:modelValue', v)
    emit('change', v)
  }
}

function valueFromClientX(clientX: number) {
  const rect = trackRef.value!.getBoundingClientRect()
  const ratio = (clientX - rect.left) / rect.width
  return min.value + ratio * range.value
}

function onPointerDown(e: PointerEvent) {
  if (props.disabled || e.button !== 0 || !trackRef.value) return
  isPressed.value = true
  isDragging.value = true
  try {
    trackRef.value.setPointerCapture(e.pointerId)
  } catch {
    /* ignore */
  }
  commit(valueFromClientX(e.clientX))
}

function onPointerMove(e: PointerEvent) {
  if (!isDragging.value || props.disabled) return
  commit(valueFromClientX(e.clientX))
}

function onPointerUp() {
  isPressed.value = false
  isDragging.value = false
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
    ref="trackRef"
    class="win-slider"
    :class="{
      'is-hovered': isHovered && !disabled,
      'is-pressed': isPressed && !disabled,
      'is-disabled': disabled,
    }"
    role="slider"
    tabindex="0"
    :aria-valuemin="min"
    :aria-valuemax="max"
    :aria-valuenow="modelValue"
    :aria-disabled="disabled || undefined"
    :aria-label="ariaLabel || undefined"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerUp"
    @pointerenter="isHovered = true"
    @pointerleave="isHovered = false"
    @keydown="onKeyDown"
    @blur="onBlur"
  >
    <div class="win-slider-track">
      <div class="win-slider-fill" :style="{ width: percent + '%' }"></div>
    </div>
    <div
      class="win-slider-thumb"
      :class="{ 'is-hovered': isHovered && !disabled, 'is-pressed': isPressed && !disabled }"
      :style="{ left: percent + '%' }"
    ></div>
  </div>
</template>

<style scoped>
.win-slider {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
  min-width: 32px;
  min-height: 32px;
  touch-action: none;
  cursor: pointer;
  outline: none;
}

.win-slider:focus-visible .win-slider-thumb {
  box-shadow:
    0 0 0 2px var(--fluent-text),
    0 1px 3px rgba(0, 0, 0, 0.25);
}

.win-slider.is-disabled {
  opacity: 0.6;
  cursor: default;
}

.win-slider-track {
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  transform: translateY(-50%);
  height: 4px;
  border-radius: 2px;
  background: var(--fluent-bg-active);
  overflow: hidden;
}

.win-slider-fill {
  height: 100%;
  background: var(--fluent-accent);
  border-radius: 2px;
}

.win-slider.is-disabled .win-slider-fill {
  background: var(--fluent-text-secondary);
}

.win-slider-thumb {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--fluent-bg-card);
  border: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.18);
  display: grid;
  place-items: center;
  transition: box-shadow 0.12s ease;
}

.win-slider-thumb::after {
  content: '';
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--fluent-accent);
  transform: scale(0.86);
  transition: transform 0.12s cubic-bezier(0.16, 1, 0.3, 1), background-color 0.12s ease;
}

.win-slider-thumb.is-hovered::after {
  background: color-mix(in srgb, var(--fluent-accent) 88%, #fff);
  transform: scale(1.167);
}

.win-slider-thumb.is-pressed::after {
  background: color-mix(in srgb, var(--fluent-accent) 76%, #fff);
  transform: scale(0.71);
}

@media (prefers-reduced-motion: reduce) {
  .win-slider-thumb,
  .win-slider-thumb::after {
    transition: none;
  }
}

/* light 主题下 thumb 外层用纯白底色，更接近 WinUI */
[data-theme='light'] .win-slider-thumb,
[data-theme='system'] .win-slider-thumb {
  background: #fff;
  border-color: rgba(0, 0, 0, 0.08);
}
</style>
