<template>
  <div class="online-view">
    <div class="topbar">
      <div class="topbar-actions">
        <button class="ghost-btn" @click="linkModal = true">打开链接</button>
        <button class="ghost-btn" @click="sourcesModal = true">音源</button>
      </div>
    </div>

    <div v-show="!detailShown" class="body">
      <Transition name="tab-flip">
        <div :key="tab" class="tab-pane">
          <OnlineSearch
            v-if="tab === 'search'"
            :current-song="currentSong"
            :pinned="false"
            @play="(m, i) => emit('play-songs', m, i)"
            @queue="(m) => emit('add-to-queue', m)"
            @add-playlist="(m) => emit('add-to-playlist', m)"
            @download="emit('download', $event)"
            @add-all="emit('add-all', $event)"
            @download-all="emit('download-all', $event)"
          />
          <OnlineHotPlaylists
            v-else-if="tab === 'playlists'"
            @open="(it) => (detail = { source: it.source as OpenTarget['source'], id: it.id, kind: 'playlist', pinned: false })"
          />
          <OnlineHotAlbums
            v-else-if="tab === 'albums'"
            @open="(it) => (detail = { source: it.source as OpenTarget['source'], id: it.id, kind: 'album', pinned: false })"
          />
          <OnlineCharts
            v-else-if="tab === 'charts'"
            :current-song="currentSong"
            :pinned="false"
            @play="(m, i) => emit('play-songs', m, i)"
            @queue="(m) => emit('add-to-queue', m)"
            @add-playlist="(m) => emit('add-to-playlist', m)"
            @download="emit('download', $event)"
            @add-all="emit('add-all', $event)"
            @download-all="emit('download-all', $event)"
          />
        </div>
      </Transition>
    </div>

    <Transition
      name="slide-up"
      appear
      @after-enter="detailShown = true"
      @before-leave="detailShown = false"
    >
      <div v-if="detail" class="overlay">
        <OnlineDetail
          :key="detail.source + '-' + detail.id + '-' + detail.kind"
          :source="detail.source"
          :id="detail.id"
          :kind="detail.kind"
          :current-song="currentSong"
          :pinned="detail.pinned"
          @play="(m, i) => emit('play-songs', m, i)"
          @queue="(m) => emit('add-to-queue', m)"
          @add-playlist="(m) => emit('add-to-playlist', m)"
          @download="emit('download', $event)"
          @add-all="emit('add-all', $event)"
          @download-all="emit('download-all', $event)"
          @toggle-pin="emit('toggle-pin', $event)"
        />
        <button class="back" @click="detail = null">
          <svg viewBox="0 0 16 16" width="16" height="16">
            <path d="M10 3.5 5.5 8l4.5 4.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      </div>
    </Transition>

    <div v-if="sourcesModal" class="modal-mask" @click.self="sourcesModal = false">
      <div class="modal sources-modal">
        <div class="modal-title">音源下载优先级</div>
        <p class="modal-hint">拖动调整平台顺序，越靠前越优先用于下载 / 播放直链。</p>
        <ul class="src-list">
          <li v-for="(s, i) in sourceOrder" :key="s.id" class="src-item">
            <span class="src-name">{{ s.name }}</span>
            <span class="src-move">
              <button :disabled="i === 0" @click="moveSource(i, -1)">↑</button>
              <button :disabled="i === sourceOrder.length - 1" @click="moveSource(i, 1)">↓</button>
            </span>
          </li>
        </ul>
        <div class="modal-actions">
          <button class="ghost-btn" @click="sourcesModal = false">关闭</button>
        </div>
      </div>
    </div>

    <div v-if="linkModal" class="modal-mask" @click.self="linkModal = false">
      <div class="modal">
        <div class="modal-title">打开歌单 / 专辑链接</div>
        <input
          v-model="linkInput"
          class="link-input"
          placeholder="粘贴歌单或专辑链接 / ID…"
          @keyup.enter="openLink"
        />
        <div class="modal-actions">
          <button class="ghost-btn" @click="linkModal = false">取消</button>
          <button class="primary-btn" @click="openLink">打开</button>
        </div>
      </div>
    </div>

    <Transition name="add-pop">
      <div v-if="addMenu" class="add-menu" :style="{ left: addMenu.x + 'px', top: addMenu.y + 'px' }">
        <button @click="addMenu!.action('playlist')">收藏到歌单</button>
        <button @click="addMenu!.action('download')">下载到本地</button>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue'
import type { Song } from '../../types'
import type { PinnedOnlineItem, OnlineTab } from '../../composables/useConfig'
import OnlineSearch from './OnlineSearch.vue'
import OnlineHotPlaylists from './OnlineHotPlaylists.vue'
import OnlineHotAlbums from './OnlineHotAlbums.vue'
import OnlineCharts from './OnlineCharts.vue'
import OnlineDetail from './OnlineDetail.vue'

interface OpenTarget {
  source: 'wy' | 'kw' | 'kg' | 'tx' | 'mg'
  id: string
  kind: 'playlist' | 'album'
  pinned?: boolean
}

const props = defineProps<{
  playlists: unknown[]
  currentSong: Song | null
  openRequest: OpenTarget | null
  tab?: OnlineTab
}>()

const emit = defineEmits<{
  (e: 'opened'): void
  (e: 'play-songs', musics: any, index: number): void
  (e: 'add-to-queue', m: any): void
  (e: 'add-to-playlist', m: any): void
  (e: 'download', m: any): void
  (e: 'add-all', musics: any): void
  (e: 'download-all', musics: any): void
  (e: 'toggle-pin', item: PinnedOnlineItem): void
}>()

