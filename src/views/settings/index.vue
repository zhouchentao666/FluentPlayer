<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted, onBeforeUnmount, defineAsyncComponent } from 'vue'
import { useRoute } from 'vue-router'
import TitleBarControls from '@/components/TitleBarControls.vue'
import {
  PaletteIcon,
  ApiIcon,
  PlayCircleIcon,
  AudioIcon,
  KeyboardIcon,
  TreeRoundDotIcon,
  MusicIcon,
  SaveIcon,
  InfoCircleIcon
} from 'tdesign-icons-vue-next'

import AppearanceSection from './sections/AppearanceSection.vue'
const AISection = defineAsyncComponent(() => import('./sections/AISection.vue'))
const PlaybackSection = defineAsyncComponent(() => import('./sections/PlaybackSection.vue'))
const AudioEffectsSection = defineAsyncComponent(() => import('./sections/AudioEffectsSection.vue'))
const HotkeySection = defineAsyncComponent(() => import('./sections/HotkeySection.vue'))
const PluginSection = defineAsyncComponent(() => import('./sections/PluginSection.vue'))
const MusicSourceSection = defineAsyncComponent(() => import('./sections/MusicSourceSection.vue'))
const StorageSection = defineAsyncComponent(() => import('./sections/StorageSection.vue'))
const AboutSection = defineAsyncComponent(() => import('./sections/AboutSection.vue'))
import SettingsSearch from '@/components/SettingsSearch.vue'
import type { SearchItem } from './searchIndex'

const { t } = useI18n()

const activeCategory = ref<string>('appearance')
const isMobile = ref(false)
const route = useRoute()
const contentPanelRef = ref<HTMLElement>()
const scrollPositions = ref<Record<string, number>>({})

const sectionAliases: Record<string, string> = {
  'playback-equalizer': 'effects-equalizer',
  'playback-audio-effect': 'effects-audio-effect'
}

const sectionCategoryMap: Record<string, string> = {
  'effects-equalizer': 'effects',
  'effects-audio-effect': 'effects',
  'playback-equalizer': 'effects',
  'playback-audio-effect': 'effects'
}

const resolveSettingsTarget = (category: string, section: string) => {
  const resolvedSection = sectionAliases[section] || section
  return {
    category: sectionCategoryMap[section] || sectionCategoryMap[resolvedSection] || category,
    section: resolvedSection
  }
}

const settingsCategories = computed(() => [
  {
    key: 'appearance',
    label: t('settings.musicSource.navAppearance'),
    icon: PaletteIcon,
    description: t('settings.musicSource.navAppearanceDesc')
  },
  {
    key: 'ai',
    label: t('settings.musicSource.navAi'),
    icon: ApiIcon,
    description: t('settings.musicSource.navAiDesc')
  },
  {
    key: 'playlist',
    label: t('settings.musicSource.navPlaylist'),
    icon: PlayCircleIcon,
    description: t('settings.musicSource.navPlaylistDesc')
  },
  {
    key: 'effects',
    label: t('settings.musicSource.navEffects'),
    icon: AudioIcon,
    description: t('settings.musicSource.navEffectsDesc')
  },
  {
    key: 'hotkeys',
    label: t('settings.musicSource.navHotkeys'),
    icon: KeyboardIcon,
    description: t('settings.musicSource.navHotkeysDesc')
  },
  {
    key: 'plugins',
    label: t('settings.musicSource.navPlugins'),
    icon: TreeRoundDotIcon,
    description: t('settings.musicSource.navPluginsDesc')
  },
  {
    key: 'music',
    label: t('settings.musicSource.navMusic'),
    icon: MusicIcon,
    description: t('settings.musicSource.navMusicDesc')
  },
  {
    key: 'storage',
    label: t('settings.musicSource.navStorage'),
    icon: SaveIcon,
    description: t('settings.musicSource.navStorageDesc')
  },
  {
    key: 'about',
    label: t('settings.musicSource.navAbout'),
    icon: InfoCircleIcon,
    description: t('settings.musicSource.navAboutDesc')
  }
])

