<script setup lang="ts">
import { useSettingsStore } from '@/store/Settings'

const { t } = useI18n()
const settingsStore = useSettingsStore()
</script>

<template>
  <div class="ai-floatball-settings">
    <div class="setting-item">
      <div class="setting-info">
        <div class="setting-title">{{ t('settings.ai.showFloatBall') }}</div>
        <div class="setting-desc">{{ t('settings.ai.showFloatBallDesc') }}</div>
      </div>
      <t-switch
        :value="settingsStore.settings.showFloatBall"
        @change="(val: any) => settingsStore.updateSettings({ showFloatBall: Boolean(val) })"
      />
    </div>
  </div>
</template>

<style scoped>
.ai-floatball-settings {
  padding: 0.5rem 0;
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

.setting-title {
  font-weight: 600;
  color: var(--td-text-color-primary);
  font-size: 0.95rem;
}

.setting-desc {
  color: var(--td-text-color-secondary);
  font-size: 0.8rem;
}
</style>
