<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { MessagePlugin } from 'tdesign-vue-next'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { musicSdk } from '@/services/musicSdk'
import { ControlAudioStore } from '@/store/ControlAudio'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { TreeRoundDotIcon } from 'tdesign-icons-vue-next'
import fonts from '@/assets/icon_font/icons'

const { t } = useI18n()

const emit = defineEmits(['switch-category'])

const QUALITY_ORDER: Record<string, number> = {
  '128k': 1, '320k': 2,
  flac: 3, flac24bit: 4, hires: 5, atmos: 6, master: 7,
}

const sortQualities = (qualities: string[]): string[] =>
  [...qualities].sort((a, b) => (QUALITY_ORDER[a] ?? 999) - (QUALITY_ORDER[b] ?? 999))

const userStore = LocalUserDetailStore()
const { userInfo } = storeToRefs(userStore)

const subsonicTesting = ref(false)

const subsonicConfig = computed({
  get: () => {
    if (!userInfo.value.subsonicConfig) {
      userInfo.value.subsonicConfig = {
        baseUrl: '',
        username: '',
        password: '',
        apiVersion: '1.16.1',
        clientName: 'Mio',
        enabled: false,
      }
    }
    return userInfo.value.subsonicConfig
  },
  set: (value) => {
    userInfo.value.subsonicConfig = value
  }
})

const syncSubsonicSource = async () => {
  const wasSubsonicSource = userInfo.value.selectSources === 'subsonic'
  userStore.ensureBuiltInSources(userInfo.value)
  if (!userInfo.value.sourceQualityMap) userInfo.value.sourceQualityMap = {}
  if (userStore.hasValidSubsonicConfig(userInfo.value)) {
    if (!userInfo.value.sourceQualityMap.subsonic) userInfo.value.sourceQualityMap.subsonic = 'flac'
    userInfo.value.selectSources = 'subsonic'
    userInfo.value.selectQuality = userInfo.value.sourceQualityMap.subsonic
  } else if (wasSubsonicSource) {
    // Subsonic disabled while it was the active source — pause and clean up
    const globalPlayStatus = useGlobalPlayStatusStore()
    const playingSource = (globalPlayStatus.player.songInfo as any)?.source
    if (playingSource === 'subsonic') {
      const audio = ControlAudioStore()
      await audio.stop()
    }
    // Remove subsonic songs from queue
    const newList = userStore.list.filter(song => (song as any).source !== 'subsonic')
    userStore.replaceSongList(newList)
    if (newList.length > 0) {
      const lastId = userStore.userInfo.lastPlaySongId
      if (lastId && !newList.find(s => s.songmid === lastId)) {
        userStore.userInfo.lastPlaySongId = newList[0].songmid
      }
    } else {
      userStore.userInfo.lastPlaySongId = null
    }
    const nextSource = Object.keys(userInfo.value.supportedSources || {})[0]
    userInfo.value.selectSources = nextSource
    userInfo.value.selectQuality = nextSource
      ? userInfo.value.supportedSources?.[nextSource]?.qualitys?.slice(-1)[0] || ''
      : ''
  }
}

const testSubsonicConnection = async () => {
  syncSubsonicSource()
  subsonicTesting.value = true
  try {
    const result = await musicSdk.request('ping', { source: 'subsonic' })
    if (result?.success) {
      subsonicConfig.value.enabled = true
      syncSubsonicSource()
      MessagePlugin.success(result.message || t('settings.musicSource.subsonicConnectSuccess'))
    } else {
      MessagePlugin.warning(result?.message || t('settings.musicSource.subsonicConnectFailed'))
    }
  } catch (error: any) {
    MessagePlugin.error(error?.message || String(error) || t('settings.musicSource.subsonicConnectFailed'))
  } finally {
    subsonicTesting.value = false
  }
}

watch(() => ({ ...userInfo.value.subsonicConfig }), syncSubsonicSource)

const hasPluginData = computed(() => {
  return !!(userInfo.value.supportedSources && Object.keys(userInfo.value.supportedSources).length > 0)
})

const currentPluginName = computed(() => {
  if (!userInfo.value.pluginId) return userInfo.value.supportedSources?.subsonic ? t('settings.musicSource.builtInSource') : ''
  return userInfo.value.pluginName || userInfo.value.pluginId || t('settings.musicSource.unknownPlugin')
})

