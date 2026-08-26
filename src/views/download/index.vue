<script setup lang="ts">
import { onMounted, computed, ref, watch, onBeforeUnmount } from 'vue'
import { useDownloadStore, DownloadStatus } from '@/store/download'
import { storeToRefs } from 'pinia'

const { t } = useI18n()
const store = useDownloadStore()
const { tasks } = storeToRefs(store)
const maxConcurrent = ref(3)
const activeTab = ref('downloading')
const searchKeyword = ref('')
const showSettings = ref(false)

onMounted(async () => {
  store.init()
  maxConcurrent.value = await store.getMaxConcurrent()
})

onBeforeUnmount(() => {
  store.destroy()
})

watch(activeTab, (val) => {
  if (val === 'completed') {
    store.validateFiles()
  }
})

const downloadingTasks = computed(() =>
  tasks.value.filter((t) =>
    [DownloadStatus.Downloading, DownloadStatus.Queued, DownloadStatus.Paused].includes(t.status)
  )
)
const completedTasks = computed(() =>
  tasks.value.filter((t) => t.status === DownloadStatus.Completed)
)
const failedTasks = computed(() =>
  tasks.value.filter((t) => [DownloadStatus.Error, DownloadStatus.Cancelled].includes(t.status))
)

const filteredTasks = computed(() => {
  let list = activeTab.value === 'downloading' ? downloadingTasks.value
    : activeTab.value === 'completed' ? completedTasks.value
    : failedTasks.value

  if (searchKeyword.value.trim()) {
    const kw = searchKeyword.value.toLowerCase()
    list = list.filter(t =>
      t.song_info?.name?.toLowerCase().includes(kw) ||
      t.song_info?.singer?.toLowerCase().includes(kw)
    )
  }

  return [...list].sort((a, b) => {
    if (activeTab.value === 'completed' || activeTab.value === 'failed') {
      return b.created_at - a.created_at
    }
    const statusOrder: Record<string, number> = {
      [DownloadStatus.Downloading]: 0,
      [DownloadStatus.Queued]: 1,
      [DownloadStatus.Paused]: 2
    }
    const sa = statusOrder[a.status] ?? 99
    const sb = statusOrder[b.status] ?? 99
    if (sa !== sb) return sa - sb
    return a.created_at - b.created_at
  })
})

const formatSpeed = (speed: number) => {
  if (speed === 0) return '0 B/s'
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s']
  let s = speed, i = 0
  while (s >= 1024 && i < units.length - 1) { s /= 1024; i++ }
  return `${s.toFixed(1)} ${units[i]}`
}

const formatSize = (size: number) => {
  if (!size) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let s = size, i = 0
  while (s >= 1024 && i < units.length - 1) { s /= 1024; i++ }
  return `${s.toFixed(1)} ${units[i]}`
}

const formatRemaining = (seconds: number | null) => {
  if (!seconds || !isFinite(seconds)) return ''
  if (seconds < 60) return t('common.unitSecond', { seconds: Math.round(seconds) })
  const m = Math.floor(seconds / 60)
  const s = Math.round(seconds % 60)
  return t('common.unitMinuteSecond', { m, s })
}

const adjustConcurrent = (delta: number) => {
  const newVal = Math.max(1, Math.min(5, maxConcurrent.value + delta))
  maxConcurrent.value = newVal
  store.setMaxConcurrent(newVal)
}

const clearCurrentTasks = () => {
  if (activeTab.value === 'downloading') {
    store.clearTasks('queue')
  } else if (activeTab.value === 'completed') {
    store.clearTasks('completed')
  } else {
    store.clearTasks('failed')
  }
}
</script>

