<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, type CSSProperties } from 'vue'
import { LocalUserDetailStore, type PlaylistRow } from '@/store/LocalUserDetail'
import { MessagePlugin } from 'tdesign-vue-next'
import LiquidGlass from '@/components/LiquidGlass.vue'
import type { SongList } from '@/types/audio'
import defaultCover from '/default-cover.png'

const { t } = useI18n()

const props = defineProps<{
  visible: boolean
  songs: SongList[]
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  'added': [playlistId: string]
}>()

const localUserStore = LocalUserDetailStore()
const loading = ref(false)
const creating = ref(false)
const showCreate = ref(false)
const newName = ref('')
const searchQuery = ref('')

// Responsive corner-radius for mobile
const isMobile = ref(false)
const mobileMql = typeof window !== 'undefined' ? window.matchMedia('(max-width: 768px)') : null

const onMobileChange = (e: MediaQueryListEvent | MediaQueryList) => {
  isMobile.value = e.matches
}

onMounted(() => {
  if (mobileMql) {
    onMobileChange(mobileMql)
    mobileMql.addEventListener('change', onMobileChange)
  }
})

onUnmounted(() => {
  if (mobileMql) {
    mobileMql.removeEventListener('change', onMobileChange)
  }
})

const cornerRadius = computed(() => {
  if (!isMobile.value) return 22
  const cssVal = getComputedStyle(document.documentElement).getPropertyValue('--mobile-card-radius')?.trim()
  if (cssVal) {
    const num = parseFloat(cssVal)
    if (Number.isFinite(num)) return num
  }
  return 18
})

const liquidGlassContentStyle: CSSProperties = {
  color: 'var(--td-text-color-primary)',
  font: 'inherit',
  lineHeight: 'normal',
  textShadow: 'none',
}

const filteredPlaylists = computed(() => {
  const list = localUserStore.playlists
  if (!searchQuery.value.trim()) return list
  const q = searchQuery.value.trim().toLowerCase()
  return list.filter(pl => pl.name.toLowerCase().includes(q))
})

watch(() => props.visible, async (val) => {
  if (val) {
    searchQuery.value = ''
    showCreate.value = false
    newName.value = ''
    if (!localUserStore.playlistsLoaded) await localUserStore.loadPlaylists()
  }
})

const handleAdd = async (playlist: PlaylistRow) => {
  if (props.songs.length === 0) return
  loading.value = true
  try {
    await localUserStore.addSongsToPlaylist(playlist.id, props.songs)
    MessagePlugin.success(t('common.addedSongsToPlaylist', { count: props.songs.length, name: playlist.name }))
    emit('added', playlist.id)
    emit('update:visible', false)
  } catch {
    MessagePlugin.error(t('common.addFailed'))
  } finally {
    loading.value = false
  }
}

const handleCreate = async () => {
  if (!newName.value.trim()) return
  creating.value = true
  try {
    const pl = await localUserStore.createPlaylist(newName.value.trim())
    if (pl) {
      await handleAdd(pl)
      newName.value = ''
      showCreate.value = false
    }
  } finally {
    creating.value = false
  }
}

const handleClose = () => {
  emit('update:visible', false)
}
</script>

