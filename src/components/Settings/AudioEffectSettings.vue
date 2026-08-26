<template>
  <div class="audio-effects-settings">
    <div class="settings-inline-header">
      <t-button theme="default" variant="text" @click="resetAll">{{ t('settings.audioEffect.resetAll') }}</t-button>
    </div>

    <div class="effects-grid">
        <!-- Surround Sound -->
        <div class="effect-card">
          <div class="card-header">
            <div class="card-title-group">
              <div class="title">{{ t('settings.audioEffect.surround') }}</div>
              <div class="mobile-card-status">
                {{ surround.enabled ? formatSurroundMode(surround.mode) : t('settings.equalizer.off') }}
              </div>
            </div>
            <t-switch v-model="surround.enabled" />
          </div>
          <div class="card-content">
            <div class="control-group">
              <label>{{ t('settings.audioEffect.surroundSimulation') }}</label>
              <t-radio-group
                v-model="surround.mode"
                class="effect-radio-group"
                :class="{ 'is-effect-disabled': !surround.enabled }"
                variant="default-filled"
                :disabled="!surround.enabled"
              >
                <t-radio-button value="off">{{ t('settings.audioEffect.surroundOff') }}</t-radio-button>
                <t-radio-button value="small">{{ t('settings.audioEffect.surroundSmall') }}</t-radio-button>
                <t-radio-button value="medium">{{ t('settings.audioEffect.surroundMedium') }}</t-radio-button>
                <t-radio-button value="large">{{ t('settings.audioEffect.surroundLarge') }}</t-radio-button>
              </t-radio-group>
            </div>
            <div class="info-text">{{ t('settings.audioEffect.surroundInfo') }}</div>
          </div>
        </div>

        <!-- Channel Balance -->
        <div class="effect-card">
          <div class="card-header">
            <div class="card-title-group">
              <div class="title">{{ t('settings.audioEffect.balance') }}</div>
              <div class="mobile-card-status">
                {{ balance.enabled ? formatBalance(balance.value) : t('settings.equalizer.off') }}
              </div>
            </div>
            <t-switch v-model="balance.enabled" />
          </div>
          <div class="card-content">
            <div class="control-group">
              <div class="balance-labels">
                <span>Left</span>
                <span>Right</span>
              </div>
              <t-slider
                v-model="balance.value"
                :min="-1"
                :max="1"
                :step="0.05"
                :disabled="!balance.enabled"
                :tooltip-format="(val: number) => formatBalance(val)"
              />
            </div>
            <div class="visual-balance">
              <div class="channel-badge left" :class="{ active: balance.value <= 0 }">
                L
              </div>
              <div class="balance-center">Center</div>
              <div class="channel-badge right" :class="{ active: balance.value >= 0 }">
                R
              </div>
            </div>
            <div class="balance-actions">
              <t-button
                size="small"
                variant="outline"
                :disabled="!balance.enabled"
                @click="balance.value = 0"
                >{{ t('settings.audioEffect.centerCalibration') }}</t-button
              >
            </div>
          </div>
        </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { watch, onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import { useAudioEffectsStore, type AudioEffectsState } from '@/store/AudioEffects'
import { invoke } from '@tauri-apps/api/core'

const { t } = useI18n()
const store = useAudioEffectsStore()
const { surround, balance } = storeToRefs(store)

const applyEffects = () => {
  invoke('player__set_balance', { value: balance.value.enabled ? balance.value.value : 0 })
}

watch(
  [() => ({ ...surround.value }), () => ({ ...balance.value })],
  () => { applyEffects() }
)

const resetAll = () => { store.resetEffects() }

type SurroundMode = AudioEffectsState['surround']['mode']

const formatSurroundMode = (mode: SurroundMode) => {
  const modeMap: Record<SurroundMode, string> = {
    off: t('settings.audioEffect.surroundOff'),
    small: t('settings.audioEffect.surroundSmall'),
    medium: t('settings.audioEffect.surroundMedium'),
    large: t('settings.audioEffect.surroundLarge')
  }
  return modeMap[mode]
}

const formatBalance = (val: number) => {
  if (val === 0) return t('settings.audioEffect.balanceCenter')
  return val < 0
    ? t('settings.audioEffect.balanceLeft', { percent: Math.abs(val * 100).toFixed(0) })
    : t('settings.audioEffect.balanceRight', { percent: Math.abs(val * 100).toFixed(0) })
}

onMounted(() => { applyEffects() })
</script>

<style scoped>
.audio-effects-settings {
  color: var(--td-text-color-primary);
}
.settings-inline-header {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 14px;
}
.effects-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  max-width: 900px;
}
.effect-card {
  background: var(--td-bg-color-container);
  border: 1px solid var(--td-component-border);
  border-radius: 8px;
  padding: 16px;
  transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease, box-shadow 0.3s ease, opacity 0.3s ease, transform 0.3s ease;
}
.effect-card:hover {
  border-color: var(--td-brand-color);
  box-shadow: var(--theme-shadow-light);
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--td-component-stroke);
  padding-bottom: 8px;
}
.card-title-group {
  min-width: 0;
}
.title {
  font-weight: 600;
  font-size: 16px;
  color: var(--td-text-color-primary);
}
.mobile-card-status {
  display: none;
}
.card-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.control-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.control-group label {
  font-size: 14px;
  color: var(--td-text-color-secondary);
}
.control-group :deep(.t-radio-group) {
  background: var(--td-bg-color-secondarycontainer);
  border: 1px solid var(--td-component-border);
}
.control-group :deep(.effect-radio-group) {
  overflow: hidden;
  border-radius: 6px;
}
.control-group :deep(.t-radio-group--filled .t-radio-button) {
  color: var(--td-text-color-secondary);
}
.control-group :deep(.t-radio-group--filled .t-radio-button:hover),
.control-group :deep(.t-radio-group--filled .t-radio-button.t-is-checked),
.control-group :deep(.t-radio-group--filled .t-radio-button.t-is-checked .t-radio-button__label),
.control-group :deep(.t-radio-group--filled .t-radio-button--checked),
.control-group :deep(.t-radio-group--filled .t-radio-button--checked .t-radio-button__label) {
  color: var(--settings-nav-label-active, var(--td-text-color-primary));
}
.control-group :deep(.t-radio-group--filled .t-radio-group__bg-block) {
  background: var(--settings-nav-active-bg, var(--td-bg-color-component-active));
  border: 1px solid var(--settings-nav-active-border, var(--td-brand-color));
  box-shadow: var(--settings-nav-active-shadow, none);
}
.control-group :deep(.effect-radio-group.is-effect-disabled) {
  background: var(--td-bg-color-secondarycontainer);
  border-color: var(--td-component-border);
}
.control-group :deep(.effect-radio-group.is-effect-disabled .t-radio-button),
.control-group :deep(.effect-radio-group.is-effect-disabled .t-radio-button__label) {
  color: var(--td-text-color-secondary) !important;
  opacity: 0.72;
}
.control-group :deep(.effect-radio-group.is-effect-disabled .t-radio-group__bg-block) {
  background: var(--td-bg-color-component) !important;
  border: 1px solid var(--td-component-border) !important;
  box-shadow: none !important;
}
.control-group :deep(.effect-radio-group.is-effect-disabled .t-radio-button.t-is-checked),
.control-group :deep(.effect-radio-group.is-effect-disabled .t-radio-button--checked),
.control-group :deep(.effect-radio-group.is-effect-disabled .t-radio-button.t-is-checked .t-radio-button__label),
.control-group :deep(.effect-radio-group.is-effect-disabled .t-radio-button--checked .t-radio-button__label) {
  color: var(--td-text-color-primary) !important;
  opacity: 0.76;
}
.info-text {
  font-size: 12px;
  color: var(--td-text-color-secondary);
  text-align: center;
}
.balance-labels {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--td-text-color-secondary);
}
.visual-balance {
  display: flex;
  justify-content: space-around;
  align-items: center;
  margin-top: 10px;
  background: var(--td-bg-color-secondarycontainer);
  padding: 12px 16px;
  border-radius: 8px;
}
.channel-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  font-size: 13px;
  font-weight: 600;
  color: var(--td-text-color-secondary);
  background: var(--td-bg-color-container);
  border: 1px solid var(--td-component-border);
  transition: opacity 0.3s ease, color 0.3s ease, border-color 0.3s ease;
  opacity: 0.4;
}
.channel-badge.active {
  opacity: 1;
  color: var(--td-brand-color);
  border-color: var(--td-brand-color);
}
.balance-center {
  font-size: 12px;
  color: var(--td-text-color-placeholder);
  letter-spacing: 0.5px;
}
.balance-actions {
  margin-top: 10px;
  text-align: center;
}

