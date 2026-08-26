<script setup lang="ts">
import { computed } from 'vue'
import { useSettingsStore } from '@/store/Settings'
import { storeToRefs } from 'pinia'
import type { BackgroundRenderPreset } from '@/types/background'
import { BACKGROUND_PRESETS } from '@/types/background'

const settingsStore = useSettingsStore()
const { settings } = storeToRefs(settingsStore)
const { t } = useI18n()

// 确保配置存在
if (!settings.value.backgroundRender) {
  settings.value.backgroundRender = {
    fullPlay: {
      preset: 'auto',
      enabled: true,
      audioResponse: true,
      renderScale: 0.5,
      flowSpeed: 1.0,
      staticMode: false,
      fps: 30
    }
  }
}

const bgSettings = computed(() => settings.value.backgroundRender!)

// FullPlay 配置
const fullPlayConfig = computed(() => bgSettings.value.fullPlay)

// 预设选项
const presetOptions = computed(() => [
  { label: t('settings.backgroundRender.presetAuto'), value: 'auto', desc: t('settings.backgroundRender.presetAutoDesc') },
  { label: t('settings.backgroundRender.presetPerformance'), value: 'performance', desc: t('settings.backgroundRender.presetPerformanceDesc') },
  { label: t('settings.backgroundRender.presetQuality'), value: 'quality', desc: t('settings.backgroundRender.presetQualityDesc') },
  { label: t('settings.backgroundRender.presetCustom'), value: 'custom', desc: t('settings.backgroundRender.presetCustomDesc') }
])

// 更新 FullPlay 配置
const updateFullPlayConfig = (key: string, value: any) => {
  settingsStore.updateSettings({
    backgroundRender: {
      ...bgSettings.value,
      fullPlay: {
        ...fullPlayConfig.value,
        [key]: value
      }
    }
  })
}

// 应用预设
const applyPreset = (preset: BackgroundRenderPreset) => {
  if (preset === 'auto' || preset === 'custom') {
    updateFullPlayConfig('preset', preset)
    return
  }

  const presetConfig = BACKGROUND_PRESETS[preset]

  updateFullPlayConfig('preset', preset)
  updateFullPlayConfig('enabled', presetConfig.enabled)
  updateFullPlayConfig('audioResponse', presetConfig.audioResponse)
  updateFullPlayConfig('renderScale', presetConfig.renderScale)
  updateFullPlayConfig('flowSpeed', presetConfig.flowSpeed)
  updateFullPlayConfig('staticMode', presetConfig.staticMode)
  updateFullPlayConfig('fps', presetConfig.fps)
}
</script>