const visibleSettingsCategories = computed(() =>
  isMobile.value
    ? settingsCategories.value.filter((category) => category.key !== 'hotkeys')
    : settingsCategories.value
)

const sectionComponents: Record<string, any> = {
  appearance: AppearanceSection,
  ai: AISection,
  playlist: PlaybackSection,
  effects: AudioEffectsSection,
  hotkeys: HotkeySection,
  plugins: PluginSection,
  music: MusicSourceSection,
  storage: StorageSection,
  about: AboutSection
}

const currentComponent = computed(() => sectionComponents[activeCategory.value])

const sidebarNavRef = ref<HTMLElement>()
const navCanScrollLeft = ref(false)
const navCanScrollRight = ref(false)

const updateNavScrollState = () => {
  const navEl = sidebarNavRef.value
  if (!navEl) return
  navCanScrollLeft.value = navEl.scrollLeft > 4
  navCanScrollRight.value = navEl.scrollLeft + navEl.clientWidth < navEl.scrollWidth - 4
}

const scrollNavToActive = (categoryKey: string) => {
  const navEl = sidebarNavRef.value
  if (!navEl) return
  const activeEl = document.getElementById(`settings-nav-${categoryKey}`)
  if (!activeEl) return
  const navRect = navEl.getBoundingClientRect()
  const elRect = activeEl.getBoundingClientRect()
  const offset = elRect.left - navRect.left - (navRect.width / 2) + (elRect.width / 2)
  navEl.scrollBy({ left: offset, behavior: 'smooth' })
  window.setTimeout(updateNavScrollState, 300)
}

const updateIsMobile = () => {
  isMobile.value = window.matchMedia('(max-width: 768px)').matches
  if (isMobile.value && activeCategory.value === 'hotkeys') {
    activeCategory.value = 'appearance'
  }
}

const switchCategory = async (categoryKey: string) => {
  if (isMobile.value && categoryKey === 'hotkeys') return
  if (activeCategory.value === categoryKey) return
  if (contentPanelRef.value) {
    scrollPositions.value[activeCategory.value] = contentPanelRef.value.scrollTop
  }
  activeCategory.value = categoryKey

  await nextTick()
  if (contentPanelRef.value) {
    contentPanelRef.value.scrollTop = scrollPositions.value[categoryKey] || 0
  }
  scrollNavToActive(categoryKey)
}

function scrollToSection(sectionId?: string) {
  if (!sectionId) return
  let attempts = 0
  const maxAttempts = 20
  const tryScroll = () => {
    const el = document.getElementById(sectionId)
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' })
      el.classList.remove('highlight-flash')
      void el.offsetWidth
      el.classList.add('highlight-flash')
      setTimeout(() => el.classList.remove('highlight-flash'), 2000)
    } else if (attempts < maxAttempts) {
      attempts++
      setTimeout(tryScroll, 100)
    }
  }
  tryScroll()
}

watch(
  () => route.query,
  async (q) => {
    const rawCategory = String(q.category || '')
    const rawSection = String(q.section || '')
    const { category, section } = resolveSettingsTarget(rawCategory, rawSection)
    if (category && category !== activeCategory.value && !(isMobile.value && category === 'hotkeys')) {
      await switchCategory(category)
      await nextTick()
    }
    if (section) {
      await nextTick()
      scrollToSection(section)
    }
  },
  { immediate: true, deep: true }
)

onMounted(async () => {
  updateIsMobile()
  await nextTick()
  updateNavScrollState()
  scrollNavToActive(activeCategory.value)
  window.addEventListener('resize', updateIsMobile)
  window.addEventListener('resize', updateNavScrollState)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', updateIsMobile)
  window.removeEventListener('resize', updateNavScrollState)
})

