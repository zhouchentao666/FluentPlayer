<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import LeaderBord from '@/components/Find/LeaderBord.vue'
import PlaylistCategory from '@/components/Find/PlaylistCategory.vue'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { usePluginStore } from '@/store/plugin'
import { useSourceAccess } from '@/composables/useSourceAccess'

const { t } = useI18n()
const router = useRouter()
const pluginStore = usePluginStore()
const localUserStore = LocalUserDetailStore()

const activeTab = ref<'songlist' | 'leaderboard'>('songlist')
const sourceStateLoading = ref(true)
let sourceStateRefreshPromise: Promise<void> | null = null
let sourceStateInitialized = false

const hasSubsonicConfig = computed(() => localUserStore.hasValidSubsonicConfig(localUserStore.userInfo))
const hasInstalledMusicSourcePlugin = computed(() => pluginStore.plugins.some(plugin => plugin.plugin_type === 'music-source'))
const hasEnabledSources = computed(() => {
  const sources = localUserStore.userInfo.supportedSources
  return !!sources && Object.keys(sources).length > 0
})
const isCheckingSourceState = computed(() => sourceStateLoading.value || pluginStore.loading)
const shouldShowSetupGuide = computed(() => (
  !isCheckingSourceState.value &&
  !hasEnabledSources.value
))

async function ensureSourceState() {
  if (sourceStateRefreshPromise) return sourceStateRefreshPromise
  if (sourceStateInitialized) return

  sourceStateLoading.value = true
  sourceStateRefreshPromise = pluginStore.initialize()
    .catch((e) => {
      console.warn('[FindView] 初始化音源状态失败:', e)
    })
    .finally(() => {
      const { validateCurrentSource } = useSourceAccess()
      validateCurrentSource()
      sourceStateLoading.value = false
      sourceStateRefreshPromise = null
      sourceStateInitialized = true
    })

  return sourceStateRefreshPromise
}

function goMusicSettings() {
  router.push({ path: '/settings', query: { category: 'music' } })
}

function goPluginSettings() {
  router.push({ path: '/settings', query: { category: 'plugins' } })
}

onMounted(() => {
  ensureSourceState()
})

onActivated(() => {
  if (sourceStateLoading.value) ensureSourceState()
})
</script>

