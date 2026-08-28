<script lang="ts" setup>
/**
 * WinUI 3 风格组合框（移植自 WinUIonWeb 的 WinComboBox，去除对 WinScrollViewer / i18n 的依赖）
 * 特性：
 * - Teleport 到 body 的浮出层，不会被父级 overflow 裁剪
 * - 自动上/下翻转与视口边界收敛，跟随滚动/缩放实时定位
 * - 键盘导航：Up/Down/Home/End/Enter/Space/Esc/Tab，字符键快速跳转
 * - 选中项左侧显示 WinUI 指示条
 * - 点击外部、滚动祖先、窗口失焦自动关闭
 */
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'

export interface ComboBoxOption {
  value: string
  label: string
  disabled?: boolean
}

const props = withDefaults(
  defineProps<{
    /** 选项：字符串数组或 {value,label} 数组 */
    options: Array<ComboBoxOption | string>
    modelValue?: string
    /** 未选中时的占位文本 */
    placeholder?: string
    disabled?: boolean
    /** 控件宽度，如 '160px' / '100%'，默认自适应内容 */
    width?: string
    /** 浮出层最大高度 */
    maxDropdownHeight?: number
    ariaLabel?: string
  }>(),
  {
    modelValue: '',
    placeholder: '请选择',
    disabled: false,
    width: '',
    maxDropdownHeight: 320,
    ariaLabel: '',
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'change', value: string): void
}>()

const normalizedOptions = computed<ComboBoxOption[]>(() =>
  (props.options || []).map(opt =>
    typeof opt === 'string' ? { value: opt, label: opt } : { ...opt, label: opt.label ?? opt.value }
  )
)

const rootEl = ref<HTMLDivElement | null>(null)
const buttonEl = ref<HTMLButtonElement | null>(null)
const flyoutEl = ref<HTMLDivElement | null>(null)
const listEl = ref<HTMLDivElement | null>(null)

const isOpen = ref(false)
const isPressed = ref(false)
const highlightIndex = ref(-1)
const flyoutStyle = ref<Record<string, string>>({})
const openUp = ref(false)
/**
 * 浮出层 Teleport 到 body，会脱离带 data-theme 的容器（本项目把 data-theme 放在 App 根 div 上），
 * 因此这里向上查找最近的 data-theme 并复制到浮出层，保证主题变量正确继承。
 */
const flyoutTheme = ref<string | null>(null)

function resolveTheme() {
  const host = rootEl.value?.closest('[data-theme]') as HTMLElement | null
  const value = host?.dataset.theme || null
  if (value === 'system') {
    flyoutTheme.value = window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'
  } else {
    flyoutTheme.value = value
  }
}

const selectedIndex = computed(() =>
  normalizedOptions.value.findIndex(o => o.value === props.modelValue)
)

const displayText = computed(() => {
  const idx = selectedIndex.value
  return idx >= 0 ? normalizedOptions.value[idx].label : ''
})

/* ---------------- 定位 ---------------- */
function updatePosition() {
  const btn = buttonEl.value
  if (!btn) return
  const rect = btn.getBoundingClientRect()
  const vh = window.innerHeight
  const vw = window.innerWidth
  const margin = 8
  const gap = 4

  const spaceBelow = vh - rect.bottom - margin
  const spaceAbove = rect.top - margin
  const desired = props.maxDropdownHeight

  // 优先向下；下方不足且上方更宽裕时向上
  const shouldOpenUp = spaceBelow < Math.min(desired, 160) && spaceAbove > spaceBelow
  openUp.value = shouldOpenUp

  const available = shouldOpenUp ? spaceAbove - gap : spaceBelow - gap
  const maxHeight = Math.max(80, Math.min(desired, available))

  const width = rect.width
  let left = rect.left
  if (left + width > vw - margin) left = Math.max(margin, vw - margin - width)
  if (left < margin) left = margin

  const style: Record<string, string> = {
    position: 'fixed',
    left: `${Math.round(left)}px`,
    minWidth: `${Math.round(width)}px`,
    maxHeight: `${Math.round(maxHeight)}px`,
    zIndex: '2600',
  }
  if (shouldOpenUp) style.bottom = `${Math.round(vh - rect.top + gap)}px`
  else style.top = `${Math.round(rect.bottom + gap)}px`

  flyoutStyle.value = style
}