<template>
  <div class="download-manager">
    <!-- Main content -->
    <div class="download-content">
      <!-- Header -->
      <div class="header">
        <div class="header-left">
          <div class="header-indicator"></div>
          <h2>{{ t('download.title') }}</h2>
        </div>
        <div class="header-right">
          <div class="search-box">
            <i class="iconfont icon-sousuo search-icon"></i>
            <input
              v-model="searchKeyword"
              :placeholder="t('download.searchTask')"
              class="search-input"
            />
          </div>
          <template v-if="activeTab === 'downloading'">
            <button class="toolbar-btn" @click="store.resumeAllTasks()">
              <i class="iconfont icon-bofang"></i>
              <span>{{ t('download.startAll') }}</span>
            </button>
            <button class="toolbar-btn" @click="store.pauseAllTasks()">
              <i class="iconfont icon-zanting"></i>
              <span>{{ t('download.pauseAll') }}</span>
            </button>
          </template>
          <button v-if="filteredTasks.length > 0" class="toolbar-btn" @click="clearCurrentTasks">
            <i class="iconfont icon-shanchu"></i>
            <span>{{ activeTab === 'downloading' ? t('download.clearQueue') : t('download.clearRecords') }}</span>
          </button>
          <button class="toolbar-btn" :class="{ active: showSettings }" @click="showSettings = !showSettings">
            <i class="iconfont icon-shezhi"></i>
          </button>
        </div>
      </div>

      <!-- Concurrent settings popup -->
      <div v-if="showSettings" class="settings-popup">
        <span class="settings-label">{{ t('download.concurrentDownloads') }}</span>
        <div class="settings-control">
          <button class="settings-btn" @click="adjustConcurrent(-1)">-</button>
          <span class="settings-value">{{ maxConcurrent }}</span>
          <button class="settings-btn" @click="adjustConcurrent(1)">+</button>
        </div>
      </div>

      <!-- Tab bar -->
      <div class="tab-bar">
        <button class="tab-pill" :class="{ active: activeTab === 'downloading' }" @click="activeTab = 'downloading'">
          <span class="tab-dot downloading"></span>
          {{ t('download.tabActive') }}
          <span v-if="downloadingTasks.length" class="tab-count">{{ downloadingTasks.length }}</span>
        </button>
        <button class="tab-pill" :class="{ active: activeTab === 'completed' }" @click="activeTab = 'completed'">
          <span class="tab-dot completed"></span>
          {{ t('download.tabCompleted') }}
          <span v-if="completedTasks.length" class="tab-count">{{ completedTasks.length }}</span>
        </button>
        <button class="tab-pill" :class="{ active: activeTab === 'failed' }" @click="activeTab = 'failed'">
          <span class="tab-dot failed"></span>
          {{ t('download.tabFailed') }}
          <span v-if="failedTasks.length" class="tab-count">{{ failedTasks.length }}</span>
        </button>
      </div>

      <!-- Task list -->
      <div class="task-list">
        <div v-if="filteredTasks.length === 0" class="empty-state">
          <div class="empty-icon">
            <i class="iconfont icon-xiazai"></i>
          </div>
          <p class="empty-text">{{ searchKeyword ? t('download.noMatch') : t('download.empty') }}</p>
        </div>
        <div v-else class="task-grid">
          <div v-for="task in filteredTasks" :key="task.id" class="task-card" :class="task.status">
            <div class="task-top">
              <div class="status-indicator" :class="task.status"></div>
              <div class="task-title">{{ task.song_info?.name || t('download.unknownSong') }}</div>
            </div>
            <div class="task-meta">
              <span class="meta-artist">{{ task.song_info?.singer }}</span>
              <span class="meta-sep">·</span>
              <span v-if="task.quality" class="meta-quality">{{ task.quality.toUpperCase() }}</span>
              <span v-if="task.quality" class="meta-sep">·</span>
              <span class="meta-size">{{ formatSize(task.downloaded_size) }} / {{ formatSize(task.total_size) }}</span>
            </div>
            <div v-if="task.status === DownloadStatus.Downloading" class="task-live">
              <span class="live-speed">{{ formatSpeed(task.speed) }}</span>
              <span v-if="task.remaining_time" class="live-remaining">{{ formatRemaining(task.remaining_time) }}</span>
            </div>
            <div class="progress-track">
              <div class="progress-fill" :class="task.status" :style="{ width: task.progress + '%' }"></div>
            </div>
            <div v-if="task.error" class="task-error">{{ task.error }}</div>
            <div class="task-actions">
              <button v-if="task.status === DownloadStatus.Downloading || task.status === DownloadStatus.Queued"
                class="action-btn" @click="store.pauseTask(task.id)" :title="t('common.pause')">
                <i class="iconfont icon-zanting"></i>
              </button>
              <button v-if="task.status === DownloadStatus.Paused"
                class="action-btn" @click="store.resumeTask(task.id)" :title="t('common.continue_')">
                <i class="iconfont icon-bofang"></i>
              </button>
              <button v-if="task.status === DownloadStatus.Error"
                class="action-btn" @click="store.retryTask(task.id)" :title="t('common.retry')">
                <i class="iconfont icon-shuaxin"></i>
              </button>
              <button v-if="task.status !== DownloadStatus.Completed && task.status !== DownloadStatus.Cancelled"
                class="action-btn danger" @click="store.cancelTask(task.id)" :title="t('common.cancel')">
                <i class="iconfont icon-guanbi"></i>
              </button>
              <button v-if="task.status === DownloadStatus.Completed"
                class="action-btn" @click="store.openFileLocation(task.file_path)" :title="t('common.openLocation')">
                <i class="iconfont icon-wenjianjia"></i>
              </button>
              <button v-if="task.status === DownloadStatus.Completed || task.status === DownloadStatus.Cancelled || task.status === DownloadStatus.Error"
                class="action-btn danger" @click="store.deleteTask(task.id)" :title="t('common.delete')">
                <i class="iconfont icon-shanchu"></i>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ==================== Main Container ==================== */
