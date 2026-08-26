<script setup lang="ts">
import { ref, computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useSettingsStore } from '@/store/Settings'
import DirectorySettings from '@/components/Settings/DirectorySettings.vue'
import MusicCache from '@/components/Settings/MusicCache.vue'
import { formatMusicInfo } from '@/utils/format'
import { invoke } from '@tauri-apps/api/core'

const { t } = useI18n()

const settingsStore = useSettingsStore()
const { settings } = storeToRefs(settingsStore)

const musicCacheRef = ref()
const directorySettingsRef = ref()

const handleDirectoryChanged = () => {
  if (musicCacheRef.value?.refreshCacheInfo) {
    musicCacheRef.value.refreshCacheInfo()
  }
}

const handleCacheCleared = () => {
  if (directorySettingsRef.value?.refreshDirectorySizes) {
    directorySettingsRef.value.refreshDirectorySizes()
  }
}

// Cache size limit options (bytes)
const cacheSizeOptions = [
  { label: '500 MB', value: 524288000 },
  { label: '1 GB', value: 1073741824 },
  { label: '2 GB', value: 2147483648 },
  { label: '5 GB', value: 5368709120 },
]

const updateCacheSizeLimit = async (value: any) => {
  const size = Number(value) || 1073741824
  settingsStore.updateSettings({ cacheSizeLimit: size })
  try {
    const dirs = await invoke<{ cacheDir: string; downloadDir: string }>('get_directories')
    await invoke('player__set_cache_config', {
      cacheDir: settings.value.autoCacheMusic !== false ? dirs.cacheDir : null,
      maxSize: size,
    })
  } catch (e) {
    console.warn('更新缓存配置失败:', e)
  }
}

const updateAutoCache = async (enabled: any) => {
  const val = !!enabled
  settingsStore.updateSettings({ autoCacheMusic: val })
  try {
    const dirs = await invoke<{ cacheDir: string; downloadDir: string }>('get_directories')
    await invoke('player__set_cache_config', {
      cacheDir: val ? dirs.cacheDir : null,
      maxSize: settings.value.cacheSizeLimit || 1073741824,
    })
  } catch (e) {
    console.warn('更新缓存配置失败:', e)
  }
}

// Filename template logic
const filenameTemplate = ref(settings.value.filenameTemplate || '%t - %s')
const previewSongInfo = computed(() => ({
  name: t('settings.storage.templateSongName'),
  singer: t('settings.storage.templateSinger'),
  albumName: t('settings.storage.templateAlbum'),
  platform: 'tx',
  quality: 'master',
  date: '2026-01-01'
}))

const updateFilenameTemplate = () => {
  settingsStore.updateSettings({
    filenameTemplate: filenameTemplate.value || '%t - %s'
  })
}

// Tag options logic
const tagWriteOptions = ref({
  basicInfo: settings.value.tagWriteOptions?.basicInfo ?? true,
  cover: settings.value.tagWriteOptions?.cover ?? true,
  lyrics: settings.value.tagWriteOptions?.lyrics ?? true,
  downloadLyrics: settings.value.tagWriteOptions?.downloadLyrics ?? false,
  lyricFormat: settings.value.tagWriteOptions?.lyricFormat ?? 'word-by-word'
})

const updateTagWriteOptions = () => {
  settingsStore.updateSettings({
    tagWriteOptions: { ...tagWriteOptions.value }
  })
}

const getTagOptionsStatus = () => {
  const enabled: string[] = []
  if (tagWriteOptions.value.basicInfo) enabled.push(t('settings.storage.tagBasicInfo'))
  if (tagWriteOptions.value.cover) enabled.push(t('settings.storage.tagCover'))
  if (tagWriteOptions.value.lyrics) enabled.push(t('settings.storage.tagLyricsInfo'))
  if (tagWriteOptions.value.downloadLyrics) enabled.push(t('settings.storage.tagDownloadLyricsFile'))

  return enabled.length > 0 ? enabled.join('、') : t('settings.storage.tagNoSelection')
}
</script>