/* ---------------- 开关 ---------------- */
async function open() {
  if (props.disabled || isOpen.value) return
  if (normalizedOptions.value.length === 0) return
  resolveTheme()
  isOpen.value = true
  highlightIndex.value = selectedIndex.value >= 0 ? selectedIndex.value : firstEnabledIndex()
  updatePosition()
  await nextTick()
  updatePosition()
  scrollHighlightIntoView('auto')
  bindGlobal()
}

function close(focusBack = false) {
  if (!isOpen.value) return
  isOpen.value = false
  highlightIndex.value = -1
  unbindGlobal()
  if (focusBack) buttonEl.value?.focus()
}

function toggle() {
  if (isOpen.value) close(true)
  else open()
}

/* ---------------- 选择 ---------------- */
function firstEnabledIndex() {
  return normalizedOptions.value.findIndex(o => !o.disabled)
}

function selectIndex(index: number) {
  const opt = normalizedOptions.value[index]
  if (!opt || opt.disabled) return
  close(true)
  if (opt.value !== props.modelValue) {
    emit('update:modelValue', opt.value)
    emit('change', opt.value)
  }
}

function moveHighlight(step: number) {
  const list = normalizedOptions.value
  if (list.length === 0) return
  let idx = highlightIndex.value
  for (let i = 0; i < list.length; i++) {
    idx = idx + step
    if (idx < 0) idx = list.length - 1
    if (idx >= list.length) idx = 0
    if (!list[idx].disabled) {
      highlightIndex.value = idx
      scrollHighlightIntoView()
      return
    }
  }
}

function setHighlightEdge(toEnd: boolean) {
  const list = normalizedOptions.value
  const indices = toEnd ? [...list.keys()].reverse() : [...list.keys()]
  for (const i of indices) {
    if (!list[i].disabled) {
      highlightIndex.value = i
      scrollHighlightIntoView()
      return
    }
  }
}

function scrollHighlightIntoView(behavior: ScrollBehavior = 'auto') {
  nextTick(() => {
    const container = listEl.value
    if (!container) return
    const el = container.querySelector<HTMLElement>(`[data-index="${highlightIndex.value}"]`)
    if (!el) return
    const cTop = container.scrollTop
    const cBottom = cTop + container.clientHeight
    const eTop = el.offsetTop
    const eBottom = eTop + el.offsetHeight
    if (eTop < cTop) container.scrollTo({ top: eTop, behavior })
    else if (eBottom > cBottom) container.scrollTo({ top: eBottom - container.clientHeight, behavior })
  })
}

/* ---------------- 键盘 ---------------- */
let typeBuffer = ''
let typeTimer: ReturnType<typeof setTimeout> | null = null

function typeAhead(char: string) {
  typeBuffer += char.toLowerCase()
  if (typeTimer) clearTimeout(typeTimer)
  typeTimer = setTimeout(() => {
    typeBuffer = ''
  }, 700)
  const list = normalizedOptions.value
  const found = list.findIndex(o => !o.disabled && o.label.toLowerCase().startsWith(typeBuffer))
  if (found >= 0) {
    if (isOpen.value) {
      highlightIndex.value = found
      scrollHighlightIntoView()
    } else {
      selectIndex(found)
    }
  }
}

function onKeyDown(e: KeyboardEvent) {
  if (props.disabled) return
  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault()
      if (!isOpen.value) open()
      else moveHighlight(1)
      break
    case 'ArrowUp':
      e.preventDefault()
      if (!isOpen.value) open()
      else moveHighlight(-1)
      break
    case 'Home':
      if (isOpen.value) {
        e.preventDefault()
        setHighlightEdge(false)
      }
      break
    case 'End':
      if (isOpen.value) {
        e.preventDefault()
        setHighlightEdge(true)
      }
      break
    case 'Enter':
      e.preventDefault()
      if (isOpen.value) selectIndex(highlightIndex.value)
      else open()
      break
    case ' ':
      e.preventDefault()
      if (isOpen.value) selectIndex(highlightIndex.value)
      else open()
      break
    case 'Escape':
      if (isOpen.value) {
        e.preventDefault()
        e.stopPropagation()
        close(true)
      }
      break
    case 'Tab':
      if (isOpen.value) close(false)
      break
    default:
      if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
        typeAhead(e.key)
      }
  }
}

/* ---------------- 全局事件 ---------------- */
function onDocPointerDown(e: PointerEvent) {
  const target = e.target as Node | null
  if (!target) return
  if (rootEl.value?.contains(target)) return
  if (flyoutEl.value?.contains(target)) return
  close(false)
}

