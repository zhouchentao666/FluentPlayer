<script setup lang="ts">
import { ref } from 'vue'
import { MessagePlugin, DialogPlugin } from 'tdesign-vue-next'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { storeToRefs } from 'pinia'
import {
  exportPlaylistToFile,
  copyPlaylistToClipboard,
  importPlaylistFromFile,
  importPlaylistFromClipboard,
  validateImportedPlaylist
} from '@/utils/playlist/playlistExportImport'
import type { SongList } from '@/types/audio'
import { CloudDownloadIcon, DeleteIcon, CloudUploadIcon } from 'tdesign-icons-vue-next'

const { t } = useI18n()
const localUserStore = LocalUserDetailStore()
const { list } = storeToRefs(localUserStore)

// 对话框控制
const exportDialogVisible = ref(false)
const importDialogVisible = ref(false)

// 文件上传相关
const fileInputRef = ref<HTMLInputElement | null>(null)
const uploadedFile = ref<File | null>(null)

// 导出播放列表
const handleExportToFile = async () => {
  try {
    if (list.value.length === 0) {
      MessagePlugin.warning(t('settings.playlist.playlistEmpty'))
      return
    }

    const filtered = list.value.filter((s) => s.source !== 'local')
    const removed = list.value.length - filtered.length
    const fileName = await exportPlaylistToFile(filtered)
    if (removed > 0) MessagePlugin.info(t('settings.playlist.removedLocalSongs', { count: removed }))
    MessagePlugin.success(t('settings.playlist.exportSuccess', { name: fileName }))
    exportDialogVisible.value = false
  } catch (error) {
    MessagePlugin.error(t('settings.playlist.importFailedFormat'))
  }
}

// 复制播放列表到剪贴板
const handleCopyToClipboard = async () => {
  try {
    if (list.value.length === 0) {
      MessagePlugin.warning(t('settings.playlist.playlistEmptyCopy'))
      return
    }

    const filtered = list.value.filter((s) => s.source !== 'local')
    const removed = list.value.length - filtered.length
    await copyPlaylistToClipboard(filtered)
    if (removed > 0) MessagePlugin.info(t('settings.playlist.removedLocalSongs', { count: removed }))
    MessagePlugin.success(t('settings.playlist.clipboardSuccess'))
    exportDialogVisible.value = false
  } catch (error) {
    MessagePlugin.error(t('settings.playlist.importFailedFormat'))
  }
}

// 触发文件选择
const triggerFileInput = () => {
  if (fileInputRef.value) {
    fileInputRef.value.click()
  }
}

// 处理文件选择
const handleFileChange = (event: Event) => {
  const input = event.target as HTMLInputElement
  if (input.files && input.files.length > 0) {
    uploadedFile.value = input.files[0]
  }
}

// 从文件导入播放列表
const handleImportFromFile = async () => {
  try {
    if (!uploadedFile.value) {
      MessagePlugin.warning(t('settings.playlist.selectFileFirst'))
      return
    }

    const importedPlaylist = await importPlaylistFromFile(uploadedFile.value)

    if (!validateImportedPlaylist(importedPlaylist)) {
      throw new Error(t('settings.playlist.importFailedFormat'))
    }

    // 合并播放列表，避免重复
    const mergedList = mergePlaylist(list.value, importedPlaylist)

    // 更新播放列表
    list.value = mergedList

    MessagePlugin.success(t('settings.playlist.importPlaylistSuccess', { count: importedPlaylist.length }))
    importDialogVisible.value = false
    uploadedFile.value = null
    if (fileInputRef.value) {
      fileInputRef.value.value = ''
    }
  } catch (error) {
    MessagePlugin.error(t('settings.playlist.importFailedFormat'))
  }
}

