import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import i18n from '@/locales'
import createLogger from '@/utils/logger'

const log = createLogger('EqualizerStore')

export const EQ_STORAGE_KEY = 'equalizer'
export const EQ_CONFIG_VERSION = 3
export const EQ_BAND_COUNT = 10
export const EQ_GAIN_MIN = -24
export const EQ_GAIN_MAX = 24
export const EQ_FLAT_PRESET_ID = 'flat'
export const EQ_FREQUENCIES = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000] as const

export const EQ_FREQ_MIN = 20
export const EQ_FREQ_MAX = 20000
export const EQ_Q_MIN = 0.1
export const EQ_Q_MAX = 10.0
export const EQ_Q_DEFAULT_PEAK = 1.414
export const EQ_Q_DEFAULT_SHELF = 0.7

export type FilterType = 'peak' | 'lowshelf' | 'highshelf' | 'lowpass' | 'highpass' | 'notch'

export const ALL_FILTER_TYPES: FilterType[] = ['peak', 'lowshelf', 'highshelf', 'lowpass', 'highpass', 'notch']

export interface EqBand {
  frequency: number   // 20-20000 Hz
  gain: number        // -24 to +24 dB
  q: number           // 0.1 to 10.0
  type: FilterType
  enabled: boolean
}

export interface EqualizerGains {
  global: number
  bands: EqBand[]
}

export interface EqualizerPreset {
  id: string
  name: string
  isDefault: boolean
  gains: EqualizerGains
}

export interface EqualizerExportData {
  version: number
  enabled: boolean
  currentPresetId: string
  gains: EqualizerGains
  presets: EqualizerPreset[]
  defaultPresets: EqualizerPreset[]
  userPresets: EqualizerPreset[]
}

interface EqualizerPersistedState {
  version: number
  enabled: boolean
  currentPresetId: string
  gains: EqualizerGains
  userPresets: EqualizerPreset[]
  logs: string[]
}

interface EqualizerHydratedState extends EqualizerPersistedState {}

interface ImportResult {
  success: boolean
  error?: string
}

interface TextPresetImportResult extends ImportResult {
  gains?: EqualizerGains
}

const PRESET_ID_PATTERN = /^[a-z0-9][a-z0-9_-]{0,63}$/

export const clampEqGain = (gain: number): number => {
  if (!Number.isFinite(gain)) return 0
  return Math.min(EQ_GAIN_MAX, Math.max(EQ_GAIN_MIN, gain))
}

export const clampEqFreq = (freq: number): number => {
  if (!Number.isFinite(freq)) return 1000
  return Math.min(EQ_FREQ_MAX, Math.max(EQ_FREQ_MIN, freq))
}

export const clampEqQ = (q: number): number => {
  if (!Number.isFinite(q)) return EQ_Q_DEFAULT_PEAK
  return Math.min(EQ_Q_MAX, Math.max(EQ_Q_MIN, q))
}

export const isValidFilterType = (type: unknown): type is FilterType => {
  return typeof type === 'string' && ALL_FILTER_TYPES.includes(type as FilterType)
}

export const getDefaultQ = (type: FilterType): number => {
  return (type === 'lowshelf' || type === 'highshelf') ? EQ_Q_DEFAULT_SHELF : EQ_Q_DEFAULT_PEAK
}

export const createFlatBands = (): EqBand[] =>
  EQ_FREQUENCIES.map(freq => ({
    frequency: freq,
    gain: 0,
    q: EQ_Q_DEFAULT_PEAK,
    type: 'peak' as FilterType,
    enabled: true
  }))

export const createFlatGains = (): EqualizerGains => ({
  global: 0,
  bands: createFlatBands()
})

export const cloneGains = (source: EqualizerGains): EqualizerGains => ({
  global: source.global,
  bands: source.bands.map(band => ({ ...band }))
})