@media (max-width: 768px) {
  .audio-effects-settings {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .settings-inline-header {
    justify-content: stretch;
    margin-bottom: 0;
  }

  .settings-inline-header :deep(.t-button) {
    width: 100%;
    min-height: var(--mobile-touch-target);
    border: 1px solid var(--mobile-glass-border);
    border-radius: var(--mobile-control-radius);
    background: color-mix(in srgb, var(--td-bg-color-container) 76%, transparent);
    touch-action: manipulation;
  }

  .effects-grid {
    display: flex;
    max-width: none;
    flex-direction: column;
    gap: 12px;
  }

  .effect-card {
    padding: 14px;
    border-color: var(--mobile-glass-border);
    border-radius: var(--mobile-card-radius-small);
    background: color-mix(in srgb, var(--td-bg-color-container) 82%, transparent);
  }

  .effect-card:hover {
    border-color: var(--mobile-glass-border);
    box-shadow: none;
  }

  .card-header {
    align-items: center;
    gap: 14px;
    min-height: var(--mobile-touch-target);
    margin-bottom: 14px;
    padding-bottom: 12px;
  }

  .card-title-group {
    flex: 1;
  }

  .title {
    overflow-wrap: anywhere;
    font-size: 15px;
    line-height: 1.35;
  }

  .mobile-card-status {
    display: block;
    margin-top: 4px;
    color: var(--td-text-color-secondary);
    font-size: 12px;
    line-height: 1.35;
  }

  .card-content {
    gap: 14px;
  }

  .control-group {
    gap: 10px;
  }

  .control-group label {
    font-size: 13px;
    line-height: 1.35;
  }

  .control-group :deep(.effect-radio-group) {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
    padding: 4px;
    border-radius: 14px;
  }

  .control-group :deep(.effect-radio-group .t-radio-button) {
    min-width: 0;
    min-height: var(--mobile-touch-target);
    border-radius: 12px;
    text-align: center;
  }

  .control-group :deep(.effect-radio-group .t-radio-button__label) {
    white-space: normal;
  }

  .control-group :deep(.effect-radio-group .t-radio-group__bg-block) {
    display: none;
  }

  .control-group :deep(.effect-radio-group .t-radio-button.t-is-checked),
  .control-group :deep(.effect-radio-group .t-radio-button--checked) {
    background: var(--settings-nav-active-bg, var(--td-bg-color-component-active));
    box-shadow: inset 0 0 0 1px var(--settings-nav-active-border, var(--td-brand-color));
  }

  .info-text {
    padding: 10px 12px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--td-bg-color-secondarycontainer) 82%, transparent);
    line-height: 1.45;
    text-align: left;
  }

  .balance-labels {
    font-size: 12px;
    line-height: 1.35;
  }

  .control-group :deep(.t-slider) {
    padding: 10px 2px;
  }

  .visual-balance {
    justify-content: space-between;
    margin-top: 0;
    padding: 14px;
    border: 1px solid var(--mobile-glass-border);
    border-radius: 14px;
    background: color-mix(in srgb, var(--td-bg-color-secondarycontainer) 86%, transparent);
  }

  .channel-badge {
    width: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    font-size: 14px;
  }

  .balance-center {
    font-size: 13px;
  }

  .balance-actions {
    margin-top: 0;
  }

  .balance-actions :deep(.t-button) {
    width: 100%;
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }
}
</style>
