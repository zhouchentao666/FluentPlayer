<script lang="ts" setup>
/**
 * WinUI 3 风格切换开关（移植自 WinUIonWeb 的 WinToggleSwitch）
 * 特性：
 * - 按下时滑块横向拉伸（squish）
 * - 拖拽切换（指针拖过半程即切换）
 * - 悬停/按下时滑块尺寸变化
 * - 键盘可访问（Space / Enter / 方向键）
 * - 支持 disabled、只读文本（onContent / offContent）
 * 保持与旧组件一致的 API：v-model / :model-value + @update:model-value
 */
import { computed, onBeforeUnmount, ref } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue?: boolean
    disabled?: boolean
    /** 开启时右侧文字，留空则不显示 */
    onContent?: string
    /** 关闭时右侧文字，留空则不显示 */
    offContent?: string
    /** 无障碍标签 */
    ariaLabel?: string
  }>(),
  {
    modelValue: false,
    disabled: false,
    onContent: '',
    offContent: '',
    ariaLabel: '',
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'change', value: boolean): void
}>()

const rootEl = ref<HTMLButtonElement | null>(null)
const isPressed = ref(false)
const isHovered = ref(false)
const isDragging = ref(false)
/** 拖拽过程中的临时视觉状态；null 表示跟随 modelValue */
const dragState = ref<boolean | null>(null)

let pointerId: number | null = null
let startX = 0
let moved = false

const checked = computed(() => (dragState.value === null ? props.modelValue : dragState.value))

const label = computed(() => (checked.value ? props.onContent : props.offContent))

function setValue(next: boolean) {
  if (props.disabled) return
  if (next === props.modelValue) return
  emit('update:modelValue', next)
  emit('change', next)
}

function toggle() {
  setValue(!props.modelValue)
}

function releasePointer() {
  if (pointerId !== null && rootEl.value?.hasPointerCapture?.(pointerId)) {
    try {
      rootEl.value.releasePointerCapture(pointerId)
    } catch {
      /* ignore */
    }
  }
  pointerId = null
}

function onPointerDown(e: PointerEvent) {
  if (props.disabled || e.button !== 0) return
  pointerId = e.pointerId
  startX = e.clientX
  moved = false
  isPressed.value = true
  isDragging.value = true
  dragState.value = props.modelValue
  try {
    rootEl.value?.setPointerCapture(e.pointerId)
  } catch {
    /* ignore */
  }
}

function onPointerMove(e: PointerEvent) {
  if (!isDragging.value || props.disabled) return
  const delta = e.clientX - startX
  // 超过 4px 视为拖拽而非点击
  if (Math.abs(delta) > 4) moved = true
  if (!moved) return
  // 轨道可用行程约 20px，过半即切换视觉状态
  if (delta > 10) dragState.value = true
  else if (delta < -10) dragState.value = false
  else dragState.value = props.modelValue
}

function onPointerUp() {
  if (!isDragging.value) return
  const target = dragState.value
  isPressed.value = false
  isDragging.value = false
  dragState.value = null
  releasePointer()
  if (props.disabled) return
  if (moved) {
    if (target !== null) setValue(target)
  } else {
    toggle()
  }
  moved = false
}

function onPointerCancel() {
  isPressed.value = false
  isDragging.value = false
  dragState.value = null
  moved = false
  releasePointer()
}

function onKeyDown(e: KeyboardEvent) {
  if (props.disabled) return
  if (e.key === ' ' || e.key === 'Enter') {
    e.preventDefault()
    isPressed.value = true
    return
  }
  if (e.key === 'ArrowLeft') {
    e.preventDefault()
    setValue(false)
  } else if (e.key === 'ArrowRight') {
    e.preventDefault()
    setValue(true)
  }
}

function onKeyUp(e: KeyboardEvent) {
  if (props.disabled) return
  if (e.key === ' ' || e.key === 'Enter') {
    e.preventDefault()
    isPressed.value = false
    toggle()
  }
}

function onBlur() {
  isPressed.value = false
}

onBeforeUnmount(() => {
  releasePointer()
})
</script>

<template>
  <div class="win-toggle-wrapper" :class="{ 'has-label': !!label }">
    <button
      ref="rootEl"
      type="button"
      role="switch"
      class="win-toggle"
      :class="{
        on: checked,
        pressed: isPressed,
        hovered: isHovered,
        dragging: isDragging,
        disabled: props.disabled,
      }"
      :disabled="props.disabled"
      :aria-checked="checked"
      :aria-label="props.ariaLabel || undefined"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerCancel"
      @pointerenter="isHovered = true"
      @pointerleave="isHovered = false"
      @keydown="onKeyDown"
      @keyup="onKeyUp"
      @blur="onBlur"
    >
      <span class="win-toggle-track">
        <span class="win-toggle-thumb"></span>
      </span>
    </button>
    <span v-if="label" class="win-toggle-label" :class="{ disabled: props.disabled }">{{ label }}</span>
  </div>
