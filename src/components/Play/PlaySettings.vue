<script lang="ts" setup>
import { computed } from 'vue'
import { playSetting as usePlaySettingStore } from '@/store/playSetting'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { storeToRefs } from 'pinia'

const { t } = useI18n()

const playSetting = usePlaySettingStore()
const globalPlayStatus = useGlobalPlayStatusStore()
const { player } = storeToRefs(globalPlayStatus)

const lightMainColor = computed(() => {
  return player.value.coverDetail.lightMainColor || 'rgba(255, 255, 255, 0.9)'
})

const seamlessModeOptions = computed(() => [
  { label: t('play.gaplessMode'), value: 'gapless' },
  { label: t('play.crossfadeMode'), value: 'crossfade' },
])

const settingSections = computed(() => [
  {
    id: 'ui',
    title: t('play.uiSettings'),
    items: [
      {
        label: t('play.showLeftPanel'),
        value: playSetting.getShowLeftPanel,
        update: (val: boolean) => playSetting.setShowLeftPanel(val)
      },
      {
        label: t('play.immersiveLyricColor'),
        value: playSetting.getIsImmersiveLyricColor,
        update: (val: boolean) => playSetting.setIsImmersiveLyricColor(val)
      },
      {
        label: t('play.lyricBlurEffect'),
        value: playSetting.getIsBlurLyric,
        update: (val: boolean) => playSetting.setIsBlurLyric(val)
      },
      {
        label: t('play.audioVisualization'),
        value: playSetting.getIsAudioVisualizer,
        update: (val: boolean) => playSetting.setIsAudioVisualizer(val)
      },
      {
        label: t('play.autoHideControlBar'),
        value: playSetting.getAutoHideBottom,
        update: (val: boolean) => playSetting.setAutoHideBottom(val)
      }
    ]
  },
  {
    id: 'playback',
    title: t('play.playbackSettings'),
    items: [
      {
        label: t('play.pauseTransition'),
        value: playSetting.getIsPauseTransition,
        update: (val: boolean) => playSetting.setIsPauseTransition(val)
      },
    ]
  },
  {
    id: 'lyrics',
    title: t('play.lyricSettings'),
    items: [
      {
        label: t('play.filterLyricSongInfo'),
        value: playSetting.getIsGrepLyricInfo,
        update: (val: boolean) => playSetting.setIsGrepLyricInfo(val)
      },
      {
        label: t('play.strictFilterMode'),
        value: playSetting.getStrictGrep,
        update: (val: boolean) => playSetting.setStrictGrep(val)
      }
    ]
  }
])
</script>

<template>
  <div class="container">
    <div class="panel-header">{{ t('play.playerStyle') }}</div>
    <div class="style-cards">
      <div
        class="style-card"
        :class="{ active: playSetting.getLayoutMode === 'cd' }"
        @click="playSetting.setLayoutMode('cd')"
      >
        <div class="card-preview cd-preview">
          <img src="../../assets/images/cd.png" shape="circle" class="cover" width="100%" />
        </div>
        <span>{{ t('play.classicVinyl') }}</span>
      </div>
      <div
        class="style-card"
        :class="{ active: playSetting.getLayoutMode === 'cover' }"
        @click="playSetting.setLayoutMode('cover')"
      >
        <div class="card-preview cover-preview">
          <img src="../../assets/images/cover-play.png" shape="circle" class="cover" width="100%" />
        </div>
        <span>{{ t('play.immersiveCover') }}</span>
      </div>
    </div>

    <template v-for="section in settingSections" :key="section.title">
      <div class="panel-header" style="margin-top: 24px">{{ section.title }}</div>
      <template v-if="section.id === 'playback'">
        <div v-for="item in section.items" :key="item.label" class="control-row">
          <span>{{ item.label }}</span>
          <t-switch :value="item.value" @update:value="item.update" />
        </div>
        <!-- 无缝换曲 -->
        <div class="control-row">
          <span>{{ t('play.seamlessSwitch') }}</span>
          <t-switch :value="playSetting.getIsSeamlessTransition" @update:value="(v: boolean) => playSetting.setIsSeamlessTransition(v)" />
        </div>
        <template v-if="playSetting.getIsSeamlessTransition">
          <div class="control-row sub-control">
            <span>{{ t('play.transitionMode') }}</span>
            <t-radio-group
              variant="default-filled"
              :value="playSetting.getSeamlessMode"
              @change="(v: any) => playSetting.setSeamlessMode(v as 'gapless' | 'crossfade')"
              size="small"
            >
              <t-radio-button v-for="opt in seamlessModeOptions" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </t-radio-button>
            </t-radio-group>
          </div>
          <div v-if="playSetting.getSeamlessMode === 'crossfade'" class="control-row sub-control">
            <span>{{ t('play.transitionDuration', { duration: (playSetting.getCrossfadeDuration / 1000).toFixed(1) }) }}</span>
            <t-slider
              :value="playSetting.getCrossfadeDuration"
              :min="500"
              :max="8000"
              :step="500"
              :label="false"
              style="flex: 1; max-width: 160px; margin-left: 12px"
              @change="(v: any) => playSetting.setCrossfadeDuration(Number(v))"
            />
          </div>
        </template>
      </template>
      <template v-else>
        <div v-for="item in section.items" :key="item.label" class="control-row">
          <span>{{ item.label }}</span>
          <t-switch :value="item.value" @update:value="item.update" />
        </div>
      </template>
    </template>
  </div>
</template>

<style lang="scss" scoped>
.container {
  border-radius: 4px;
  flex: 1;
  height: 100%;
  box-sizing: border-box;
  overflow: auto;
  scrollbar-width: none;
}
.panel-header {
  color: rgba(255, 255, 255, 0.95);
  font-size: 18px;
  font-weight: 600;
  margin-bottom: 20px;
  letter-spacing: 0.5px;
}

.style-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.style-card {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 16px;
  padding: 16px;
  cursor: pointer;
  border: 2px solid transparent;
  transition: background-color 0.3s cubic-bezier(0.25, 0.8, 0.25, 1), border-color 0.3s cubic-bezier(0.25, 0.8, 0.25, 1), color 0.3s cubic-bezier(0.25, 0.8, 0.25, 1), box-shadow 0.3s cubic-bezier(0.25, 0.8, 0.25, 1), opacity 0.3s cubic-bezier(0.25, 0.8, 0.25, 1), transform 0.3s cubic-bezier(0.25, 0.8, 0.25, 1);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;

  &:hover {
    background: rgba(255, 255, 255, 0.1);
    transform: translateY(-2px);
  }

  &.active {
    background: rgba(255, 255, 255, 0.15);
    border-color: v-bind(lightMainColor);
    box-shadow: 0 8px 20px -5px rgba(0, 0, 0, 0.3);
  }

  .card-preview {
    width: 100%;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 10px;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: content-box;
    &.cd-preview {
      padding: 10px;
    }

    &.cover-preview {
      padding: 10px;
    }
  }

  span {
    font-size: 14px;
    color: rgba(255, 255, 255, 0.8);
    font-weight: 500;
  }
}

.control-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 8px;

  span {
    color: rgba(241, 241, 241, 0.8);
    font-size: 14px;
    font-weight: 500;
  }

  &.sub-control {
    padding-left: 20px;
    margin-top: 2px;
  }
}
</style>