// 从剪贴板导入播放列表
const handleImportFromClipboard = async () => {
  try {
    const importedPlaylist = await importPlaylistFromClipboard()

    if (!validateImportedPlaylist(importedPlaylist)) {
      throw new Error(t('settings.playlist.clipboardFormatError'))
    }

    // 合并播放列表，避免重复
    const mergedList = mergePlaylist(list.value, importedPlaylist)

    // 更新播放列表
    list.value = mergedList

    MessagePlugin.success(t('settings.playlist.importPlaylistSuccess', { count: importedPlaylist.length }))
    importDialogVisible.value = false
  } catch (error) {
    MessagePlugin.error(t('settings.playlist.importFailedFormat'))
  }
}

// 合并播放列表，避免重复
const mergePlaylist = (currentList: SongList[], importedList: SongList[]): SongList[] => {
  const result = [...currentList]
  const existingIds = new Set(currentList.map((song) => song.songmid))

  for (const song of importedList) {
    if (!existingIds.has(song.songmid)) {
      result.push(song)
      existingIds.add(song.songmid)
    }
  }

  return result
}

// 清空播放列表
const handleClearPlaylist = () => {
  const confirm = DialogPlugin.confirm({
    header: t('settings.playlist.confirmClearTitle'),
    body: t('settings.playlist.confirmClearBody'),
    theme: 'warning',
    confirmBtn: {
      theme: 'danger',
      content: t('common.clear')
    },
    cancelBtn: t('common.cancel'),
    onConfirm: () => {
      list.value = []
      confirm.destroy()
      MessagePlugin.success(t('settings.playlist.clearSuccess'))
    }
  })
}

// 获取播放列表统计信息
const playlistStats = ref({
  totalSongs: 0,
  totalDuration: 0,
  artists: new Set<string>()
})

// 计算播放列表统计信息
const updatePlaylistStats = () => {
  const stats = {
    totalSongs: list.value?.length || 0,
    totalDuration: 0,
    artists: new Set<string>()
  }

  if (list.value && list.value.length > 0) {
    list.value.forEach((song) => {
      // 处理新的 interval 字段格式
      if (typeof song.interval === 'string' && song.interval.includes(':')) {
        // 如果是 "05:41" 格式，转换为秒数
        const [minutes, seconds] = song.interval.split(':').map(Number)
        stats.totalDuration += minutes * 60 + seconds
      } else {
        // 如果是数字字符串，转换为秒数
        const duration = parseInt(song.interval || '0', 10)
        if (!isNaN(duration)) {
          stats.totalDuration += duration / 1000
        }
      }

      // 处理歌手信息
      if (song.singer) {
        // 如果歌手名包含分隔符，分割处理
        const singers = song.singer
          .split(/[\/、&]/)
          .map((s) => s.trim())
          .filter((s) => s)
        singers.forEach((singer) => stats.artists.add(singer))
      }
    })
  }

  playlistStats.value = stats
}

// 格式化时间
const formatDuration = (seconds: number): string => {
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const remainingSeconds = Math.floor(seconds % 60)

  if (hours > 0) {
    return t('settings.playlist.formatDuration', { hours, minutes })
  } else {
    return t('settings.playlist.formatDurationShort', { minutes, seconds: remainingSeconds })
  }
}

// 监听播放列表变化
import { onMounted, watch } from 'vue'

onMounted(() => {
  updatePlaylistStats()
})

watch(
  () => list.value,
  () => {
    updatePlaylistStats()
  },
  { deep: true }
)
</script>

