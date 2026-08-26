<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useSettingsStore } from '@/store/Settings'

const settingsStore = useSettingsStore()
const { t } = useI18n()

const fontList = ref<string[]>([])
const loading = ref(false)

const fontFamilyArray = computed({
  get: () => {
    const val = settingsStore.settings.lyricFontFamily || 'lyricfont'
    return val.split(',').map(f => f.trim()).filter(Boolean)
  },
  set: (val: string[]) => {
    settingsStore.updateSettings({ lyricFontFamily: val.join(', ') })
  }
})

const fontRate = computed({
  get: () => settingsStore.settings.FullPlayLyricFontRate || 1,
  set: (val: number) => settingsStore.updateSettings({ FullPlayLyricFontRate: val })
})

const loadFonts = async () => {
  loading.value = true
  try {
    if ((window as any).electron?.ipcRenderer) {
      const fonts = await (window as any).electron.ipcRenderer.invoke('get-font-list')
      if (fonts) fontList.value = fonts
    } else {
      fontList.value = [
        'lyricfont', 'PingFang SC', 'Helvetica Neue', 'Arial',
        'Microsoft YaHei', 'SimHei', 'SimSun', 'KaiTi', 'FangSong',
        'STHeiti', 'STKaiti', 'STSong', 'STFangsong'
      ]
    }
  } catch (error) {
    fontList.value = ['lyricfont', 'PingFang SC', 'Microsoft YaHei', 'SimHei', 'Arial']
  } finally {
    loading.value = false
  }
}

const previewFontSize = computed(() => {
  const rate = settingsStore.settings.FullPlayLyricFontRate || 1
  const base = settingsStore.settings.lyricFontSize || 36
  return `${Math.round(base * rate)}px`
})

onMounted(() => {
  loadFonts()
})
</script>

<template>
  <t-card :title="t('settings.lyricFont.title')" hover-shadow>
    <div class="lyric-font-settings">
      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.lyricFont.lyricFont') }}</div>
          <div class="setting-desc">{{ t('settings.lyricFont.lyricFontDesc') }}</div>
        </div>
        <t-select
          v-model="fontFamilyArray"
          :loading="loading"
          multiple
          filterable
          creatable
          style="min-width: 0;"
          :popup-props="{ attach: 'body' }"
          :placeholder="t('settings.lyricFont.selectFont')"
        >
          <t-option v-for="font in fontList" :key="font" :value="font" :label="font" />
        </t-select>
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.lyricFont.fontRate') }}</div>
          <div class="setting-desc">{{ (settingsStore.settings.FullPlayLyricFontRate || 1).toFixed(1) }}x</div>
        </div>
        <t-input-number
          v-model="fontRate"
          :min="0.1"
          :max="2"
          :step="0.1"
          :decimal-places="1"
          style="width: 120px;"
        />
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.lyricFont.fontWeight') }}</div>
          <div class="setting-desc">{{ settingsStore.settings.lyricFontWeight || 700 }}</div>
        </div>
        <t-slider
          :value="settingsStore.settings.lyricFontWeight || 700"
          :min="100"
          :max="900"
          :step="100"
          style="width: 200px;"
          @change="(val: any) => settingsStore.updateSettings({ lyricFontWeight: Number(val) })"
        />
      </div>

      <div class="font-preview">
        <div class="preview-label">{{ t('settings.lyricFont.fontPreview') }}</div>
        <div
          class="preview-text"
          :style="{
            fontFamily: settingsStore.settings.lyricFontFamily || 'lyricfont',
            fontSize: previewFontSize,
            fontWeight: settingsStore.settings.lyricFontWeight || 700
          }"
        >
          {{ t('settings.lyricFont.fontPreviewText') }}
        </div>
      </div>
    </div>
  </t-card>
</template>

<style scoped>
.lyric-font-settings {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.875rem 1rem;
  border: 1px solid var(--td-border-level-1-color);
  background: var(--td-bg-color-page);
  border-radius: 0.5rem;
}

.setting-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.setting-title { font-weight: 600; color: var(--td-text-color-primary); font-size: 0.95rem; }
.setting-desc { color: var(--td-text-color-secondary); font-size: 0.8rem; }

.font-preview {
  padding: 1rem;
  background: var(--td-bg-color-page);
  border-radius: 0.5rem;
  border: 1px solid var(--td-border-level-1-color);
  text-align: center;
}

.preview-label { font-size: 0.75rem; color: var(--td-text-color-secondary); margin-bottom: 0.5rem; }
.preview-text { color: var(--td-text-color-primary); line-height: 1.4; word-break: break-all; }
@media (max-width: 768px) {
  .lyric-font-settings {
    gap: 10px;
  }

  .setting-item {
    align-items: flex-start;
    flex-direction: column;
    gap: 10px;
    padding: 10px 12px;
    overflow: visible;
  }

  .setting-item .t-select {
    width: 100% !important;
    flex: 1 1 auto !important;
  }

  .setting-info {
    width: 100%;
  }

  .font-preview {
    padding: 12px;
  }

  .preview-text {
    font-size: clamp(20px, 8vw, 32px) !important;
  }
}
</style>