const currentSourceQualities = computed(() => {
  if (!hasPluginData.value || !userInfo.value.selectSources) return []
  const selectedSource = userInfo.value.supportedSources?.[userInfo.value.selectSources]
  return sortQualities(selectedSource?.qualitys || [])
})

const globalQualityOptions = computed(() => {
  const sources = userInfo.value.supportedSources || {}
  const keys = Object.keys(sources)
  if (keys.length === 0) return []
  const arrays = keys.map((k) => sources[k].qualitys || [])
  const set = new Set(arrays[0])
  for (let i = 1; i < arrays.length; i++) {
    for (const q of Array.from(set)) {
      if (!arrays[i].includes(q)) set.delete(q)
    }
  }
  return sortQualities(Array.from(set))
})

const globalQualitySelected = ref<string>('')

watch(() => globalQualityOptions.value, (opts) => {
  if (!opts || opts.length === 0) { globalQualitySelected.value = ''; return }
  if (!opts.includes(globalQualitySelected.value)) { globalQualitySelected.value = opts[opts.length - 1] }
}, { immediate: true })

const getSourceQuality = (sourceKey: string) => {
  return userInfo.value.sourceQualityMap?.[sourceKey]
    || (userInfo.value.selectSources === sourceKey ? userInfo.value.selectQuality : undefined)
}

const selectQualityTag = (quality: string) => {
  userInfo.value.selectQuality = quality
  if (!userInfo.value.sourceQualityMap) userInfo.value.sourceQualityMap = {}
  const key = userInfo.value.selectSources as string
  userInfo.value.sourceQualityMap[key] = quality
}

const applyGlobalQuality = (q: string) => {
  if (!q) return
  if (!userInfo.value.sourceQualityMap) userInfo.value.sourceQualityMap = {}
  const sources = userInfo.value.supportedSources || {}
  Object.keys(sources).forEach((key) => {
    const arr = sources[key].qualitys || []
    if (arr.includes(q)) {
      userInfo.value.sourceQualityMap![key] = q
    }
  })
  const currentKey = userInfo.value.selectSources as string
  const arr = sources[currentKey]?.qualitys || []
  if (arr.includes(q)) userInfo.value.selectQuality = q
}

const selectSource = (sourceKey: string) => {
  if (!hasPluginData.value) return
  userInfo.value.selectSources = sourceKey
  const source = userInfo.value.supportedSources?.[sourceKey]
  if (!userInfo.value.sourceQualityMap) userInfo.value.sourceQualityMap = {}
  if (source && source.qualitys && source.qualitys.length > 0) {
    const saved = userInfo.value.sourceQualityMap[sourceKey]
    const useQuality = saved && source.qualitys.includes(saved) ? saved : source.qualitys[source.qualitys.length - 1]
    userInfo.value.sourceQualityMap[sourceKey] = useQuality
    userInfo.value.selectQuality = useQuality
  }
}

const getQualityDisplayName = (quality: string) => {
  const qualityMap: Record<string, string> = {
    '128k': t('settings.musicSource.quality128k'),
    '320k': t('settings.musicSource.quality320k'),
    flac: t('settings.musicSource.qualityFlac'),
    flac24bit: t('settings.musicSource.qualityFlac24bit'),
    hires: t('settings.musicSource.qualityHires'),
    atmos: t('settings.musicSource.qualityAtmos'),
    master: t('settings.musicSource.qualityMaster')
  }
  return qualityMap[quality] || quality
}

const getQualityDescription = (quality: string) => {
  const descriptions: Record<string, string> = {
    '128k': t('settings.musicSource.quality128kDesc'),
    '320k': t('settings.musicSource.quality320kDesc'),
    flac: t('settings.musicSource.qualityFlacDesc'),
    flac24bit: t('settings.musicSource.qualityFlac24bitDesc'),
    hires: t('settings.musicSource.qualityHiresDesc'),
    atmos: t('settings.musicSource.qualityAtmosDesc'),
    master: t('settings.musicSource.qualityMasterDesc')
  }
  return descriptions[quality] || t('settings.musicSource.qualityCustomDesc')
}

const getCurrentSourceName = () => {
  if (!hasPluginData.value || !userInfo.value.selectSources) return t('settings.musicSource.noSelection')
  const source = userInfo.value.supportedSources?.[userInfo.value.selectSources]
  return source?.name || userInfo.value.selectSources
}

const goPlugin = () => { emit('switch-category', 'plugins') }