.download-manager {
  padding: 20px;
  height: 100%;
  display: flex;
  flex-direction: column;
  color: var(--td-text-color-primary);
  position: relative;
  background: var(--td-bg-color-container);
}

/* ==================== Content Layer ==================== */
.download-content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

/* ==================== Header ==================== */
.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  flex-shrink: 0;
  gap: 12px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-indicator {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--td-brand-color);
  box-shadow: 0 0 8px color-mix(in srgb, var(--td-brand-color) 40%, transparent);
  flex-shrink: 0;
}

.header h2 {
  margin: 0;
  font-size: 1.75rem;
  font-weight: 700;
  line-height: 1.3;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

/* Search box */
.search-box {
  position: relative;
  display: flex;
  align-items: center;
}

.search-icon {
  position: absolute;
  left: 12px;
  font-size: 14px;
  color: var(--td-text-color-placeholder);
  pointer-events: none;
}

.search-input {
  padding: 7px 14px 7px 34px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
  border-radius: 999px;
  outline: none;
  font-size: 13px;
  width: 180px;
  background: color-mix(in srgb, var(--td-bg-color-component) 60%, transparent);
  color: var(--td-text-color-primary);
  transition: border-color var(--motion-duration-quick) var(--motion-ease-standard),
              box-shadow var(--motion-duration-quick) var(--motion-ease-standard),
              width var(--motion-duration-standard) var(--motion-ease-standard);
}

.search-input::placeholder {
  color: var(--td-text-color-placeholder);
}

.search-input:focus {
  border-color: var(--td-brand-color);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--td-brand-color) 15%, transparent);
}

/* Toolbar buttons */
.toolbar-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 6px 12px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--td-text-color-primary) 4%, transparent);
  color: var(--td-text-color-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard),
              color var(--motion-duration-quick) var(--motion-ease-standard),
              border-color var(--motion-duration-quick) var(--motion-ease-standard);
  white-space: nowrap;
}

.toolbar-btn:hover {
  background: color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  color: var(--td-text-color-primary);
  border-color: color-mix(in srgb, var(--td-text-color-primary) 18%, transparent);
}

.toolbar-btn.active {
  background: color-mix(in srgb, var(--td-brand-color) 12%, transparent);
  border-color: color-mix(in srgb, var(--td-brand-color) 25%, transparent);
  color: var(--td-brand-color);
}

.toolbar-btn .iconfont {
  font-size: 14px;
}

/* ==================== Settings Popup ==================== */
.settings-popup {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  margin-bottom: 12px;
  border-radius: 12px;
  background: color-mix(in srgb, var(--td-bg-color-component) 70%, transparent);
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  backdrop-filter: blur(var(--glass-blur-control));
  -webkit-backdrop-filter: blur(var(--glass-blur-control));
  flex-shrink: 0;
}

.settings-label {
  font-size: 13px;
  color: var(--td-text-color-secondary);
}

.settings-control {
  display: flex;
  align-items: center;
  gap: 12px;
}

.settings-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-text-color-primary);
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard),
              border-color var(--motion-duration-quick) var(--motion-ease-standard);
}

.settings-btn:hover {
  background: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
  border-color: color-mix(in srgb, var(--td-text-color-primary) 20%, transparent);
}

