<template>
  <div class="directory-settings">
      <div class="section-header">
        <span class="section-title">{{ t('settings.storage.directoryConfig') }}</span>
        <t-button theme="default" size="small" @click="resetDirectories"> {{ t('settings.storage.resetToDefault') }} </t-button>
      </div>

      <div class="directory-section">
        <h4>{{ t('settings.storage.cacheDirectory') }}</h4>
        <p class="directory-description">{{ t('settings.storage.cacheDirectoryDesc') }}</p>

        <div class="directory-item">
          <div class="directory-info">
            <div class="directory-path">
              <t-input
                v-model="directories.cacheDir"
                readonly
                :placeholder="t('settings.storage.cacheDirPlaceholder')"
                class="path-input"
              />
            </div>
            <div class="directory-size">
              <t-tag theme="primary" variant="light">
                {{ cacheDirSize.formatted }}
              </t-tag>
            </div>
          </div>

          <div class="directory-actions">
            <t-button theme="default" @click="selectCacheDir"> {{ t('settings.storage.selectDirectory') }} </t-button>
            <t-button theme="default" variant="outline" @click="openCacheDir"> {{ t('settings.storage.openDirectory') }} </t-button>
          </div>
        </div>
      </div>

      <t-divider />

      <div class="directory-section">
        <h4>{{ t('settings.storage.downloadDirectory') }}</h4>
        <p class="directory-description">{{ t('settings.storage.downloadDirectoryDesc') }}</p>

        <div class="directory-item">
          <div class="directory-info">
            <div class="directory-path">
              <t-input
                v-model="directories.downloadDir"
                readonly
                :placeholder="t('settings.storage.downloadDirPlaceholder')"
                class="path-input"
              />
            </div>
            <div class="directory-size">
              <t-tag theme="success" variant="light">
                {{ downloadDirSize.formatted }}
              </t-tag>
            </div>
          </div>

          <div class="directory-actions">
            <t-button theme="default" @click="selectDownloadDir"> {{ t('settings.storage.selectDirectory') }} </t-button>
            <t-button theme="default" variant="outline" @click="openDownloadDir">
              {{ t('settings.storage.openDirectory') }}
            </t-button>
          </div>
        </div>
      </div>

      <div class="save-section">
        <t-button theme="primary" size="large" :loading="isSaving" @click="saveDirectories">
          {{ t('settings.storage.saveSettings') }}
        </t-button>
      </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, toRaw } from 'vue'
import { MessagePlugin, DialogPlugin } from 'tdesign-vue-next'

const { t } = useI18n()

// 定义事件
const emit = defineEmits<{
  'directory-changed': []
  'cache-cleared': []
}>()

// 响应式数据
const directories = ref({
  cacheDir: '',
  downloadDir: ''
})

const cacheDirSize = ref({ size: 0, formatted: '0 B' })
const downloadDirSize = ref({ size: 0, formatted: '0 B' })
const isSaving = ref(false)

// 格式化文件大小
const formatSize = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return (bytes / Math.pow(1024, i)).toFixed(2) + ' ' + units[i]
}

// 加载目录配置
const loadDirectories = async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const dirs = await invoke('get_directories') as { cacheDir: string; downloadDir: string }
    if (dirs) {
      directories.value = dirs
    }

    // 获取目录大小
    await Promise.all([updateCacheDirSize(), updateDownloadDirSize()])
  } catch (error) {
    console.error('加载目录配置失败:', error)
    MessagePlugin.error(t('settings.storage.loadDirFailed'))
  }
}

// 更新缓存目录大小
const updateCacheDirSize = async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const size = await invoke('get_directory_size', { path: directories.value.cacheDir }) as number
    cacheDirSize.value = { size, formatted: formatSize(size) }
  } catch (error) {
    console.error('获取缓存目录大小失败:', error)
  }
}

// 更新下载目录大小
const updateDownloadDirSize = async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const size = await invoke('get_directory_size', { path: directories.value.downloadDir }) as number
    downloadDirSize.value = { size, formatted: formatSize(size) }
  } catch (error) {
    console.error('获取下载目录大小失败:', error)
  }
}

