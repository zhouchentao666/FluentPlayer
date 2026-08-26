<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { playSetting as usePlaySetting } from '@/store/playSetting'
import { useSettingsStore } from '@/store/Settings'
import PlaylistSettings from '@/components/Settings/PlaylistSettings.vue'
import AudioOutputSettings from '@/components/Settings/AudioOutputSettings.vue'
import DlnaDeviceSettings from '@/components/Settings/DlnaDeviceSettings.vue'
import BackgroundRenderSettings from '@/components/Settings/BackgroundRenderSettings.vue'

const { t } = useI18n()

const playSettingStore = usePlaySetting()
const { isJumpLyric, bgPlaying, isAudioVisualizer } = storeToRefs(playSettingStore)
const settingsStore = useSettingsStore()
const { settings } = storeToRefs(settingsStore)
</script>

<template>
  <div class="settings-section">
    <div id="playback-playlist" class="setting-group">
      <h3>{{ t('settings.playback.playlistManagement') }}</h3>
      <PlaylistSettings />
    </div>

    <div id="playback-audio-output" class="setting-group">
      <h3>{{ t('settings.playback.audioOutput') }}</h3>
      <AudioOutputSettings embedded />
    </div>

    <div id="playback-dlna" class="setting-group">
      <DlnaDeviceSettings />
    </div>

    <!-- 背景效果 -->
    <div id="playback-background" class="setting-group">
      <BackgroundRenderSettings />
    </div>

    <!-- 播放显示 -->
    <div id="playback-performance" class="setting-group">
      <h3>{{ t('settings.playback.fullscreenPerformance') }}</h3>

      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.playback.jumpLyric') }}</div>
          <div class="item-desc">{{ t('settings.playback.jumpLyricDesc') }}</div>
        </div>
        <t-switch v-model="isJumpLyric" @change="playSettingStore.setIsDumpLyric(isJumpLyric)" />
      </div>

      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.playback.bgAnimation') }}</div>
          <div class="item-desc">{{ t('settings.playback.bgAnimationDesc') }}</div>
        </div>
        <t-switch v-model="bgPlaying" @change="playSettingStore.setBgPlaying(bgPlaying)" />
      </div>

      <div class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.playback.audioVisualizer') }}</div>
          <div class="item-desc">{{ t('settings.playback.audioVisualizerDesc') }}</div>
        </div>
        <t-switch
          v-model="isAudioVisualizer"
          @change="playSettingStore.setIsAudioVisualizer(isAudioVisualizer)"
        />
      </div>

      <div id="playback-route-preload" class="setting-item">
        <div class="item-info">
          <div class="item-title">{{ t('settings.playback.routePreload') }}</div>
          <div class="item-desc">{{ t('settings.playback.routePreloadDesc') }}</div>
        </div>
        <t-switch
          :value="settings.routePreloadEnabled"
          @change="(val: unknown) => settingsStore.updateSettings({ routePreloadEnabled: Boolean(val) })"
        />
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
  background: var(--settings-group-bg);
  border-radius: 0.75rem;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
  border: 1px solid var(--settings-group-border);
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

  .setting-item {
    align-items: flex-start;
    flex-direction: column;
    gap: 10px;
    padding: 10px 12px;

    .item-info {
      width: 100%;
    }
  }
}
</style>