const DEFAULT_PRESET_DATA: Array<Omit<EqualizerPreset, 'isDefault' | 'gains'> & { bandGains: number[] }> = [
  { id: EQ_FLAT_PRESET_ID, name: 'Flat', bandGains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
  { id: 'pop', name: 'Pop', bandGains: [4, 3, 2, 0, -2, -2, 0, 2, 3, 4] },
  { id: 'rock', name: 'Rock', bandGains: [5, 4, 3, 1, -1, -1, 1, 3, 4, 5] },
  { id: 'jazz', name: 'Jazz', bandGains: [3, 2, 1, 2, -2, -2, 0, 2, 3, 4] },
  { id: 'classical', name: 'Classical', bandGains: [4, 3, 2, 1, -1, -1, 0, 2, 3, 4] },
  { id: 'bass_boost', name: 'Bass Boost', bandGains: [6, 5, 4, 2, 0, 0, 0, 0, 0, 0] },
  { id: 'vocal_boost', name: 'Vocal Boost', bandGains: [-2, -2, -1, 1, 4, 4, 2, 1, 0, -1] },
  { id: 'treble_boost', name: 'Treble Boost', bandGains: [0, 0, 0, 0, 0, 1, 3, 5, 6, 7] }
]

export const DEFAULT_EQUALIZER_PRESETS: EqualizerPreset[] = DEFAULT_PRESET_DATA.map((preset) => ({
  id: preset.id,
  name: preset.name,
  isDefault: true,
  gains: {
    global: 0,
    bands: preset.bandGains.map((gain, index) => ({
      frequency: EQ_FREQUENCIES[index],
      gain,
      q: EQ_Q_DEFAULT_PEAK,
      type: 'peak' as FilterType,
      enabled: true
    }))
  }
}))

const DEFAULT_PRESET_IDS = new Set(DEFAULT_EQUALIZER_PRESETS.map((preset) => preset.id))
const DEFAULT_PRESET_BY_ID = new Map(DEFAULT_EQUALIZER_PRESETS.map((preset) => [preset.id, preset]))

const TEXT_NUMBER_PATTERN = '[+-]?(?:\\d+(?:\\.\\d*)?|\\.\\d+)'
const TEXT_PREAMP_PATTERN = new RegExp(`^Preamp\\s*:\\s*(${TEXT_NUMBER_PATTERN})\\s*dB\\b`, 'i')
const TEXT_FILTER_PATTERN = new RegExp(`^Filter\\s+(\\d+)\\s*:\\s*(ON|OFF)\\s+(.+?)\\s+Fc\\s*(${TEXT_NUMBER_PATTERN})\\s*Hz\\s+Gain\\s*(${TEXT_NUMBER_PATTERN})\\s*dB\\s+Q\\s*(${TEXT_NUMBER_PATTERN})\\s*$`, 'i')

const TEXT_FILTER_TYPE_MAP: Record<string, FilterType> = {
  pk: 'peak',
  peak: 'peak',
  peaking: 'peak',
  peakeq: 'peak',
  peakingeq: 'peak',
  bell: 'peak',
  ls: 'lowshelf',
  lsc: 'lowshelf',
  lowshelf: 'lowshelf',
  lowshelving: 'lowshelf',
  hs: 'highshelf',
  hsc: 'highshelf',
  highshelf: 'highshelf',
  highshelving: 'highshelf',
  lp: 'lowpass',
  lpf: 'lowpass',
  lowpass: 'lowpass',
  lowpassfilter: 'lowpass',
  hp: 'highpass',
  hpf: 'highpass',
  highpass: 'highpass',
  highpassfilter: 'highpass',
  no: 'notch',
  notch: 'notch',
  bandreject: 'notch',
  bandstop: 'notch'
}

const LEGACY_PRESET_NAME_TO_ID: Record<string, string> = {
  'Flat': 'flat',
  'Flat(原声)': 'flat',
  'Pop': 'pop',
  'Pop(流行)': 'pop',
  'Rock': 'rock',
  'Rock(摇滚)': 'rock',
  'Jazz': 'jazz',
  'Jazz(爵士)': 'jazz',
  'Classical': 'classical',
  'Classical(古典)': 'classical',
  'Bass Boost': 'bass_boost',
  'Bass Boost(低音增强)': 'bass_boost',
  'Vocal Boost': 'vocal_boost',
  'Vocal Boost(人声增强)': 'vocal_boost',
  'Treble Boost': 'treble_boost',
  'Treble Boost(高音增强)': 'treble_boost'
}

const isRecord = (value: unknown): value is Record<string, unknown> => (
  typeof value === 'object' && value !== null && !Array.isArray(value)
)

const isFiniteNumber = (value: unknown): value is number => (
  typeof value === 'number' && Number.isFinite(value)
)

const clonePreset = (preset: EqualizerPreset): EqualizerPreset => ({
  id: preset.id,
  name: preset.name,
  isDefault: preset.isDefault,
  gains: cloneGains(preset.gains)
})

const normalizeGainValue = (value: unknown, strict: boolean, label: string): number => {
  if (!isFiniteNumber(value)) {
    throw new Error(`${label} 不是有效数字`)
  }
  if (value < EQ_GAIN_MIN || value > EQ_GAIN_MAX) {
    if (strict) {
      throw new Error(`${label} 超出 ${EQ_GAIN_MIN}..${EQ_GAIN_MAX} dB 范围`)
    }
    return clampEqGain(value)
  }
  return value
}

const migrateBands = (bands: unknown): EqBand[] => {
  // If it's an array of numbers (v2 format), migrate
  if (Array.isArray(bands) && bands.length > 0 && typeof bands[0] === 'number') {
    return (bands as number[]).map((gain, index) => ({
      frequency: clampEqFreq(EQ_FREQUENCIES[index] ?? 1000),
      gain: clampEqGain(gain),
      q: EQ_Q_DEFAULT_PEAK,
      type: 'peak' as FilterType,
      enabled: true
    }))
  }
  // If it's already EqBand[], validate and fix
  if (Array.isArray(bands)) {
    return (bands as unknown[]).map((band, index) => {
      if (typeof band === 'object' && band !== null && !Array.isArray(band)) {
        const b = band as Record<string, unknown>
        return {
          frequency: clampEqFreq(typeof b.frequency === 'number' ? b.frequency : EQ_FREQUENCIES[index] ?? 1000),
          gain: clampEqGain(typeof b.gain === 'number' ? b.gain : 0),
          q: clampEqQ(typeof b.q === 'number' ? b.q : EQ_Q_DEFAULT_PEAK),
          type: isValidFilterType(b.type) ? b.type : 'peak',
          enabled: typeof b.enabled === 'boolean' ? b.enabled : true,
        }
      }
      return {
        frequency: EQ_FREQUENCIES[index] ?? 1000,
        gain: 0,
        q: EQ_Q_DEFAULT_PEAK,
        type: 'peak' as FilterType,
        enabled: true,
      }
    })
  }
  return createFlatBands()
}

const normalizeGains = (value: unknown, strict: boolean, label: string): EqualizerGains => {
  // v2 format: bare number array
  if (Array.isArray(value) && value.length > 0 && typeof value[0] === 'number') {
    return {
      global: 0,
      bands: migrateBands(value)
    }
  }

  if (isRecord(value)) {
    const global = normalizeGainValue(value.global, strict, `${label}.global`)
    const bands = migrateBands(value.bands)
    return { global, bands }
  }

  if (strict) throw new Error(`${label} 格式无效`)
  return createFlatGains()
}

const parseTextNumber = (value: string): number | null => {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

const normalizeTextFilterType = (value: string): FilterType | null => {
  const compact = value.trim().toLowerCase().replace(/[^a-z0-9]+/g, '')
  return TEXT_FILTER_TYPE_MAP[compact] ?? null
}

const parseTextPreset = (text: string): TextPresetImportResult => {
  const bands = createFlatBands()
  let global = 0
  let hasParsedBand = false

  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#') || line.startsWith('//')) continue

    if (/^Preamp\b/i.test(line)) {
      const match = line.match(TEXT_PREAMP_PATTERN)
      const parsed = match ? parseTextNumber(match[1]) : null
      if (parsed === null) return { success: false }
      global = clampEqGain(parsed)
      continue
    }

    if (!/^Filter\b/i.test(line)) continue

    const match = line.match(TEXT_FILTER_PATTERN)
    if (!match) return { success: false }

    const index = Number.parseInt(match[1], 10) - 1
    if (!Number.isInteger(index) || index < 0) return { success: false }
    if (index >= EQ_BAND_COUNT) continue

    const type = normalizeTextFilterType(match[3])
    const frequency = parseTextNumber(match[4])
    const gain = parseTextNumber(match[5])
    const q = parseTextNumber(match[6])
    if (!type || frequency === null || gain === null || q === null) {
      return { success: false }
    }

    bands[index] = {
      frequency: clampEqFreq(frequency),
      gain: type === 'lowpass' || type === 'highpass' || type === 'notch' ? 0 : clampEqGain(gain),
      q: clampEqQ(q),
      type,
      enabled: match[2].toUpperCase() === 'ON'
    }
    hasParsedBand = true
  }

  if (!hasParsedBand) return { success: false }

  return {
    success: true,
    gains: {
      global,
      bands
    }
  }
}

const getLegacyPresetIdByName = (name: string): string | undefined => {
  const trimmed = name.trim()
  return LEGACY_PRESET_NAME_TO_ID[trimmed] ?? LEGACY_PRESET_NAME_TO_ID[trimmed.replace(/\s+/g, ' ')]
}

const sanitizePresetIdPart = (value: string): string => (
  value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/^-+|-+$/g, '')
)

