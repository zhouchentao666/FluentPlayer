<script setup lang="ts">
import { ref, onMounted, inject, computed, type Ref } from 'vue'
import type { Song, Playlist as LocalPlaylist } from '../../types'
import type { Playlist } from '@online/lib/playlists'
import type { Album } from '@online/lib/albums'
import type { MusicInfo } from '@online/types/music'
import type { AppSettings, PinnedOnlineItem } from '../../composables/useConfig'
import { musicInfoToSong } from '@online/player'
import { useOnlineSources } from '@online/store'
import { parsePlaylistLink } from '@online/lib/playlists/openLink'
import { toast } from '../../composables/useToast'
import OnlineSearch from './OnlineSearch.vue'
import AudioMatch from './AudioMatch.vue'
import OnlineHotPlaylists from './OnlineHotPlaylists.vue'
import OnlineHotAlbums from './OnlineHotAlbums.vue'
import OnlineCharts from './OnlineCharts.vue'
import ComboBox, { type ComboBoxOption } from '../settings/ComboBox.vue'
import ToggleSwitch from '../settings/ToggleSwitch.vue'
import { downloadSong, downloadMany } from '@online/lib/download'
import type { OnlineTab } from '../../composables/useConfig'

type OpenTarget = { source: 'wy' | 'kw' | 'kg' | 'tx' | 'mg'; id: string; kind: 'playlist' | 'album' }

const props = defineProps<{
  playlists: LocalPlaylist[]
  currentSong: Song | null
  tab: OnlineTab
}>()
const emit = defineEmits<{
  (e: 'play-songs', songs: Song[], index: number): void
  (e: 'add-to-queue', song: Song): void
  (e: 'add-to-playlist', playlistId: string, songs: Song[]): void
  (e: 'open-detail', target: OpenTarget): void
  (e: 'update:tab', tab: OnlineTab): void
  (e: 'comment', m: MusicInfo): void
}>()

const settings = inject<Ref<AppSettings>>('settings')

const TABS: { id: OnlineTab; label: string }[] = [
  { id: 'playlists', label: '歌单' },
  { id: 'albums', label: '专辑' },
  { id: 'charts', label: '排行榜' },
  { id: 'search', label: '搜索' },
]
const tabTitle = computed(() => TABS.find((t) => t.id === props.tab)?.label ?? '')

/** 音源管理改为顶栏按钮弹层，让主标签维持「歌单 / 专辑 / 排行榜 / 搜索」四项。 */
const sourcesModal = ref(false)

/** 听歌识曲弹层 */
const recognizeModal = ref(false)
function openRecognize() {
  recognizeModal.value = true
}
function onRecognizePlay(musics: MusicInfo[], index: number) {
  emit('play-songs', musics.map(musicInfoToSong), index)
}
function onRecognizeSearch(keyword: string) {
  recognizeModal.value = false
  emit('update:tab', 'search')
  // 搜索页此时可能尚未挂载，延迟到挂载后再派发事件，确保监听器已注册
  setTimeout(() => {
    window.dispatchEvent(new CustomEvent('fluent-recognize-search', { detail: keyword }))
  }, 50)
}

const addMenu = ref<{ musics: MusicInfo[]; title: string } | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)

function onPlay(musics: MusicInfo[], index: number) {
  emit('play-songs', musics.map(musicInfoToSong), index)
}
function onQueue(m: MusicInfo) {
  emit('add-to-queue', musicInfoToSong(m))
}
function openAddMenu(musics: MusicInfo[], title: string) {
  addMenu.value = { musics, title }
}
function onAddPlaylist(m: MusicInfo) {
  openAddMenu([m], '收藏到歌单')
}
function onAddAll(musics: MusicInfo[]) {
  openAddMenu(musics, `收藏 ${musics.length} 首到歌单`)
}
function confirmAdd(plId: string) {
  if (!addMenu.value) return
  emit('add-to-playlist', plId, addMenu.value.musics.map(musicInfoToSong))
  addMenu.value = null
  toast('已添加到歌单', 'success')
}
function onDownload(m: MusicInfo) {
  // 真正的离线文件下载：解析直链并保存为本地文件
  downloadSong(m)
}
function onDownloadAll(musics: MusicInfo[]) {
  // 批量离线下载到所选文件夹
  downloadMany(musics)
}
function onOpen(item: Playlist | Album, kind: 'playlist' | 'album') {
  emit('open-detail', {
    source: item.source as 'wy' | 'kw' | 'kg' | 'tx' | 'mg',
    id: item.id,
    kind,
  })
}

