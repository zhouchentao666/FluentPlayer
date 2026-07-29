<script lang="ts" setup>
import { ref, computed } from 'vue'

const props = defineProps<{
  modelValue?: string
  placeholder?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string | undefined]
}>()

const isRecording = ref(false)

const displayValue = computed(() => {
  if (!props.modelValue) return props.placeholder || '点击设置快捷键'
  return formatKey(props.modelValue)
})

function formatKey(key: string) {
  if (key === ' ' || key === 'Space') return '空格'
  const map: Record<string, string> = {
    ArrowUp: '↑',
    ArrowDown: '↓',
    ArrowLeft: '←',
    ArrowRight: '→',
    Enter: '回车',
    Escape: 'Esc',
    Tab: 'Tab',
    Backspace: '退格',
    Delete: '删除',
    AudioVolumeMute: '静音键',
    AudioVolumeDown: '音量减',
    AudioVolumeUp: '音量加',
    MediaPlayPause: '播放/暂停',
    MediaTrackNext: '下一首',
    MediaTrackPrevious: '上一首',
  }
  return map[key] || key
}

function handleClick() {
  isRecording.value = true
  window.addEventListener('keydown', onKeyDown, { once: true })
}

function onKeyDown(e: KeyboardEvent) {
  e.preventDefault()
  e.stopPropagation()
  if (e.key === 'Escape') {
    emit('update:modelValue', undefined)
  } else if (e.key !== 'Tab') {
    emit('update:modelValue', e.key)
  }
  isRecording.value = false
}

function clear(e: Event) {
  e.stopPropagation()
  emit('update:modelValue', undefined)
}
</script>

<template>
  <div class="hotkey-input" :class="{ recording: isRecording }" @click="handleClick">
    <span class="value">{{ isRecording ? '按任意键...' : displayValue }}</span>
    <button v-if="modelValue" class="clear-btn" @click="clear">✕</button>
  </div>
</template>

<style scoped>
.hotkey-input {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 120px;
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
  font-size: 13px;
  cursor: pointer;
  user-select: none;
  transition: border-color 0.18s ease, background 0.18s ease;
}

.hotkey-input:hover {
  background: var(--fluent-bg-active);
}

.hotkey-input.recording {
  border-color: var(--fluent-accent);
  background: rgba(var(--fluent-accent-rgb), 0.08);
}

.value {
  min-width: 80px;
  text-align: center;
}

.clear-btn {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--fluent-text-secondary);
  font-size: 11px;
  cursor: pointer;
}

.clear-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--fluent-text);
}
</style>