const createUniqueUserPresetId = (name: string, usedIds: Set<string>): string => {
  const base = sanitizePresetIdPart(name) || 'preset'
  let id = `user-${base}`
  let index = 2
  while (usedIds.has(id) || DEFAULT_PRESET_IDS.has(id)) {
    id = `user-${base}-${index}`
    index += 1
  }
  usedIds.add(id)
  return id
}

const validateImportedPresetId = (value: unknown, strict: boolean, name: string, usedIds: Set<string>): string => {
  if (typeof value !== 'string' || !value.trim()) {
    if (strict) throw new Error(`预设 "${name}" 缺少有效 id`)
    return createUniqueUserPresetId(name, usedIds)
  }

  const id = value.trim()
  if (!PRESET_ID_PATTERN.test(id)) {
    if (strict) throw new Error(`预设 "${name}" 的 id 格式无效`)
    return createUniqueUserPresetId(name, usedIds)
  }

  if (DEFAULT_PRESET_IDS.has(id)) {
    if (strict) throw new Error(`预设 "${name}" 使用了保留的默认预设 id`)
    return createUniqueUserPresetId(name, usedIds)
  }

  if (usedIds.has(id)) {
    if (strict) throw new Error(`预设 id "${id}" 重复`)
    return createUniqueUserPresetId(name, usedIds)
  }

  usedIds.add(id)
  return id
}

