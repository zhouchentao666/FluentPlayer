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
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="7 10 12 15 17 10" />
                    <line x1="12" y1="15" x2="12" y2="3" />
                  </svg>
                </div>
                <div class="glass-title-text">
                  <h2 class="glass-title">{{ t('common.importPlaylist.title', { name: pluginName }) }}</h2>
                </div>
              </div>
              <button class="glass-close-btn" @click="handleClose">
                <i class="iconfont icon-a-quxiaoguanbi" />
              </button>
            </div>

            <!-- Content -->
            <div class="glass-content">
              <!-- Loading -->
              <div v-if="loading" class="glass-loading">
                <span class="glass-spinner" />
              </div>

              <!-- Error -->
              <div v-else-if="error" class="glass-empty">
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" style="opacity:0.3">
                  <circle cx="12" cy="12" r="10" />
                  <line x1="15" y1="9" x2="9" y2="15" />
                  <line x1="9" y1="9" x2="15" y2="15" />
                </svg>
                <p>{{ error }}</p>
                <button class="glass-btn primary" style="width:auto;padding:7px 18px" @click="loadPlaylists">{{ t('common.retry') }}</button>
              </div>

              <!-- Playlist List -->
              <div v-else-if="playlists.length > 0" class="playlist-list">
                <div v-for="pl in playlists" :key="pl.id" class="playlist-item">
                  <div class="item-cover">
                    <img v-if="pl.coverImg" :src="pl.coverImg" alt="" loading="lazy" />
                    <div v-else class="cover-placeholder">
                      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M9 18V5l12-2v13" />
                        <circle cx="6" cy="18" r="3" />
                        <circle cx="18" cy="16" r="3" />
                      </svg>
                    </div>
                  </div>
                  <div class="item-info">
                    <span class="item-name">{{ pl.name }}</span>
                    <div class="item-meta">
                      <span>{{ t('common.songCount', { count: pl.songCount }) }}</span>
                      <span v-if="pl.description" class="item-desc">{{ pl.description }}</span>
                    </div>
                  </div>
                  <button
                    :class="['glass-btn', 'primary', 'import-btn', { loading: importingId === pl.id }]"
                    :disabled="!!importingId"
                    @click="doImport(pl)"
                  >
                    <span v-if="importingId === pl.id" class="glass-spinner" />
                    <template v-else>
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="7 10 12 15 17 10" />
                        <line x1="12" y1="15" x2="12" y2="3" />
                      </svg>
                      {{ t('common.import') }}
                    </template>
                  </button>
                </div>
              </div>

              <!-- Empty -->
              <div v-else class="glass-empty">
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" style="opacity:0.3">
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                </svg>
                <p>{{ t('common.importPlaylist.noPlaylistFound') }}</p>
              </div>
            </div>
          </div>
        </LiquidGlass>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, type CSSProperties } from 'vue'
import { MessagePlugin } from 'tdesign-vue-next'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import LiquidGlass from '@/components/LiquidGlass.vue'
import type { SongList } from '@/types/audio'

const { t } = useI18n()

interface ServicePlaylist {
  id: string
  name: string
  songCount: number
  coverImg?: string
  description?: string
}