<template>
  <div class="find-container" :aria-busy="isCheckingSourceState">
    <div v-show="!shouldShowSetupGuide" class="page-header">
      <h2>{{ t('music.find.title') }}</h2>
      <p>{{ t('music.find.subtitle') }}</p>
    </div>

    <div v-if="isCheckingSourceState" class="source-state source-loading" role="status" aria-live="polite">
      <t-loading size="large" :text="t('music.find.checkingSource')" />
    </div>

    <div v-else-if="shouldShowSetupGuide" class="source-state setup-state" aria-labelledby="source-setup-title">
      <section class="setup-guide">
        <div class="setup-orb" aria-hidden="true">
          <img class="setup-logo" src="/icon.png" alt="" />
        </div>
        <div class="setup-copy">
          <h3 id="source-setup-title">{{ t('music.find.configGuide') }}</h3>
          <p>{{ t('music.find.guideDescription') }}</p>
        </div>
        <div class="setup-actions">
          <div class="setup-action-card primary-card">
            <div>
              <h4>{{ t('music.find.configSubsonic') }}</h4>
              <p>{{ t('music.find.subsonicDesc') }}</p>
            </div>
            <t-button theme="primary" @click="goMusicSettings">{{ t('music.find.goToSourceSettings') }}</t-button>
          </div>
          <div class="setup-action-card">
            <div>
              <h4>{{ t('music.find.installPlugin') }}</h4>
              <p>{{ t('music.find.installPluginDesc') }}</p>
            </div>
            <t-button variant="outline" @click="goPluginSettings">{{ t('music.find.goToPluginManage') }}</t-button>
          </div>
        </div>
      </section>
    </div>

    <template v-else>
      <div class="segment-tabs">
        <button
          class="segment-tab"
          :class="{ active: activeTab === 'songlist' }"
          @click="activeTab = 'songlist'"
        >
          {{ t('music.find.tabPlaylist') }}
        </button>
        <button
          class="segment-tab"
          :class="{ active: activeTab === 'leaderboard' }"
          @click="activeTab = 'leaderboard'"
        >
          {{ t('music.find.tabLeaderboard') }}
        </button>
      </div>

      <div class="tab-content">
        <PlaylistCategory v-show="activeTab === 'songlist'" />
        <div v-show="activeTab === 'leaderboard'" class="leaderboard-pane">
          <LeaderBord />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.find-container {
  padding-top: 1rem;
  width: 100%;
  height: 100%;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.page-header {
  margin: 0 2rem 1rem;
}

.page-header h2 {
  border-left: 8px solid var(--td-brand-color-3);
  padding-left: 12px;
  border-radius: 8px;
  line-height: 1.5em;
  color: var(--td-text-color-primary);
  margin-bottom: 0.5rem;
  font-size: 1.875rem;
  font-weight: 600;
}

.page-header p {
  color: var(--td-text-color-secondary);
  font-size: 0.85rem;
}

.segment-tabs {
  display: flex;
  margin: 0 2rem 1rem;
  background: var(--td-bg-color-secondarycontainer);
  border-radius: 8px;
  padding: 3px;
  gap: 2px;
}

.segment-tab {
  flex: 1;
  padding: 6px 16px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--td-text-color-secondary);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.25s ease, border-color 0.25s ease, color 0.25s ease, box-shadow 0.25s ease, opacity 0.25s ease, transform 0.25s ease;
}

.segment-tab:hover {
  color: var(--td-text-color-primary);
}

