<script setup lang="ts">
import { ref, onMounted } from 'vue'

const { t } = useI18n()

interface LyricOption {
  fontSize: number
  mainColor: string
  fontWeight: number
  position: 'left' | 'center' | 'right' | 'both'
  alwaysShowPlayInfo: boolean
  animation: boolean
  showYrc: boolean
  showTran: boolean
  isDoubleLine: boolean
  fontFamily: string
}

const defaultOptions: LyricOption = {
  fontSize: 36,
  mainColor: '#ffffff',
  fontWeight: 700,
  position: 'center',
  alwaysShowPlayInfo: false,
  animation: true,
  showYrc: true,
  showTran: true,
  isDoubleLine: false,
  fontFamily: 'lyricfont'
}

const options = ref<LyricOption>({ ...defaultOptions })
const isDesktopLyricOpen = ref(false)

const loadOptions = async () => {
  try {
    if ((window as any).electron?.ipcRenderer) {
      const saved = await (window as any).electron.ipcRenderer.invoke('get-desktop-lyric-option')
      if (saved) Object.assign(options.value, saved)
      const state = await (window as any).electron.ipcRenderer.invoke('get-lyric-open-state')
      isDesktopLyricOpen.value = !!state
    }
  } catch {
    // Silently ignore errors in non-Electron environment
  }
}

const applyOptions = () => {
  try {
    if ((window as any).electron?.ipcRenderer) {
      (window as any).electron.ipcRenderer.send('set-desktop-lyric-option', options.value, true)
    }
  } catch {
    // Silently ignore errors in non-Electron environment
  }
}

const toggleDesktopLyric = async (val: any) => {
  isDesktopLyricOpen.value = Boolean(val)
  try {
    if ((window as any).electron?.ipcRenderer) {
      (window as any).electron.ipcRenderer.send('change-desktop-lyric', val)
    }
  } catch {
    // Silently ignore errors in non-Electron environment
  }
}

const updateOption = <K extends keyof LyricOption>(key: K, value: LyricOption[K]) => {
  options.value[key] = value
  applyOptions()
}

onMounted(() => {
  loadOptions()
})
</script>

<template>
  <t-card :title="t('settings.desktopLyric.title')" hover-shadow>
    <div class="desktop-lyric-style">
      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.desktopLyric.lyricSwitch') }}</div>
          <div class="setting-desc">{{ t('settings.desktopLyric.lyricSwitchDesc') }}</div>
        </div>
        <t-switch :value="isDesktopLyricOpen" @change="toggleDesktopLyric" />
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.desktopLyric.fontSize') }}</div>
          <div class="setting-desc">{{ options.fontSize }}px</div>
        </div>
        <t-input-number v-model="options.fontSize" :min="12" :max="96" :step="1" style="width: 120px;" @change="applyOptions" />
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.desktopLyric.fontWeight') }}</div>
        </div>
        <t-select :value="options.fontWeight" style="width: 120px;" @change="(v: any) => updateOption('fontWeight', Number(v))">
          <t-option :value="400" :label="t('settings.desktopLyric.fontWeightThin')" />
          <t-option :value="500" :label="t('settings.desktopLyric.fontWeightMedium')" />
          <t-option :value="600" :label="t('settings.desktopLyric.fontWeightSemibold')" />
          <t-option :value="700" :label="t('settings.desktopLyric.fontWeightBold')" />
          <t-option :value="800" :label="t('settings.desktopLyric.fontWeightExtraBold')" />
        </t-select>
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.desktopLyric.lyricColor') }}</div>
        </div>
        <t-color-picker :value="options.mainColor" @change="(v: string) => updateOption('mainColor', v)" />
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.desktopLyric.lyricPosition') }}</div>
        </div>
        <t-select :value="options.position" style="width: 120px;" @change="(v: any) => updateOption('position', v)">
          <t-option value="left" :label="t('settings.desktopLyric.positionLeft')" />
          <t-option value="center" :label="t('settings.desktopLyric.positionCenter')" />
          <t-option value="right" :label="t('settings.desktopLyric.positionRight')" />
          <t-option value="both" :label="t('settings.desktopLyric.positionBoth')" />
        </t-select>
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.desktopLyric.showYrc') }}</div>
          <div class="setting-desc">{{ t('settings.desktopLyric.showYrcDesc') }}</div>
        </div>
        <t-switch :value="options.showYrc" @change="(v: any) => updateOption('showYrc', Boolean(v))" />
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-title">{{ t('settings.desktopLyric.showTran') }}</div>
        </div>
        <t-switch :value="options.showTran" @change="(v: any) => updateOption('showTran', Boolean(v))" />
      </div>
    </div>
  </t-card>
</template>

<style scoped>
.desktop-lyric-style {
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
</style>