<template>
  <Teleport to="body">
    <Transition name="glass-fade">
      <div v-if="visible" class="liquid-glass-overlay" @click.self="handleClose">
        <div class="overlay-drag-bar" data-tauri-drag-region />
        <LiquidGlass
          class="liquid-glass-panel"
          :corner-radius="cornerRadius"
          :displacement-scale="48"
          :blur-amount="0.08"
          :saturation="180"
          :aberration-intensity="1.5"
          padding="0"
          mode="standard"
          :content-style="liquidGlassContentStyle"
          @click.stop
        >
          <div class="liquid-glass-panel__content">
            <!-- Header -->
            <div class="glass-header" data-tauri-drag-region>
              <div class="glass-title-group">
                <div class="glass-icon">
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 15V6a3 3 0 0 0-3-3H6a3 3 0 0 0-3 3v9" />
                    <path d="M3 15l3.5-3.5a2.12 2.12 0 0 1 3 0L12 14" />
                    <path d="M12 14l2.5-2.5a2.12 2.12 0 0 1 3 0L21 15" />
                    <circle cx="9" cy="8" r="1.5" />
                  </svg>
                </div>
                <div class="glass-title-text">
                  <h2 class="glass-title">{{ t('common.addToPlaylistTitle') }}</h2>
                  <div class="glass-status">
                    <span class="status-label">{{ t('common.songCount', { count: songs.length }) }}</span>
                  </div>
                </div>
              </div>
              <button class="glass-close-btn" @click="handleClose">
                <i class="iconfont icon-a-quxiaoguanbi" />
              </button>
            </div>

            <!-- Search -->
            <div class="glass-search">
              <svg class="search-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input
                v-model="searchQuery"
                :placeholder="t('common.searchPlaylist')"
                spellcheck="false"
              />
              <button v-if="searchQuery" class="search-clear" @click="searchQuery = ''">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>

            <!-- Loading -->
            <div v-if="loading" class="glass-loading">
              <span class="glass-spinner" />
            </div>

            <!-- Empty -->
            <div v-else-if="localUserStore.playlists.length === 0" class="glass-empty">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" style="opacity:0.3">
                <path d="M21 15V6a3 3 0 0 0-3-3H6a3 3 0 0 0-3 3v9" />
                <path d="M3 15l3.5-3.5a2.12 2.12 0 0 1 3 0L12 14" />
                <circle cx="9" cy="8" r="1.5" />
              </svg>
              <p>{{ t('common.noPlaylistYet') }}</p>
              <button class="glass-btn primary" style="width:auto;padding:7px 18px" @click="showCreate = true">{{ t('common.createPlaylist') }}</button>
            </div>

            <!-- No results -->
            <div v-else-if="filteredPlaylists.length === 0" class="glass-empty">
              <p>{{ t('common.noMatchPlaylist') }}</p>
            </div>

            <!-- Playlist list -->
            <div v-else class="playlist-list">
              <div
                v-for="pl in filteredPlaylists"
                :key="pl.id"
                class="playlist-item"
                @click="handleAdd(pl)"
              >
                <div class="item-cover">
                  <img
                    v-if="pl.coverImgUrl && pl.coverImgUrl !== 'default-cover'"
                    :src="pl.coverImgUrl"
                    alt=""
                    loading="lazy"
                  />
                  <img v-else :src="defaultCover" alt="" loading="lazy" />
                </div>
                <div class="item-info">
                  <span class="item-name">{{ pl.name }}</span>
                  <div class="item-meta">
                    <span class="meta-count">{{ t('common.songCountShort', { count: pl.songCount }) }}</span>
                    <span v-if="pl.source" class="meta-source">{{ pl.source }}</span>
                  </div>
                </div>
                <svg class="item-arrow" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="9 18 15 12 9 6" />
                </svg>
              </div>
            </div>

            <!-- Create entry -->
            <div class="create-entry">
              <button v-if="!showCreate" class="glass-btn outline create-trigger" @click="showCreate = true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
                </svg>
                {{ t('common.createPlaylist') }}
              </button>
              <div v-else class="create-form">
                <input
                  v-model="newName"
                  :placeholder="t('common.playlistName')"
                  class="create-input"
                  @keydown.enter="handleCreate"
                />
                <button class="glass-btn primary" style="width:auto;padding:8px 16px" :disabled="!newName.trim() || creating" @click="handleCreate">
                  <span v-if="creating" class="glass-spinner" />
                  <template v-else>{{ t('common.createAndAdd') }}</template>
                </button>
                <button class="glass-btn outline" style="width:auto;padding:8px 14px" @click="showCreate = false; newName = ''">{{ t('common.cancel') }}</button>
              </div>
            </div>
          </div>
        </LiquidGlass>
      </div>
    </Transition>
  </Teleport>
</template>

<style lang="scss" scoped>
// ========================================
// Liquid Glass Design — Add to Playlist Dialog
// ========================================

// --- Overlay ---
.liquid-glass-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
  -webkit-backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
}

.overlay-drag-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 38px;
  z-index: 2;
}

// --- Glass Panel ---
.liquid-glass-panel {
  width: min(440px, calc(100vw - 32px));
  max-width: 100%;
  flex: 0 0 auto;
}

.liquid-glass-panel__content {
  position: relative;
  width: 100%;
  overflow: hidden;
  border-radius: 22px;
  padding: 28px;
}