<template>
  <div class="settings-section">
    <div id="storage-directory">
      <DirectorySettings
        ref="directorySettingsRef"
        class="setting-group"
        @directory-changed="handleDirectoryChanged"
        @cache-cleared="handleCacheCleared"
      />
    </div>
    <div id="storage-cache" style="margin-top: 20px" class="setting-group">
      <MusicCache ref="musicCacheRef" @cache-cleared="handleCacheCleared" />
    </div>

    <!-- 缓存策略 -->
    <div id="storage-cache-strategy" class="setting-group">
      <h3>{{ t('settings.storage.cacheStrategy') }}</h3>
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.storage.autoCacheMusic') }}</div>
          <div class="item-desc">{{ t('settings.storage.autoCacheMusicDesc') }}</div>
        </div>
        <t-switch
          v-model="settings.autoCacheMusic"
          @change="updateAutoCache"
        />
      </div>
      <div class="setting-item" v-if="settings.autoCacheMusic !== false">
        <div class="item-info">
          <div class="item-title">{{ t('settings.storage.cacheSizeLimit') }}</div>
          <div class="item-desc">{{ t('settings.storage.cacheSizeLimitDesc') }}</div>
        </div>
        <t-radio-group
          :value="settings.cacheSizeLimit || 1073741824"
          variant="default-filled"
          @change="updateCacheSizeLimit"
        >
          <t-radio-button v-for="opt in cacheSizeOptions" :key="opt.value" :value="opt.value">
            {{ opt.label }}
          </t-radio-button>
        </t-radio-group>
      </div>
    </div>

    <!-- 本地音乐扫描设置 -->
    <div id="storage-local-scan" class="setting-group">
      <h3>{{ t('settings.storage.localScanTitle') }}</h3>
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.storage.skipHiddenFiles') }}</div>
          <div class="item-desc">{{ t('settings.storage.skipHiddenFilesDesc') }}</div>
        </div>
        <t-switch
          v-model="settings.skipHiddenFiles"
          @change="() => settingsStore.saveSettings()"
        />
      </div>
    </div>

    <!-- 下载文件名格式设置 -->
    <div id="storage-filename" class="setting-group">
      <h3>{{ t('settings.storage.filenameFormat') }}</h3>
      <p>{{ t('settings.storage.filenameFormatDesc') }}</p>

      <div class="template-tip">
        <div class="template-tip-item">
          <t-tag>%t</t-tag>
          <span>{{ t('settings.storage.templateSongName') }}</span>
        </div>
        <div class="template-tip-item">
          <t-tag>%s</t-tag>
          <span>{{ t('settings.storage.templateSinger') }}</span>
        </div>
        <div class="template-tip-item">
          <t-tag>%a</t-tag>
          <span>{{ t('settings.storage.templateAlbum') }}</span>
        </div>
        <div class="template-tip-item">
          <t-tag>%u</t-tag>
          <span>{{ t('settings.storage.templatePlatform') }}</span>
        </div>
        <t-tooltip :content="t('settings.storage.templateQualityTip')">
          <div class="template-tip-item">
            <t-tag>%q</t-tag>
            <span style="display: flex; align-items: center">
              {{ t('settings.storage.templateQuality') }}
              <t-icon name="info-circle" size="12" style="margin-left: 0.2em" />
            </span>
          </div>
        </t-tooltip>
        <div class="template-tip-item">
          <t-tag>%d</t-tag>
          <span>{{ t('settings.storage.templateDate') }}</span>
        </div>
      </div>

      <div class="setting-item">
        <t-input
          v-model="filenameTemplate"
          :placeholder="t('settings.storage.filenamePlaceholder')"
          @change="updateFilenameTemplate"
        />
      </div>

      <div class="preview-container">
        <div>{{ t('settings.storage.preview') }}</div>
        <div>{{ formatMusicInfo(filenameTemplate || '%t - %s', previewSongInfo) }}</div>
      </div>
    </div>

    <!-- 标签写入设置 -->
    <div id="storage-tags" class="setting-group">
      <h3>{{ t('settings.storage.tagWriteSettings') }}</h3>
      <p>{{ t('settings.storage.tagWriteDesc') }}</p>

      <div class="tag-options">
        <div class="tag-option">
          <t-checkbox v-model="tagWriteOptions.basicInfo" @change="updateTagWriteOptions">
            {{ t('settings.storage.tagBasicInfo') }}
          </t-checkbox>
          <p class="option-desc">{{ t('settings.storage.tagBasicInfoDesc') }}</p>
        </div>

        <div class="tag-option">
          <t-checkbox v-model="tagWriteOptions.cover" @change="updateTagWriteOptions">
            {{ t('settings.storage.tagCover') }}
          </t-checkbox>
          <p class="option-desc">{{ t('settings.storage.tagCoverDesc') }}</p>
        </div>

        <div class="tag-option">
          <t-checkbox v-model="tagWriteOptions.lyrics" @change="updateTagWriteOptions">
            {{ t('settings.storage.tagLyricsInfo') }}
          </t-checkbox>
          <p class="option-desc">{{ t('settings.storage.tagLyricsInfoDesc') }}</p>
        </div>

        <div class="tag-option">
          <t-checkbox v-model="tagWriteOptions.downloadLyrics" @change="updateTagWriteOptions">
            {{ t('settings.storage.tagDownloadLyricsFile') }}
          </t-checkbox>
          <p class="option-desc">{{ t('settings.storage.tagDownloadLyricsFileDesc') }}</p>
        </div>

        <div class="tag-option lyric-format-options">
          <t-radio-group
            v-model="tagWriteOptions.lyricFormat"
            :disabled="!tagWriteOptions.lyrics && !tagWriteOptions.downloadLyrics"
            @change="updateTagWriteOptions"
          >
            <t-radio-button value="lrc">{{ t('settings.storage.tagStandardLrc') }}</t-radio-button>
            <t-radio-button value="word-by-word">{{ t('settings.storage.tagWordByWord') }}</t-radio-button>
          </t-radio-group>
          <p class="option-desc">{{ t('settings.storage.tagLyricFormatDesc') }}</p>
        </div>
      </div>

      <div class="tag-options-status">
        <div class="status-summary">
          <span class="status-label">{{ t('settings.storage.tagCurrentConfig') }}</span>
          <span class="status-value">
            {{ getTagOptionsStatus() }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.settings-section {
  animation: fadeInUp 0.4s ease-out;
  animation-fill-mode: both;
}

.setting-group {
  background: var(--settings-group-bg, var(--td-bg-color-container));
  border-radius: 0.75rem;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
  border: 1px solid var(--settings-group-border, var(--td-border-level-1-color));
  box-shadow: 0 1px 3px var(--settings-group-shadow);
  animation: fadeInUp 0.4s ease-out;
  animation-fill-mode: both;

  @for $i from 1 through 5 {
    &:nth-child(#{$i}) {
      animation-delay: #{$i * 0.1}s;
    }
  }

  h3 {
    margin: 0 0 0.5rem;
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--td-text-color-primary);
  }

  > p {
    margin: 0 0 1.5rem;
    color: var(--td-text-color-secondary);
    font-size: 0.875rem;
  }
}

