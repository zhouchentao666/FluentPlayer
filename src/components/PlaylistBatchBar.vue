<script lang="ts" setup>
import { ref, computed } from 'vue'
import type { Playlist, Song } from '../types'
import ComboBox, { type ComboBoxOption } from './settings/ComboBox.vue'

const props = defineProps<{
  selectedSongs: Song[]
  playlists: Playlist[]
  currentPlaylistId: string
  showDownload?: boolean
  showRemove?: boolean
  showReplace?: boolean
}>()

const emit = defineEmits<{
  remove: []
  'add-to-playlist': [playlistId: string]
  'replace-to-playlist': [playlistId: string]
  'select-all': []
  'clear-selection': []
  close: []
  download: []
}>()

const targetId = ref('')

const targetPlaylists = computed(() =>
  props.playlists.filter(p => p.id !== props.currentPlaylistId)
)

const hasTarget = computed(() => targetPlaylists.value.some(p => p.id === targetId.value))

const targetOptions = computed<ComboBoxOption[]>(() =>
  targetPlaylists.value.map(p => ({ value: p.id, label: p.name }))
)

function handleAdd() {
  if (!hasTarget.value) return
  emit('add-to-playlist', targetId.value)
}

function handleReplace() {
  if (!hasTarget.value) return
  emit('replace-to-playlist', targetId.value)
}
</script>

<template>
  <div class="batch-bar">
    <div class="batch-info">
      已选 <strong>{{ props.selectedSongs.length }}</strong> 首
    </div>

    <div class="batch-actions">
      <div class="action-wrap" data-tip="全选">
        <button class="icon-btn" @click="emit('select-all')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 11l3 3L22 4" />
            <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" />
          </svg>
        </button>
      </div>

      <div class="action-wrap" data-tip="清空选择">
        <button class="icon-btn" @click="emit('clear-selection')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6h14" />
            <path d="M10 11v6M14 11v6" />
          </svg>
        </button>
      </div>

      <div class="action-wrap" data-tip="从歌单移除" v-if="props.showRemove !== false">
        <button class="icon-btn danger" @click="emit('remove')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          </svg>
        </button>
      </div>

      <div class="divider" v-if="props.showRemove !== false"></div>

      <ComboBox
        v-model="targetId"
        class="batch-select"
        width="140px"
        placeholder="目标歌单"
        aria-label="选择目标歌单"
        :options="targetOptions"
      />

      <div class="action-wrap" data-tip="添加到歌单">
        <button class="icon-btn" :disabled="!hasTarget" @click="handleAdd">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 5v14M5 12h14" />
          </svg>
        </button>
      </div>

      <div class="action-wrap" data-tip="替换到歌单" v-if="props.showReplace !== false">
        <button class="icon-btn" :disabled="!hasTarget" @click="handleReplace">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M7 10h14l-4-4M17 14H3l4 4" />
          </svg>
        </button>
      </div>

      <div class="divider" v-if="props.showDownload"></div>

      <div class="action-wrap" data-tip="下载" v-if="props.showDownload">
        <button class="icon-btn" @click="emit('download')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 3v12M7 10l5 5 5-5" />
            <path d="M5 21h14" />
          </svg>
        </button>
      </div>

      <div class="divider"></div>

      <div class="action-wrap" data-tip="完成">
        <button class="icon-btn primary" @click="emit('close')">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M20 6L9 17l-5-5" />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.batch-bar {
  position: absolute;
  left: 50%;
  bottom: 20px;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 8px 14px;
  border-radius: 14px;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(24px) saturate(150%);
  -webkit-backdrop-filter: blur(24px) saturate(150%);
  box-shadow: 0 10px 36px rgba(0, 0, 0, 0.28);
  border: 1px solid var(--fluent-border);
  z-index: 20;
  animation: slide-up 0.25s cubic-bezier(0.22, 1, 0.36, 1);
}

@keyframes slide-up {
  from {
    opacity: 0;
    transform: translate(-50%, 20px);
  }
  to {
    opacity: 1;
    transform: translate(-50%, 0);
  }
}

.batch-info {
  font-size: 13px;
  white-space: nowrap;
  color: var(--fluent-text);
  padding-right: 6px;
}

.batch-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.divider {
  width: 1px;
  height: 22px;
  background: var(--fluent-border);
  margin: 0 4px;
}

.icon-btn {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--fluent-text);
  cursor: pointer;
  transition: background 0.18s ease, color 0.18s ease, opacity 0.18s ease;
}

.icon-btn:hover:not(:disabled) {
  background: var(--fluent-bg-hover);
}

.icon-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.icon-btn svg {
  width: 18px;
  height: 18px;
}

.icon-btn.danger {
  color: rgba(255, 90, 90, 1);
}

.icon-btn.danger:hover:not(:disabled) {
  background: rgba(255, 70, 70, 0.12);
}

.icon-btn.primary {
  color: var(--fluent-accent);
}

.icon-btn.primary:hover:not(:disabled) {
  background: rgba(var(--fluent-accent-rgb), 0.12);
}

.action-wrap {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.action-wrap::before {
  content: attr(data-tip);
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%) translateY(4px);
  padding: 5px 10px;
  border-radius: 6px;
  background: var(--fluent-bg-tooltip, rgba(30, 30, 30, 0.92));
  color: var(--fluent-text);
  font-size: 12px;
  white-space: nowrap;
  pointer-events: none;
  opacity: 0;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.action-wrap:hover::before {
  opacity: 1;
  transform: translateX(-50%) translateY(0);
}

.batch-select {
  flex: none;
}
</style>
