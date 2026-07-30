<script lang="ts" setup>
import { ref, watch, nextTick } from 'vue'
import type { Song } from '../types'
import { OpenImageFile, ReadImageFile, ReadLyrics } from '@bridge/app'
import { localMetadata, setLocalMetadata } from '../composables/useLocalMetadata'

const props = defineProps<{
  song: Song
}>()

const emit = defineEmits<{
  close: []
  save: []
}>()

const title = ref('')
const artist = ref('')
const album = ref('')
const cover = ref('')
const lyrics = ref('')
const isLoadingCover = ref(false)
const isLoadingDefaults = ref(false)

const defaultTitle = ref('')
const defaultArtist = ref('')
const defaultAlbum = ref('')
const defaultLyrics = ref('')

function setDefaults() {
  defaultTitle.value = props.song.metadata?.title ?? props.song.title ?? ''
  defaultArtist.value = props.song.metadata?.artist ?? ''
  defaultAlbum.value = props.song.metadata?.album ?? ''
}

async function loadDefaultLyrics() {
  try {
    defaultLyrics.value = await ReadLyrics(props.song.path)
  } catch {
    defaultLyrics.value = ''
  }
}

async function loadFromSong() {
  isLoadingDefaults.value = true
  setDefaults()
  await loadDefaultLyrics()
  isLoadingDefaults.value = false
  const override = localMetadata.value[props.song.path]
  title.value = override?.title ?? defaultTitle.value
  artist.value = override?.artist ?? defaultArtist.value
  album.value = override?.album ?? defaultAlbum.value
  cover.value = override?.cover ?? ''
  lyrics.value = override?.lyrics ?? defaultLyrics.value
}

watch(() => props.song, loadFromSong, { immediate: true })

async function selectCover() {
  try {
    const path = await OpenImageFile()
    if (!path) return
    isLoadingCover.value = true
    cover.value = await ReadImageFile(path)
  } catch {
    // ignore
  } finally {
    isLoadingCover.value = false
  }
}

function clearCover() {
  cover.value = ''
}

function changed(value: string, defaultValue: string): string | undefined {
  const trimmed = value.trim()
  return trimmed && trimmed !== defaultValue.trim() ? trimmed : undefined
}

function restoreDefaults() {
  title.value = defaultTitle.value
  artist.value = defaultArtist.value
  album.value = defaultAlbum.value
  cover.value = ''
  lyrics.value = defaultLyrics.value
}

function save() {
  setLocalMetadata(props.song.path, {
    title: changed(title.value, defaultTitle.value),
    artist: changed(artist.value, defaultArtist.value),
    album: changed(album.value, defaultAlbum.value),
    cover: cover.value.trim() || undefined,
    lyrics: changed(lyrics.value, defaultLyrics.value),
  })
  emit('save')
  emit('close')
}

function handleBackdropClick() {
  emit('close')
}

nextTick(() => {
  const input = document.querySelector('.editor-modal input') as HTMLInputElement | null
  input?.focus()
})
</script>

<template>
  <div class="editor-overlay" @click="handleBackdropClick">
    <div class="editor-modal" @click.stop>
      <div class="editor-header">
        <div>
          <h2>编辑歌曲信息</h2>
          <p class="header-hint">修改仅保存在本地，不会覆盖原歌曲文件元数据。</p>
        </div>
        <button class="close-btn" @click="emit('close')">✕</button>
      </div>

      <div class="editor-body">
        <label class="field">
          <span class="label">标题</span>
          <input v-model="title" type="text" placeholder="歌曲标题" />
        </label>

        <label class="field">
          <span class="label">艺术家</span>
          <input v-model="artist" type="text" placeholder="艺术家" />
        </label>

        <label class="field">
          <span class="label">专辑</span>
          <input v-model="album" type="text" placeholder="专辑" />
        </label>

        <label class="field">
          <span class="label">封面</span>
          <div class="cover-field">
            <input v-model="cover" type="text" placeholder="图片地址或点击下方按钮选择" readonly />
            <button type="button" :disabled="isLoadingCover" @click="selectCover">
              {{ isLoadingCover ? '...' : '选择' }}
            </button>
          </div>
          <div v-if="cover" class="cover-preview">
            <img :src="cover" alt="cover" />
            <button type="button" class="clear-cover" @click="clearCover">清除</button>
          </div>
        </label>

        <label class="field">
          <span class="label">歌词</span>
          <textarea v-model="lyrics" rows="8" :disabled="isLoadingDefaults" placeholder="粘贴歌词文本（支持 LRC / YRC / LRC A2）"></textarea>
        </label>
      </div>

      <div class="editor-footer">
        <button class="btn secondary" @click="restoreDefaults">恢复默认</button>
        <div class="actions">
          <button class="btn secondary" @click="emit('close')">取消</button>
          <button class="btn primary" @click="save">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.editor-overlay {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.45);
  z-index: 200;
}

.editor-modal {
  width: min(520px, calc(100vw - 40px));
  max-height: calc(100vh - 60px);
  display: flex;
  flex-direction: column;
  border: 1px solid var(--fluent-border);
  border-radius: 12px;
  background: var(--fluent-bg-card);
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.25);
  overflow: hidden;
}

.editor-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--fluent-border);
}

.editor-header h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.header-hint {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--fluent-text-secondary);
}

.close-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--fluent-text-secondary);
  font-size: 14px;
  cursor: pointer;
  transition: background 0.18s ease;
}

.close-btn:hover {
  background: var(--fluent-bg-hover);
}

.editor-body {
  padding: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.label {
  font-size: 12px;
  font-weight: 500;
  color: var(--fluent-text-secondary);
}

.field input,
.field textarea {
  padding: 8px 12px;
  border: 1px solid var(--fluent-border);
  border-radius: 8px;
  background: var(--fluent-bg-glass);
  color: var(--fluent-text);
  font-size: 13px;
  outline: none;
  transition: border-color 0.18s ease, background 0.18s ease;
}

.field input:focus,
.field textarea:focus {
  border-color: var(--fluent-accent);
  background: var(--fluent-bg-card);
}

.field textarea {
  resize: vertical;
  font-family: inherit;
  line-height: 1.5;
}

.cover-field {
  display: flex;
  gap: 8px;
}

.cover-field input {
  flex: 1;
}

.cover-field button {
  padding: 0 14px;
  border: 1px solid var(--fluent-border);
  border-radius: 8px;
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.18s ease;
}

.cover-field button:hover {
  background: var(--fluent-bg-active);
}

.cover-field button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.cover-preview {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 4px;
}

.cover-preview img {
  width: 64px;
  height: 64px;
  object-fit: cover;
  border-radius: 8px;
  border: 1px solid var(--fluent-border);
}

.clear-cover {
  padding: 6px 12px;
  border: 1px solid var(--fluent-border);
  border-radius: 6px;
  background: transparent;
  color: var(--fluent-text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.18s ease;
}

.clear-cover:hover {
  background: var(--fluent-bg-hover);
}

.editor-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 20px;
  border-top: 1px solid var(--fluent-border);
}

.hint {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}

.actions {
  display: flex;
  gap: 8px;
}

.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.18s ease;
}

.btn.secondary {
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
}

.btn.secondary:hover {
  background: var(--fluent-bg-active);
}

.btn.primary {
  background: var(--fluent-accent);
  color: #fff;
}

.btn.primary:hover {
  opacity: 0.9;
}
</style>