.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.875rem 1rem;
  border: 1px solid var(--settings-feature-border, var(--td-border-level-1-color));
  background: var(--settings-feature-bg, var(--td-bg-color-container));
  border-radius: 0.5rem;
  margin-top: 0.75rem;

  .item-info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;

    .item-title {
      font-weight: 600;
      color: var(--td-text-color-primary);
      font-size: 0.95rem;
      line-height: 1.2;
    }

    .item-desc {
      color: var(--td-text-color-secondary);
      font-size: 0.8rem;
      line-height: 1.2;
    }
  }
}

.setting-item :deep(.t-radio-group--filled) {
  overflow: hidden;
  background: var(--td-bg-color-secondarycontainer);
  border: 1px solid var(--td-component-border);
  border-radius: 6px;
}

.setting-item :deep(.t-radio-group--filled .t-radio-button) {
  color: var(--td-text-color-secondary);
}

.setting-item :deep(.t-radio-group--filled .t-radio-button:hover),
.setting-item :deep(.t-radio-group--filled .t-radio-button.t-is-checked),
.setting-item :deep(.t-radio-group--filled .t-radio-button.t-is-checked .t-radio-button__label),
.setting-item :deep(.t-radio-group--filled .t-radio-button--checked),
.setting-item :deep(.t-radio-group--filled .t-radio-button--checked .t-radio-button__label) {
  color: var(--settings-nav-label-active, var(--td-text-color-primary));
}