const getRawPresetList = (
  record: Record<string, unknown>,
  strict: boolean
): { list: unknown[]; source: 'userPresets' | 'presets' | 'none' } => {
  if ('userPresets' in record) {
    if (Array.isArray(record.userPresets)) {
      return { list: record.userPresets, source: 'userPresets' }
    }
    if (strict) throw new Error('userPresets 必须是数组')
  }

  if ('presets' in record) {
    if (Array.isArray(record.presets)) {
      return { list: record.presets, source: 'presets' }
    }
    if (strict) throw new Error('presets 必须是数组')
  }

  return { list: [], source: 'none' }
}

const normalizeUserPresets = (record: Record<string, unknown>, strict: boolean): EqualizerPreset[] => {
  const { list, source } = getRawPresetList(record, strict)
  const usedIds = new Set<string>()
  const result: EqualizerPreset[] = []

  list.forEach((item, index) => {
    if (!isRecord(item)) {
      if (strict) throw new Error(`预设 #${index + 1} 格式无效`)
      return
    }

    const rawName = item.name
    const name = typeof rawName === 'string' ? rawName.trim() : ''
    if (!name) {
      if (strict) throw new Error(`预设 #${index + 1} 缺少名称`)
      return
    }

    if ('isDefault' in item && typeof item.isDefault !== 'boolean') {
      if (strict) throw new Error(`预设 "${name}" 的 isDefault 必须是布尔值`)
      return
    }

    const rawId = typeof item.id === 'string' ? item.id.trim() : ''
    const legacyDefaultId = getLegacyPresetIdByName(name)
    const hasDefaultId = DEFAULT_PRESET_IDS.has(rawId)
    const markedDefault = item.isDefault === true
    const isLegacyBuiltinPreset = source === 'presets' && !rawId && Boolean(legacyDefaultId) && item.originalGains === undefined

    if (source === 'userPresets' && (markedDefault || hasDefaultId)) {
      if (strict) throw new Error(`用户预设 "${name}" 不能标记为默认预设`)
      return
    }

    if (source === 'presets' && (hasDefaultId || isLegacyBuiltinPreset)) {
      return
    }

    if (markedDefault) {
      if (strict) throw new Error(`预设 "${name}" 使用了无效的默认预设身份`)
      return
    }

    const gains = normalizeGains(item.gains, strict, `预设 "${name}".gains`)
    const shouldRequireId = strict && (source === 'userPresets' || Boolean(rawId))
    const id = validateImportedPresetId(rawId || undefined, shouldRequireId, name, usedIds)

    result.push({
      id,
      name,
      isDefault: false,
      gains
    })
  })

  return result
}