const handleSearchSelect = async (item: SearchItem) => {
  if (isMobile.value && item.category === 'hotkeys') return
  if (activeCategory.value !== item.category) {
    await switchCategory(item.category)
    await nextTick()
    await new Promise((resolve) => setTimeout(resolve, 50))
  }
  let attempts = 0
  const maxAttempts = 10
  const tryScroll = () => {
    const element = document.getElementById(item.id)
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' })
      element.classList.remove('highlight-flash')
      void element.offsetWidth
      element.classList.add('highlight-flash')
      setTimeout(() => element.classList.remove('highlight-flash'), 2000)
    } else if (attempts < maxAttempts) {
      attempts++
      setTimeout(tryScroll, 100)
    }
  }
  tryScroll()
}
</script>

<template>
  <div class="main-container">
    <div class="header" data-tauri-drag-region>
      <TitleBarControls :title="t('settings.title')" :show-back="true" :show-account="false">
        <template #extra>
          <div style="flex-shrink: 0">
            <SettingsSearch :hidden-categories="isMobile ? ['hotkeys'] : []" @select="handleSearchSelect" />
          </div>
        </template>
      </TitleBarControls>
    </div>
    <div class="settings-layout">
      <div
        class="sidebar"
        :class="{
          'can-scroll-left': navCanScrollLeft,
          'can-scroll-right': navCanScrollRight
        }"
      >
        <nav ref="sidebarNavRef" class="sidebar-nav" @scroll="updateNavScrollState">
          <div
            v-for="category in visibleSettingsCategories"
            :id="`settings-nav-${category.key}`"
            :key="category.key"
            class="nav-item"
            :class="{ active: activeCategory === category.key }"
            @click="switchCategory(category.key)"
          >
            <div class="nav-icon">
              <component :is="category.icon" />
            </div>
            <div class="nav-content">
              <div class="nav-label">{{ category.label }}</div>
              <div class="nav-description">{{ category.description }}</div>
            </div>
          </div>
        </nav>
      </div>
      <div class="content-panel">
        <div ref="contentPanelRef" class="panel-content">
          <KeepAlive>
            <component
              :is="currentComponent"
              :key="activeCategory"
              @switch-category="switchCategory"
            />
          </KeepAlive>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.main-container {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--settings-main-bg);
}

.header {
  display: flex;
  align-items: center;
  background: var(--settings-header-bg);
  padding: 1.5rem;
  z-index: 1000;
}

.settings-layout {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.sidebar {
  width: 280px;
  background: var(--settings-sidebar-bg);
  padding-right: 5px;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding-left: 5px;

  .sidebar-nav {
    flex: 1;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.875rem 1.5rem;
    margin-top: 5px;
    cursor: pointer;
    transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
    border-left: 3px solid transparent;
    border-radius: 5px;

    &:hover {
      background: var(--settings-nav-hover-bg);
    }

    &.active {
      background: var(--settings-nav-active-bg);
      border-left-color: var(--settings-nav-active-border);
      box-shadow: var(--settings-nav-active-shadow, none);

      .nav-icon {
        color: var(--settings-nav-icon-active);
      }

      .nav-label {
        color: var(--settings-nav-label-active);
        font-weight: 600;
      }
    }

    .nav-icon {
      width: 20px;
      height: 20px;
      color: var(--settings-nav-icon-color);
      display: flex;
      justify-content: center;
      align-items: center;

      svg {
        width: 100%;
        height: 100%;
      }

      transition: color 0.2s ease;
    }

    .nav-content {
      flex: 1;
      min-width: 0;

      .nav-label {
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--settings-nav-label-color);
        margin-bottom: 0.125rem;
        transition: color 0.2s ease;
      }

      .nav-description {
        font-size: 0.75rem;
        color: var(--settings-nav-desc-color);
        line-height: 1.3;
      }
    }
  }
}

.content-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 10px;

  .panel-content {
    flex: 1;
    overflow-y: auto;
    background: var(--settings-main-bg);
    scroll-behavior: smooth;
  }
}