.segment-tab.active {
  background: var(--td-bg-color-container);
  color: var(--td-brand-color);
  font-weight: 600;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.tab-content {
  flex: 1;
  overflow: hidden;
  position: relative;
}

.leaderboard-pane {
  height: 100%;
  overflow-y: auto;
}

.source-state {
  flex: 1;
  min-height: 0;
  margin: 0 2rem 1rem;
  display: flex;
  align-items: center;
  justify-content: center;
}

.source-loading {
  min-height: 320px;
  border: 1px solid var(--td-border-level-1-color);
  border-radius: 16px;
  background: var(--td-bg-color-container);
}

.setup-state {
  overflow-x: hidden;
  overflow-y: auto;
  padding: 24px 0;
  -webkit-overflow-scrolling: touch;
}

.setup-guide {
  width: min(880px, 100%);
  max-width: 100%;
  margin: auto;
  padding: clamp(24px, 4vw, 40px);
  display: flex;
  flex-direction: column;
  gap: clamp(16px, 3vw, 24px);
  text-align: center;
  border: 1px dashed var(--td-border-level-1-color);
  border-radius: 20px;
  background:
    radial-gradient(circle at top, color-mix(in srgb, var(--td-brand-color) 12%, transparent), transparent 42%),
    var(--td-bg-color-container);
}

.setup-orb {
  width: 72px;
  height: 72px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 22px;
  background: linear-gradient(135deg, var(--td-brand-color-1), var(--td-brand-color-3));
}

.setup-logo {
  width: 100%;
  height: 100%;
  border-radius: 22px;
  object-fit: cover;
  display: block;
}

.setup-copy h3 {
  margin: 0 0 10px;
  color: var(--td-text-color-primary);
  font-size: 1.5rem;
  font-weight: 600;
}

.setup-copy p {
  max-width: 580px;
  margin: 0 auto;
  color: var(--td-text-color-secondary);
  line-height: 1.7;
}

.setup-actions {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.setup-action-card {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 20px;
  min-height: 168px;
  padding: 22px;
  text-align: left;
  border: 1px solid var(--td-border-level-1-color);
  border-radius: 16px;
  background: var(--td-bg-color-secondarycontainer);
}

.setup-action-card.primary-card {
  background: color-mix(in srgb, var(--td-brand-color) 8%, var(--td-bg-color-container));
  border-color: color-mix(in srgb, var(--td-brand-color) 26%, var(--td-border-level-1-color));
}

.setup-action-card h4 {
  margin: 0 0 8px;
  color: var(--td-text-color-primary);
  font-size: 1rem;
  font-weight: 600;
}

.setup-action-card p {
  margin: 0;
  color: var(--td-text-color-secondary);
  line-height: 1.6;
}

@media (max-width: 768px) {
  .find-container {
    padding: var(--mobile-page-top-gutter) 0 0;
    overflow: hidden;
  }

  .page-header {
    margin: 0 var(--mobile-page-gutter) 1rem;
  }

  .page-header h2 {
    border-left: none;
    padding-left: 0;
    font-size: clamp(2rem, 9vw, 2.6rem);
    line-height: 1.1;
    letter-spacing: -0.04em;
  }

  .page-header p {
    font-size: 1rem;
  }

  .segment-tabs {
    margin: 0 var(--mobile-page-gutter) 1rem;
    min-height: var(--mobile-touch-target);
    padding: 4px;
    border-radius: var(--mobile-control-radius);
    background: var(--mobile-glass-bg-strong);
    border: 0.5px solid var(--mobile-glass-border);
  }

  .segment-tab {
    min-height: 36px;
    border-radius: var(--mobile-control-radius);
    font-size: 15px;
    touch-action: manipulation;
  }

  .tab-content,
  .leaderboard-pane {
    min-width: 0;
    min-height: 0;
    max-width: 100%;
    -webkit-overflow-scrolling: touch;
  }

  .source-state {
    min-height: 0;
    margin: 0 var(--mobile-page-gutter) 1rem;
  }

  .source-loading {
    min-height: 220px;
    border-radius: var(--mobile-card-radius, 16px);
    background: var(--mobile-glass-bg-strong);
    border: 0.5px solid var(--mobile-glass-border);
  }

  .setup-state {
    align-items: stretch;
    justify-content: flex-start;
    overflow-x: hidden;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .setup-guide {
    width: 100%;
    padding: clamp(16px, 5vw, 24px);
    gap: clamp(12px, 3.5vw, 16px);
    border-radius: var(--mobile-card-radius, 18px);
    box-sizing: border-box;
    align-items: center;
  }

  .setup-orb {
    width: 60px;
    height: 60px;
    border-radius: 18px;
    flex: 0 0 auto;
  }

  .setup-logo {
    border-radius: 18px;
  }

  .setup-copy h3 {
    font-size: 1.2rem;
  }

  .setup-copy p {
    font-size: 0.9rem;
  }

  .setup-actions {
    grid-template-columns: 1fr;
    width: 100%;
  }

  .setup-action-card {
    min-height: auto;
    padding: 14px;
    gap: 12px;
    text-align: center;
  }

  .setup-action-card h4 {
    font-size: 0.95rem;
  }

  .setup-action-card p {
    font-size: 0.82rem;
  }

  .setup-action-card :deep(.t-button) {
    width: 100%;
    min-height: var(--mobile-touch-target);
  }
}

@media (max-width: 768px) and (max-height: 640px) {
  .setup-state {
    padding-top: 6px;
  }

  .setup-guide {
    padding: 14px;
    gap: 10px;
  }

  .setup-orb {
    width: 48px;
    height: 48px;
    border-radius: 14px;
  }

  .setup-logo {
    border-radius: 14px;
  }

  .setup-copy h3 {
    margin-bottom: 6px;
    font-size: 1.08rem;
  }

  .setup-copy p {
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .setup-action-card {
    padding: 12px;
    gap: 10px;
  }

  .setup-action-card h4 {
    margin-bottom: 4px;
  }

  .setup-action-card p {
    line-height: 1.45;
  }
}
</style>