// ---- 固定到侧栏 ----
const pinnedList = () => settings?.value.pinnedOnlinePlaylists ?? []
function isPinned(t: OpenTarget): boolean {
  return pinnedList().some((p) => p.source === t.source && p.id === t.id && p.kind === t.kind)
}
function onTogglePin(item: PinnedOnlineItem) {
  if (!settings) return
  const list = settings.value.pinnedOnlinePlaylists ?? []
  const idx = list.findIndex((p) => p.source === item.source && p.id === item.id && p.kind === item.kind)
  if (idx >= 0) {
    settings.value.pinnedOnlinePlaylists = list.filter((_, i) => i !== idx)
    toast('已取消固定', 'success')
  } else {
    settings.value.pinnedOnlinePlaylists = [...list, item]
    toast('已固定到侧栏', 'success')
  }
}

// ---- 打开外部歌单 / 专辑链接 ----
const linkModal = ref(false)
const linkSource = ref<'wy' | 'kw' | 'kg' | 'tx' | 'mg'>('wy')
const linkKind = ref<'playlist' | 'album'>('playlist')
const linkText = ref('')
const linkLoading = ref(false)
const linkSources: { id: 'wy' | 'kw' | 'kg' | 'tx' | 'mg'; label: string }[] = [
  { id: 'wy', label: '网易云' },
  { id: 'kw', label: '酷我' },
  { id: 'kg', label: '酷狗' },
  { id: 'tx', label: 'QQ' },
  { id: 'mg', label: '咪咕' },
]
const linkSourceOptions = computed<ComboBoxOption[]>(() =>
  linkSources.map(s => ({ value: s.id, label: s.label }))
)
function setLinkSource(value: string) {
  linkSource.value = value as typeof linkSource.value
}
async function openExternalLink() {
  const link = linkText.value.trim()
  if (!link) return
  linkLoading.value = true
  try {
    const id = await parsePlaylistLink(linkSource.value, link)
    linkModal.value = false
    linkText.value = ''
    emit('open-detail', { source: linkSource.value, id, kind: linkKind.value })
  } catch (err) {
    toast((err as Error).message || '链接解析失败', 'error')
  } finally {
    linkLoading.value = false
  }
}

// ---- 自定义音源 ----
const { state, importScript, importScriptFromUrl, removeScript, toggleEnabled, initOnlineSources } =
  useOnlineSources()
const importUrl = ref('')

onMounted(() => {
  initOnlineSources().catch(() => {})
})

async function onPickFile(e: Event) {
  const input = e.target as HTMLInputElement
  const files = input.files ? Array.from(input.files) : []
  if (files.length === 0) return
  let okCount = 0
  let failCount = 0
  const errors: string[] = []
  for (const file of files) {
    try {
      const text = await file.text()
      await importScript(text)
      okCount += 1
    } catch (err) {
      failCount += 1
      errors.push(`${file.name}: ${(err as Error).message}`)
    }
  }
  if (okCount > 0) {
    toast(`已导入 ${okCount} 个音源${failCount > 0 ? `，${failCount} 个失败` : ''}`, failCount > 0 ? 'warning' : 'success')
  } else if (errors.length > 0) {
    toast(errors[0], 'error')
  }
  input.value = ''
}
async function onImportUrl() {
  const u = importUrl.value.trim()
  if (!u) return
  try {
    await importScriptFromUrl(u)
    toast('音源已导入', 'success')
    importUrl.value = ''
  } catch (err) {
    toast((err as Error).message, 'error')
  }
}
function fmtPlatforms(s?: Record<string, unknown>): string {
  if (!s) return '通用'
  const keys = Object.keys(s)
  return keys.length ? keys.join(' / ') : '通用'
}
</script>