const findPresetInLists = (presetId: string, userPresets: EqualizerPreset[]): EqualizerPreset | undefined => {
  return DEFAULT_PRESET_BY_ID.get(presetId) ?? userPresets.find((preset) => preset.id === presetId)
}

const resolveCurrentPresetId = (
  record: Record<string, unknown>,
  userPresets: EqualizerPreset[],
  strict: boolean
): string => {
  const ids = new Set([...DEFAULT_PRESET_IDS, ...userPresets.map((preset) => preset.id)])

  if (typeof record.currentPresetId === 'string' && record.currentPresetId.trim()) {
    const id = record.currentPresetId.trim()
    if (ids.has(id)) return id
    if (strict) throw new Error(`当前预设 id "${id}" 不存在`)
  }

  if (typeof record.currentPreset === 'string' && record.currentPreset.trim()) {
    const legacyName = record.currentPreset.trim()
    const defaultId = getLegacyPresetIdByName(legacyName)
    if (defaultId && ids.has(defaultId)) return defaultId

    const userPreset = userPresets.find((preset) => preset.name === legacyName)
    if (userPreset) return userPreset.id

    if (strict) throw new Error(`当前预设 "${legacyName}" 不存在`)
  }

  return EQ_FLAT_PRESET_ID
}

const hydrateEqualizerState = (raw: unknown, strict: boolean): EqualizerHydratedState => {
  if (!isRecord(raw)) {
    if (strict) throw new Error('配置根节点必须是对象')
    return {
      version: EQ_CONFIG_VERSION,
      enabled: false,
      currentPresetId: EQ_FLAT_PRESET_ID,
      gains: createFlatGains(),
      userPresets: [],
      logs: []
    }
  }

  const userPresets = normalizeUserPresets(raw, strict)
  const currentPresetId = resolveCurrentPresetId(raw, userPresets, strict)
  const currentPreset = findPresetInLists(currentPresetId, userPresets)
  const rawGains = raw.gains
  const gains = rawGains === undefined
    ? (strict ? normalizeGains(rawGains, strict, 'gains') : cloneGains(currentPreset?.gains ?? createFlatGains()))
    : normalizeGains(rawGains, strict, 'gains')

  const logs = Array.isArray(raw.logs)
    ? raw.logs.filter((item): item is string => typeof item === 'string').slice(0, 100)
    : []

  return {
    version: EQ_CONFIG_VERSION,
    enabled: typeof raw.enabled === 'boolean' ? raw.enabled : false,
    currentPresetId,
    gains,
    userPresets,
    logs
  }
}

const loadPersistedState = (): EqualizerHydratedState => {
  if (typeof localStorage === 'undefined') {
    return hydrateEqualizerState(undefined, false)
  }

  const rawText = localStorage.getItem(EQ_STORAGE_KEY)
  if (!rawText) {
    return hydrateEqualizerState(undefined, false)
  }

  try {
    return hydrateEqualizerState(JSON.parse(rawText), false)
  } catch (error) {
    log.warn('读取均衡器持久化配置失败，已回退默认值', error)
    return hydrateEqualizerState(undefined, false)
  }
}