function onScrollOrResize() {
  if (!isOpen.value) return
  updatePosition()
}

function onWindowBlur() {
  close(false)
}

let globalBound = false
function bindGlobal() {
  if (globalBound) return
  globalBound = true
  document.addEventListener('pointerdown', onDocPointerDown, true)
  window.addEventListener('scroll', onScrollOrResize, true)
  window.addEventListener('resize', onScrollOrResize)
  window.addEventListener('blur', onWindowBlur)
}

function unbindGlobal() {
  if (!globalBound) return
  globalBound = false
  document.removeEventListener('pointerdown', onDocPointerDown, true)
  window.removeEventListener('scroll', onScrollOrResize, true)
  window.removeEventListener('resize', onScrollOrResize)
  window.removeEventListener('blur', onWindowBlur)
}

// 选项变化时若已展开则重新定位；选项清空则关闭
watch(
  () => normalizedOptions.value.length,
  len => {
    if (!isOpen.value) return
    if (len === 0) close(false)
    else nextTick(updatePosition)
  }
)

onBeforeUnmount(() => {
  unbindGlobal()
  if (typeTimer) clearTimeout(typeTimer)
})
</script>

<template>
  <div ref="rootEl" class="win-combo-root" :style="props.width ? { width: props.width } : undefined">
    <button
      ref="buttonEl"
      type="button"
      class="win-combo"
      :class="{ open: isOpen, pressed: isPressed, disabled: props.disabled }"
      :disabled="props.disabled"
      role="combobox"
      aria-haspopup="listbox"
      :aria-expanded="isOpen"
      :aria-label="props.ariaLabel || undefined"
      @click="toggle"
      @keydown="onKeyDown"
      @pointerdown="isPressed = true"
      @pointerup="isPressed = false"
      @pointerleave="isPressed = false"
      @blur="isPressed = false"
    >
      <span class="win-combo-text" :class="{ placeholder: selectedIndex < 0 }">
        {{ displayText || props.placeholder }}
      </span>
      <span class="win-combo-chevron" :class="{ up: isOpen && openUp }">
        <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
          <path
            d="M2.5 4.5L6 8L9.5 4.5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </span>
    </button>

    <Teleport to="body">
      <Transition :name="openUp ? 'combo-flyout-up' : 'combo-flyout-down'">
        <div
          v-if="isOpen"
          ref="flyoutEl"
          class="win-combo-flyout"
          :data-theme="flyoutTheme || undefined"
          :style="flyoutStyle"
          role="listbox"
          @keydown="onKeyDown"
        >
          <div ref="listEl" class="win-combo-list">
            <div
              v-for="(opt, index) in normalizedOptions"
              :key="opt.value"
              class="win-combo-item"
              :class="{
                selected: opt.value === props.modelValue,
                highlighted: index === highlightIndex,
                disabled: opt.disabled,
              }"
              :data-index="index"
              role="option"
              :aria-selected="opt.value === props.modelValue"
              :aria-disabled="opt.disabled || undefined"
              @pointerenter="!opt.disabled && (highlightIndex = index)"
              @click="selectIndex(index)"
            >
              <span class="win-combo-indicator"></span>
              <span class="win-combo-item-text">{{ opt.label }}</span>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.win-combo-root {
  position: relative;
  display: inline-flex;
  vertical-align: middle;
  max-width: 100%;
}

/* ---------------- 按钮本体 ---------------- */
.win-combo {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-width: 0;
  box-sizing: border-box;
  min-height: 32px;
  padding: 5px 10px 5px 11px;
  border: 1px solid var(--fluent-input-border);
  border-radius: 6px;
  background: var(--fluent-input-bg);
  color: var(--fluent-text);
  font-size: 13px;
  font-family: inherit;
  line-height: 20px;
  text-align: left;
  cursor: pointer;
  outline: none;
  transition:
    background-color 0.12s ease,
    border-color 0.12s ease;
}

.win-combo:hover:not(.disabled) {
  background: var(--fluent-bg-hover);
}

.win-combo.pressed:not(.disabled),
.win-combo.open:not(.disabled) {
  background: var(--fluent-bg-active);
}

.win-combo:focus-visible {
  outline: 2px solid var(--fluent-text);
  outline-offset: 1px;
}

.win-combo.disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.win-combo-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.win-combo-text.placeholder {
  color: var(--fluent-text-secondary);
}

.win-combo-chevron {
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--fluent-text-secondary);
  transition: transform 0.15s cubic-bezier(0.16, 1, 0.3, 1);
}