.setting-item :deep(.t-radio-group--filled .t-radio-group__bg-block) {
  background: var(--settings-nav-active-bg, var(--td-bg-color-component-active));
  border: 1px solid var(--settings-nav-active-border, var(--td-brand-color));
  box-shadow: var(--settings-nav-active-shadow, none);
}

// 文件名模板样式
.template-tip {
  display: flex;
  align-items: center;
  gap: 2em;
  color: var(--td-text-color-secondary);
}

.template-tip-item {
  display: flex;
  align-items: center;
  gap: 0.5em;
  color: var(--td-text-color-secondary);
}

.preview-container {
  display: flex;
  align-items: center;
  gap: 0.5em;
  margin: 0.5em 0 0 0;
  color: var(--td-text-color-secondary);
}

.preview-container > div:last-child {
  color: var(--td-text-color-primary);
  font-weight: 500;
}

// 标签写入设置样式
.tag-options {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  margin-bottom: 1.5rem;

  .tag-option {
    padding: 1rem;
    background: var(--settings-feature-bg, var(--td-bg-color-container));
    border-radius: 0.5rem;
    border: 1px solid var(--td-border-level-1-color);

    .option-desc {
      margin: 0.5rem 0 0 1.5rem;
      font-size: 0.875rem;
      color: var(--td-text-color-secondary);
      line-height: 1.4;
    }
  }

  .lyric-format-options {
    padding-top: 1rem;
    margin-top: 1rem;
    border-top: 1px solid var(--td-border-level-1-color);
  }
}

.tag-options-status {
  background: var(--td-bg-color-page);
  padding: 1rem;
  border-radius: 0.5rem;
  border: 1px solid var(--td-border-level-1-color);
}

.status-summary {
  display: flex;
  align-items: center;
  gap: 0.5rem;

  .status-label {
    font-weight: 500;
    color: var(--td-text-color-secondary);
    font-size: 0.875rem;
  }

  .status-value {
    font-weight: 600;
    color: var(--td-text-color-primary);
    font-size: 0.875rem;
  }
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (max-width: 768px) {
  .setting-group {
    padding: 14px;
    margin-bottom: 10px;

    h3 {
      font-size: 16px;
      line-height: 1.35;
    }

    > p {
      margin-bottom: 12px;
      font-size: 12px;
      line-height: 1.45;
    }
  }

  .setting-item {
    align-items: flex-start;
    flex-direction: column;
    gap: 10px;
    padding: 10px 12px;

    .item-info {
      width: 100%;
    }
  }

  .template-tip {
    gap: 8px;
    flex-wrap: wrap;
  }

  .template-tip-item {
    align-items: center;
    gap: 6px;
    min-height: 28px;
    padding: 4px 8px;
    border-radius: 999px;
    background: var(--settings-feature-bg, var(--td-bg-color-container));
  }

  .preview-container {
    align-items: flex-start;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--settings-feature-bg, var(--td-bg-color-container));
    word-break: break-all;
  }

  .tag-options {
    gap: 10px;
    margin-bottom: 12px;

    .tag-option {
      padding: 12px;

      .option-desc {
        margin: 6px 0 0 0;
        font-size: 12px;
        line-height: 1.45;
      }
    }

    .lyric-format-options {
      margin-top: 0;
      padding-top: 12px;
    }
  }

  .tag-options-status {
    padding: 12px;
  }

  .status-summary {
    align-items: flex-start;
    flex-direction: column;
    gap: 4px;
  }
}
</style>