.content-panel :deep(.t-input),
.content-panel :deep(.t-input-number .t-input),
.content-panel :deep(.t-select .t-input),
.content-panel :deep(.t-textarea__inner) {
  background-color: var(--td-bg-color-specialcomponent, var(--td-bg-color-component));
  border-color: var(--td-border-level-2-color, var(--td-component-border));
  color: var(--td-text-color-primary);
}

.content-panel :deep(.t-input:hover),
.content-panel :deep(.t-input-number .t-input:hover),
.content-panel :deep(.t-select .t-input:hover),
.content-panel :deep(.t-textarea__inner:hover) {
  background-color: var(--td-bg-color-component-hover);
  border-color: var(--td-brand-color);
}

.content-panel :deep(.t-input:focus),
.content-panel :deep(.t-input--focused),
.content-panel :deep(.t-input-number .t-input:focus),
.content-panel :deep(.t-input-number .t-input--focused),
.content-panel :deep(.t-select .t-input:focus),
.content-panel :deep(.t-select .t-input--focused),
.content-panel :deep(.t-textarea__inner:focus) {
  background-color: var(--td-bg-color-specialcomponent, var(--td-bg-color-component));
  border-color: var(--td-brand-color);
  box-shadow: 0 0 0 2px var(--td-brand-color-focus);
}

.content-panel :deep(.t-input__inner),
.content-panel :deep(.t-textarea__inner) {
  color: var(--td-text-color-primary);
}

.content-panel :deep(.t-input__inner::placeholder),
.content-panel :deep(.t-textarea__inner::placeholder) {
  color: var(--td-text-color-placeholder);
}

.content-panel :deep(.t-input__prefix),
.content-panel :deep(.t-input__suffix),
.content-panel :deep(.t-input__prefix .t-icon),
.content-panel :deep(.t-input__suffix .t-icon),
.content-panel :deep(.t-fake-arrow),
.content-panel :deep(.t-input-number__decrease .t-icon),
.content-panel :deep(.t-input-number__increase .t-icon) {
  color: var(--td-text-color-secondary);
}

.content-panel :deep(.t-input-number__decrease),
.content-panel :deep(.t-input-number__increase) {
  border-color: var(--td-border-level-2-color, var(--td-component-border));
}

.content-panel :deep(.t-input-number__decrease:hover:not(.t-is-disabled)),
.content-panel :deep(.t-input-number__increase:hover:not(.t-is-disabled)) {
  background-color: var(--td-bg-color-component-hover);
  border-color: var(--td-brand-color);
  color: var(--td-brand-color);
}

.content-panel :deep(.t-input-number__decrease:hover:not(.t-is-disabled) .t-icon),
.content-panel :deep(.t-input-number__increase:hover:not(.t-is-disabled) .t-icon) {
  color: var(--td-brand-color);
}

