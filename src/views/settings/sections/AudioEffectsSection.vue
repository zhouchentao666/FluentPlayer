<script setup lang="ts">
import EqualizerSettings from '@/components/Settings/EqualizerSettings.vue'
import AudioEffectSettings from '@/components/Settings/AudioEffectSettings.vue'

const { t } = useI18n()
</script>

<template>
  <div class="settings-section">
    <div id="effects-equalizer" class="setting-group">
      <h3>{{ t('settings.effects.equalizer') }}</h3>
      <EqualizerSettings />
    </div>

    <div id="effects-audio-effect" class="setting-group">
      <h3>{{ t('settings.effects.audioEffect') }}</h3>
      <AudioEffectSettings />
    </div>
  </div>
</template>

<style lang="scss" scoped>
.settings-section {
  animation: fadeInUp 0.4s ease-out;
  animation-fill-mode: both;
}

.setting-group {
  background: var(--settings-group-bg);
  border-radius: 0.75rem;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
  border: 1px solid var(--settings-group-border);
  box-shadow: 0 1px 3px var(--settings-group-shadow);
  animation: fadeInUp 0.4s ease-out;
  animation-fill-mode: both;

  @for $i from 1 through 2 {
    &:nth-child(#{$i}) {
      animation-delay: #{$i * 0.1}s;
    }
  }

  h3 {
    margin: 0 0 0.5rem;
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--settings-text-primary);
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
  }
}
</style>
