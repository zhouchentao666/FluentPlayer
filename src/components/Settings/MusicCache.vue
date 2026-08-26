<template>
  <div class="music-cache">
      <div class="card-header">
        <span class="card-title">{{ t('settings.storage.localCacheConfig') }}</span>
        <span class="cache-stats">
          {{ t('settings.storage.existingCacheSize') }}{{ cacheInfo.sizeFormatted || '0 B' }}
          <span v-if="cacheInfo.count > 0">（{{ t('settings.storage.fileCount', { count: cacheInfo.count }) }}）</span>
        </span>
      </div>
      <div class="card-body">
        <t-button
          size="large"
          :loading="cacheInfo.clearing"
          :disabled="!cacheInfo.count || cacheInfo.count === 0"
          block
          @click="clearCache"
        >
          {{ cacheInfo.clearing ? t('settings.storage.clearingCache') : t('settings.storage.clearLocalCache') }}
        </t-button>
        <div v-if="!cacheInfo.count || cacheInfo.count === 0" class="no-cache-tip">
          {{ t('settings.storage.noCache') }}
        </div>
      </div>
  </div>
</template>

<script lang="ts" setup>
import { DialogPlugin, MessagePlugin } from 'tdesign-vue-next'
import { onMounted, ref } from 'vue'

const { t } = useI18n()

// 定义事件
const emit = defineEmits<{
  'cache-cleared': []
}>()

interface CacheInfo {
  count: number
  size: number
  sizeFormatted: string
  clearing: boolean
}

const cacheInfo = ref<CacheInfo>({
  count: 0,
  size: 0,
  sizeFormatted: '0 B',
  clearing: false
})

// 格式化文件大小
const formatSize = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return (bytes / Math.pow(1024, i)).toFixed(2) + ' ' + units[i]
}

const loadCacheInfo = async (forceRefresh = false) => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const res = await invoke('get_cache_info', { forceRefresh }) as { count: number; size: number }
    if (res) {
      cacheInfo.value = {
        count: res.count || 0,
        size: res.size || 0,
        sizeFormatted: formatSize(res.size || 0),
        clearing: false
      }
    }
  } catch (error) {
    console.error('获取缓存信息失败:', error)
    MessagePlugin.error(t('settings.storage.getCacheInfoFailed'))
  }
}

onMounted(() => {
  loadCacheInfo()
})

const clearCache = () => {
  const confirm = DialogPlugin.confirm({
    header: t('settings.storage.confirmClearCache'),
    body: t('settings.storage.clearCacheBody'),
    confirmBtn: t('settings.storage.confirmClear'),
    cancelBtn: t('settings.storage.letMeThink'),
    placement: 'center',
    onClose: () => {
      confirm.hide()
    },
    onConfirm: async () => {
      confirm.hide()

      try {
        // 显示加载状态
        cacheInfo.value = { ...cacheInfo.value, clearing: true }

        // 执行清除操作
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('clear_cache')

        MessagePlugin.success(t('settings.storage.clearSuccess'))

        // 发射缓存清除事件
        emit('cache-cleared')

        // 立即重置缓存信息显示
        cacheInfo.value = {
          count: 0,
          size: 0,
          sizeFormatted: '0 B',
          clearing: false
        }

        // 多次尝试重新加载，确保获取到最新状态
        let retryCount = 0
        const maxRetries = 3

        const reloadWithRetry = async () => {
          retryCount++
          await loadCacheInfo(true)

          if (cacheInfo.value.count > 0 && retryCount < maxRetries) {
            setTimeout(reloadWithRetry, 1000)
          }
        }

        // 延迟一下再开始重新加载
        setTimeout(reloadWithRetry, 300)
      } catch (error) {
        console.error('清除缓存失败:', error)
        MessagePlugin.error(t('settings.storage.clearCacheFailed'))
        // 清除加载状态
        cacheInfo.value = { ...cacheInfo.value, clearing: false }
      }
    }
  })
}

// 刷新缓存信息（供父组件调用）
const refreshCacheInfo = async () => {
  await loadCacheInfo(true)
}

// 暴露方法给父组件
defineExpose({
  refreshCacheInfo
})
</script>

<style lang="scss" scoped>
.music-cache {
  width: 100%;

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--td-border-level-1-color);
    margin-bottom: 16px;
  }

  .card-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--td-text-color-primary);
    white-space: nowrap;
  }

  .cache-stats {
    font-size: 13px;
    color: var(--td-text-color-secondary);
    white-space: nowrap;
    text-align: right;
  }

  .card-body {
    padding: 4px 0 0;
    text-align: center;

    .no-cache-tip {
      margin-top: 10px;
      color: var(--td-text-color-placeholder);
      font-size: 14px;
    }
  }
}

@media (max-width: 768px) {
  .music-cache {
    .card-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 6px;
      padding-bottom: 12px;
      margin-bottom: 12px;
    }

    .card-title {
      font-size: 15px;
    }

    .cache-stats {
      font-size: 12px;
      white-space: normal;
      text-align: left;
    }

    .card-body {
      :deep(.t-button) {
        width: 100%;
      }
    }
  }
}
</style>