<template>
  <div class="playlist-settings">
    <div class="playlist-stats-card">
      <div class="section-title">{{ t('settings.playlist.stats') }}</div>
        <div class="stats-content">
          <div class="stat-item">
            <t-icon name="play" />
            <div class="stat-info">
              <div class="stat-label">{{ t('settings.playlist.songCount') }}</div>
              <div class="stat-value">{{ t('settings.playlist.songUnit', { count: playlistStats.totalSongs }) }}</div>
            </div>
          </div>

          <div class="stat-item">
            <t-icon name="time" />
            <div class="stat-info">
              <div class="stat-label">{{ t('settings.playlist.totalDuration') }}</div>
              <div class="stat-value">{{ formatDuration(playlistStats.totalDuration) }}</div>
            </div>
          </div>

          <div class="stat-item">
            <t-icon name="user-circle" />
            <div class="stat-info">
              <div class="stat-label">{{ t('settings.playlist.artistCount') }}</div>
              <div class="stat-value">{{ t('settings.playlist.artistUnit', { count: playlistStats.artists.size }) }}</div>
            </div>
          </div>
        </div>
    </div>

    <div class="playlist-actions-card">
      <div class="section-title">{{ t('settings.playlist.management') }}</div>
        <div class="action-buttons">
          <t-button theme="primary" @click="exportDialogVisible = true">
            <template #icon>
              <CloudDownloadIcon />
            </template>
            {{ t('settings.playlist.exportPlaylist') }}
          </t-button>

          <t-button theme="primary" @click="importDialogVisible = true">
            <template #icon>
              <CloudUploadIcon />
            </template>
            {{ t('settings.playlist.importPlaylist') }}
          </t-button>

          <t-button theme="danger" @click="handleClearPlaylist">
            <template #icon>
              <DeleteIcon />
            </template>
            {{ t('settings.playlist.clearPlaylist') }}
          </t-button>
        </div>

        <div class="feature-description">
          <h4>{{ t('settings.playlist.featureDescription') }}</h4>
          <ul>
            <li>{{ t('settings.playlist.exportDesc') }}</li>
            <li>{{ t('settings.playlist.importDesc') }}</li>
            <li>{{ t('settings.playlist.clearDesc') }}</li>
          </ul>
        </div>
    </div>

    <!-- 导出对话框 -->
    <t-dialog
      v-model:visible="exportDialogVisible"
      :header="t('settings.playlist.exportDialogTitle')"
      :on-close="() => (exportDialogVisible = false)"
      width="500px"
      attach="body"
    >
      <template #body>
        <div class="dialog-content">
          <p class="dialog-description">{{ t('settings.playlist.selectExportMethod') }}</p>

          <div class="export-options">
            <t-card
              :title="t('settings.playlist.exportToFile')"
              :description="t('settings.playlist.exportToFileDesc')"
              class="export-option-card"
              @click="handleExportToFile"
            >
            </t-card>

            <t-card
              :title="t('settings.playlist.copyToClipboard')"
              :description="t('settings.playlist.copyToClipboardDesc')"
              class="export-option-card"
              @click="handleCopyToClipboard"
            >
              <template #avatar>
                <t-icon name="copy" size="large" />
              </template>
            </t-card>
          </div>
        </div>
      </template>

      <template #footer>
        <t-button theme="default" @click="exportDialogVisible = false">{{ t('common.cancel') }}</t-button>
      </template>
    </t-dialog>

    <!-- 导入对话框 -->
    <t-dialog
      v-model:visible="importDialogVisible"
      :header="t('settings.playlist.importDialogTitle')"
      :on-close="() => (importDialogVisible = false)"
      width="700px"
      attach="body"
    >
      <template #body>
        <div class="dialog-content">
          <p class="dialog-description">{{ t('settings.playlist.selectImportMethod') }}</p>

          <div class="import-options">
            <t-card
              :title="t('settings.playlist.importFromFile')"
              :description="t('settings.playlist.importFromFileDesc')"
              class="import-option-card"
            >
              <template #footer>
                <div class="file-upload-area">
                  <input
                    ref="fileInputRef"
                    type="file"
                    accept=".cmpl,.cpl"
                    style="display: none"
                    @change="handleFileChange"
                  />

                  <t-button theme="primary" variant="outline" @click="triggerFileInput">
                    {{ t('settings.playlist.selectFile') }}
                  </t-button>

                  <span v-if="uploadedFile" class="file-name">
                    {{ t('settings.playlist.selected', { name: uploadedFile.name }) }}
                  </span>

                  <t-button theme="primary" :disabled="!uploadedFile" @click="handleImportFromFile">
                    {{ t('settings.playlist.doImport') }}
                  </t-button>
                </div>
              </template>
            </t-card>

            <t-card
              :title="t('settings.playlist.importFromClipboard')"
              :description="t('settings.playlist.importFromClipboardDesc')"
              class="import-option-card"
            >
              <template #footer>
                <t-button theme="primary" @click="handleImportFromClipboard">
                  {{ t('settings.playlist.fromClipboardImport') }}
                </t-button>
              </template>
            </t-card>
          </div>
        </div>
      </template>

      <template #footer>
        <t-button theme="default" @click="importDialogVisible = false">{{ t('common.cancel') }}</t-button>
      </template>
    </t-dialog>
  </div>
