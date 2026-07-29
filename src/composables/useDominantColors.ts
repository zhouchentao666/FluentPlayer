import { ref, watch, type Ref } from 'vue'
import { extractDominantColors, FALLBACK_PALETTE } from '../utils/paletteExtractor'

export function useDominantColors(coverUrl: Ref<string | null>) {
  const dominantColors = ref<string[]>(FALLBACK_PALETTE)
  let lastUrl: string | null = null

  async function update(url: string | null) {
    if (!url) {
      dominantColors.value = FALLBACK_PALETTE
      lastUrl = null
      return
    }
    if (url === lastUrl) return
    lastUrl = url
    dominantColors.value = await extractDominantColors(url)
  }

  watch(coverUrl, update, { immediate: true })

  return { dominantColors }
}