.settings-value {
  font-size: 16px;
  font-weight: 600;
  min-width: 20px;
  text-align: center;
}

/* ==================== Tab Bar ==================== */
.tab-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
  flex-shrink: 0;
}

.tab-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border: none;
  border-radius: 999px;
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-text-color-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard),
              color var(--motion-duration-quick) var(--motion-ease-standard);
}

.tab-pill:hover {
  background: color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
}

.tab-pill.active {
  background: color-mix(in srgb, var(--td-brand-color) 15%, transparent);
  color: var(--td-brand-color);
}

.tab-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.tab-dot.downloading {
  background: var(--td-brand-color);
}

.tab-dot.completed {
  background: var(--td-success-color);
}

.tab-dot.failed {
  background: var(--td-error-color);
}

.tab-count {
  font-size: 11px;
  font-weight: 600;
  padding: 1px 7px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
}

.tab-pill.active .tab-count {
  background: color-mix(in srgb, var(--td-brand-color) 20%, transparent);
}

/* ==================== Task List ==================== */
.task-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  -webkit-overflow-scrolling: touch;
}

.task-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 12px;
}

/* ==================== Task Card — Liquid Glass ==================== */
.task-card {
  background: linear-gradient(
    165deg,
    color-mix(in srgb, var(--td-bg-color-container) 60%, transparent) 0%,
    color-mix(in srgb, var(--td-bg-color-container) 48%, transparent) 100%
  );
  backdrop-filter: blur(var(--glass-blur-control)) saturate(180%);
  -webkit-backdrop-filter: blur(var(--glass-blur-control)) saturate(180%);
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  border-radius: 16px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  box-shadow: var(--glass-shadow-control);
  transition:
    transform var(--motion-duration-standard) var(--motion-ease-out),
    box-shadow var(--motion-duration-standard) var(--motion-ease-out),
    border-color var(--motion-duration-standard) var(--motion-ease-out);
}

.task-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.12);
  border-color: color-mix(in srgb, var(--td-text-color-primary) 18%, transparent);
}

/* Status-specific card styles */
.task-card.downloading .status-indicator {
  background: var(--td-brand-color);
  box-shadow: 0 0 6px color-mix(in srgb, var(--td-brand-color) 50%, transparent);
  animation: pulse-dot 2s ease-in-out infinite;
}

.task-card.queued .status-indicator {
  background: var(--td-warning-color);
}

.task-card.paused .status-indicator {
  background: var(--td-warning-color);
  opacity: 0.6;
}

.task-card.completed .status-indicator {
  background: var(--td-success-color);
}

.task-card.completed .progress-fill {
  background: linear-gradient(90deg, var(--td-success-color), color-mix(in srgb, var(--td-success-color) 70%, var(--td-success-color-hover)));
}

.task-card.error .status-indicator {
  background: var(--td-error-color);
}

.task-card.cancelled .status-indicator {
  background: var(--td-text-color-placeholder);
}

@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

/* ==================== Task Card Inner ==================== */
.task-top {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.task-title {
  font-weight: 600;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--td-text-color-secondary);
  flex-wrap: wrap;
}

.meta-artist {
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.meta-sep {
  color: var(--td-text-color-placeholder);
}

.meta-quality {
  background: color-mix(in srgb, var(--td-bg-color-secondarycontainer) 85%, transparent);
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  padding: 0 5px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 500;
}

.meta-size {
  color: var(--td-text-color-placeholder);
}

.task-live {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
}

.live-speed {
  color: var(--td-brand-color);
  font-weight: 500;
}

.live-remaining {
  color: var(--td-text-color-placeholder);
}

/* ==================== Progress Bar ==================== */
.progress-track {
  height: 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--td-brand-color), color-mix(in srgb, var(--td-brand-color) 70%, var(--td-brand-color-8)));
  transition: width 0.3s ease;
  min-width: 0;
}

.task-card.downloading .progress-fill {
  background: linear-gradient(90deg, var(--td-brand-color), color-mix(in srgb, var(--td-brand-color) 70%, var(--td-brand-color-8)));
  animation: progress-shimmer 2s ease-in-out infinite;
}

@keyframes progress-shimmer {
  0% { filter: brightness(1); }
  50% { filter: brightness(1.2); }
}

