import i18n from '@/locales'

export const QUALITY_ORDER = [
  'master',
  'atmos_plus',
  'atmos',
  'hires',
  'flac24bit',
  'flac',
  '320k',
  '128k'
] as const

export type KnownQuality = (typeof QUALITY_ORDER)[number]
export type QualityInput = KnownQuality | string | { type: string; size?: string }

function getDisplayNameMap(): Record<string, string> {
  const t = i18n.global.t
  return {
    '128k': t('quality.display128k'),
    '320k': t('quality.display320k'),
    flac: t('quality.displayFlac'),
    flac24bit: t('quality.displayFlac24bit'),
    hires: t('quality.displayHires'),
    atmos: t('quality.displayAtmos'),
    atmos_plus: t('quality.displayAtmosPlus'),
    master: t('quality.displayMaster')
  }
}

export function getQualityDisplayName(quality: QualityInput | null | undefined): string {
  if (!quality) return ''
  const type = typeof quality === 'object' ? (quality as any).type : quality
  return getDisplayNameMap()[type] || String(type || '')
}

export function compareQuality(aType: string, bType: string): number {
  const ia = QUALITY_ORDER.indexOf(aType as KnownQuality)
  const ib = QUALITY_ORDER.indexOf(bType as KnownQuality)
  const va = ia === -1 ? QUALITY_ORDER.length : ia
  const vb = ib === -1 ? QUALITY_ORDER.length : ib
  return va - vb
}

export function normalizeTypes(
  types: Array<string | { type: string; size?: string }> | null | undefined
): string[] {
  if (!types || !Array.isArray(types)) return []
  return types
    .map((t) => (typeof t === 'object' ? (t as any).type : t))
    .filter((t): t is string => Boolean(t))
}

export function getHighestQualityType(
  types: Array<string | { type: string; size?: string }> | null | undefined
): string | null {
  const arr = normalizeTypes(types)
  if (!arr.length) return null
  return arr.sort(compareQuality)[0]
}

export function buildQualityFormats(
  input:
    | Array<{ type: string; size?: string }>
    | Record<string, { size?: string }>
    | null
    | undefined
): Array<{ type: string; size?: string }> {
  if (!input) return []
  let list: Array<{ type: string; size?: string }>
  if (Array.isArray(input)) {
    list = input.map((i) => (typeof i === 'string' ? { type: i } : { type: i.type, size: i.size }))
  } else {
    list = Object.keys(input).map((k) => ({ type: k, size: input[k]?.size }))
  }
  return list.sort((a, b) => compareQuality(a.type, b.type))
}

export function filterByPluginQualities(
  qualities: Array<{ type: string; size?: string }>,
  pluginQualities: string[] | undefined
): Array<{ type: string; size?: string }> {
  if (!pluginQualities || pluginQualities.length === 0) return qualities
  const filtered = qualities.filter(q => pluginQualities.includes(q.type))
  return filtered.length > 0 ? filtered : qualities
}

export function calculateBestQuality(
  availableTypes: Array<string | { type: string; size?: string }> | null | undefined,
  targetQuality: string
): string | null {
  const normalizedTypes = normalizeTypes(availableTypes)
  if (!normalizedTypes.length) return null

  if (normalizedTypes.includes(targetQuality)) return targetQuality

  const targetIndex = QUALITY_ORDER.indexOf(targetQuality as KnownQuality)
  if (targetIndex === -1) return getHighestQualityType(normalizedTypes)

  const candidates = normalizedTypes.filter((t) => {
    const index = QUALITY_ORDER.indexOf(t as KnownQuality)
    return index !== -1 && index >= targetIndex
  })

  if (candidates.length > 0) {
    return candidates.sort(compareQuality)[0]
  }

  return normalizedTypes.sort(compareQuality)[normalizedTypes.length - 1]
}
