<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { Window, System } from '@bridge/runtime'

const emit = defineEmits<{
  close: []
}>()

const isMaximised = ref(false)

async function updateState() {
  isMaximised.value = await Window.IsMaximised()
}

function minimise() {
  Window.Minimise()
}

async function toggleMaximise() {
  Window.ToggleMaximise()
  await updateState()
}

function closeWindow() {
  emit('close')
}

onMounted(async () => {
  await updateState()
  if (System.IsWindows()) {
    // Windows theme is set to SystemDefault in Go window options
  }
})
</script>

<template>
  <div class="title-bar" data-tauri-drag-region @dblclick="toggleMaximise">
    <div class="title" data-tauri-drag-region>tideaudio</div>
    <div class="drag-region" data-tauri-drag-region></div>
    <div class="window-controls">
      <button class="control-btn" @click="minimise" @dblclick.stop>
        <svg viewBox="0 0 12 12" width="12" height="12">
          <rect x="1" y="5.5" width="10" height="1" fill="currentColor" />
        </svg>
      </button>
      <button class="control-btn" @click="toggleMaximise" @dblclick.stop>
        <svg v-if="isMaximised" viewBox="0 0 12 12" width="12" height="12">
          <path
            d="M1.5 3.5v7h7v-7h-7zm1 1h5v5h-5v-5zm6-2v-1h-8v8h1v-7h7z"
            fill="currentColor"
          />
        </svg>
        <svg v-else viewBox="0 0 12 12" width="12" height="12">
          <rect
            x="1.5"
            y="1.5"
            width="9"
            height="9"
            stroke="currentColor"
            stroke-width="1"
            fill="none"
          />
        </svg>
      </button>
      <button class="control-btn close" @click="closeWindow" @dblclick.stop>
        <svg viewBox="0 0 12 12" width="12" height="12">
          <path
            d="M1.5 1.5l9 9M10.5 1.5l-9 9"
            stroke="currentColor"
            stroke-width="1.2"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.title-bar {
  height: 36px;
  display: flex;
  align-items: center;
  padding: 0 10px 0 14px;
  user-select: none;
  color: var(--fluent-text);
  background: var(--fluent-bg-titlebar);
  border-bottom: 1px solid var(--fluent-border);
  text-align: left;
}

.title {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  opacity: 0.95;
}

.drag-region {
  flex: 1;
  height: 100%;
}

.window-controls {
  display: flex;
  gap: 2px;
}

.control-btn {
  width: 42px;
  height: 26px;
  border: none;
  background: transparent;
  color: inherit;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.18s ease;
}

.control-btn:hover {
  background: var(--fluent-bg-hover);
}

.control-btn.close:hover {
  background: var(--fluent-close-hover);
  color: white;
}
</style>
