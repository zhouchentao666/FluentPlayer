<script lang="ts" setup>
/**
 * WinUI 3 复选框（1:1 还原 WinUIonWeb 的 WinCheckBox 视觉/交互）
 * 视觉参数对齐 WinCheckBox：
 *   - 20x20 方框，1px 边框，4px 圆角
 *   - 未选：透明底 + 强边框；选中/半选：accent 填充 + accent 边框
 *   - 勾号 9x5，1.6px，rotate(-45deg)；半选显示减号
 *   - hover / active 切换 accent-hover / accent-pressed
 * 交互：点击切换、Space/Enter 切换、三态(indeterminate)支持、disabled
 * 接口：v-model / :model-value + @update:model-value + @change
 */
import { computed, ref } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue?: boolean
    /** 三态：当前为 null（半选/部分选中）时显示减号 */
    modelModifiers?: { indeterminate?: boolean }
    indeterminate?: boolean
    disabled?: boolean
    label?: string
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

const isPressed = ref(false)
const isHovered = ref(false)

const isChecked = computed(() => props.modelValue === true)
const isIndeterminate = computed(() => props.indeterminate)

function setValue(next: boolean) {
  if (props.disabled) return
  if (next === props.modelValue) return
  emit('update:modelValue', next)
  emit('change', next)
}

function toggle() {
  setValue(!props.modelValue)
}

function onKeyDown(e: KeyboardEvent) {
  if (props.disabled) return
  if (e.key === ' ' || e.key === 'Enter') {
    e.preventDefault()
    toggle()
  }
}
</script>

<template>
  <div class="win-checkbox" :class="{ 'has-label': !!label }">
    <div
      class="checkbox-box"
      :class="{
        'is-checked': isChecked,
        'is-indeterminate': isIndeterminate,
        'is-disabled': disabled,
        'is-hovered': isHovered && !disabled,
        'is-pressed': isPressed && !disabled,
      }"
      role="checkbox"
      :tabindex="disabled ? -1 : 0"
      :aria-checked="isIndeterminate ? 'mixed' : isChecked"
      :aria-disabled="disabled || undefined"
      :aria-label="ariaLabel || undefined"
      @click="toggle"
      @keydown="onKeyDown"
      @pointerenter="isHovered = true"
      @pointerleave="isHovered = false"
      @pointerdown="isPressed = true"
      @pointerup="isPressed = false"
      @pointercancel="isPressed = false"
    >
      <span v-if="isIndeterminate" class="checkbox-dash" aria-hidden="true"></span>
    </div>
    <span v-if="label" class="checkbox-content" :class="{ disabled }">{{ label }}</span>
  </div>
</template>

<style scoped>
.win-checkbox {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 32px;
  width: fit-content;
  color: var(--fluent-text);
  font-size: 14px;
  line-height: 20px;
  user-select: none;
  cursor: pointer;
}

.win-checkbox.has-label .checkbox-box {
  cursor: pointer;
}

.checkbox-box {
  position: relative;
  width: 20px;
  height: 20px;
  min-width: 20px;
  box-sizing: border-box;
  border: 1px solid var(--fluent-input-border);
  border-radius: 4px;
  background: transparent;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition:
    background-color 0.12s cubic-bezier(0.16, 1, 0.3, 1),
    border-color 0.12s cubic-bezier(0.16, 1, 0.3, 1);
}

.checkbox-box:focus-visible {
  outline: 2px solid var(--fluent-text);
  outline-offset: 2px;
}

/* 未选 hover / active */
.checkbox-box.is-hovered:not(.is-checked):not(.is-indeterminate) {
  background: var(--fluent-bg-hover);
}

.checkbox-box.is-pressed:not(.is-checked):not(.is-indeterminate) {
  background: var(--fluent-bg-active);
}

/* 选中 / 半选：accent 填充 */
.checkbox-box.is-checked,
.checkbox-box.is-indeterminate {
  background: var(--fluent-accent);
  border-color: var(--fluent-accent);
}

.checkbox-box.is-hovered.is-checked,
.checkbox-box.is-hovered.is-indeterminate {
  background: color-mix(in srgb, var(--fluent-accent) 88%, #fff);
  border-color: color-mix(in srgb, var(--fluent-accent) 88%, #fff);
}

.checkbox-box.is-pressed.is-checked,
.checkbox-box.is-pressed.is-indeterminate {
  background: color-mix(in srgb, var(--fluent-accent) 76%, #fff);
  border-color: color-mix(in srgb, var(--fluent-accent) 76%, #fff);
}

/* 勾号（9x5，1.6px，rotate -45deg） */
.checkbox-box.is-checked::after {
  content: '';
  position: absolute;
  width: 9px;
  height: 5px;
  left: 50%;
  top: 50%;
  border-left: 1.6px solid var(--fluent-text);
  border-bottom: 1.6px solid var(--fluent-text);
  transform: translate(-50%, -60%) rotate(-45deg);
}

/* 半选减号 */
.checkbox-dash {
  width: 10px;
  height: 2px;
  border-radius: 1px;
  background: var(--fluent-text);
}

/* 禁用态 */
.checkbox-box.is-disabled {
  pointer-events: none;
  cursor: default;
  background: var(--fluent-bg-active);
  border-color: var(--fluent-border);
  opacity: 0.5;
}

.checkbox-box.is-disabled.is-checked,
.checkbox-box.is-disabled.is-indeterminate {
  background: var(--fluent-text-secondary);
  border-color: var(--fluent-text-secondary);
  opacity: 0.45;
}

.checkbox-content {
  min-width: 0;
  cursor: pointer;
}

.checkbox-content.disabled {
  color: var(--fluent-text-secondary);
  opacity: 0.6;
  cursor: not-allowed;
}

@media (prefers-reduced-motion: reduce) {
  .checkbox-box {
    transition: none;
  }
}
</style>