export const useEqualizerStore = defineStore('equalizer', () => {
  const initialState = loadPersistedState()

  const enabled = ref(initialState.enabled)
  const currentPresetId = ref(initialState.currentPresetId)
  const gains = ref<EqualizerGains>(cloneGains(initialState.gains))
  const userPresets = ref<EqualizerPreset[]>(initialState.userPresets.map(clonePreset))
  const logs = ref<string[]>([...initialState.logs])

  const defaultPresets = computed(() => DEFAULT_EQUALIZER_PRESETS.map(clonePreset))
  const presets = computed(() => [
    ...defaultPresets.value,
    ...userPresets.value.map(clonePreset)
  ])
  const currentPreset = computed(() => findPresetById(currentPresetId.value))

  let syncQueued = false

  function findPresetById(presetId: string): EqualizerPreset | undefined {
    const preset = findPresetInLists(presetId, userPresets.value)
    return preset ? clonePreset(preset) : undefined
  }

  function addLog(message: string) {
    const timestamp = new Date().toISOString()
    logs.value.unshift(`[${timestamp}] ${message}`)
    if (logs.value.length > 100) logs.value.pop()
  }

  function toPersistedState(): EqualizerPersistedState {
    return {
      version: EQ_CONFIG_VERSION,
      enabled: enabled.value,
      currentPresetId: currentPresetId.value,
      gains: cloneGains(gains.value),
      userPresets: userPresets.value.map(clonePreset),
      logs: [...logs.value]
    }
  }

  function persistState() {
    if (typeof localStorage === 'undefined') return
    try {
      localStorage.setItem(EQ_STORAGE_KEY, JSON.stringify(toPersistedState()))
    } catch (error) {
      log.warn('保存均衡器配置失败', error)
    }
  }

  function getBackendPayload() {
    return {
      enabled: enabled.value,
      globalGain: gains.value.global,
      global_gain: gains.value.global,
      bands: gains.value.bands.map(band => ({
        frequency: band.frequency,
        gain: band.gain,
        q: band.q,
        type: band.type,
        enabled: band.enabled
      }))
    }
  }

  async function syncToBackend() {
    try {
      const result = await invoke<{ success?: boolean; error?: string }>('player__set_eq_settings', getBackendPayload())
      if (result && result.success === false) {
        throw new Error(result.error || 'Rust 均衡器命令返回失败')
      }
    } catch (error) {
      log.warn('同步均衡器设置到 Rust 失败', error)
    }
  }

  function queueBackendSync() {
    if (syncQueued) return
    syncQueued = true
    queueMicrotask(() => {
      syncQueued = false
      void syncToBackend()
    })
  }

  function setEnabled(val: boolean) {
    enabled.value = val
    addLog(val ? i18n.global.t('play.equalizerEnabled') : i18n.global.t('play.equalizerDisabled'))
  }

  function setGlobalGain(gain: number) {
    gains.value = {
      ...gains.value,
      global: clampEqGain(gain)
    }
    addLog(`Adjusted global gain to ${gains.value.global.toFixed(1)}dB`)
  }

  function setBandGain(index: number, gain: number) {
    if (index < 0 || index >= EQ_BAND_COUNT) return
    const bands = [...gains.value.bands]
    bands[index] = { ...bands[index], gain: clampEqGain(gain) }
    gains.value = { ...gains.value, bands }
    addLog(`Adjusted band ${bands[index].frequency}Hz to ${clampEqGain(gain).toFixed(1)}dB`)
  }

  function setBandFrequency(index: number, frequency: number) {
    if (index < 0 || index >= EQ_BAND_COUNT) return
    const bands = [...gains.value.bands]
    bands[index] = { ...bands[index], frequency: clampEqFreq(frequency) }
    gains.value = { ...gains.value, bands }
  }

  function setBandQ(index: number, q: number) {
    if (index < 0 || index >= EQ_BAND_COUNT) return
    const bands = [...gains.value.bands]
    bands[index] = { ...bands[index], q: clampEqQ(q) }
    gains.value = { ...gains.value, bands }
  }

  function setBandType(index: number, type: FilterType) {
    if (index < 0 || index >= EQ_BAND_COUNT) return
    const bands = [...gains.value.bands]
    const defaultQ = getDefaultQ(type)
    bands[index] = { ...bands[index], type, q: defaultQ }
    // For LP/HP/Notch, gain is not applicable — set to 0
    if (['lowpass', 'highpass', 'notch'].includes(type)) {
      bands[index] = { ...bands[index], gain: 0 }
    }
    gains.value = { ...gains.value, bands }
  }

  function setGains(newGains: EqualizerGains | number[]) {
    gains.value = normalizeGains(newGains, false, 'gains')
    addLog(`${i18n.global.t('play.gainUpdated')} [${gains.value.bands.map((band) => band.gain.toFixed(1)).join(', ')}], global ${gains.value.global.toFixed(1)}`)
  }

  function applyPreset(presetId: string): boolean {
    const preset = findPresetById(presetId)
    if (!preset) return false
    currentPresetId.value = preset.id
    gains.value = cloneGains(preset.gains)
    addLog(`${i18n.global.t('play.presetSwitched')}: ${preset.name}`)
    return true
  }

  function setCurrentPreset(presetId: string) {
    applyPreset(presetId)
  }

  function resetToCurrentPreset(): boolean {
    return applyPreset(currentPresetId.value)
  }

  function isPresetNameTaken(name: string, excludeId?: string): boolean {
    const normalizedName = name.trim().toLocaleLowerCase()
    if (!normalizedName) return false
    return presets.value.some((preset) => (
      preset.id !== excludeId && preset.name.trim().toLocaleLowerCase() === normalizedName
    ))
  }

  function createUserPresetFromGains(name: string, sourceGains: EqualizerGains, applyGains = true): ImportResult {
    const trimmedName = name.trim()
    if (!trimmedName) return { success: false, error: i18n.global.t('settings.equalizer.textImportNameRequired') }
    if (isPresetNameTaken(trimmedName)) return { success: false, error: i18n.global.t('settings.equalizer.presetExists', { name: trimmedName }) }

    const usedIds = new Set(userPresets.value.map((preset) => preset.id))
    const id = createUniqueUserPresetId(trimmedName, usedIds)
    const presetGains = normalizeGains(sourceGains, false, `预设 "${trimmedName}".gains`)
    const preset: EqualizerPreset = {
      id,
      name: trimmedName,
      isDefault: false,
      gains: cloneGains(presetGains)
    }
    userPresets.value.push(preset)
    currentPresetId.value = id
    if (applyGains) {
      gains.value = cloneGains(presetGains)
    }
    addLog(`Saved new preset "${trimmedName}"`)
    return { success: true }
  }

  function createUserPreset(name: string): ImportResult {
    return createUserPresetFromGains(name, gains.value, false)
  }

  function updateUserPreset(presetId: string): ImportResult {
    const preset = userPresets.value.find((item) => item.id === presetId)
    if (!preset) return { success: false, error: '默认预设不能被覆盖' }
    preset.gains = cloneGains(gains.value)
    addLog(`Updated preset "${preset.name}"`)
    return { success: true }
  }

  function deleteUserPreset(presetId: string): ImportResult {
    const index = userPresets.value.findIndex((preset) => preset.id === presetId)
    if (index === -1) return { success: false, error: '默认预设不能被删除' }

    const [removed] = userPresets.value.splice(index, 1)
    if (currentPresetId.value === presetId) {
      applyPreset(EQ_FLAT_PRESET_ID)
    }
    addLog(`Deleted preset: ${removed.name}`)
    return { success: true }
  }

  function exportConfig(): EqualizerExportData {
    const exportedDefaultPresets = DEFAULT_EQUALIZER_PRESETS.map(clonePreset)
    const exportedUserPresets = userPresets.value.map(clonePreset)
    return {
      version: EQ_CONFIG_VERSION,
      enabled: enabled.value,
      currentPresetId: currentPresetId.value,
      gains: cloneGains(gains.value),
      presets: [...exportedDefaultPresets, ...exportedUserPresets],
      defaultPresets: exportedDefaultPresets,
      userPresets: exportedUserPresets
    }
  }

  function importConfig(raw: unknown): ImportResult {
    try {
      const nextState = hydrateEqualizerState(raw, true)
      enabled.value = nextState.enabled
      currentPresetId.value = nextState.currentPresetId
      gains.value = cloneGains(nextState.gains)
      userPresets.value = nextState.userPresets.map(clonePreset)
      addLog('Imported configuration')
      void syncToBackend()
      return { success: true }
    } catch (error) {
      const message = error instanceof Error ? error.message : '未知错误'
      log.warn('导入均衡器配置失败', error)
      return { success: false, error: message }
    }
  }

  watch(
    [enabled, currentPresetId, gains, userPresets, logs],
    persistState,
    { deep: true }
  )

  watch(
    [enabled, gains],
    queueBackendSync,
    { deep: true }
  )

  persistState()

  return {
    enabled,
    currentPresetId,
    currentPreset: currentPresetId,
    gains,
    userPresets,
    defaultPresets,
    presets,
    currentPresetDetail: currentPreset,
    logs,
    addLog,
    findPresetById,
    setEnabled,
    setGlobalGain,
    setBandGain,
    setBandFrequency,
    setBandQ,
    setBandType,
    setGains,
    setCurrentPreset,
    applyPreset,
    resetToCurrentPreset,
    isPresetNameTaken,
    createUserPreset,
    createUserPresetFromGains,
    parseTextPreset,
    updateUserPreset,
    deleteUserPreset,
    exportConfig,
    importConfig,
    syncToBackend
  }
})
