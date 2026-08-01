<script lang="ts" setup>
import { ref, onMounted, computed } from 'vue'
import { Events, Window } from '@bridge/runtime'
import { LoadConfig, SaveConfig, ReadMetadata, ReadLyrics, OpenImageFile, ReadImageFile, EmitMetadataChanged } from '@bridge/app'
import type { AppConfig } from '@bridge/models'
import { localMetadata, type LocalSongMetadata } from './composables/useLocalMetadata'
import type { Song, SongMetadata } from './types'
import ComboBox, { type ComboBoxOption } from './components/settings/ComboBox.vue'

const song = ref<Song | null>(null)
const loading = ref(true)
const error = ref('')
const theme = ref<'system' | 'light' | 'dark'>('system')

const title = ref('')
const artist = ref('')
const album = ref('')
const cover = ref('')
const lyrics = ref('')
const lyricsFormat = ref<'auto' | 'lrc' | 'lrc-a2' | 'yrc' | 'qrc' | 'eslrc' | 'ttml'>('auto')
const isLoadingCover = ref(false)
const isLoadingDefaults = ref(false)

const defaultTitle = ref('')
const defaultArtist = ref('')
const defaultAlbum = ref('')
const defaultLyrics = ref('')

const effectiveTheme = computed(() => {
  if (theme.value === 'system') {
    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'
  }
  return theme.value
})

const bgColor = computed(() => effectiveTheme.value === 'light' ? '#ffffff' : '#000000')

const lyricsFormats = [
  { value: 'auto', label: '自动检测', desc: '自动识别歌词格式' },
  { value: 'lrc', label: 'LRC', desc: '标准逐行歌词，最常见格式' },
  { value: 'lrc-a2', label: 'LRC A2', desc: '增强版 LRC，支持逐字时间戳' },
  { value: 'yrc', label: 'YRC', desc: '网易云逐字歌词' },
  { value: 'qrc', label: 'QRC', desc: 'QQ音乐逐字歌词' },
  { value: 'eslrc', label: 'ESLyRiC', desc: 'Lyricify 逐字音节歌词' },
  { value: 'ttml', label: 'TTML', desc: 'AMLL 主格式，支持翻译/音译/对唱等全部特性' },
]

const currentFormatDesc = computed(() => {
  const format = lyricsFormats.find(f => f.value === lyricsFormat.value)
  return format?.desc || ''
})

const lyricsFormatOptions = computed<ComboBoxOption[]>(() =>
  lyricsFormats.map(f => ({ value: f.value, label: f.label }))
)

function setLyricsFormat(value: string) {
  lyricsFormat.value = value as typeof lyricsFormat.value
}

function pathToTitle(path: string): string {
  return path.replace(/\\/g, '/').split('/').pop()?.replace(/\.[^.]+$/, '') ?? path
}

async function loadSong(path: string): Promise<Song> {
  let metadata: SongMetadata | undefined
  try {
    const meta = await ReadMetadata(path)
    metadata = {
      title: meta.title ?? '',
      artist: meta.artist ?? '',
      album: meta.album ?? '',
      genre: meta.genre ?? '',
      year: meta.year ?? '',
      duration: meta.duration ?? 0,
      bitrate: meta.bitrate ?? 0,
    }
  } catch {
    metadata = undefined
  }
  return {
    id: 'editor',
    path,
    title: metadata?.title || pathToTitle(path),
    metadata,
  }
}

function setDefaults(song: Song) {
  defaultTitle.value = song.metadata?.title ?? song.title ?? ''
  defaultArtist.value = song.metadata?.artist ?? ''
  defaultAlbum.value = song.metadata?.album ?? ''
}

async function loadDefaultLyrics(path: string) {
  try {
    defaultLyrics.value = await ReadLyrics(path)
  } catch {
    defaultLyrics.value = ''
  }
}

async function loadFromMetadata(song: Song) {
  isLoadingDefaults.value = true
  setDefaults(song)
  await loadDefaultLyrics(song.path)
  isLoadingDefaults.value = false
  const override = localMetadata.value[song.path]
  title.value = override?.title ?? defaultTitle.value
  artist.value = override?.artist ?? defaultArtist.value
  album.value = override?.album ?? defaultAlbum.value
  cover.value = override?.cover ?? ''
  lyrics.value = override?.lyrics ?? defaultLyrics.value
  lyricsFormat.value = override?.lyricsFormat ?? 'auto'
}

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
  lyricsFormat.value = 'auto'
}

