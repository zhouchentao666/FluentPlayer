import { computed } from 'vue'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'

const FALLBACK_SOURCE_NAMES: Record<string, string> = {
  wy: '小芸',
  kg: '小苟',
  mg: '菇菇',
  tx: '小鹅',
  kw: '小蜗',
  bd: '波点音乐',
  git: 'GitCode',
  subsonic: 'Subsonic',
}

export function useSourceAccess() {
  const userStore = LocalUserDetailStore()

  const enabledSources = computed(() => userStore.userInfo.supportedSources || {})

  const enabledSourceKeys = computed(() => new Set(Object.keys(enabledSources.value)))

  const isSourceEnabled = (sourceKey: string): boolean => {
    return enabledSourceKeys.value.has(sourceKey)
  }

  const getSourceName = (sourceKey: string): string => {
    const entry = enabledSources.value[sourceKey]
    if (entry?.name) return entry.name
    return FALLBACK_SOURCE_NAMES[sourceKey] || sourceKey
  }

  const getSourceOptions = (options?: { excludeSubsonic?: boolean }) => {
    const excludeSubsonic = options?.excludeSubsonic ?? false
    return Object.entries(enabledSources.value)
      .filter(([key]) => !excludeSubsonic || key !== 'subsonic')
      .map(([key, source]) => ({
        key,
        name: source?.name || FALLBACK_SOURCE_NAMES[key] || key,
      }))
  }

  const filterByEnabledSources = <T extends { source?: string }>(items: T[]): T[] => {
    return items.filter(item =>
      !item.source || item.source === 'local' || isSourceEnabled(item.source)
    )
  }

  const validateCurrentSource = (): string => {
    const current = userStore.userInfo.selectSources as string
    if (current && isSourceEnabled(current)) return current
    const firstKey = Object.keys(enabledSources.value)[0] || ''
    userStore.userInfo.selectSources = firstKey
    return firstKey
  }

  return {
    enabledSources,
    enabledSourceKeys,
    isSourceEnabled,
    getSourceName,
    getSourceOptions,
    filterByEnabledSources,
    validateCurrentSource,
    FALLBACK_SOURCE_NAMES,
  }
}
