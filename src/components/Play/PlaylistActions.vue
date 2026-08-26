<script setup lang="ts">
import { ref } from 'vue'
import { MessagePlugin, DialogPlugin } from 'tdesign-vue-next'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import {
  exportPlaylistToFile,
  copyPlaylistToClipboard,
  importPlaylistFromFile,
  importPlaylistFromClipboard,
  validateImportedPlaylist
} from '@/utils/playlist/playlistExportImport'
import { CloudDownloadIcon } from 'tdesign-icons-vue-next'
import type { SongList } from '@/types/audio'
import { storeToRefs } from 'pinia'

const { t } = useI18n()

const localUserStore = LocalUserDetailStore()
const { list } = storeToRefs(localUserStore)

const exportDialogVisible = ref(false)
const importDialogVisible = ref(false)
const fileInputRef = ref<HTMLInputElement | null>(null)
const uploadedFile = ref<File | null>(null)

const handleExportToFile = async () => {
  try {
    if (list.value.length === 0) {
      MessagePlugin.warning(t('play.playlistEmptyCannotExport'))
      return
    }
    const fileName = await exportPlaylistToFile(list.value)
    MessagePlugin.success(t('play.playlistExported', { fileName }))
    exportDialogVisible.value = false
  } catch (error) {
    MessagePlugin.error(`${t('play.exportFailed')}: ${(error as Error).message}`)
  }
}

const handleCopyToClipboard = async () => {
  try {
    if (list.value.length === 0) {
      MessagePlugin.warning(t('play.playlistEmptyCannotCopy'))
      return
    }
    await copyPlaylistToClipboard(list.value)
    MessagePlugin.success(t('play.playlistCopied'))
    exportDialogVisible.value = false
  } catch (error) {
    MessagePlugin.error(`${t('play.copyFailed')}: ${(error as Error).message}`)
  }
}

const triggerFileInput = () => {
  if (fileInputRef.value) fileInputRef.value.click()
}

const handleFileChange = (event: Event) => {
  const input = event.target as HTMLInputElement
  if (input.files && input.files.length > 0) {
    uploadedFile.value = input.files[0]
  }
}

const handleImportFromFile = async () => {
  try {
    if (!uploadedFile.value) {
      MessagePlugin.warning(t('play.selectFileFirst'))
      return
    }
    const importedPlaylist = await importPlaylistFromFile(uploadedFile.value)
    if (!validateImportedPlaylist(importedPlaylist)) {
      throw new Error(t('play.importFormatIncorrect'))
    }
    const mergedList = mergePlaylist(list.value, importedPlaylist)
    list.value = mergedList
    MessagePlugin.success(t('play.importedSongs', { count: importedPlaylist.length }))
    importDialogVisible.value = false
    uploadedFile.value = null
    if (fileInputRef.value) fileInputRef.value.value = ''
  } catch (error) {
    MessagePlugin.error(`${t('play.importFailed')}: ${(error as Error).message}`)
  }
}

const handleImportFromClipboard = async () => {
  try {
    const importedPlaylist = await importPlaylistFromClipboard()
    if (!validateImportedPlaylist(importedPlaylist)) {
      throw new Error(t('play.clipboardFormatIncorrect'))
    }
    const mergedList = mergePlaylist(list.value, importedPlaylist)
    list.value = mergedList
    MessagePlugin.success(t('play.importedSongs', { count: importedPlaylist.length }))
    importDialogVisible.value = false
  } catch (error) {
    MessagePlugin.error(`${t('play.clipboardImportFailed')}: ${(error as Error).message}`)
  }
}

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

const handleClearPlaylist = () => {
  const dialog = DialogPlugin.confirm({
    header: t('play.confirmClearTitle'),
    body: t('play.confirmClearBody'),
    theme: 'warning',
    confirmBtn: { theme: 'danger', content: t('common.clear') },
    cancelBtn: t('common.cancel'),
    onConfirm: () => {
      list.value = []
      dialog.destroy()
      MessagePlugin.success(t('play.playlistCleared'))
    }
  })
}
</script>

<template>
  <div class="playlist-actions">
    <div class="action-buttons">
      <t-button theme="primary" variant="outline" @click="exportDialogVisible = true">
        <CloudDownloadIcon />
        {{ t('play.exportPlaylist') }}
      </t-button>
      <t-button theme="primary" variant="outline" @click="importDialogVisible = true">
        {{ t('play.importPlaylist') }}
      </t-button>
      <t-button theme="danger" variant="outline" @click="handleClearPlaylist">
        {{ t('play.clearPlaylistAction') }}
      </t-button>
    </div>
    <t-dialog
      v-model:visible="exportDialogVisible"
      :header="t('play.exportPlaylist')"
      :on-close="() => (exportDialogVisible = false)"
      width="500px"
      attach="body"
    >
      <template #body>
        <div class="dialog-content">
          <p class="dialog-description">{{ t('play.selectExportMethod') }}</p>
          <div class="export-options">
            <t-card
              :title="t('play.exportToFile')"
              :description="t('play.exportToFileDesc')"
              class="export-option-card"
              @click="handleExportToFile"
            >
              <template #avatar><t-icon name="file" size="large" /></template>
            </t-card>
            <t-card
              :title="t('play.copyToClipboard')"
              :description="t('play.copyToClipboardDesc')"
              class="export-option-card"
              @click="handleCopyToClipboard"
            >
              <template #avatar><t-icon name="copy" size="large" /></template>
            </t-card>
          </div>
        </div>
      </template>
      <template #footer>
        <t-button theme="default" @click="exportDialogVisible = false">{{ t('common.cancel') }}</t-button>
      </template>
    </t-dialog>
    <t-dialog
      v-model:visible="importDialogVisible"
      :header="t('play.importPlaylist')"
      :on-close="() => (importDialogVisible = false)"
      width="500px"
      attach="body"
    >
      <template #body>
        <div class="dialog-content">
          <p class="dialog-description">{{ t('play.selectImportMethod') }}</p>
          <div class="import-options">
            <t-card
              :title="t('play.importFromFile')"
              :description="t('play.importFromFileDesc')"
              class="import-option-card"
            >
              <template #avatar><t-icon name="file" size="large" /></template>
              <template #footer>
                <div class="file-upload-area">
                  <input
                    ref="fileInputRef"
                    type="file"
                    accept=".cpl,.cmpl"
                    style="display: none"
                    @change="handleFileChange"
                  />
                  <t-button theme="primary" variant="outline" @click="triggerFileInput">
                    {{ t('play.selectFile') }}
                  </t-button>
                  <t-button theme="primary" :disabled="!uploadedFile" @click="handleImportFromFile">
                    {{ t('play.importBtn') }}
                  </t-button>
                </div>
              </template>
            </t-card>
            <t-card
              :title="t('play.importFromClipboard')"
              :description="t('play.importFromClipboardDesc')"
              class="import-option-card"
            >
              <template #avatar><t-icon name="paste" size="large" /></template>
              <template #footer>
                <t-button theme="primary" @click="handleImportFromClipboard">
                  {{ t('play.importFromClipboardBtn') }}
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
.playlist-actions {
  padding: 16px;
}

.action-buttons {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
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
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
  }
}

.file-upload-area {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: center;
}
</style>