async function save() {
  if (!song.value) return
  const path = song.value.path
  const newMeta: LocalSongMetadata = {
    title: changed(title.value, defaultTitle.value),
    artist: changed(artist.value, defaultArtist.value),
    album: changed(album.value, defaultAlbum.value),
    cover: cover.value.trim() || undefined,
    lyrics: changed(lyrics.value, defaultLyrics.value),
    lyricsFormat: lyricsFormat.value === 'auto' ? undefined : lyricsFormat.value,
  }
  // 合并更新，确保 Vue 响应式检测
  const existing = localMetadata.value[path] || {}
  const merged = { ...existing, ...newMeta }
  // 移除 undefined 字段
  const keys: (keyof LocalSongMetadata)[] = ['title', 'artist', 'album', 'cover', 'lyrics', 'lyricsFormat']
  for (const key of keys) {
    if (merged[key] === undefined) {
      delete merged[key]
    }
  }
  localMetadata.value = { ...localMetadata.value, [path]: merged }

  try {
    const config = await LoadConfig()
    const appConfig: AppConfig = {
      playlists: (config.playlists as any) ?? null,
      settings: {
        ...(config.settings as any),
        localMetadata: { ...localMetadata.value },
      },
      playback: config.playback as any,
      window: config.window as any,
    }
    await SaveConfig(appConfig)
    await EmitMetadataChanged()
  } catch {
    // ignore save errors
  }
  close()
}

function close() {
  Window.Close().catch(() => window.close())
}

onMounted(async () => {
  const params = new URLSearchParams(window.location.search)
  const path = params.get('path') || ''
  if (!path) {
    error.value = '未指定歌曲路径'
    loading.value = false
    return
  }
  try {
    const config = await LoadConfig()
    if (config.settings?.localMetadata) {
      localMetadata.value = { ...config.settings.localMetadata as Record<string, LocalSongMetadata> }
    }
    if (config.settings?.theme) {
      theme.value = config.settings.theme as 'system' | 'light' | 'dark'
    }
    song.value = await loadSong(path)
    await loadFromMetadata(song.value)
  } catch (e) {
    error.value = '加载歌曲信息失败'
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div
    class="editor-root"
    :data-theme="effectiveTheme"
    :style="{ backgroundColor: bgColor }"
  >
    <div v-if="loading" class="state">加载中...</div>
    <div v-else-if="error" class="state">{{ error }}</div>
    <template v-else-if="song">
      <div class="editor-header">
        <div>
          <h2>编辑歌曲信息</h2>
          <p class="header-hint">修改仅保存在本地，不会覆盖原歌曲文件元数据。</p>
        </div>
        <button class="close-btn" @click="close">✕</button>
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
          <div class="lyrics-header">
            <span class="label">歌词</span>
            <ComboBox
              class="format-select"
              width="150px"
              aria-label="歌词格式"
              :options="lyricsFormatOptions"
              :model-value="lyricsFormat"
              @update:model-value="setLyricsFormat"
            />
          </div>
          <p class="format-desc">{{ currentFormatDesc }}</p>
          <textarea v-model="lyrics" rows="8" :disabled="isLoadingDefaults" placeholder="粘贴歌词文本"></textarea>
          <a class="lyrics-help" href="https://amll.dev/guides/lyric/formats" target="_blank">查看歌词格式文档</a>
        </label>
      </div>

      <div class="editor-footer">
        <button class="btn secondary" @click="restoreDefaults">恢复默认</button>
        <div class="actions">
          <button class="btn secondary" @click="close">取消</button>
          <button class="btn primary" @click="save">保存</button>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.editor-root {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  color: var(--fluent-text);
  font-family: "Segoe UI Variable", "Segoe UI", -apple-system, BlinkMacSystemFont, sans-serif;
}

.state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  font-size: 13px;
  color: var(--fluent-text-secondary);
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

.lyrics-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.format-select {
  flex: none;
}

.format-select :deep(.win-combo) {
  min-height: 28px;
  font-size: 12px;
}

.format-desc {
  margin: 0;
  font-size: 11px;
  color: var(--fluent-text-secondary);
  opacity: 0.8;
}

.lyrics-help {
  font-size: 11px;
  color: var(--fluent-accent);
  text-decoration: none;
  opacity: 0.9;
}

.lyrics-help:hover {
  opacity: 1;
}

.editor-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 20px;
  border-top: 1px solid var(--fluent-border);
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