<template>
  <div class="online-view">
    <div class="topbar">
      <div class="topbar-title">{{ tabTitle }}</div>
      <div class="topbar-actions">
        <button class="open-link-btn" title="打开外部歌单 / 专辑链接" @click="linkModal = true">
          <svg viewBox="0 0 16 16" width="15" height="15"><path d="M6.5 9.5l3-3M7 4h4v4M9.5 6.5L12 4M4 12V6h3" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
          打开链接
        </button>
        <button class="open-link-btn" title="自定义音源管理" @click="sourcesModal = true">
          <svg viewBox="0 0 16 16" width="15" height="15"><path d="M8 2v12M3.5 5.5v5M12.5 5.5v5" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/></svg>
          音源
        </button>
        <button class="open-link-btn" title="听歌识曲" @click="openRecognize">
          <svg viewBox="0 0 16 16" width="15" height="15"><path d="M8 9.5a2.5 2.5 0 0 0 2.5-2.5V3a2.5 2.5 0 0 0-5 0v4A2.5 2.5 0 0 0 8 9.5zm3.5-2.5a3.5 3.5 0 0 1-7 0H3a5 5 0 0 0 4.5 4.95V15h1v-2.55A5 5 0 0 0 13 7h-1.5z" fill="currentColor"/></svg>
          听歌识曲
        </button>
      </div>
    </div>

    <div class="body">
      <Transition name="view-flip" mode="out-in">
        <OnlineSearch
          v-if="tab === 'search'"
          key="search"
          @play="onPlay"
          @queue="onQueue"
          @add-playlist="onAddPlaylist"
          @download="onDownload"
          @comment="(m) => emit('comment', m)"
          @open="onOpen"
        />
        <OnlineHotPlaylists v-else-if="tab === 'playlists'" key="playlists" @open="(item) => onOpen(item, 'playlist')" />
        <OnlineHotAlbums v-else-if="tab === 'albums'" key="albums" @open="(item) => onOpen(item, 'album')" />
        <OnlineCharts
          v-else-if="tab === 'charts'"
          key="charts"
          @play="onPlay"
          @queue="onQueue"
          @add-playlist="onAddPlaylist"
          @download="onDownload"
          @add-all="onAddAll"
          @download-all="onDownloadAll"
          @comment="(m) => emit('comment', m)"
        />
      </Transition>
    </div>

    <!-- 自定义音源管理 -->
    <div v-if="sourcesModal" class="modal-mask" @click.self="sourcesModal = false">
      <div class="sources-modal">
        <div class="sources-view">
        <div class="sources-head">
          <h2 class="title">自定义音源</h2>
          <div class="import-row">
            <button class="ghost" @click="fileInput?.click()">导入脚本文件</button>
            <input
              ref="fileInput"
              type="file"
              accept=".js,.txt"
              multiple
              style="display: none"
              @change="onPickFile"
            />
            <input v-model="importUrl" class="url-input" placeholder="粘贴音源脚本链接…" />
            <button class="ghost" @click="onImportUrl">从链接导入</button>
          </div>
        </div>

        <div v-if="state.error" class="src-error">{{ state.error }}</div>

        <div v-if="state.scripts.length === 0" class="state">
          暂无自定义音源。导入 LX 格式音源脚本后，即可在播放 / 搜索中自动使用。
        </div>

        <div v-else class="src-list">
          <div v-for="s in state.scripts" :key="s.id" class="src-item">
            <div class="src-main">
              <div class="src-name">{{ s.name }}</div>
              <div class="src-meta">
                <span v-if="s.author">作者：{{ s.author }}</span>
                <span v-if="s.version">版本：{{ s.version }}</span>
                <span>平台：{{ fmtPlatforms(s.sources) }}</span>
              </div>
            </div>
            <div class="src-actions">
              <ToggleSwitch
                :model-value="s.enabled"
                :aria-label="`启用音源 ${s.name}`"
                @update:model-value="toggleEnabled(s.id)"
              />
              <button class="del" title="删除" @click="removeScript(s.id)">删除</button>
            </div>
          </div>
        </div>
        </div>
        <button class="cancel" @click="sourcesModal = false">关闭</button>
      </div>
    </div>

    <!-- 打开外部歌单 / 专辑链接 -->
    <div v-if="linkModal" class="modal-mask" @click.self="linkModal = false">
      <div class="link-modal">
        <div class="link-title">打开外部歌单 / 专辑</div>
        <div class="link-hint">粘贴分享链接（如网易云歌单 / 专辑链接），选择对应平台后打开。</div>
        <div class="link-row">
          <ComboBox
            class="link-select"
            width="110px"
            aria-label="选择平台"
            :options="linkSourceOptions"
            :model-value="linkSource"
            @update:model-value="setLinkSource"
          />
          <div class="link-kind">
            <button :class="{ active: linkKind === 'playlist' }" @click="linkKind = 'playlist'">歌单</button>
            <button :class="{ active: linkKind === 'album' }" @click="linkKind = 'album'">专辑</button>
          </div>
          <input
            v-model="linkText"
            class="link-input"
            placeholder="https://music.163.com/playlist?id=..."
            @keyup.enter="openExternalLink"
          />
        </div>
        <div class="link-actions">
          <button class="cancel" @click="linkModal = false">取消</button>
          <button class="confirm" :disabled="linkLoading" @click="openExternalLink">
            {{ linkLoading ? '解析中…' : '打开' }}
          </button>
        </div>
      </div>
    </div>

    <!-- 收藏到歌单菜单 -->
    <div v-if="addMenu" class="modal-mask" @click.self="addMenu = null">
      <div class="add-menu">
        <div class="add-menu-title">{{ addMenu.title }}</div>
        <div class="add-menu-list">
          <button
            v-for="pl in playlists"
            :key="pl.id"
            class="add-menu-item"
            @click="confirmAdd(pl.id)"
          >
            {{ pl.name }}
            <span class="cnt">{{ pl.songs.length }}</span>
          </button>
          <div v-if="playlists.length === 0" class="state small">暂无歌单</div>
        </div>
        <button class="cancel" @click="addMenu = null">取消</button>
      </div>
    </div>

    <!-- 听歌识曲 -->
    <div v-if="recognizeModal" class="modal-mask" @click.self="recognizeModal = false">
      <div class="recognize-modal">
        <AudioMatch
          @play="onRecognizePlay"
          @search="onRecognizeSearch"
          @close="recognizeModal = false"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.online-view {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 28px 6px;
  flex-shrink: 0;
}
.topbar-actions {
  display: inline-flex;
  gap: 8px;
  flex-shrink: 0;
}
.open-link-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 34px;
  padding: 0 14px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 13px;
  cursor: pointer;
  flex-shrink: 0;
}
.open-link-btn:hover {
  background: var(--fluent-bg-hover);
}
.topbar-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--fluent-text);
}
.body {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
/*
 * 在线子标签切换：与主界面 view-flip 动画保持一致
 */
.view-flip-enter-active {
  position: absolute;
  inset: 0;
  transition: transform 320ms cubic-bezier(0.22, 1, 0.36, 1);
}
.view-flip-leave-active {
  position: absolute;
  inset: 0;
  transition: opacity 320ms cubic-bezier(0.22, 1, 0.36, 1);
}
.view-flip-enter-from {
  transform: translateY(56px);
}
.view-flip-leave-to {
  opacity: 0;
}

/* 音源管理 */
.sources-modal {
  width: 620px;
  max-width: 92%;
  max-height: 78%;
  display: flex;
  flex-direction: column;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(24px) saturate(160%);
  border: 1px solid var(--fluent-border);
  border-radius: 16px;
  padding: 8px 18px 18px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
}
.sources-modal > .cancel {
  align-self: flex-end;
  margin-top: 4px;
}
.sources-view {
  padding: 18px 10px 8px;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}
.sources-head {
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin-bottom: 18px;
}
.title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fluent-text);
  margin: 0;
}
.import-row {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  align-items: center;
}
.ghost {
  height: 36px;
  padding: 0 16px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 13px;
  cursor: pointer;
}
.ghost:hover {
  background: var(--fluent-bg-hover);
}
.url-input {
  flex: 1;
  min-width: 220px;
  height: 36px;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid var(--fluent-input-border);
  background: var(--fluent-input-bg);
  color: var(--fluent-text);
  outline: none;
  font-size: 13px;
}
.url-input:focus {
  border-color: var(--fluent-accent);
}
.src-error {
  color: #f87171;
  font-size: 13px;
  margin-bottom: 12px;
}
.src-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.src-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  border-radius: 12px;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
}
.src-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--fluent-text);
}
.src-meta {
  display: flex;
  gap: 14px;
  flex-wrap: wrap;
  margin-top: 4px;
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.src-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.del {
  border: 1px solid var(--fluent-border);
  background: transparent;
  color: var(--fluent-text-secondary);
  padding: 6px 12px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
}
.del:hover {
  color: #f87171;
  border-color: #f87171;
}
.state {
  color: var(--fluent-text-secondary);
  padding: 30px 0;
  text-align: center;
  font-size: 14px;
}
.state.small {
  padding: 12px 0;
}

/* 收藏菜单 */
.modal-mask {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 30;
}
/* 听歌识曲弹层：为内部组件提供其使用的 CSS 变量映射 */
.recognize-modal {
  width: 420px;
  max-width: 92%;
  max-height: 82%;
  display: flex;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(24px) saturate(160%);
  border: 1px solid var(--fluent-border);
  border-radius: 16px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
  overflow: hidden;
}
.add-menu {
  width: 360px;
  max-width: 90%;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(24px) saturate(160%);
  border: 1px solid var(--fluent-border);
  border-radius: 16px;
  padding: 18px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
}
.add-menu-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--fluent-text);
  margin-bottom: 12px;
}
.add-menu-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 320px;
  overflow-y: auto;
}
.add-menu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border: none;
  border-radius: 10px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 14px;
  cursor: pointer;
  text-align: left;
}
.add-menu-item:hover {
  background: var(--fluent-bg-hover);
}
.cnt {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.cancel {
  width: 100%;
  margin-top: 14px;
  height: 36px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: transparent;
  color: var(--fluent-text-secondary);
  cursor: pointer;
}

/* 打开外部链接弹窗 */
.link-modal {
  width: 440px;
  max-width: 92%;
  background: var(--fluent-bg-glass);
  backdrop-filter: blur(24px) saturate(160%);
  border: 1px solid var(--fluent-border);
  border-radius: 16px;
  padding: 20px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
}
.link-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--fluent-text);
  margin-bottom: 8px;
}
.link-hint {
  font-size: 12.5px;
  color: var(--fluent-text-secondary);
  line-height: 1.5;
  margin-bottom: 14px;
}
.link-row {
  display: flex;
  gap: 10px;
}
.link-select {
  flex-shrink: 0;
}
.link-select :deep(.win-combo) {
  min-height: 38px;
  border-radius: 10px;
}
.link-kind {
  display: inline-flex;
  height: 38px;
  border-radius: 10px;
  border: 1px solid var(--fluent-input-border);
  background: var(--fluent-input-bg);
  overflow: hidden;
  flex-shrink: 0;
}
.link-kind button {
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  font-size: 13px;
  padding: 0 12px;
  cursor: pointer;
}
.link-kind button.active {
  background: var(--fluent-accent);
  color: #fff;
}
.link-input {
  flex: 1;
  min-width: 0;
  height: 38px;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid var(--fluent-input-border);
  background: var(--fluent-input-bg);
  color: var(--fluent-text);
  outline: none;
  font-size: 13px;
}
.link-input:focus,

.link-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 16px;
}
.link-actions .cancel {
  width: auto;
  margin: 0;
  padding: 0 18px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
}
.link-actions .confirm {
  height: 36px;
  padding: 0 20px;
  border: none;
  border-radius: 10px;
  background: var(--fluent-accent);
  color: #fff;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
}
.link-actions .confirm:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
