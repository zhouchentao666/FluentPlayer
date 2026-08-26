<script setup lang="ts">
import { ref, computed } from 'vue'
import { storeToRefs } from 'pinia'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { useSettingsStore } from '@/store/Settings'
import TitleBarControls from '@/components/TitleBarControls.vue'
import LyricFontSettings from '@/components/Settings/LyricFontSettings.vue'
import DesktopLyricStyle from '@/components/Settings/DesktopLyricStyle.vue'

const { t } = useI18n()

const userStore = LocalUserDetailStore()
const { userInfo } = storeToRefs(userStore)
const settingsStore = useSettingsStore()
const { settings } = storeToRefs(settingsStore)

const languageOptions = computed(() => [
  { label: t('settings.languageSystem'), value: 'system' },
  { label: t('settings.languageZhCN'), value: 'zh-CN' },
  { label: t('settings.languageEnUS'), value: 'en-US' }
])

const currentStyle = ref<'windows' | 'traffic-light'>(
  userInfo.value.topBarStyle ? 'traffic-light' : 'windows'
)

const switchStyle = (style: 'windows' | 'traffic-light'): void => {
  currentStyle.value = style
  userInfo.value.topBarStyle = style === 'traffic-light'
}
</script>

<template>
  <div class="settings-section">
    <t-card :title="t('settings.appearance.title')" class="setting-card" hover-shadow>
      <div id="appearance-language" class="setting-group-item">
        <div class="setting-label">
          <h4>{{ t('settings.language') }}</h4>
          <p>{{ t('settings.languageDesc') }}</p>
        </div>
        <t-select
          :value="settings.language || 'system'"
          :options="languageOptions"
          style="max-width: 260px"
          @change="(val: unknown) => settingsStore.updateLanguage(val as 'system' | 'zh-CN' | 'en-US')"
        />
      </div>

      <t-divider />

      <div id="appearance-titlebar" class="setting-group-item mobile-hidden">
        <div class="setting-label">
          <h4>{{ t('settings.appearance.titleBarStyle') }}</h4>
          <p>{{ t('settings.appearance.titleBarStyleDesc') }}</p>
        </div>
        <div class="style-buttons">
          <t-button :theme="currentStyle === 'windows' ? 'primary' : 'default'" @click="switchStyle('windows')">
            {{ t('settings.appearance.windowsStyle') }}
          </t-button>
          <t-button :theme="currentStyle === 'traffic-light' ? 'primary' : 'default'" @click="switchStyle('traffic-light')">
            {{ t('settings.appearance.trafficLightStyle') }}
          </t-button>
        </div>
        <div class="style-preview">
          <div class="preview-item">
            <h4>{{ t('settings.appearance.windowsPreviewTitle') }}</h4>
            <div class="mock-titlebar">
              <div class="mock-title">{{ t('settings.appearance.windowsPreviewBar') }}</div>
              <TitleBarControls control-style="windows" />
            </div>
          </div>
          <div class="preview-item">
            <h4>{{ t('settings.appearance.trafficLightPreviewTitle') }}</h4>
            <div class="mock-titlebar">
              <div class="mock-title">{{ t('settings.appearance.trafficLightPreviewBar') }}</div>
              <TitleBarControls control-style="traffic-light" />
            </div>
          </div>
        </div>
      </div>

      <t-divider class="mobile-hidden" />

      <div id="appearance-close-behavior" class="setting-group-item mobile-hidden">
        <div class="setting-label">
          <h4>{{ t('settings.appearance.closeBehavior') }}</h4>
          <p>{{ t('settings.appearance.closeBehaviorDesc') }}</p>
        </div>
        <div class="setting-control" style="display: flex; align-items: center; gap: 10px">
          <t-switch
            :value="settings.closeToTray"
            @change="(val: any) => settingsStore.updateSettings({ closeToTray: Boolean(val) })"
          />
          <span class="setting-text">{{ settings.closeToTray ? t('settings.appearance.closeToTray') : t('settings.appearance.directExit') }}</span>
        </div>
      </div>
    </t-card>

    <div class="setting-spacer"></div>
    <div id="appearance-lyric-font">
      <LyricFontSettings />
    </div>

    <div class="setting-spacer"></div>
    <div id="appearance-desktop-lyric">
      <DesktopLyricStyle />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.settings-section {
  animation: fadeInUp 0.4s ease-out;
  animation-fill-mode: both;
}

.setting-group-item {
  margin-bottom: 24px;
  &:last-child { margin-bottom: 0; }
}

.setting-label {
  margin-bottom: 16px;
  h4 { margin: 0 0 4px; font-size: 14px; font-weight: 600; color: var(--td-text-color-primary); }
  p { margin: 0; font-size: 12px; color: var(--td-text-color-secondary); }
}

.style-buttons {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
  .t-button { transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease; &:hover { transform: translateY(-1px); } }
}

.style-preview {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;

  .preview-item {
    background: var(--td-bg-color-page);
    padding: 1rem;
    border-radius: 0.5rem;
    border: 1px solid var(--td-border-level-1-color);
    h4 { margin: 0 0 0.75rem; font-size: 0.875rem; font-weight: 600; color: var(--td-text-color-primary); }
  }
}

.mock-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  background: var(--td-bg-color-container);
  border-radius: 0.375rem;
  border: 1px solid var(--td-border-level-1-color);
  .mock-title { font-weight: 500; color: var(--td-text-color-primary); font-size: 0.875rem; }
}

.setting-spacer { height: 24px; }

@keyframes fadeInUp {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

@media (max-width: 768px) {
  .mobile-hidden {
    display: none !important;
  }

  .setting-group-item {
    margin-bottom: 16px;
  }

  .setting-label {
    margin-bottom: 12px;
  }

  .style-buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 12px;

    .t-button {
      min-width: 0;
      padding: 0 10px;
      font-size: 13px;
    }
  }

  .style-preview {
    grid-template-columns: 1fr;
    gap: 10px;

    .preview-item {
      padding: 12px;

      h4 {
        margin-bottom: 8px;
        font-size: 13px;
      }
    }
  }

  .mock-titlebar {
    padding: 10px 12px;

    .mock-title {
      max-width: 120px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 12px;
    }
  }
}

@media (max-width: 380px) {
  .style-buttons {
    grid-template-columns: 1fr;
  }
}
</style>