// 选择缓存目录
const selectCacheDir = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({ directory: true, multiple: false })
    if (selected) {
      const path = typeof selected === 'string' ? selected : (selected as string[])[0]
      directories.value.cacheDir = path
      await updateCacheDirSize()
      MessagePlugin.success(t('settings.storage.cacheDirSelected'))
    }
  } catch (error) {
    console.error('选择缓存目录失败:', error)
    MessagePlugin.error(t('settings.storage.selectDirFailed'))
  }
}

// 选择下载目录
const selectDownloadDir = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({ directory: true, multiple: false })
    if (selected) {
      const path = typeof selected === 'string' ? selected : (selected as string[])[0]
      directories.value.downloadDir = path
      await updateDownloadDirSize()
      MessagePlugin.success(t('settings.storage.downloadDirSelected'))
    }
  } catch (error) {
    console.error('选择下载目录失败:', error)
    MessagePlugin.error(t('settings.storage.selectDirFailed'))
  }
}

// 打开缓存目录
const openCacheDir = async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('open_directory', { path: directories.value.cacheDir })
  } catch (error) {
    console.error('打开缓存目录失败:', error)
    MessagePlugin.error(t('settings.storage.openDirFailed'))
  }
}

// 打开下载目录
const openDownloadDir = async () => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('open_directory', { path: directories.value.downloadDir })
  } catch (error) {
    console.error('打开下载目录失败:', error)
    MessagePlugin.error(t('settings.storage.openDirFailed'))
  }
}

// 保存目录设置
const saveDirectories = async () => {
  isSaving.value = true

  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('save_directories', { directories: toRaw(directories.value) })
    MessagePlugin.success(t('settings.storage.saveDirSuccess'))
    emit('directory-changed')
  } catch (error) {
    console.error('保存目录设置失败:', error)
    MessagePlugin.error(t('settings.storage.saveSettingsFailed'))
  } finally {
    isSaving.value = false
  }
}

// 重置为默认目录
const resetDirectories = async () => {
  const confirm = DialogPlugin.confirm({
    header: t('settings.storage.resetDirTitle'),
    body: t('settings.storage.resetDirBody'),
    confirmBtn: t('settings.storage.confirmReset'),
    cancelBtn: t('common.cancel'),
    onConfirm: async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core')
        const result = await invoke('reset_directories') as { cacheDir: string; downloadDir: string }
        if (result) {
          directories.value = result
          await Promise.all([updateCacheDirSize(), updateDownloadDirSize()])
          MessagePlugin.success(t('settings.storage.resetSuccess'))
          emit('directory-changed')
        }
      } catch (error) {
        console.error('重置目录设置失败:', error)
        MessagePlugin.error(t('settings.storage.resetFailed'))
      }
      confirm.hide()
    }
  })
}

// 刷新目录大小（供父组件调用）
const refreshDirectorySizes = async () => {
  await Promise.all([updateCacheDirSize(), updateDownloadDirSize()])
}

// 暴露方法给父组件
defineExpose({
  refreshDirectorySizes
})

// 组件挂载时加载配置
onMounted(() => {
  loadDirectories()
})
</script>

<style lang="scss" scoped>
.directory-settings {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--td-border-level-1-color);
  margin-bottom: 16px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--td-text-color-primary);
  white-space: nowrap;
}

.directory-section {
  margin-bottom: 24px;

  h4 {
    margin: 0 0 8px 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--td-text-color-primary);
  }

  .directory-description {
    margin: 0 0 16px 0;
    font-size: 14px;
    color: var(--td-text-color-secondary);
  }
}

.directory-item {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.directory-info {
  display: flex;
  align-items: center;
  gap: 12px;

  .directory-path {
    flex: 1;

    .path-input {
      width: 100%;
    }
  }

  .directory-size {
    flex-shrink: 0;
  }
}

.directory-actions {
  display: flex;
  gap: 8px;
}

.save-section {
  margin-top: 24px;
  padding-top: 24px;
  border-top: 1px solid var(--td-border-level-1-color);
  text-align: center;
}

.cache-management {
  margin-top: 24px;
}

/* 响应式设计 */
@media (max-width: 768px) {
  .directory-info {
    flex-direction: column;
    align-items: stretch;
  }

  .directory-actions {
    justify-content: stretch;

    button {
      flex: 1;
    }
  }
}
</style>