</template>

<style scoped>
.win-toggle-wrapper {
  display: inline-flex;
  align-items: center;
  vertical-align: middle;
}

.win-toggle-wrapper.has-label {
  gap: 12px;
}

.win-toggle {
  display: inline-flex;
  align-items: center;
  padding: 0;
  margin: 0;
  border: none;
  background: transparent;
  cursor: pointer;
  outline: none;
  -webkit-tap-highlight-color: transparent;
  touch-action: none;
}

.win-toggle:focus-visible .win-toggle-track {
  outline: 2px solid var(--fluent-text);
  outline-offset: 3px;
}

.win-toggle.disabled {
  cursor: not-allowed;
}

/* ---------------- 轨道 ---------------- */
.win-toggle-track {
  position: relative;
  display: block;
  box-sizing: border-box;
  width: 40px;
  height: 20px;
  border-radius: 10px;
  border: 1px solid var(--fluent-input-border);
  background: var(--fluent-bg-hover);
  transition:
    background-color 0.15s cubic-bezier(0.16, 1, 0.3, 1),
    border-color 0.15s cubic-bezier(0.16, 1, 0.3, 1);
}

.win-toggle.hovered:not(.disabled) .win-toggle-track {
  background: var(--fluent-bg-active);
}

.win-toggle.pressed:not(.disabled) .win-toggle-track {
  background: var(--fluent-bg-active);
}

/* 开启态 */
.win-toggle.on .win-toggle-track {
  background: var(--fluent-accent);
  border-color: transparent;
}

.win-toggle.on.hovered:not(.disabled) .win-toggle-track {
  background: color-mix(in srgb, var(--fluent-accent) 90%, #fff);
  border-color: transparent;
}

.win-toggle.on.pressed:not(.disabled) .win-toggle-track {
  background: color-mix(in srgb, var(--fluent-accent) 80%, #fff);
  border-color: transparent;
}

/* 禁用态 */
.win-toggle.disabled .win-toggle-track {
  background: transparent;
  border-color: var(--fluent-border);
  opacity: 0.5;
}

.win-toggle.on.disabled .win-toggle-track {
  background: var(--fluent-text-secondary);
  border-color: transparent;
  opacity: 0.4;
}

/* ---------------- 滑块 ---------------- */
.win-toggle-thumb {
  position: absolute;
  top: 50%;
  left: 3px;
  width: 12px;
  height: 12px;
  border-radius: 999px;
  background: var(--fluent-text);
  transform: translate(0, -50%);
  transform-origin: center left;
  transition:
    width 0.15s cubic-bezier(0.16, 1, 0.3, 1),
    height 0.15s cubic-bezier(0.16, 1, 0.3, 1),
    transform 0.2s cubic-bezier(0.16, 1, 0.3, 1),
    background-color 0.15s cubic-bezier(0.16, 1, 0.3, 1);
  will-change: transform, width;
}

/* 开启态：滑块移到右侧 */
.win-toggle.on .win-toggle-thumb {
  background: #fff;
  transform: translate(20px, -50%);
}

/* 悬停：滑块略微放大 */
.win-toggle.hovered:not(.disabled):not(.pressed) .win-toggle-thumb {
  width: 14px;
  height: 14px;
}

.win-toggle.hovered:not(.disabled):not(.pressed).on .win-toggle-thumb {
  transform: translate(19px, -50%);
}

/* 按下：滑块横向拉伸（WinUI squish 效果） */
.win-toggle.pressed:not(.disabled) .win-toggle-thumb {
  width: 17px;
  height: 14px;
}

.win-toggle.pressed:not(.disabled).on .win-toggle-thumb {
  transform: translate(16px, -50%);
}

/* 禁用态滑块 */
.win-toggle.disabled .win-toggle-thumb {
  background: var(--fluent-text-secondary);
}

.win-toggle.on.disabled .win-toggle-thumb {
  background: #fff;
}

/* ---------------- 文本 ---------------- */
.win-toggle-label {
  font-size: 13px;
  line-height: 20px;
  color: var(--fluent-text);
  user-select: none;
}

.win-toggle-label.disabled {
  color: var(--fluent-text-secondary);
  opacity: 0.6;
}

@media (prefers-reduced-motion: reduce) {
  .win-toggle-track,
  .win-toggle-thumb {
    transition: none;
  }
}
</style>
