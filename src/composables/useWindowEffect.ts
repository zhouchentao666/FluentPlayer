import { computed, onMounted, onUnmounted, ref, watch, type Ref } from 'vue'
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

  function updateSystemLight() {
    systemLight.value = window.matchMedia('(prefers-color-scheme: light)').matches
  }

  onMounted(() => {
    updateSystemLight()
    window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', updateSystemLight)
  })

  onUnmounted(() => {
    window.matchMedia('(prefers-color-scheme: light)').removeEventListener('change', updateSystemLight)
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