// ==================
// Header
// ==================
.glass-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 18px;
}

.glass-title-group {
  display: flex;
  align-items: center;
  gap: 12px;
}

.glass-icon {
  width: 42px;
  height: 42px;
  border-radius: 13px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, rgba(var(--td-brand-color-rgb, 0, 82, 204), 0.18), rgba(140, 80, 255, 0.12));
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  box-shadow: 0 3px 10px color-mix(in srgb, var(--td-brand-color) 12%, transparent);

  svg {
    color: var(--td-brand-color, #0052d9);
    filter: drop-shadow(0 0 3px rgba(100, 140, 255, 0.25));
  }
}

.glass-title-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.glass-title {
  font-size: 17px;
  font-weight: 600;
  margin: 0;
  color: var(--td-text-color-primary);
  line-height: 1.2;
}

.glass-status {
  display: flex;
  align-items: center;
  gap: 5px;
}

.status-label {
  font-size: 11px;
  color: var(--td-text-color-primary);
  opacity: 0.55;
}

.glass-close-btn {
  width: 30px;
  height: 30px;
  border-radius: 9px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 3%, transparent);
  color: var(--td-text-color-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);

  &:hover {
    background: rgba(255, 80, 80, 0.15);
    border-color: rgba(255, 80, 80, 0.25);
    color: var(--td-error-color, #d54941);
  }

  .iconfont { font-size: 13px; }
}

// ==================
// Search
// ==================
.glass-search {
  position: relative;
  margin-bottom: 14px;

  .search-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--td-text-color-placeholder);
    pointer-events: none;
  }

  input {
    width: 100%;
    padding: 10px 36px 10px 38px;
    border-radius: 11px;
    border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
    background: color-mix(in srgb, var(--td-bg-color-component) 50%, transparent);
    color: var(--td-text-color-primary);
    font-size: 13px;
    outline: none;
    transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
    box-sizing: border-box;

    &::placeholder { color: var(--td-text-color-placeholder); }

    &:focus {
      border-color: var(--td-brand-color, #0052d9);
      background: color-mix(in srgb, var(--td-bg-color-component) 65%, transparent);
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
    }
  }

  .search-clear {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    opacity: 0.4;
    display: flex;
    align-items: center;
    color: var(--td-text-color-secondary);

    &:hover { opacity: 0.7; }
  }
}

// ==================
// Loading / Empty
// ==================
.glass-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 48px;
}

.glass-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 36px;
  gap: 12px;
  color: var(--td-text-color-secondary);
  font-size: 13px;
}

// ==================
// Playlist List
// ==================
.playlist-list {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.playlist-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 13px;
  cursor: pointer;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);

  &:hover {
    background: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
    transform: translateX(3px);
  }

  &:active {
    transform: translateX(0);
  }
}

.item-cover {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  overflow: hidden;
  flex-shrink: 0;
  background: color-mix(in srgb, var(--td-text-color-primary) 4%, transparent);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
}

.item-info {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
  flex: 1;
}

.item-name {
  font-size: 13.5px;
  font-weight: 500;
  color: var(--td-text-color-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11.5px;
  color: var(--td-text-color-placeholder);
}

.meta-source {
  padding: 0 6px;
  border-radius: 5px;
  background: color-mix(in srgb, var(--td-text-color-primary) 4%, transparent);
  font-size: 10.5px;
}

.item-arrow {
  flex-shrink: 0;
  color: var(--td-text-color-placeholder);
  opacity: 0.4;
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard);
}

.playlist-item:hover .item-arrow {
  opacity: 0.7;
}

// ==================
// Create Entry
// ==================
.create-entry {
  padding-top: 14px;
  border-top: 1px solid color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  margin-top: 14px;
  flex-shrink: 0;
}

.create-trigger {
  width: 100%;
  justify-content: center;
  gap: 6px;
}

.create-form {
  display: flex;
  gap: 8px;
  align-items: center;
}