.win-combo.pressed:not(.disabled) .win-combo-chevron {
  transform: translateY(1px);
}

.win-combo-chevron.up {
  transform: rotate(180deg);
}
</style>

<style>
/* 浮出层 Teleport 到 body，需为非 scoped 样式 */
/* 浮出层脱离了主题容器，但 CSS 变量定义在 :root / [data-theme] 上，body 下同样可继承 */
.win-combo-flyout {
  box-sizing: border-box;
  padding: 4px;
  border: 1px solid var(--fluent-border);
  border-radius: 8px;
  /* 底色用不透明基色 + 主题浮层叠加，保证在任意背景上都清晰可读 */
  background:
    linear-gradient(var(--fluent-bg-card), var(--fluent-bg-card)),
    var(--combo-flyout-base, #2b2b2b);
  backdrop-filter: blur(30px) saturate(160%);
  -webkit-backdrop-filter: blur(30px) saturate(160%);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.28);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.win-combo-flyout[data-theme='light'] {
  --combo-flyout-base: #f7f7f7;
}

.win-combo-list {
  overflow-y: auto;
  overflow-x: hidden;
  scrollbar-width: thin;
  scrollbar-color: var(--fluent-scrollbar) transparent;
}

.win-combo-list::-webkit-scrollbar {
  width: 6px;
}

.win-combo-list::-webkit-scrollbar-thumb {
  background: var(--fluent-scrollbar);
  border-radius: 3px;
}

.win-combo-list::-webkit-scrollbar-track {
  background: transparent;
}

.win-combo-item {
  position: relative;
  display: flex;
  align-items: center;
  min-height: 32px;
  padding: 5px 11px 5px 12px;
  border-radius: 5px;
  color: var(--fluent-text);
  font-size: 13px;
  line-height: 20px;
  cursor: pointer;
  user-select: none;
  transition: background-color 0.1s ease;
}

.win-combo-item + .win-combo-item {
  margin-top: 2px;
}

.win-combo-item.highlighted:not(.disabled) {
  background: var(--fluent-bg-hover);
}

.win-combo-item.selected {
  background: var(--fluent-bg-active);
}

.win-combo-item.selected.highlighted {
  background: var(--fluent-bg-active);
}

.win-combo-item.disabled {
  color: var(--fluent-text-secondary);
  opacity: 0.5;
  cursor: not-allowed;
}

.win-combo-item-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* WinUI 选中指示条 */
.win-combo-indicator {
  position: absolute;
  left: 0;
  top: 50%;
  width: 3px;
  height: 0;
  border-radius: 2px;
  background: var(--fluent-accent);
  transform: translateY(-50%);
  transition:
    height 0.15s cubic-bezier(0.16, 1, 0.3, 1),
    opacity 0.15s ease;
  opacity: 0;
}

.win-combo-item.selected .win-combo-indicator {
  height: 16px;
  opacity: 1;
}

.win-combo-item.selected:active .win-combo-indicator {
  height: 10px;
}

/* ---------------- 浮出动画 ---------------- */
.combo-flyout-down-enter-active {
  animation: combo-flyout-down-in 0.15s cubic-bezier(0.16, 1, 0.3, 1);
}

.combo-flyout-up-enter-active {
  animation: combo-flyout-up-in 0.15s cubic-bezier(0.16, 1, 0.3, 1);
}

.combo-flyout-down-leave-active,
.combo-flyout-up-leave-active {
  animation: combo-flyout-out 0.1s ease forwards;
}

@keyframes combo-flyout-down-in {
  from {
    opacity: 0;
    transform: translateY(-6px) scaleY(0.96);
    transform-origin: top center;
  }
  to {
    opacity: 1;
    transform: translateY(0) scaleY(1);
    transform-origin: top center;
  }
}

@keyframes combo-flyout-up-in {
  from {
    opacity: 0;
    transform: translateY(6px) scaleY(0.96);
    transform-origin: bottom center;
  }
  to {
    opacity: 1;
    transform: translateY(0) scaleY(1);
    transform-origin: bottom center;
  }
}

@keyframes combo-flyout-out {
  to {
    opacity: 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .combo-flyout-down-enter-active,
  .combo-flyout-up-enter-active,
  .combo-flyout-down-leave-active,
  .combo-flyout-up-leave-active {
    animation: none;
  }
  .win-combo-indicator,
  .win-combo-chevron {
    transition: none;
  }
}
</style>