@media (max-width: 768px) {
  .main-container {
    height: 100dvh;
    min-height: 100dvh;
    background: var(--mobile-page-bg, var(--settings-main-bg));
    overflow: hidden;
  }

  .header {
    padding: calc(var(--mobile-safe-top) + 10px) var(--mobile-page-gutter) 10px;
    background: var(--mobile-glass-bg-strong, var(--settings-header-bg));
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
  }

  .header :deep(.settings-search) {
    width: min(46vw, 220px);
  }

  .settings-layout {
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .sidebar {
    width: 100%;
    max-width: 100%;
    max-height: none;
    flex: 0 0 auto;
    padding: 6px var(--mobile-page-gutter) 8px;
    background: transparent;
    border-bottom: none;
    overflow: hidden;
    position: relative;
    box-sizing: border-box;
  }

  .sidebar::before,
  .sidebar::after {
    content: '';
    position: absolute;
    top: 6px;
    bottom: 8px;
    width: 24px;
    z-index: 2;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.3s ease;
  }

  .sidebar.can-scroll-left::before {
    left: var(--mobile-page-gutter);
    background: linear-gradient(to right, var(--mobile-page-bg, var(--settings-main-bg)), transparent);
    opacity: 1;
  }

  .sidebar.can-scroll-right::after {
    right: var(--mobile-page-gutter);
    background: linear-gradient(to left, var(--mobile-page-bg, var(--settings-main-bg)), transparent);
    opacity: 1;
  }

  .sidebar .sidebar-nav {
    display: flex;
    gap: 6px;
    width: 100%;
    max-width: 100%;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 2px 0 6px;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
    box-sizing: border-box;
  }

  .sidebar .sidebar-nav::-webkit-scrollbar {
    display: none;
    width: 0;
    height: 0;
  }

  .sidebar .nav-item {
    min-width: 0;
    flex: 0 0 auto;
    min-height: var(--mobile-touch-target);
    margin: 0;
    padding: 0 12px;
    border: 1px solid var(--mobile-glass-border);
    border-radius: 20px;
    background: var(--mobile-glass-bg);
    touch-action: manipulation;
    transition:
      background-color 0.25s cubic-bezier(0.4, 0, 0.2, 1),
      border-color 0.25s cubic-bezier(0.4, 0, 0.2, 1),
      color 0.25s cubic-bezier(0.4, 0, 0.2, 1),
      box-shadow 0.25s cubic-bezier(0.4, 0, 0.2, 1),
      opacity 0.25s cubic-bezier(0.4, 0, 0.2, 1),
      transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .sidebar .nav-item:hover {
    background: var(--settings-nav-hover-bg);
  }

  .sidebar .nav-item.active {
    background: var(--settings-nav-active-bg);
    border-color: var(--settings-nav-active-border);
    box-shadow: var(--settings-nav-active-shadow, 0 2px 8px rgba(15, 23, 42, 0.12));
  }

  .sidebar .nav-item.active .nav-icon {
    color: var(--settings-nav-icon-active);
  }

  .sidebar .nav-item.active .nav-label {
    color: var(--settings-nav-label-active);
    font-weight: 600;
  }

  .sidebar .nav-icon {
    width: 16px;
    height: 16px;
  }

  .sidebar .nav-description {
    display: none;
  }

  .sidebar .nav-label {
    margin: 0;
    font-size: 13px;
    white-space: nowrap;
    line-height: 1;
  }

  .content-panel {
    flex: 1 1 0;
    height: 0;
    min-height: 0;
    padding: 0 var(--mobile-page-gutter);
    overflow: hidden;
  }

  .content-panel .panel-content {
    flex: 1;
    height: 100%;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
    border: 1px solid var(--mobile-glass-border);
    border-radius: var(--mobile-card-radius);
    background: var(--mobile-glass-bg, var(--settings-main-bg));
    padding: 10px 10px calc(var(--mobile-content-bottom-inset) + 12px);
    box-sizing: border-box;
  }

  .content-panel :deep(.settings-section) {
    min-height: 100%;
  }

  .content-panel :deep(.setting-group),
  .content-panel :deep(.section) {
    padding: 14px;
    margin-bottom: 10px;
    border-radius: var(--mobile-card-radius-small);
  }

  .content-panel :deep(.setting-card) {
    margin-bottom: 10px;
    border-radius: var(--mobile-card-radius-small);
    overflow: hidden;
  }

  .content-panel :deep(.setting-item),
  .content-panel :deep(.hotkey-row),
  .content-panel :deep(.tag-option) {
    min-height: var(--mobile-touch-target);
    padding: 10px 12px;
    border-radius: var(--mobile-card-radius-small);
  }

  .content-panel :deep(.setting-item),
  .content-panel :deep(.hotkey-row) {
    align-items: flex-start;
    flex-direction: column;
    gap: 8px;
  }

  .content-panel :deep(.setting-item .setting-control),
  .content-panel :deep(.setting-item .style-buttons),
  .content-panel :deep(.setting-item .setting-info) {
    width: 100%;
  }

  .content-panel :deep(.hotkey-actions),
  .content-panel :deep(.template-tip),
  .content-panel :deep(.preview-container),
  .content-panel :deep(.status-summary) {
    width: 100%;
    flex-wrap: wrap;
  }

  .content-panel :deep(.t-input),
  .content-panel :deep(.t-input-number),
  .content-panel :deep(.t-select),
  .content-panel :deep(.t-textarea) {
    width: 100% !important;
  }

  .content-panel :deep(.t-button),
  .content-panel :deep(.t-input),
  .content-panel :deep(.t-input-number),
  .content-panel :deep(.t-select) {
    min-height: var(--mobile-touch-target);
  }

  .content-panel :deep(.t-slider) {
    width: 100% !important;
  }

  .content-panel :deep(.t-radio-group) {
    flex-wrap: wrap;
    width: 100%;
  }

  .content-panel :deep(.t-radio-button) {
    flex: 1;
    min-width: 0;
    text-align: center;
  }

  .content-panel :deep(.t-card) {
    border-radius: var(--mobile-card-radius-small);
    overflow: hidden;
  }

  .content-panel :deep(.t-card__header) {
    padding: 14px 14px 8px;
  }

  .content-panel :deep(.t-card__body) {
    padding: 8px 14px 14px;
  }

  .content-panel :deep(.t-card__actions) {
    margin-left: 8px;
  }

  .content-panel :deep(.t-space) {
    flex-wrap: wrap;
  }

  .content-panel :deep(.setting-label),
  .content-panel :deep(.setting-info),
  .content-panel :deep(.item-info) {
    width: 100%;
    min-width: 0;
  }

  .content-panel :deep(.setting-label h4),
  .content-panel :deep(.setting-title),
  .content-panel :deep(.item-title) {
    font-size: 14px;
    line-height: 1.35;
  }

  .content-panel :deep(.setting-label p),
  .content-panel :deep(.setting-desc),
  .content-panel :deep(.item-desc),
  .content-panel :deep(.option-desc) {
    font-size: 12px;
    line-height: 1.45;
  }

  .content-panel :deep(.file-path) {
    max-width: 100%;
  }

  .content-panel :deep(.font-preview),
  .content-panel :deep(.preview-container),
  .content-panel :deep(.visualizer-container) {
    width: 100%;
    min-width: 0;
  }

  .content-panel :deep(.tag-options) {
    gap: 10px;
  }

  .content-panel :deep(.lyric-format-options .t-radio-group),
  .content-panel :deep(.presets .t-radio-group),
  .content-panel :deep(.control-group .t-radio-group) {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .content-panel :deep(.t-divider) {
    margin: 12px 0;
  }

  .content-panel :deep(.setting-spacer) {
    height: 10px;
  }

  .content-panel :deep(.setting-group-item) {
    margin-bottom: 16px;
  }

}

.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

.fade-slide-enter-to,
.fade-slide-leave-from {
  opacity: 1;
  transform: translateX(0);
}

.sidebar,
.panel-content {
  &::-webkit-scrollbar {
    width: 0;
    height: 0;
  }
  &::-webkit-scrollbar-track {
    background: transparent;
  }
  &::-webkit-scrollbar-thumb {
    background: transparent;
  }
  scrollbar-width: none;
  -ms-overflow-style: none;
}

:deep(.highlight-flash) {
  position: relative;
}

:deep(.highlight-flash::after) {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  animation: flashHighlight 2s ease-out;
  z-index: 10;
}

@keyframes flashHighlight {
  0% {
    background-color: rgba(var(--td-brand-color-rgb), 0.2);
    box-shadow: 0 0 0 2px var(--td-brand-color);
  }
  50% {
    background-color: rgba(var(--td-brand-color-rgb), 0.1);
  }
  100% {
    background-color: transparent;
    box-shadow: none;
  }
}
</style>
