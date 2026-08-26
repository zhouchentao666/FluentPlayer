<script setup lang="ts">
import HotkeySettings from '@/components/Settings/HotkeySettings.vue'

const { t } = useI18n()
</script>

<template>
  <div class="settings-section">
    <div id="hotkey-settings" class="setting-group">
      <h3>{{ t('settings.hotkey.title') }}</h3>
      <p>{{ t('settings.hotkey.description') }}</p>
      <HotkeySettings />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.settings-section { animation: fadeInUp 0.4s ease-out; animation-fill-mode: both; }
.setting-group {
  background: var(--settings-group-bg, var(--td-bg-color-container));
  border-radius: 0.75rem; padding: 1.5rem; margin-bottom: 1.5rem;
  border: 1px solid var(--settings-group-border, var(--td-border-level-1-color));
  box-shadow: 0 1px 3px var(--settings-group-shadow);
  animation: fadeInUp 0.4s ease-out; animation-fill-mode: both;
  h3 { margin: 0 0 0.5rem; font-size: 1.125rem; font-weight: 600; color: var(--td-text-color-primary); }
  > p { margin: 0 0 1.5rem; color: var(--td-text-color-secondary); font-size: 0.875rem; }
}
@keyframes fadeInUp { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }
</style>