</template>

<style lang="scss" scoped>
.playlist-settings {
  margin-bottom: 2rem;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--td-text-color-primary);
  margin-bottom: 12px;
}

.playlist-stats-card,
.playlist-actions-card {
  margin-bottom: 1.5rem;
}

.stats-content {
  display: flex;
  flex-wrap: wrap;
  gap: 24px;
  margin-top: 8px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 12px;

  .t-icon {
    font-size: 24px;
    color: var(--td-brand-color);
  }

  .stat-info {
    .stat-label {
      font-size: 14px;
      color: var(--td-text-color-secondary);
      margin-bottom: 4px;
    }

    .stat-value {
      font-size: 18px;
      font-weight: 600;
      color: var(--td-text-color-primary);
    }
  }
}

.action-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  margin-bottom: 24px;
}

.feature-description {
  background-color: var(--td-bg-color-container-hover);
  padding: 16px;
  border-radius: 8px;

  h4 {
    font-size: 16px;
    margin-top: 0;
    margin-bottom: 12px;
    color: var(--td-text-color-primary);
  }

  ul {
    margin: 0;
    padding-left: 20px;

    li {
      margin-bottom: 8px;
      color: var(--td-text-color-secondary);

      &:last-child {
        margin-bottom: 0;
      }

      strong {
        color: var(--td-text-color-primary);
      }
    }
  }
}

.dialog-content {
  padding: 16px 0;
}

.dialog-description {
  margin-bottom: 16px;
  font-size: 14px;
  color: var(--td-text-color-secondary);
}

.export-options,
.import-options {
  display: flex;
  gap: 16px;

  @media (max-width: 500px) {
    flex-direction: column;
  }
}

.export-option-card,
.import-option-card {
  flex: 1;
  cursor: pointer;
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;

  &:hover {
    transform: translateY(-4px);
    box-shadow: var(--theme-shadow-medium);
  }
}

.file-upload-area {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: center;
}

.file-name {
  font-size: 12px;
  color: var(--td-text-color-secondary);
  margin: 8px 0;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
@media (max-width: 768px) {
  .playlist-settings {
    margin-bottom: 0;
  }

  .playlist-stats-card,
  .playlist-actions-card {
    margin-bottom: 10px;
  }

  .stats-content {
    display: grid;
    grid-template-columns: 1fr;
    gap: 10px;
  }

  .stat-item {
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--td-bg-color-page);

    .t-icon {
      font-size: 22px;
    }
  }

  .action-buttons {
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    margin-bottom: 14px;
  }

  .feature-description {
    padding: 12px;

    h4 {
      font-size: 14px;
      margin-bottom: 8px;
    }

    ul {
      padding-left: 18px;
    }

    li {
      font-size: 12px;
      line-height: 1.5;
    }
  }

  .dialog-content {
    padding: 8px 0;
  }

  .export-options,
  .import-options {
    flex-direction: column;
    gap: 10px;
  }

  .export-option-card:hover,
  .import-option-card:hover {
    transform: none;
  }
}
</style>
