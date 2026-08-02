import { computed, onMounted, onUnmounted, ref, watch, type Ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { ReadImageFile } from '@bridge/app'
import { useDominantColors } from './useDominantColors'
import type { AppSettings } from './useConfig'

export function useWindowEffect(
  settings: Ref<AppSettings>,
  coverUrl: Ref<string | null>
) {
  const { dominantColors } = useDominantColors(coverUrl)
  const customImageDataUrl = ref<string | null>(null)
  const systemLight = ref(false)
  const windowFocused = ref(true)

  function updateSystemLight() {
    systemLight.value = window.matchMedia('(prefers-color-scheme: light)').matches
  }

  const unlisteners: Array<() => void> = []

  onMounted(() => {
    updateSystemLight()
    window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', updateSystemLight)
    // 监听窗口焦点：用于「失焦保持材质」开关
    try {
      const win = getCurrentWindow()
      Promise.all([
        win.listen('focus', () => { windowFocused.value = true }),
        win.listen('blur', () => { windowFocused.value = false }),
      ]).then((fns: Array<() => void>) => {
        fns.forEach((fn) => unlisteners.push(fn))
      }).catch(() => {})
    } catch {
      // 非 Tauri 环境忽略
    }
  })

  onUnmounted(() => {
    window.matchMedia('(prefers-color-scheme: light)').removeEventListener('change', updateSystemLight)
    unlisteners.forEach((fn) => fn())
  })

  async function loadCustomImage(path: string) {
    if (!path) {
      customImageDataUrl.value = null
      return
    }
    try {
      customImageDataUrl.value = await ReadImageFile(path)
    } catch {
      customImageDataUrl.value = null
    }
  }

  watch(() => settings.value.customImagePath, loadCustomImage, { immediate: true })

  const isLight = computed(() => {
    if (settings.value.theme === 'light') return true
    if (settings.value.theme === 'dark') return false
    return systemLight.value
  })

  const hasCustomImage = computed(() => Boolean(customImageDataUrl.value))

  const appStyle = computed(() => {
    const base: Record<string, string> = { '--fluent-accent': settings.value.accentColor }
    const effect = settings.value.windowEffect
    if (effect === 'none') {
      return {
        ...base,
        background: 'var(--fluent-bg-card)',
        backdropFilter: 'none',
      }
    }
    if (effect === 'acrylic' || (effect === 'custom-image' && !hasCustomImage.value)) {
      // 关闭「失焦保持材质」且窗口失焦时，降级为纯色背景（移除模糊层）
      if (!settings.value.keepMaterialOnBlur && !windowFocused.value) {
        return {
          ...base,
          background: 'var(--fluent-bg-card)',
          backdropFilter: 'none',
        }
      }
      return base
    }
    return {
      ...base,
      background: 'transparent',
      backdropFilter: 'none',
    }
  })

  const layerStyle = computed(() => {
    const effect = settings.value.windowEffect
    if (effect === 'custom-image') {
      const url = customImageDataUrl.value
      if (!url) return null
      const opacity = settings.value.customImageOpacity / 100
      const mask = isLight.value
        ? `rgba(255, 255, 255, ${opacity})`
        : `rgba(0, 0, 0, ${opacity})`
      return {
        backgroundImage: `linear-gradient(${mask}, ${mask}), url(${url})`,
        backgroundSize: 'cover',
        backgroundPosition: 'center',
        filter: `blur(${settings.value.customImageBlur}px)`,
      }
    }
    if (effect === 'song-color') {
      const colors = dominantColors.value.length
        ? dominantColors.value
        : ['#333', '#666']
      const gradient = `linear-gradient(135deg, ${colors.join(', ')})`
      const blur = settings.value.songColorBlur
      if (!isLight.value) {
        return {
          backgroundImage: gradient,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          filter: `blur(${blur}px)`,
        }
      }
      const opacity = settings.value.songColorOpacity / 100
      const mask = `rgba(255, 255, 255, ${opacity})`
      return {
        backgroundImage: `linear-gradient(${mask}, ${mask}), ${gradient}`,
        backgroundSize: 'cover',
        backgroundPosition: 'center',
        filter: `blur(${blur}px)`,
      }
    }
    return null
  })

  return {
    appStyle,
    layerStyle,
  }
}