const props = defineProps<{
  visible: boolean
  pluginId: string
  pluginName: string
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>()

const playlists = ref<ServicePlaylist[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const importingId = ref<string | null>(null)

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
  document.body.style.overflow = ''
  hiddenDialogBodies.forEach((el) => {
    ;(el as HTMLElement).style.overflow = ''
  })
  hiddenDialogBodies = []
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

let hiddenDialogBodies: Element[] = []

watch(
  () => props.visible,
  (val) => {
    if (val) {
      document.body.style.overflow = 'hidden'
      document.querySelectorAll('.t-dialog__body').forEach((el) => {
        ;(el as HTMLElement).style.overflow = 'hidden'
        hiddenDialogBodies.push(el)
      })
      if (props.pluginId) loadPlaylists()
    } else {
      document.body.style.overflow = ''
      hiddenDialogBodies.forEach((el) => {
        ;(el as HTMLElement).style.overflow = ''
      })
      hiddenDialogBodies = []
    }
  }
)

function handleClose() {
  emit('update:visible', false)
}

async function loadPlaylists() {
  loading.value = true
  error.value = null
  try {
    const res = await (window as any).api.plugins.callMethod(
      props.pluginId,
      'getPlaylists',
      JSON.stringify([])
    )
    if (!res?.success) {
      throw new Error(res?.error || t('common.importPlaylist.fetchFailed'))
    }

    const payload = res.data
    playlists.value = Array.isArray(payload)
      ? payload
      : Array.isArray(payload?.playlists)
        ? payload.playlists
        : []
  } catch (e: any) {
    error.value = e.message || t('common.importPlaylist.fetchFailed')
  } finally {
    loading.value = false
  }
}

async function doImport(pl: ServicePlaylist) {
  importingId.value = pl.id
  try {
    const songsRes = await (window as any).api.plugins.callMethod(
      props.pluginId,
      'getPlaylistSongs',
      JSON.stringify([pl.id])
    )

    if (!songsRes?.success) {
      throw new Error(songsRes?.error || t('common.importPlaylist.fetchSongsFailed'))
    }

    const payload = songsRes.data
    const songs: SongList[] = Array.isArray(payload)
      ? payload
      : Array.isArray(payload?.songs)
        ? payload.songs
        : []

    if (songs.length === 0) {
      MessagePlugin.warning(t('common.importPlaylist.noImportableSongs', { name: pl.name }))
      return
    }

    const localUserStore = LocalUserDetailStore()
    const created = await localUserStore.createPlaylist(
      pl.name,
      t('common.importPlaylist.importFrom', { name: props.pluginName, count: songs.length }),
      'service'
    )

    if (!created) {
      throw new Error(t('common.importPlaylist.createLocalFailed'))
    }

    const added = await localUserStore.addSongsToPlaylist(created.id, songs)
    if (pl.coverImg) {
      await localUserStore.updatePlaylistCover(created.id, pl.coverImg)
    }

    if (added > 0) {
      MessagePlugin.success(t('common.importPlaylist.importSuccess', { count: added, name: pl.name }))
    } else {
      MessagePlugin.warning(t('common.importPlaylist.noNewSongs', { name: pl.name }))
    }
  } catch (e: any) {
    MessagePlugin.error(t('common.importPlaylist.importFailed', { error: e.message }))
  } finally {
    importingId.value = null
  }
}
</script>

<style lang="scss" scoped>
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
  width: min(500px, calc(100vw - 32px));
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
  min-width: 0;
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
  min-width: 0;
}

.glass-title {
  font-size: 17px;
  font-weight: 600;
  margin: 0;
  color: var(--td-text-color-primary);
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
  flex-shrink: 0;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);

  &:hover {
    background: rgba(255, 80, 80, 0.15);
    border-color: rgba(255, 80, 80, 0.25);
    color: var(--td-error-color, #d54941);
  }

  .iconfont { font-size: 13px; }
}

// ==================
// Content
// ==================
.glass-content {
  animation: content-in var(--motion-duration-quick) var(--motion-ease-out);
}

@keyframes content-in {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
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
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);

  &:hover {
    background: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
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

.cover-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--td-text-color-placeholder);
  opacity: 0.5;
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

.item-desc {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

// ==================
// Import Button
// ==================
.import-btn {
  width: auto;
  padding: 7px 14px;
  flex-shrink: 0;
  font-size: 12px;
  gap: 5px;
}

// ==================
// Buttons
// ==================
.glass-btn {
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
  will-change: transform;
  animation: spin 0.6s linear infinite;
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
    overflow: hidden;
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

  .glass-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
  }

  .glass-close-btn {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
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