.task-card.error .progress-fill {
  background: linear-gradient(90deg, var(--td-error-color), color-mix(in srgb, var(--td-error-color) 70%, var(--td-error-color-hover)));
}

/* ==================== Task Error ==================== */
.task-error {
  font-size: 12px;
  color: var(--td-error-color);
  line-height: 1.4;
}

/* ==================== Task Actions ==================== */
.task-actions {
  display: flex;
  gap: 6px;
  align-items: center;
}

.action-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: none;
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-text-color-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard),
              color var(--motion-duration-quick) var(--motion-ease-standard);
}

.action-btn:hover {
  background: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
  color: var(--td-text-color-primary);
}

.action-btn.danger {
  color: var(--td-error-color);
}

.action-btn.danger:hover {
  background: color-mix(in srgb, var(--td-error-color) 12%, transparent);
  color: var(--td-error-color);
}

.action-btn .iconfont {
  font-size: 14px;
}

/* ==================== Empty State ==================== */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: var(--td-text-color-secondary);
  background: linear-gradient(
    165deg,
    color-mix(in srgb, var(--td-bg-color-container) 50%, transparent) 0%,
    color-mix(in srgb, var(--td-bg-color-container) 35%, transparent) 100%
  );
  border-radius: var(--mobile-card-radius-small, 16px);
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  backdrop-filter: blur(var(--glass-blur-control)) saturate(180%);
  -webkit-backdrop-filter: blur(var(--glass-blur-control)) saturate(180%);
}

.empty-icon {
  font-size: 48px;
  color: var(--td-text-color-placeholder);
  margin-bottom: 16px;
  opacity: 0.5;
}

.empty-icon .iconfont {
  font-size: 48px;
}

.empty-text {
  font-size: 14px;
  margin: 0;
}

/* ==================== Mobile (<=768px) ==================== */
@media (max-width: 768px) {
  .download-manager {
    min-width: 0;
    padding: var(--mobile-page-top-gutter) var(--mobile-page-gutter) 0;
    box-sizing: border-box;
    border-radius: 0;
    border: none;
    overflow: hidden;
    background: var(--td-bg-color-container);
  }

  .header {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
    margin-bottom: 12px;
  }

  .header-left {
    align-items: center;
  }

  .header h2 {
    font-size: clamp(1.5rem, 6vw, 1.75rem);
  }

  .header-right {
    flex-wrap: wrap;
    gap: 8px;
  }

  .search-box {
    flex: 1 1 100%;
  }

  .search-input {
    width: 100%;
    min-height: var(--mobile-touch-target);
    padding-left: 38px;
    font-size: 16px;
  }

  .toolbar-btn {
    min-height: var(--mobile-touch-target);
    padding: 6px 14px;
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .settings-popup {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }

  .settings-control {
    justify-content: center;
  }

  .settings-btn {
    width: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
  }

  .tab-bar {
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
    padding-bottom: 4px;
  }

  .tab-bar::-webkit-scrollbar {
    display: none;
  }

  .tab-pill {
    white-space: nowrap;
    min-height: var(--mobile-touch-target);
    padding: 8px 18px;
    touch-action: manipulation;
  }

  .task-list {
    padding-bottom: calc(var(--mobile-safe-bottom, 0px) + 12px);
  }

  .task-grid {
    grid-template-columns: 1fr;
    gap: 10px;
  }

  .task-card {
    border-radius: var(--mobile-card-radius-small);
    padding: 14px;
  }

  .task-card:hover {
    transform: none;
  }

  .task-title {
    font-size: 15px;
    white-space: normal;
    overflow: visible;
    text-overflow: unset;
    line-height: 1.4;
  }

  .meta-artist {
    max-width: none;
  }

  .progress-track {
    height: 8px;
  }

  .task-actions {
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  }

  .action-btn {
    width: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    touch-action: manipulation;
  }

  .action-btn .iconfont {
    font-size: 16px;
  }

  .empty-state {
    padding: 48px 20px;
    border-radius: var(--mobile-card-radius-small);
  }
}

/* ==================== Reduced Motion ==================== */
@media (prefers-reduced-motion: reduce) {
  .task-card.downloading .status-indicator {
    animation: none !important;
  }

  .task-card.downloading .progress-fill {
    animation: none !important;
  }

  .task-card:hover {
    transform: none;
  }
}
</style>
