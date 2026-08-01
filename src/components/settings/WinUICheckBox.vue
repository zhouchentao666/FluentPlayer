<script lang="ts" setup>
/**
 * WinUI 3 风格复选框（移植自 WinUIonWeb 的 WinCheckBox）
 * 特性：
 * - 方形勾选框 + 选中时显示对勾（squish 缩放动画）
 * - 悬停/按下边框与底色变化，使用 WinUI accent 主题色
 * - 支持 disabled、indeterminate（部分选中）三态
 * - 键盘可访问（Space / Enter 切换）
 * 保持与旧原生 checkbox 一致的使用方式：v-model / :model-value + @update:model-value + @change
 */
import { computed, ref } from 'vue'

const props = withDefaults(
  defineProps<{
    /** 选中态；当 indeterminate 为 true 时显示减号（部分选中） */
    modelValue?: boolean
    /** 三态：部分选中（显示减号） */
    indeterminate?: boolean
    disabled?: boolean
    /** 复选框右侧标签，留空则不显示 */
    label?: string
    /** 无障碍标签 */
    ariaLabel?: string
  }>(),
  {
    modelValue: false,
    indeterminate: false,
    disabled: false,
    label: '',
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

const checked = computed(() => props.modelValue)

function setValue(next: boolean) {
  if (props.disabled) return
  if (next === props.modelValue) return
  emit('update:modelValue', next)
  emit('change', next)
}

function toggle() {
  setValue(!props.modelValue)
}

function onPointerDown(e: PointerEvent) {
  if (props.disabled || e.button !== 0) return
  isPressed.value = true
}

function onPointerUp() {
  isPressed.value = false
}

function onPointerCancel() {
  isPressed.value = false
}

function onKeyDown(e: KeyboardEvent) {
  if (props.disabled) return
  if (e.key === ' ' || e.key === 'Enter') {
    e.preventDefault()
    toggle()
  }
}

function onBlur() {
  isPressed.value = false
}
</script>

<template>
  <div class="win-checkbox-wrapper" :class="{ 'has-label': !!label }">
    <button
      ref="rootEl"
      type="button"
      role="checkbox"
      class="win-checkbox"
      :class="{
        checked,
        indeterminate: props.indeterminate,
        pressed: isPressed,
        hovered: isHovered,
        disabled: props.disabled,
      }"
      :disabled="props.disabled"
      :aria-checked="props.indeterminate ? 'mixed' : checked"
      :aria-label="props.ariaLabel || undefined"
      @pointerdown="onPointerDown"
      @pointerup="onPointerUp"
      @pointercancel="onPointerCancel"
      @pointerenter="isHovered = true"
      @pointerleave="isHovered = false"
      @click="toggle"
      @keydown="onKeyDown"
      @blur="onBlur"
    >
      <svg class="win-checkbox-glyph" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
        <path
          v-if="!props.indeterminate"
          class="win-checkbox-check"
          d="M3.5 8.5L6.5 11.5L12.5 4.5"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          v-else
          class="win-checkbox-dash"
          d="M4 8H12"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
        />
      </svg>
    </button>
    <span v-if="label" class="win-checkbox-label" :class="{ disabled: props.disabled }">{{ label }}</span>
  </div>
</template>

<style scoped>
.win-checkbox-wrapper {
  display: inline-flex;
  align-items: center;
  vertical-align: middle;
}

.win-checkbox-wrapper.has-label {
  gap: 8px;
}

.win-checkbox {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  box-sizing: border-box;
  width: 20px;
  height: 20px;
  padding: 0;
  margin: 0;
  border: 1px solid var(--fluent-input-border);
  border-radius: 4px;
  background: var(--fluent-input-bg);
  color: #fff;
  cursor: pointer;
  outline: none;
  -webkit-tap-highlight-color: transparent;
  transition:
    background-color 0.12s ease,
    border-color 0.12s ease,
    transform 0.12s ease;
}

.win-checkbox:hover:not(.disabled) {
  border-color: var(--fluent-input-border-hover, var(--fluent-text-secondary));
}

.win-checkbox:focus-visible {
  outline: 2px solid var(--fluent-text);
  outline-offset: 2px;
}

.win-checkbox.pressed:not(.disabled) {
  transform: scale(0.92);
}

/* 选中 / 部分选中：填充 accent */
.win-checkbox.checked,
.win-checkbox.indeterminate {
  background: var(--fluent-text);
  border-color: transparent;
}

.win-checkbox.checked.hovered:not(.disabled),
.win-checkbox.indeterminate.hovered:not(.disabled) {
  background: color-mix(in srgb, var(--fluent-text) 90%, #fff);
}

.win-checkbox.disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.win-checkbox-glyph {
  display: block;
}

.win-checkbox-check {
  stroke-dasharray: 16;
  stroke-dashoffset: 16;
  transition: stroke-dashoffset 0.15s cubic-bezier(0.16, 1, 0.3, 1);
}

.win-checkbox.checked .win-checkbox-check {
  stroke-dashoffset: 0;
}

.win-checkbox-dash {
  opacity: 0;
  transition: opacity 0.1s ease;
}

.win-checkbox.indeterminate .win-checkbox-dash {
  opacity: 1;
}

.win-checkbox-label {
  font-size: 13px;
  line-height: 20px;
  color: var(--fluent-text);
  user-select: none;
  cursor: pointer;
}

.win-checkbox-label.disabled {
  color: var(--fluent-text-secondary);
  opacity: 0.6;
  cursor: not-allowed;
}

@media (prefers-reduced-motion: reduce) {
  .win-checkbox,
  .win-checkbox-check {
    transition: none;
  }
}
</style>
