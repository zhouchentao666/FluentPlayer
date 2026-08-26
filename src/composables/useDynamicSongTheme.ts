import { computed, onBeforeUnmount, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import type { Color } from '@/utils/color/colorExtractor'

interface DynamicSongThemeOptions {
  onThemeChange?: () => void
}

interface DynamicThemePalette {
  brand1: string
  brand2: string
  brand3: string
  brand4: string
  brand5: string
  brand6: string
  brand7: string
  brand8: string
  brand9: string
  brand10: string
  brandLight: string
  pageBg: string
  surfaceBg: string
  elevatedBg: string
  componentBg: string
  componentHoverBg: string
  componentActiveBg: string
  textPrimary: string
  textSecondary: string
  textMuted: string
  borderLight: string
  borderMedium: string
  shadowLight: string
  shadowMedium: string
  playerBg: string
  playerIdleBg: string
  mobileGlassBg: string
  mobileGlassBgStrong: string
  mobileGlassBorder: string
  settingsNavHoverBg: string
  settingsNavActiveBg: string
  settingsNavActiveBorder: string
  settingsNavIconColor: string
  settingsNavIconActive: string
  settingsNavLabelColor: string
  settingsNavLabelActive: string
  settingsNavDescColor: string
  settingsNavActiveShadow: string
  settingsGroupShadow: string
  settingsEqVisualizerBg: string
  settingsEqVisualizerTrail: string
  settingsEqVisualizerBarStart: string
  settingsEqVisualizerBarEnd: string
}

const DYNAMIC_THEME_PROPERTIES = [
  '--td-brand-color-1',
  '--td-brand-color-2',
  '--td-brand-color-3',
  '--td-brand-color-4',
  '--td-brand-color-5',
  '--td-brand-color-6',
  '--td-brand-color-7',
  '--td-brand-color-8',
  '--td-brand-color-9',
  '--td-brand-color-10',
  '--td-brand-color-light',
  '--td-brand-color-focus',
  '--td-brand-color-disabled',
  '--td-brand-color-hover',
  '--td-brand-color',
  '--td-brand-color-active',
  '--td-text-color-primary',
  '--td-text-color-secondary',
  '--td-text-color-placeholder',
  '--td-text-color-brand',
  '--td-text-color-link',
  '--td-bg-color-page',
  '--td-bg-color-container',
  '--td-bg-color-container-hover',
  '--td-bg-color-container-active',
  '--td-bg-color-secondarycontainer',
  '--td-bg-color-secondarycontainer-hover',
  '--td-bg-color-secondarycontainer-active',
  '--td-bg-color-component',
  '--td-bg-color-component-hover',
  '--td-bg-color-component-active',
  '--td-bg-color-specialcomponent',
  '--td-component-stroke',
  '--td-component-border',
  '--td-border-level-1-color',
  '--td-border-level-2-color',
  '--theme-bg-primary',
  '--theme-bg-secondary',
  '--theme-bg-tertiary',
  '--theme-text-primary',
  '--theme-text-secondary',
  '--theme-text-tertiary',
  '--theme-text-muted',
  '--theme-border-light',
  '--theme-border-medium',
  '--theme-border-strong',
  '--theme-border',
  '--theme-hover-bg',
  '--theme-shadow-light',
  '--theme-shadow-medium',
  '--theme-shadow-hover',
  '--theme-card-bg',
  '--theme-card-shadow',
  '--theme-card-shadow-hover',
  '--theme-header-bg',
  '--theme-badge-bg',
  '--find-card-info-bg',
  '--find-song-count-bg',
  '--song-list-header-bg',
  '--song-list-header-border',
  '--song-list-header-text',
  '--song-list-item-border',
  '--song-list-item-hover',
  '--song-list-item-current',
  '--song-list-item-playing',
  '--song-list-track-number',
  '--song-list-btn-bg-hover',
  '--recent-song-item-border',
  '--recent-song-item-hover',
  '--settings-main-bg',
  '--settings-header-bg',
  '--settings-sidebar-bg',
  '--settings-sidebar-border',
  '--settings-nav-hover-bg',
  '--settings-nav-active-bg',
  '--settings-nav-active-border',
  '--settings-nav-icon-color',
  '--settings-nav-icon-active',
  '--settings-nav-label-color',
  '--settings-nav-label-active',
  '--settings-nav-desc-color',
  '--settings-nav-active-shadow',
  '--settings-content-bg',
  '--settings-group-bg',
  '--settings-group-border',
  '--settings-group-shadow',
  '--settings-text-primary',
  '--settings-text-secondary',
  '--settings-source-card-hover-border',
  '--settings-source-card-active-border',
  '--settings-source-card-active-bg',
  '--settings-source-icon-bg',
  '--settings-plugin-prompt-bg',
  '--settings-plugin-prompt-border',
  '--settings-eq-visualizer-bg',
  '--settings-eq-visualizer-trail',
  '--settings-eq-visualizer-bar-start',
  '--settings-eq-visualizer-bar-end',
  '--settings-preview-bg',
  '--settings-preview-border',
  '--settings-feature-bg',
  '--settings-feature-border',
  '--settings-api-tips-bg',
  '--settings-api-tips-border',
  '--settings-source-card-bg',
  '--settings-source-card-border',
  '--settings-quality-container-bg',
  '--settings-quality-container-border',
  '--settings-status-item-bg',
  '--settings-status-item-border',
  '--settings-tech-item-bg',
  '--settings-tech-item-border',
  '--settings-developer-item-bg',
  '--settings-developer-item-border',
  '--settings-tag-option-bg',
  '--settings-tag-option-border',
  '--settings-tag-status-bg',
  '--settings-tag-status-border',
  '--plugins-card-selected-bg',
  '--plugins-card-selected-border',
  '--player-bg-default',
  '--player-bg-idle',
  '--player-text-idle',
  '--player-text-hover-idle',
  '--player-btn-bg-idle',
  '--player-btn-bg-hover-idle',
  '--mobile-glass-bg',
  '--mobile-glass-bg-strong',
  '--mobile-glass-border',
  '--mobile-page-bg'
]

const clampChannel = (value: number): number => Math.max(0, Math.min(255, Math.round(value)))
const clampAlpha = (value: number): number => Math.max(0, Math.min(1, value))

const mixChannel = (a: number, b: number, amount: number): number =>
  clampChannel(a + (b - a) * amount)

const mixColor = (color: Color, target: Color, amount: number): Color => ({
  r: mixChannel(color.r, target.r, amount),
  g: mixChannel(color.g, target.g, amount),
  b: mixChannel(color.b, target.b, amount)
})

const rgba = (color: Color, alpha = 1): string =>
  `rgba(${clampChannel(color.r)}, ${clampChannel(color.g)}, ${clampChannel(color.b)}, ${clampAlpha(alpha)})`

const rgb = (color: Color): string =>
  `rgb(${clampChannel(color.r)}, ${clampChannel(color.g)}, ${clampChannel(color.b)})`

const createPalette = (baseColor: Color, useBlackText: boolean): DynamicThemePalette => {
  const white: Color = { r: 255, g: 255, b: 255 }
  const black: Color = { r: 0, g: 0, b: 0 }
  const brand = mixColor(baseColor, useBlackText ? black : white, useBlackText ? 0.12 : 0.08)
  const pageBg = useBlackText
    ? mixColor(baseColor, white, 0.9)
    : mixColor(baseColor, black, 0.76)
  const surfaceBg = useBlackText
    ? mixColor(baseColor, white, 0.84)
    : mixColor(baseColor, black, 0.66)
  const elevatedBg = useBlackText
    ? mixColor(baseColor, white, 0.78)
    : mixColor(baseColor, black, 0.58)
  const componentBg = useBlackText
    ? mixColor(baseColor, white, 0.72)
    : mixColor(baseColor, black, 0.5)
  const componentHoverBg = useBlackText
    ? mixColor(baseColor, white, 0.64)
    : mixColor(baseColor, white, 0.02)
  const componentActiveBg = useBlackText
    ? mixColor(baseColor, white, 0.56)
    : mixColor(baseColor, white, 0.12)
  const textBase = useBlackText ? black : white
  const borderBase = useBlackText ? black : white
  const settingsNavActiveTextBase = useBlackText ? black : white
  const settingsNavActiveBg = useBlackText
    ? mixColor(baseColor, white, 0.7)
    : mixColor(baseColor, black, 0.34)
  const settingsNavHoverBg = useBlackText
    ? mixColor(baseColor, white, 0.8)
    : mixColor(baseColor, white, 0.04)
  const settingsNavActiveBorder = mixColor(
    brand,
    useBlackText ? black : white,
    useBlackText ? 0.08 : 0.16
  )

  return {
    brand1: rgba(mixColor(brand, white, 0.86), useBlackText ? 1 : 0.2),
    brand2: rgb(mixColor(brand, white, 0.72)),
    brand3: rgb(mixColor(brand, white, 0.5)),
    brand4: rgb(mixColor(brand, white, 0.28)),
    brand5: rgb(brand),
    brand6: rgb(mixColor(brand, black, 0.12)),
    brand7: rgb(mixColor(brand, black, 0.24)),
    brand8: rgb(mixColor(brand, black, 0.38)),
    brand9: rgb(mixColor(brand, black, 0.52)),
    brand10: rgb(mixColor(brand, black, 0.66)),
    brandLight: rgba(mixColor(brand, white, 0.72), useBlackText ? 0.68 : 0.22),
    pageBg: rgb(pageBg),
    surfaceBg: rgba(surfaceBg, useBlackText ? 0.96 : 0.92),
    elevatedBg: rgba(elevatedBg, useBlackText ? 0.98 : 0.94),
    componentBg: rgba(componentBg, useBlackText ? 0.78 : 0.64),
    componentHoverBg: rgba(componentHoverBg, useBlackText ? 0.9 : 0.72),
    componentActiveBg: rgba(componentActiveBg, useBlackText ? 0.96 : 0.8),
    textPrimary: rgba(textBase, useBlackText ? 0.9 : 0.92),
    textSecondary: rgba(textBase, useBlackText ? 0.62 : 0.68),
    textMuted: rgba(textBase, useBlackText ? 0.46 : 0.5),
    borderLight: rgba(borderBase, useBlackText ? 0.1 : 0.12),
    borderMedium: rgba(borderBase, useBlackText ? 0.16 : 0.2),
    shadowLight: useBlackText ? '0 2px 10px rgba(15, 23, 42, 0.08)' : '0 2px 12px rgba(0, 0, 0, 0.28)',
    shadowMedium: useBlackText ? '0 8px 24px rgba(15, 23, 42, 0.12)' : '0 12px 32px rgba(0, 0, 0, 0.36)',
    playerBg: rgba(useBlackText ? mixColor(baseColor, white, 0.78) : mixColor(baseColor, black, 0.35), 0.88),
    playerIdleBg: rgba(useBlackText ? mixColor(baseColor, white, 0.86) : mixColor(baseColor, black, 0.45), 0.92),
    mobileGlassBg: rgba(useBlackText ? mixColor(baseColor, white, 0.78) : mixColor(baseColor, black, 0.36), 0.78),
    mobileGlassBgStrong: rgba(useBlackText ? mixColor(baseColor, white, 0.88) : mixColor(baseColor, black, 0.46), 0.9),
    mobileGlassBorder: rgba(borderBase, useBlackText ? 0.1 : 0.18),
    settingsNavHoverBg: rgba(settingsNavHoverBg, useBlackText ? 0.86 : 0.7),
    settingsNavActiveBg: rgba(settingsNavActiveBg, useBlackText ? 0.92 : 0.82),
    settingsNavActiveBorder: rgba(settingsNavActiveBorder, useBlackText ? 0.72 : 0.9),
    settingsNavIconColor: rgba(textBase, useBlackText ? 0.5 : 0.58),
    settingsNavIconActive: rgba(settingsNavActiveTextBase, useBlackText ? 0.86 : 0.94),
    settingsNavLabelColor: rgba(textBase, useBlackText ? 0.72 : 0.82),
    settingsNavLabelActive: rgba(settingsNavActiveTextBase, useBlackText ? 0.92 : 0.96),
    settingsNavDescColor: rgba(textBase, useBlackText ? 0.48 : 0.58),
    settingsNavActiveShadow: useBlackText
      ? '0 4px 14px rgba(15, 23, 42, 0.1)'
      : '0 6px 18px rgba(0, 0, 0, 0.24)',
    settingsGroupShadow: useBlackText ? 'rgba(15, 23, 42, 0.08)' : 'rgba(0, 0, 0, 0.28)',
    settingsEqVisualizerBg: rgb(useBlackText ? mixColor(baseColor, white, 0.9) : mixColor(baseColor, black, 0.72)),
    settingsEqVisualizerTrail: rgba(useBlackText ? mixColor(baseColor, white, 0.72) : mixColor(baseColor, black, 0.42), 0.28),
    settingsEqVisualizerBarStart: rgb(brand),
    settingsEqVisualizerBarEnd: rgb(mixColor(brand, useBlackText ? black : white, useBlackText ? 0.22 : 0.28))
  }
}

const setThemeProperty = (root: HTMLElement, property: string, value: string): void => {
  root.style.setProperty(property, value, 'important')
}

const applyDynamicTheme = (palette: DynamicThemePalette): void => {
  const root = document.documentElement
  setThemeProperty(root, '--td-brand-color-1', palette.brand1)
  setThemeProperty(root, '--td-brand-color-2', palette.brand2)
  setThemeProperty(root, '--td-brand-color-3', palette.brand3)
  setThemeProperty(root, '--td-brand-color-4', palette.brand4)
  setThemeProperty(root, '--td-brand-color-5', palette.brand5)
  setThemeProperty(root, '--td-brand-color-6', palette.brand6)
  setThemeProperty(root, '--td-brand-color-7', palette.brand7)
  setThemeProperty(root, '--td-brand-color-8', palette.brand8)
  setThemeProperty(root, '--td-brand-color-9', palette.brand9)
  setThemeProperty(root, '--td-brand-color-10', palette.brand10)
  setThemeProperty(root, '--td-brand-color-light', palette.brandLight)
  setThemeProperty(root, '--td-brand-color-focus', palette.brand2)
  setThemeProperty(root, '--td-brand-color-disabled', palette.brand3)
  setThemeProperty(root, '--td-brand-color-hover', palette.brand4)
  setThemeProperty(root, '--td-brand-color', palette.brand5)
  setThemeProperty(root, '--td-brand-color-active', palette.brand6)
  setThemeProperty(root, '--td-text-color-primary', palette.textPrimary)
  setThemeProperty(root, '--td-text-color-secondary', palette.textSecondary)
  setThemeProperty(root, '--td-text-color-placeholder', palette.textMuted)
  setThemeProperty(root, '--td-text-color-brand', palette.brand5)
  setThemeProperty(root, '--td-text-color-link', palette.brand5)
  setThemeProperty(root, '--td-bg-color-page', palette.pageBg)
  setThemeProperty(root, '--td-bg-color-container', palette.elevatedBg)
  setThemeProperty(root, '--td-bg-color-container-hover', palette.componentHoverBg)
  setThemeProperty(root, '--td-bg-color-container-active', palette.componentActiveBg)
  setThemeProperty(root, '--td-bg-color-secondarycontainer', palette.surfaceBg)
  setThemeProperty(root, '--td-bg-color-secondarycontainer-hover', palette.componentHoverBg)
  setThemeProperty(root, '--td-bg-color-secondarycontainer-active', palette.componentActiveBg)
  setThemeProperty(root, '--td-bg-color-component', palette.componentBg)
  setThemeProperty(root, '--td-bg-color-component-hover', palette.componentHoverBg)
  setThemeProperty(root, '--td-bg-color-component-active', palette.componentActiveBg)
  setThemeProperty(root, '--td-bg-color-specialcomponent', palette.componentBg)
  setThemeProperty(root, '--td-component-stroke', palette.borderLight)
  setThemeProperty(root, '--td-component-border', palette.borderMedium)
  setThemeProperty(root, '--td-border-level-1-color', palette.borderLight)
  setThemeProperty(root, '--td-border-level-2-color', palette.borderMedium)

  setThemeProperty(root, '--theme-bg-primary', palette.elevatedBg)
  setThemeProperty(root, '--theme-bg-secondary', palette.surfaceBg)
  setThemeProperty(root, '--theme-bg-tertiary', palette.pageBg)
  setThemeProperty(root, '--theme-text-primary', palette.textPrimary)
  setThemeProperty(root, '--theme-text-secondary', palette.textSecondary)
  setThemeProperty(root, '--theme-text-tertiary', palette.textMuted)
  setThemeProperty(root, '--theme-text-muted', palette.textMuted)
  setThemeProperty(root, '--theme-border-light', palette.borderLight)
  setThemeProperty(root, '--theme-border-medium', palette.borderMedium)
  setThemeProperty(root, '--theme-border-strong', palette.borderMedium)
  setThemeProperty(root, '--theme-border', palette.borderMedium)
  setThemeProperty(root, '--theme-hover-bg', palette.componentHoverBg)
  setThemeProperty(root, '--theme-shadow-light', palette.shadowLight)
  setThemeProperty(root, '--theme-shadow-medium', palette.shadowMedium)
  setThemeProperty(root, '--theme-shadow-hover', palette.shadowMedium)
  setThemeProperty(root, '--theme-card-bg', palette.elevatedBg)
  setThemeProperty(root, '--theme-card-shadow', palette.shadowLight)
  setThemeProperty(root, '--theme-card-shadow-hover', palette.shadowMedium)
  setThemeProperty(root, '--theme-header-bg', palette.surfaceBg)
  setThemeProperty(root, '--theme-badge-bg', palette.componentBg)

  setThemeProperty(root, '--find-card-info-bg', palette.elevatedBg)
  setThemeProperty(root, '--find-song-count-bg', palette.componentBg)
  setThemeProperty(root, '--song-list-header-bg', palette.surfaceBg)
  setThemeProperty(root, '--song-list-header-border', palette.borderLight)
  setThemeProperty(root, '--song-list-header-text', palette.textMuted)
  setThemeProperty(root, '--song-list-item-border', palette.borderLight)
  setThemeProperty(root, '--song-list-item-hover', palette.componentHoverBg)
  setThemeProperty(root, '--song-list-item-current', palette.brandLight)
  setThemeProperty(root, '--song-list-item-playing', palette.brandLight)
  setThemeProperty(root, '--song-list-track-number', palette.textMuted)
  setThemeProperty(root, '--song-list-btn-bg-hover', palette.brandLight)
  setThemeProperty(root, '--recent-song-item-border', palette.borderLight)
  setThemeProperty(root, '--recent-song-item-hover', palette.componentHoverBg)

  setThemeProperty(root, '--settings-main-bg', palette.pageBg)
  setThemeProperty(root, '--settings-header-bg', palette.elevatedBg)
  setThemeProperty(root, '--settings-sidebar-bg', palette.elevatedBg)
  setThemeProperty(root, '--settings-sidebar-border', palette.borderMedium)
  setThemeProperty(root, '--settings-nav-hover-bg', palette.settingsNavHoverBg)
  setThemeProperty(root, '--settings-nav-active-bg', palette.settingsNavActiveBg)
  setThemeProperty(root, '--settings-nav-active-border', palette.settingsNavActiveBorder)
  setThemeProperty(root, '--settings-nav-icon-color', palette.settingsNavIconColor)
  setThemeProperty(root, '--settings-nav-icon-active', palette.settingsNavIconActive)
  setThemeProperty(root, '--settings-nav-label-color', palette.settingsNavLabelColor)
  setThemeProperty(root, '--settings-nav-label-active', palette.settingsNavLabelActive)
  setThemeProperty(root, '--settings-nav-desc-color', palette.settingsNavDescColor)
  setThemeProperty(root, '--settings-nav-active-shadow', palette.settingsNavActiveShadow)
  setThemeProperty(root, '--settings-content-bg', palette.pageBg)
  setThemeProperty(root, '--settings-group-bg', palette.elevatedBg)
  setThemeProperty(root, '--settings-group-border', palette.borderMedium)
  setThemeProperty(root, '--settings-group-shadow', palette.settingsGroupShadow)
  setThemeProperty(root, '--settings-text-primary', palette.textPrimary)
  setThemeProperty(root, '--settings-text-secondary', palette.textSecondary)
  setThemeProperty(root, '--settings-source-card-hover-border', palette.brand3)
  setThemeProperty(root, '--settings-source-card-active-border', palette.brand5)
  setThemeProperty(root, '--settings-source-card-active-bg', palette.brandLight)
  setThemeProperty(root, '--settings-source-icon-bg', palette.componentBg)
  setThemeProperty(root, '--settings-plugin-prompt-bg', `linear-gradient(135deg, ${palette.surfaceBg} 0%, ${palette.elevatedBg} 100%)`)
  setThemeProperty(root, '--settings-plugin-prompt-border', palette.borderMedium)
  setThemeProperty(root, '--settings-eq-visualizer-bg', palette.settingsEqVisualizerBg)
  setThemeProperty(root, '--settings-eq-visualizer-trail', palette.settingsEqVisualizerTrail)
  setThemeProperty(root, '--settings-eq-visualizer-bar-start', palette.settingsEqVisualizerBarStart)
  setThemeProperty(root, '--settings-eq-visualizer-bar-end', palette.settingsEqVisualizerBarEnd)
  setThemeProperty(root, '--settings-preview-bg', palette.surfaceBg)
  setThemeProperty(root, '--settings-preview-border', palette.borderMedium)
  setThemeProperty(root, '--settings-feature-bg', palette.surfaceBg)
  setThemeProperty(root, '--settings-feature-border', palette.borderMedium)
  setThemeProperty(root, '--settings-api-tips-bg', palette.surfaceBg)
  setThemeProperty(root, '--settings-api-tips-border', palette.borderMedium)
  setThemeProperty(root, '--settings-source-card-bg', palette.elevatedBg)
  setThemeProperty(root, '--settings-source-card-border', palette.borderMedium)
  setThemeProperty(root, '--settings-quality-container-bg', palette.surfaceBg)
  setThemeProperty(root, '--settings-quality-container-border', palette.borderMedium)
  setThemeProperty(root, '--settings-status-item-bg', palette.surfaceBg)
  setThemeProperty(root, '--settings-status-item-border', palette.borderMedium)
  setThemeProperty(root, '--settings-tech-item-bg', palette.surfaceBg)
  setThemeProperty(root, '--settings-tech-item-border', palette.borderMedium)
  setThemeProperty(root, '--settings-developer-item-bg', palette.surfaceBg)
  setThemeProperty(root, '--settings-developer-item-border', palette.borderMedium)
  setThemeProperty(root, '--settings-tag-option-bg', palette.surfaceBg)
  setThemeProperty(root, '--settings-tag-option-border', palette.borderMedium)
  setThemeProperty(root, '--settings-tag-status-bg', palette.surfaceBg)
  setThemeProperty(root, '--settings-tag-status-border', palette.borderMedium)

  setThemeProperty(root, '--plugins-card-selected-bg', palette.brandLight)
  setThemeProperty(root, '--plugins-card-selected-border', palette.brand5)
  setThemeProperty(root, '--player-bg-default', palette.playerBg)
  setThemeProperty(root, '--player-bg-idle', palette.playerIdleBg)
  setThemeProperty(root, '--player-text-idle', palette.textSecondary)
  setThemeProperty(root, '--player-text-hover-idle', palette.textPrimary)
  setThemeProperty(root, '--player-btn-bg-idle', palette.brandLight)
  setThemeProperty(root, '--player-btn-bg-hover-idle', palette.brand2)
  setThemeProperty(root, '--mobile-glass-bg', palette.mobileGlassBg)
  setThemeProperty(root, '--mobile-glass-bg-strong', palette.mobileGlassBgStrong)
  setThemeProperty(root, '--mobile-glass-border', palette.mobileGlassBorder)
  setThemeProperty(root, '--mobile-page-bg', `linear-gradient(180deg, ${palette.surfaceBg} 0%, ${palette.pageBg} 100%)`)
  root.dataset.songTheme = 'dynamic'
}

const clearDynamicTheme = (): void => {
  const root = document.documentElement
  DYNAMIC_THEME_PROPERTIES.forEach((property) => root.style.removeProperty(property))
  delete root.dataset.songTheme
}

export function useDynamicSongTheme(options: DynamicSongThemeOptions = {}) {
  const globalPlayStatus = useGlobalPlayStatusStore()
  const { player } = storeToRefs(globalPlayStatus)

  const palette = computed(() => {
    if (!player.value.songInfo?.songmid || !player.value.cover) return null
    const color = player.value.coverDetail.ColorObject
    if (!color) return null
    return createPalette(color, player.value.coverDetail.useBlackText === true)
  })

  const stop = watch(
    palette,
    (nextPalette) => {
      if (!nextPalette) {
        clearDynamicTheme()
      } else {
        applyDynamicTheme(nextPalette)
      }
      options.onThemeChange?.()
    },
    { immediate: true }
  )

  onBeforeUnmount(() => {
    stop()
    clearDynamicTheme()
    options.onThemeChange?.()
  })
}