.create-input {
  flex: 1;
  padding: 9px 12px;
  border-radius: 10px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  background: color-mix(in srgb, var(--td-bg-color-component) 50%, transparent);
  color: var(--td-text-color-primary);
  font-size: 13px;
  outline: none;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);

  &::placeholder { color: var(--td-text-color-placeholder); }

  &:focus {
    border-color: var(--td-brand-color, #0052d9);
    background: color-mix(in srgb, var(--td-bg-color-component) 65%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
  }
}

// ==================
// Buttons
// ==================
.glass-btn {
  width: 100%;
  padding: 10px 18px;
  border-radius: 11px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-text-color-primary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
    transform: translateY(-1px);
    box-shadow: var(--glass-shadow-control);
  }

  &:active:not(:disabled) {
    transform: translateY(0);
    box-shadow: none;
  }

  &:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  &.primary {
    background: var(--td-brand-color, #0052d9);
    border-color: var(--td-brand-color, #0052d9);
    color: #fff;

    &:hover:not(:disabled) {
      background: var(--td-brand-color-hover, #4787f0);
      border-color: var(--td-brand-color-hover, #4787f0);
      box-shadow: 0 4px 16px color-mix(in srgb, var(--td-brand-color) 30%, transparent);
    }
  }

  &.outline {
    background: transparent;
    border-color: color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
    color: var(--td-text-color-primary);

    &:hover:not(:disabled) {
      background: color-mix(in srgb, var(--td-text-color-primary) 3%, transparent);
      border-color: color-mix(in srgb, var(--td-text-color-primary) 14%, transparent);
    }
  }
}

// ==================
// Spinner
// ==================
.glass-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  border-top-color: currentColor;
  border-radius: 50%;
  will-change: transform; animation: spin 0.6s linear infinite;
  display: inline-block;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

// ==================
// Transition
// ==================
.glass-fade-enter-active .liquid-glass-panel {
  animation: glass-in var(--motion-duration-standard) var(--motion-ease-out);
}
.glass-fade-leave-active .liquid-glass-panel {
  animation: glass-in var(--motion-duration-quick) var(--motion-ease-out) reverse;
}
.glass-fade-enter-active,
.glass-fade-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard);
}
.glass-fade-enter-from,
.glass-fade-leave-to {
  opacity: 0;
}

@keyframes glass-in {
  from {
    opacity: 0;
    transform: scale(0.96) translateY(8px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@media (max-width: 768px) {
  .liquid-glass-overlay {
    align-items: flex-end;
    justify-content: center;
    padding: calc(var(--mobile-safe-top) + 12px) var(--mobile-page-gutter) calc(var(--mobile-safe-bottom) + 12px);
    background: var(--mobile-scrim);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
  }

  .overlay-drag-bar {
    display: none;
  }

  .liquid-glass-panel {
    width: min(440px, 100%);
    max-height: min(82dvh, 680px);
    display: flex;
  }

  :deep(.glass) {
    height: 100%;
  }

  :deep(.liquid-glass__content) {
    height: 100%;
    overflow: hidden;
  }

  .liquid-glass-panel__content {
    border-radius: var(--mobile-card-radius);
    padding: 20px 16px calc(var(--mobile-safe-bottom) + 16px);
    height: 100%;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  .liquid-glass-panel__content::before {
    content: '';
    display: block;
    width: 38px;
    height: 4px;
    border-radius: 999px;
    background: rgba(120, 120, 128, 0.36);
    margin: -8px auto 12px;
  }

  .glass-header {
    margin-bottom: 14px;
  }

  .glass-title-group {
    min-width: 0;
  }

  .glass-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
  }

  .glass-close-btn,
  .search-clear {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .glass-search input,
  .create-input {
    min-height: var(--mobile-touch-target);
    font-size: 16px;
  }

  .playlist-list {
    -webkit-overflow-scrolling: touch;
  }

  .playlist-item {
    min-height: 60px;
    padding: 10px 12px;
    border-radius: var(--mobile-card-radius-small);
    touch-action: manipulation;
  }

  .create-entry {
    padding-top: 12px;
    margin-top: 12px;
  }

  .create-form {
    flex-direction: column;
    align-items: stretch;
  }

  .create-form .glass-btn,
  .glass-btn {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }
}

@media (prefers-reduced-motion: reduce) {
  .glass-spinner {
    animation: none !important;
  }

  .liquid-glass-panel {
    animation: none !important;
    transition: none !important;
    transform: none !important;
  }
}
</style>
