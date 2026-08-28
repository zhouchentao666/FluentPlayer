<script lang="ts" setup>
import { ref, nextTick } from 'vue'
import { type Playlist } from '../../types'

const props = defineProps<{
  playlist: Playlist
  selected: boolean
}>()

const emit = defineEmits<{
  (e: 'select', id: string): void
  (e: 'rename', id: string, name: string): void
  (e: 'delete', id: string): void
  (e: 'drop-songs', payload: { sourcePlaylistId: string; songIds: string[] }): void
}>()

const isEditing = ref(false)
const editName = ref('')
const inputRef = ref<HTMLInputElement | null>(null)
const isDragOver = ref(false)

function startRename() {
  isEditing.value = true
  editName.value = props.playlist.name
  nextTick(() => {
    inputRef.value?.focus()
    inputRef.value?.select()
  })
}

function confirmRename() {
  const name = editName.value.trim()
  if (name) {
    emit('rename', props.playlist.id, name)
  }
  isEditing.value = false
  editName.value = ''
}

function cancelRename() {
  isEditing.value = false
  editName.value = ''
}

function onClick() {
  if (isEditing.value) return
  emit('select', props.playlist.id)
}

function onDragOver(e: DragEvent) {
  const types = e.dataTransfer?.types || []
  if (!types.includes('application/sugarplayer-song')) return
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  isDragOver.value = true
}

function onDragLeave() {
  isDragOver.value = false
}

function onDrop(e: DragEvent) {
  isDragOver.value = false
  const raw = e.dataTransfer?.getData('application/sugarplayer-song')
  if (!raw) return
  const { songId, sourcePlaylistId } = JSON.parse(raw) as { songId: string; sourcePlaylistId: string }
  if (sourcePlaylistId === props.playlist.id) return
  e.preventDefault()
  emit('drop-songs', { sourcePlaylistId, songIds: [songId] })
}
</script>

<template>
  <li
    :class="['playlist-item', { active: selected, 'drag-over': isDragOver }]"
    @click="onClick"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
  >
    <template v-if="isEditing">
      <input
        ref="inputRef"
        v-model="editName"
        class="edit-input"
        type="text"
        @keydown.enter="confirmRename"
        @keydown.esc="cancelRename"
        @blur="confirmRename"
        @click.stop
      />
    </template>
    <template v-else>
      <span class="icon">♥</span>
      <span class="name">{{ playlist.name }}</span>
      <span class="spacer"></span>
      <span class="actions">
        <button class="action-btn" title="重命名" @click.stop="startRename">✎</button>
        <button
          v-if="playlist.id !== 'favorites'"
          class="action-btn delete"
          title="删除"
          @click.stop="emit('delete', playlist.id)"
        >🗑</button>
      </span>
    </template>
  </li>
</template>

<style scoped>
.playlist-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 13px;
  transition: background 0.18s ease, transform 0.1s ease;
}

.playlist-item:hover {
  background: var(--fluent-bg-hover);
}

.playlist-item:active {
  transform: scale(0.995);
}

.playlist-item.active {
  background: var(--fluent-bg-active);
}

.playlist-item.drag-over {
  background: var(--fluent-bg-active);
  outline: 1px dashed var(--fluent-accent);
  outline-offset: -2px;
}

.playlist-item .icon {
  font-size: 12px;
  opacity: 0.8;
  flex-shrink: 0;
}

.playlist-item .name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 0 1 auto;
  min-width: 0;
}

.spacer {
  flex: 1;
  min-width: 8px;
}

.actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.18s ease;
}

.playlist-item:hover .actions {
  opacity: 1;
}

.action-btn {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: inherit;
  font-size: 11px;
  cursor: pointer;
  transition: background 0.18s ease;
}

.action-btn:hover {
  background: var(--fluent-bg-hover);
}

.action-btn.delete:hover {
  background: var(--fluent-close-hover);
  color: white;
}

.edit-input {
  width: 100%;
  box-sizing: border-box;
  padding: 4px 8px;
  border: 1px solid var(--fluent-input-border);
  border-radius: 6px;
  background: var(--fluent-input-bg);
  color: inherit;
  font-size: 13px;
  outline: none;
}
</style>