const detail = ref<OpenTarget | null>(null)
const detailShown = ref(false)
const linkModal = ref(false)
const linkInput = ref('')
const sourcesModal = ref(false)
const sourceOrder = ref([
  { id: 'wy', name: '网易云' },
  { id: 'tx', name: 'QQ音乐' },
  { id: 'kg', name: '酷狗' },
  { id: 'kw', name: '酷我' },
  { id: 'mg', name: '咪咕' },
])
const addMenu = ref<{ x: number; y: number; action: (kind: 'playlist' | 'download') => void } | null>(
  null
)

function openLink() {
  if (!linkInput.value.trim()) return
  detail.value = { source: 'wy', id: linkInput.value.trim(), kind: 'playlist', pinned: false }
  linkModal.value = false
  linkInput.value = ''
}

function moveSource(i: number, dir: -1 | 1) {
  const j = i + dir
  if (j < 0 || j >= sourceOrder.value.length) return
  const arr = sourceOrder.value
  ;[arr[i], arr[j]] = [arr[j], arr[i]]
}

function openAddMenu(x: number, y: number, action: (kind: 'playlist' | 'download') => void) {
  addMenu.value = { x, y, action }
}
function closeAddMenu() {
  addMenu.value = null
}

watch(detail, (d) => {
  if (!d) closeAddMenu()
})

watch(
  () => props.openRequest,
  (r) => {
    if (r) {
      detail.value = { ...r }
      emit('opened')
    }
  },
  { immediate: true }
)

function onGlobalClick() {
  if (addMenu.value) closeAddMenu()
}
window.addEventListener('click', onGlobalClick)
onBeforeUnmount(() => window.removeEventListener('click', onGlobalClick))
</script>

<style scoped>
.online-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;
  overflow: hidden;
}
.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 22px 10px;
  flex-shrink: 0;
}
.topbar-actions {
  display: flex;
  gap: 10px;
  margin-left: auto;
}
.ghost-btn {
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 13px;
  padding: 6px 14px;
  border-radius: 8px;
  cursor: pointer;
}
.ghost-btn:hover {
  background: var(--fluent-bg-hover);
}
.body {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.tab-pane {
  position: absolute;
  inset: 0;
  overflow-y: auto;
}
.tab-flip-enter-active {
  transition: transform 320ms cubic-bezier(0.22, 1, 0.36, 1);
  z-index: 2;
}
.tab-flip-leave-active {
  transition: opacity 320ms cubic-bezier(0.22, 1, 0.36, 1);
  z-index: 1;
}
.tab-flip-enter-from {
  transform: translateY(56px);
}
.tab-flip-leave-to {
  opacity: 0;
}
.overlay {
  position: absolute;
  inset: 0;
  z-index: 20;
  background: transparent;
}
.back {
  position: absolute;
  top: 16px;
  left: 16px;
  z-index: 5;
  width: 34px;
  height: 34px;
  border-radius: 50%;
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}
.back:hover {
  background: var(--fluent-bg-hover);
}
.slide-up-enter-active {
  transition: transform 340ms cubic-bezier(0.22, 1, 0.36, 1);
}
.slide-up-leave-active {
  transition: transform 340ms cubic-bezier(0.22, 1, 0.36, 1);
}
.slide-up-enter-from,
.slide-up-leave-to {
  transform: translateY(100%);
}
.modal-mask {
  position: absolute;
  inset: 0;
  z-index: 30;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
}
.modal {
  width: 380px;
  max-width: calc(100% - 40px);
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
  border-radius: 14px;
  padding: 20px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
}
.sources-modal {
  width: 320px;
}
.modal-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--fluent-text);
  margin-bottom: 10px;
}
.modal-hint {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  margin: 0 0 12px;
}
.link-input {
  width: 100%;
  box-sizing: border-box;
  padding: 9px 12px;
  border-radius: 9px;
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg);
  color: var(--fluent-text);
  font-size: 14px;
  outline: none;
}
.link-input:focus {
  border-color: var(--fluent-accent);
}
.src-list {
  list-style: none;
  margin: 0 0 12px;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.src-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--fluent-bg);
  border-radius: 9px;
}
.src-name {
  color: var(--fluent-text);
  font-size: 14px;
}
.src-move button {
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  width: 28px;
  height: 26px;
  border-radius: 6px;
  margin-left: 4px;
  cursor: pointer;
}
.src-move button:disabled {
  opacity: 0.4;
  cursor: default;
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 12px;
}
.primary-btn {
  border: none;
  background: var(--fluent-accent);
  color: #fff;
  font-size: 13px;
  padding: 7px 16px;
  border-radius: 8px;
  cursor: pointer;
}
.add-menu {
  position: fixed;
  z-index: 50;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  padding: 4px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
}
.add-menu button {
  border: none;
  background: transparent;
  color: var(--fluent-text);
  text-align: left;
  padding: 8px 14px;
  border-radius: 7px;
  font-size: 13px;
  cursor: pointer;
  white-space: nowrap;
}
.add-menu button:hover {
  background: var(--fluent-bg-hover);
}
.add-pop-enter-active,
.add-pop-leave-active {
  transition: opacity 120ms ease;
}
.add-pop-enter-from,
.add-pop-leave-to {
  opacity: 0;
}
</style>