const getSourceIcon = (key: string) => {
  return fonts[key] || null
}
</script>

<template>
  <div class="settings-section">
    <div class="setting-group subsonic-config">
      <h3>{{ t('settings.musicSource.subsonicTitle') }}</h3>
      <div class="subsonic-form">
        <div class="subsonic-switch-row">
          <span>{{ t('settings.musicSource.subsonicEnable') }}</span>
          <t-switch v-model="subsonicConfig.enabled" @change="syncSubsonicSource" />
        </div>
        <t-input v-model="subsonicConfig.baseUrl" :label="t('settings.musicSource.subsonicServer')" placeholder="https://navidrome.example.com" clearable />
        <t-input v-model="subsonicConfig.username" :label="t('settings.musicSource.subsonicUsername')" :placeholder="t('settings.musicSource.subsonicUsername')" clearable />
        <t-input v-model="subsonicConfig.password" :label="t('settings.musicSource.subsonicPassword')" type="password" :placeholder="t('settings.musicSource.subsonicPassword')" clearable />
        <div class="subsonic-inline">
          <t-input v-model="subsonicConfig.apiVersion" :label="t('settings.musicSource.subsonicApiVersion')" placeholder="1.16.1" />
          <t-input v-model="subsonicConfig.clientName" :label="t('settings.musicSource.subsonicClientName')" placeholder="Mio" />
        </div>
        <div class="subsonic-actions">
          <t-button theme="primary" :loading="subsonicTesting" @click="testSubsonicConnection">
            {{ t('settings.musicSource.subsonicTestConnection') }}
          </t-button>
          <span class="subsonic-hint">{{ t('settings.musicSource.subsonicHint') }}</span>
        </div>
      </div>
    </div>

    <div v-if="hasPluginData" class="music-config-container">
      <div class="setting-group">
        <div class="plugin-info">
          <span class="plugin-name">{{ t('settings.musicSource.currentConfig', { name: currentPluginName }) }}</span>
          <span class="plugin-status">{{ t('settings.musicSource.enabled') }}</span>
        </div>
      </div>

      <div id="music-source" class="setting-group">
        <h3>{{ t('settings.musicSource.musicSourceSelect') }}</h3>
        <div class="source-cards">
          <div
            v-for="(source, key) in userInfo.supportedSources"
            :key="key"
            class="source-card"
            :class="{ active: userInfo.selectSources === String(key) }"
            @click="selectSource(String(key))"
          >
            <div class="source-icon">
              <component :is="getSourceIcon(String(key))" v-if="getSourceIcon(String(key))" style="font-size: 2em" />
              <span v-else style="font-size: 1.5em">{{ source.name?.charAt(0) || '?' }}</span>
            </div>
            <div class="source-info">
              <div class="source-name">{{ source.name }}</div>
              <div class="source-type">
                <span>{{ source.type || t('settings.musicSource.sourceType') }}</span>
                <t-tag
                  v-if="getSourceQuality(String(key))"
                  size="small"
                  theme="primary"
                  variant="light"
                  class="source-quality-tag"
                >
                  {{ getQualityDisplayName(getSourceQuality(String(key))!) }}
                </t-tag>
              </div>
            </div>
            <div v-if="userInfo.selectSources === String(key)" class="source-check">
              <i class="iconfont icon-check" />
            </div>
          </div>
        </div>
      </div>

      <div v-if="currentSourceQualities.length > 0" id="music-quality" class="setting-group">
        <h3>{{ t('settings.musicSource.qualitySelect') }} <span class="quality-source-hint">— {{ getCurrentSourceName() }}</span></h3>
        <div class="quality-tags-container">
          <div
            v-for="quality in currentSourceQualities"
            :key="quality"
            class="quality-tag"
            :class="{ active: userInfo.selectQuality === quality }"
            @click="selectQualityTag(quality)"
          >
            <span class="quality-tag-name">{{ getQualityDisplayName(quality) }}</span>
          </div>
        </div>
        <div v-if="userInfo.selectQuality" class="quality-description">
          <p class="quality-hint">{{ getQualityDescription(userInfo.selectQuality) }}</p>
        </div>
      </div>

      <div v-if="globalQualityOptions.length > 0" class="setting-group">
        <h3>{{ t('settings.musicSource.globalQuality') }}</h3>
        <t-select v-model="globalQualitySelected" @change="(v: any) => applyGlobalQuality(String(v))">
          <t-option v-for="q in globalQualityOptions" :key="q" :value="q" :label="getQualityDisplayName(q)" />
        </t-select>
      </div>

      <div class="setting-group">
        <h3>{{ t('settings.musicSource.configStatus') }}</h3>
        <div class="config-status">
          <div class="status-item">
            <span class="status-label">{{ t('settings.musicSource.musicSourceLabel') }}:</span>
            <span class="status-value">{{ getCurrentSourceName() }}</span>
          </div>
          <div class="status-item">
            <span class="status-label">{{ t('settings.musicSource.qualityLabel') }}:</span>
            <span class="status-value">{{ getQualityDisplayName(userInfo.selectQuality || '') }}</span>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="plugin-prompt">
      <div class="prompt-icon">
        <TreeRoundDotIcon />
      </div>
      <div class="prompt-content">
        <h4>{{ t('settings.musicSource.noPluginTitle') }}</h4>
        <p>{{ t('settings.musicSource.noPluginDesc') }}</p>
        <t-button theme="primary" @click="goPlugin">
          <i class="iconfont icon-shezhi" />
          {{ t('settings.musicSource.goToPlugin') }}
        </t-button>
      </div>
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
  @for $i from 1 through 5 { &:nth-child(#{$i}) { animation-delay: #{$i * 0.1}s; } }
  .subsonic-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .subsonic-switch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    color: var(--settings-text-primary, var(--td-text-color-primary));
  }
  .subsonic-inline {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }
  .subsonic-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .subsonic-hint {
    color: var(--settings-text-secondary, var(--td-text-color-secondary));
    font-size: 0.8125rem;
  }
  h3 { margin: 0 0 0.5rem; font-size: 1.125rem; font-weight: 600; color: var(--settings-text-primary, var(--td-text-color-primary)); }
}
.music-config-container {
  .plugin-info {
    display: flex; align-items: center; gap: 1rem; padding: 1rem;
    background: linear-gradient(135deg, var(--td-brand-color-1) 0%, var(--td-brand-color-2) 100%);
    border-radius: 0.75rem; border: 1px solid var(--td-brand-color-3);
    .plugin-name { font-weight: 600; font-size: 1rem; color: var(--td-brand-color-6); }
    .plugin-status { background: var(--td-brand-color-5); color: var(--td-text-color-anti); padding: 0.25rem 0.75rem; border-radius: 1rem; font-size: 0.75rem; font-weight: 500; }
  }
  .source-cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 1rem; }
  .source-card {
    display: flex; align-items: center; gap: 1rem; padding: 1rem;
    background: var(--settings-source-card-bg, var(--td-bg-color-container));
    border: 2px solid var(--settings-source-card-border, var(--td-border-level-1-color));
    border-radius: 0.75rem; cursor: pointer; transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
    &:hover { border-color: var(--settings-source-card-hover-border, var(--td-brand-color-3)); box-shadow: var(--theme-shadow-light); }
    &.active { border-color: var(--settings-source-card-active-border, var(--td-brand-color)); background: var(--settings-source-card-active-bg, var(--td-brand-color-1)); box-shadow: 0 0 0 3px var(--td-brand-color-2); }
    .source-icon {
      width: 2.5rem; height: 2.5rem; background: var(--settings-source-icon-bg, var(--td-brand-color-1)); border-radius: 50%;
      display: flex; align-items: center; justify-content: center; color: var(--settings-text-secondary, var(--td-text-color-secondary));
    }
    .source-info { flex: 1;
      .source-name { font-weight: 600; font-size: 0.875rem; color: var(--settings-text-primary, var(--td-text-color-primary)); margin-bottom: 0.125rem; }
      .source-type {
        font-size: 0.75rem; color: var(--settings-text-secondary, var(--td-text-color-secondary));
        display: flex; align-items: center; gap: 0.375rem;
      }
    }
    .source-quality-tag {
      font-size: 0.625rem;
      line-height: 1;
      padding: 0 0.25rem;
      border-radius: 0.25rem;
    }
    .source-check { color: var(--td-brand-color-5); font-size: 1.125rem; }
  }
  .quality-source-hint {
    font-size: 0.875rem; font-weight: 400; color: var(--settings-text-secondary, var(--td-text-color-secondary));
  }
  .quality-tags-container {
    display: flex; flex-wrap: wrap; gap: 0.5rem; padding: 1rem 0;
  }
  .quality-tag {
    padding: 0.5rem 1rem; border-radius: 0.5rem; cursor: pointer;
    border: 1.5px solid var(--td-border-level-2-color);
    background: var(--td-bg-color-container);
    transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
    user-select: none;
    .quality-tag-name { font-size: 0.8125rem; font-weight: 500; color: var(--td-text-color-primary); }
    &:hover { border-color: var(--td-brand-color-3); box-shadow: var(--theme-shadow-light); }
    &.active {
      border-color: var(--td-brand-color); background: var(--td-brand-color-1);
      box-shadow: 0 0 0 2px var(--td-brand-color-2);
      .quality-tag-name { color: var(--td-brand-color); font-weight: 600; }
    }
  }
  .quality-description { text-align: center; margin-top: 1rem;
    p { margin: 0.5rem 0;
      &:first-child { font-size: 1rem; font-weight: 600; color: var(--settings-text-primary, var(--td-text-color-primary)); }
      &.quality-hint { font-size: 0.875rem; color: var(--settings-text-secondary, var(--td-text-color-secondary)); }
    }
  }
  .config-status { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;
    .status-item { display: flex; justify-content: space-between; align-items: center; padding: 1rem; background: var(--settings-status-item-bg, var(--td-bg-color-page)); border-radius: 0.5rem; border: 1px solid var(--settings-status-item-border, var(--td-border-level-1-color));
      .status-label { font-weight: 500; color: var(--settings-text-secondary, var(--td-text-color-secondary)); font-size: 0.875rem; }
      .status-value { font-weight: 600; color: var(--settings-text-primary, var(--td-text-color-primary)); font-size: 0.875rem; }
    }
  }
}
.plugin-prompt {
  display: flex; align-items: center; gap: 1.5rem; padding: 2rem;
  background: var(--settings-plugin-prompt-bg, var(--td-bg-color-container)); border-radius: 1rem; border: 2px dashed var(--settings-plugin-prompt-border, var(--td-border-level-1-color));
  .prompt-icon {
    width: 3rem; height: 3rem; background: linear-gradient(135deg, var(--td-brand-color-5) 0%, var(--td-brand-color-6) 100%);
    border-radius: 50%; display: flex; align-items: center; justify-content: center;
    flex-shrink: 0; color: var(--td-text-color-anti); font-size: 1.5rem;
  }
  .prompt-content {
    flex: 1;
    h4 { color: var(--settings-text-primary, var(--td-text-color-primary)); margin: 0 0 0.5rem 0; font-size: 1.125rem; font-weight: 600; }
    p { color: var(--settings-text-secondary, var(--td-text-color-secondary)); margin: 0 0 1.5rem 0; line-height: 1.5; }
  }
}
@media (max-width: 768px) {
  .music-config-container {
    .plugin-info {
      align-items: flex-start;
      flex-direction: column;
      gap: 8px;
      padding: 12px;
    }

    .subsonic-config {
      .subsonic-inline {
        grid-template-columns: 1fr;
        gap: 8px;
      }

      .subsonic-actions {
        align-items: flex-start;
        flex-direction: column;
        gap: 8px;
      }
    }

    .source-cards {
      grid-template-columns: 1fr;
      gap: 10px;
    }

    .source-card {
      gap: 10px;
      padding: 12px;
      border-width: 1px;

      .source-icon {
        width: 36px;
        height: 36px;
      }

      .source-info {
        min-width: 0;
      }

      .source-type {
        flex-wrap: wrap;
      }
    }

    .quality-tags-container {
      gap: 8px;
      padding: 10px 0;
    }

    .quality-tag {
      flex: 1 1 calc(50% - 4px);
      min-width: 0;
      padding: 10px 8px;
      text-align: center;
    }

    .quality-description {
      margin-top: 8px;
      text-align: left;
    }

    .config-status {
      grid-template-columns: 1fr;
      gap: 8px;

      .status-item {
        align-items: flex-start;
        flex-direction: column;
        gap: 4px;
        padding: 12px;
      }
    }
  }

  .plugin-prompt {
    flex-direction: column;
    text-align: center;
    gap: 12px;
    padding: 18px 14px;

    .prompt-icon {
      width: 44px;
      height: 44px;
      font-size: 20px;
    }

    .prompt-content p {
      margin-bottom: 12px;
      font-size: 13px;
    }
  }
}
@keyframes fadeInUp { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }
</style>