<template>
  <div class="background-render-settings">
    <!-- FullPlay 背景效果 -->
    <div class="settings-section">
      <h3>{{ t('settings.backgroundRender.title') }}</h3>

      <!-- 预设选择 -->
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.backgroundRender.presetMode') }}</div>
          <div class="item-desc">{{ t('settings.backgroundRender.presetModeDesc') }}</div>
        </div>
        <t-select
          :value="fullPlayConfig.preset"
          :options="presetOptions"
          @change="(val: unknown) => applyPreset(val as BackgroundRenderPreset)"
          style="width: 200px"
        />
      </div>

      <!-- 总开关 -->
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.backgroundRender.enableBgEffect') }}</div>
          <div class="item-desc">{{ t('settings.backgroundRender.enableBgEffectDesc') }}</div>
        </div>
        <t-switch
          :value="fullPlayConfig.enabled"
          @change="updateFullPlayConfig('enabled', $event)"
        />
      </div>

      <!-- 音频响应 -->
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.backgroundRender.audioResponse') }}</div>
          <div class="item-desc">{{ t('settings.backgroundRender.audioResponseDesc') }}</div>
        </div>
        <t-switch
          :value="fullPlayConfig.audioResponse"
          @change="updateFullPlayConfig('audioResponse', $event)"
          :disabled="!fullPlayConfig.enabled"
        />
      </div>

      <!-- 渲染强度 -->
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.backgroundRender.effectIntensity') }}</div>
          <div class="item-desc">
            {{ t('settings.backgroundRender.renderScaleDesc', { value: fullPlayConfig.renderScale.toFixed(1) }) }}
            <span v-if="fullPlayConfig.renderScale <= 0.3" class="tag-low">{{ t('settings.backgroundRender.intensityLow') }}</span>
            <span v-else-if="fullPlayConfig.renderScale <= 0.6" class="tag-medium">{{ t('settings.backgroundRender.intensityMedium') }}</span>
            <span v-else class="tag-high">{{ t('settings.backgroundRender.intensityHigh') }}</span>
          </div>
        </div>
        <t-slider
          :value="fullPlayConfig.renderScale"
          :min="0.1"
          :max="1.0"
          :step="0.1"
          @change="updateFullPlayConfig('renderScale', $event)"
          :disabled="!fullPlayConfig.enabled || fullPlayConfig.preset !== 'custom'"
          style="width: 200px"
        />
      </div>

      <!-- 流动速度 -->
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.backgroundRender.flowSpeed') }}</div>
          <div class="item-desc">{{ t('settings.backgroundRender.flowSpeedDesc', { value: fullPlayConfig.flowSpeed.toFixed(1) }) }}</div>
        </div>
        <t-slider
          :value="fullPlayConfig.flowSpeed"
          :min="0.1"
          :max="5.0"
          :step="0.1"
          @change="updateFullPlayConfig('flowSpeed', $event)"
          :disabled="!fullPlayConfig.enabled || fullPlayConfig.preset !== 'custom'"
          style="width: 200px"
        />
      </div>

      <!-- 静态模式 -->
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.backgroundRender.staticMode') }}</div>
          <div class="item-desc">{{ t('settings.backgroundRender.staticModeDesc') }}</div>
        </div>
        <t-switch
          :value="fullPlayConfig.staticMode"
          @change="updateFullPlayConfig('staticMode', $event)"
          :disabled="!fullPlayConfig.enabled || fullPlayConfig.preset !== 'custom'"
        />
      </div>

      <!-- FPS 限制 -->
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.backgroundRender.fpsLimit') }}</div>
          <div class="item-desc">{{ t('settings.backgroundRender.fpsLimitDesc', { value: fullPlayConfig.fps }) }}</div>
        </div>
        <t-slider
          :value="fullPlayConfig.fps"
          :min="15"
          :max="60"
          :step="5"
          @change="updateFullPlayConfig('fps', $event)"
          :disabled="!fullPlayConfig.enabled || fullPlayConfig.preset !== 'custom'"
          style="width: 200px"
        />
      </div>

      <!-- 重置按钮 -->
      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.backgroundRender.resetConfig') }}</div>
          <div class="item-desc">{{ t('settings.backgroundRender.resetConfigDesc') }}</div>
        </div>
        <t-button size="small" variant="outline" @click="applyPreset('auto')">
          {{ t('settings.backgroundRender.resetConfig') }}
        </t-button>
      </div>
    </div>

    <!-- 性能提示 -->
    <div v-if="fullPlayConfig.renderScale > 0.7 && fullPlayConfig.fps > 45" class="performance-warning">
      <t-icon name="info-circle" />
      <span>{{ t('settings.backgroundRender.performanceWarning') }}</span>
    </div>
  </div>
</template>

<style scoped lang="scss">
.background-render-settings {
  .settings-section {
    padding: 0;
    margin-bottom: 0;

    h3 {
      margin: 0 0 1rem;
      font-size: 1.125rem;
      font-weight: 600;
      color: var(--settings-text-primary);
    }
  }

  .setting-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.875rem 1rem;
    border: 1px solid var(--settings-feature-border);
    background: var(--settings-feature-bg);
    border-radius: 0.5rem;
    margin-top: 0.75rem;

    .item-info {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
      flex: 1;
      margin-right: 1rem;

      .item-title {
        font-weight: 600;
        color: var(--settings-text-primary);
        font-size: 0.95rem;
        line-height: 1.2;
      }

      .item-desc {
        color: var(--settings-text-secondary);
        font-size: 0.8rem;
        line-height: 1.2;
      }
    }
  }

  .performance-warning {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: var(--td-warning-color-1);
    border: 1px solid var(--td-warning-color-3);
    border-radius: 0.5rem;
    color: var(--td-warning-color-6);
    font-size: 0.875rem;

    .t-icon {
      flex-shrink: 0;
    }
  }

  .tag-low,
  .tag-medium,
  .tag-high {
    display: inline-block;
    padding: 0.125rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-weight: 600;
    margin-left: 0.5rem;
  }

  .tag-low {
    background: var(--td-success-color-1);
    color: var(--td-success-color-6);
  }

  .tag-medium {
    background: var(--td-warning-color-1);
    color: var(--td-warning-color-6);
  }

  .tag-high {
    background: var(--td-error-color-1);
    color: var(--td-error-color-6);
  }
}
</style>
