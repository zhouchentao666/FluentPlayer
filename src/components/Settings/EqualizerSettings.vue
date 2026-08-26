<template>
  <div class="equalizer-settings">
    <div class="settings-inline-header">
      <div class="eq-mobile-summary">
        <div class="eq-mobile-title">{{ currentPresetDisplayName }}</div>
        <div class="eq-mobile-status">{{ enabled ? t('settings.equalizer.on') : t('settings.equalizer.off') }}</div>
      </div>
      <t-space class="eq-header-actions" align="center">
        <t-switch
          v-model="enabled"
          :label="[t('settings.equalizer.on'), t('settings.equalizer.off')]"
          @change="(val: unknown) => handleEnabledChange(Boolean(val))"
        />
        <t-button theme="default" variant="text" @click="resetToCurrentPreset">
          {{ t('settings.equalizer.reset') }}
        </t-button>
      </t-space>
    </div>

    <div class="eq-content">
        <!-- Frequency Response Curve Canvas -->
        <div class="visualizer-container" ref="canvasContainerRef">
          <canvas ref="canvasRef"></canvas>
        </div>

        <!-- Band Gain Sliders -->
        <div class="sliders-container">
          <div
            v-for="(band, index) in gains.bands"
            :key="index"
            class="slider-group"
            :class="{ 'slider-group--selected': selectedBandIndex === index }"
            @click="selectedBandIndex = index"
          >
            <div class="slider-wrapper">
              <t-slider
                :value="band.gain"
                :min="EQ_GAIN_MIN"
                :max="EQ_GAIN_MAX"
                :step="0.1"
                layout="vertical"
                :show-tooltip="true"
                :disabled="!enabled || isGainDisabled(band.type)"
                @change="(val: number | number[]) => onBandGainChange(index, val as number)"
              />
            </div>
            <span class="freq-label">{{ formatFreq(band.frequency) }}</span>
            <span class="gain-label">{{ isGainDisabled(band.type) ? '--' : `${band.gain.toFixed(1)}dB` }}</span>
          </div>
        </div>

        <!-- Selected Band Parameter Panel -->
        <div v-if="selectedBandIndex >= 0 && selectedBandIndex < gains.bands.length" class="band-params-panel">
          <div class="band-params-header">
            <span class="band-params-title">{{ t('settings.equalizer.bandParams', { index: selectedBandIndex + 1 }) }}</span>
            <span class="band-params-freq">{{ formatFreq(gains.bands[selectedBandIndex].frequency) }}</span>
          </div>
          <div class="band-params-body">
            <div class="param-row">
              <label class="param-label">{{ t('settings.equalizer.frequency') }}</label>
              <div class="param-control">
                <input
                  type="range"
                  class="param-slider"
                  :min="0"
                  :max="1"
                  :step="0.001"
                  :value="freqToSliderValue(gains.bands[selectedBandIndex].frequency)"
                  :disabled="!enabled"
                  @input="onFreqSliderChange"
                />
              </div>
              <span class="param-value">{{ formatFreq(gains.bands[selectedBandIndex].frequency) }}</span>
            </div>
            <div class="param-row">
              <label class="param-label">{{ t('settings.equalizer.qFactor') }}</label>
              <div class="param-control">
                <input
                  type="range"
                  class="param-slider"
                  :min="EQ_Q_MIN"
                  :max="EQ_Q_MAX"
                  :step="0.01"
                  :value="gains.bands[selectedBandIndex].q"
                  :disabled="!enabled"
                  @input="onQSliderChange"
                />
              </div>
              <span class="param-value">{{ gains.bands[selectedBandIndex].q.toFixed(2) }}</span>
            </div>
            <div class="param-row">
              <label class="param-label">{{ t('settings.equalizer.filterType') }}</label>
              <div class="param-control">
                <t-select
                  :value="gains.bands[selectedBandIndex].type"
                  :disabled="!enabled"
                  size="small"
                  @change="(val: unknown) => onTypeChange(val as FilterType)"
                >
                  <t-option
                    v-for="ft in ALL_FILTER_TYPES"
                    :key="ft"
                    :value="ft"
                    :label="filterTypeLabel(ft)"
                  />
                </t-select>
              </div>
            </div>
          </div>
        </div>

        <!-- Preset Selector -->
        <div class="controls-row">
          <div class="preset-controls">
            <t-select
              v-model="selectedPresetId"
              :placeholder="t('settings.equalizer.selectPreset')"
              class="preset-select"
              @change="(val: unknown) => handlePresetChange(val as string)"
            >
              <t-option
                v-for="preset in presets"
                :key="preset.id"
                :label="displayPresetName(preset)"
                :value="preset.id"
              />
            </t-select>

            <t-button
              v-if="canModifyCurrentPreset"
              theme="primary"
              variant="text"
              size="small"
              @click="saveCurrentToPreset"
            >
              <template #icon><SaveIcon /></template>
              {{ t('settings.equalizer.savePreset') }}
            </t-button>

            <t-button
              v-if="canModifyCurrentPreset"
              theme="danger"
              variant="text"
              size="small"
              @click="confirmDeletePreset"
            >
              <template #icon><DeleteIcon /></template>
              {{ t('settings.equalizer.deletePreset') }}
            </t-button>
          </div>

          <div class="action-buttons">
            <t-button theme="primary" variant="outline" @click="savePresetDialogVisible = true">
              {{ t('settings.equalizer.saveAsPreset') }}
            </t-button>
            <t-button theme="default" variant="outline" @click="exportConfig">
              {{ t('settings.equalizer.exportConfig') }}
            </t-button>
            <t-button theme="default" variant="outline" @click="triggerImport">
              {{ t('settings.equalizer.importConfig') }}
            </t-button>
            <t-button theme="default" variant="outline" @click="openPresetVault">
              {{ t('settings.equalizer.downloadPresets') }}
            </t-button>
            <input
              ref="fileInputRef"
              type="file"
              accept=".json,.txt,application/json,text/plain"
              style="display: none"
              @change="handleFileImport"
            />
          </div>
        </div>

        <div class="mobile-action-grid">
          <t-button
            v-if="canModifyCurrentPreset"
            theme="primary"
            variant="outline"
            size="small"
            @click="saveCurrentToPreset"
          >
            <template #icon><SaveIcon /></template>
            {{ t('settings.equalizer.savePreset') }}
          </t-button>
          <t-button
            v-if="canModifyCurrentPreset"
            theme="danger"
            variant="outline"
            size="small"
            @click="confirmDeletePreset"
          >
            <template #icon><DeleteIcon /></template>
            {{ t('settings.equalizer.deletePreset') }}
          </t-button>
          <t-button theme="primary" variant="outline" size="small" @click="savePresetDialogVisible = true">
            {{ t('settings.equalizer.saveAsPreset') }}
          </t-button>
          <t-button theme="default" variant="outline" size="small" @click="exportConfig">
            {{ t('settings.equalizer.exportConfig') }}
          </t-button>
          <t-button theme="default" variant="outline" size="small" @click="triggerImport">
            {{ t('settings.equalizer.importConfig') }}
          </t-button>
          <t-button theme="default" variant="outline" size="small" @click="openPresetVault">
            {{ t('settings.equalizer.downloadPresets') }}
          </t-button>
        </div>

        <!-- Global Gain -->
        <div class="global-gain-control">
          <div class="global-gain-header">
            <span class="global-gain-title">{{ t('settings.equalizer.globalGain') }}</span>
            <span class="global-gain-value">{{ gains.global.toFixed(1) }}dB</span>
          </div>
          <t-slider
            v-model="gains.global"
            :min="EQ_GAIN_MIN"
            :max="EQ_GAIN_MAX"
            :step="0.1"
            :show-tooltip="true"
            :disabled="!enabled"
            @change="(val: number | number[]) => onGlobalGainChange(val as number)"
          />
      </div>
    </div>

    <t-dialog
      v-model:visible="savePresetDialogVisible"
      :header="t('settings.equalizer.saveNewPresetTitle')"
      @confirm="saveNewPreset"
    >
      <t-input v-model="newPresetName" :placeholder="t('settings.equalizer.presetNamePlaceholder')" />
    </t-dialog>

    <t-dialog
      v-model:visible="textImportDialogVisible"
      :header="t('settings.equalizer.textImportPresetTitle')"
      @confirm="saveTextImportPreset"
      @close="resetTextImportDialog"
    >
      <t-input
        v-model="textImportPresetName"
        :placeholder="t('settings.equalizer.textImportPresetNamePlaceholder')"
      />
    </t-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { DialogPlugin, MessagePlugin } from 'tdesign-vue-next'
import { DeleteIcon, SaveIcon } from 'tdesign-icons-vue-next'
import {
  ALL_FILTER_TYPES,
  EQ_FLAT_PRESET_ID,
  EQ_FREQ_MAX,
  EQ_FREQ_MIN,
  EQ_GAIN_MAX,
  EQ_GAIN_MIN,
  EQ_Q_MAX,
  EQ_Q_MIN,
  type EqualizerGains,
  type EqualizerPreset,
  type FilterType,
  useEqualizerStore
} from '@/store/Equalizer'

const { t } = useI18n()
const eqStore = useEqualizerStore()
const { enabled, currentPresetId, gains, presets, currentPresetDetail } = storeToRefs(eqStore)

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SAMPLE_RATE = 44100
const CANVAS_HEIGHT = 250
const SPECTRUM_BAND_COUNT = 128
const SILENCE_DB = -80
const RESPONSE_POINTS = 300
const PLOT_INSET_X = 16
const PLOT_INSET_Y = 16

const GRID_FREQS = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000]
const GRID_DB = [-24, -18, -12, -6, 0, 6, 12, 18, 24]
const PRESET_VAULT_URL = 'https://mio888888.github.io/EQVault/'

const FILTER_TYPE_I18N_KEYS: Record<FilterType, string> = {
  peak: 'settings.equalizer.typePeak',
  lowshelf: 'settings.equalizer.typeLowShelf',
  highshelf: 'settings.equalizer.typeHighShelf',
  lowpass: 'settings.equalizer.typeLowPass',
  highpass: 'settings.equalizer.typeHighPass',
  notch: 'settings.equalizer.typeNotch',
}

const GAIN_DISABLED_TYPES = new Set<string>(['lowpass', 'highpass', 'notch'])

const PRESET_LABEL_KEYS: Record<string, string> = {
  flat: 'settings.equalizer.presetFlat',
  pop: 'settings.equalizer.presetPop',
  rock: 'settings.equalizer.presetRock',
  jazz: 'settings.equalizer.presetJazz',
  classical: 'settings.equalizer.presetClassical',
  bass_boost: 'settings.equalizer.presetBassBoost',
  vocal_boost: 'settings.equalizer.presetVocalBoost',
  treble_boost: 'settings.equalizer.presetTrebleBoost'
}

// ---------------------------------------------------------------------------
// Refs
// ---------------------------------------------------------------------------

const canvasRef = ref<HTMLCanvasElement | null>(null)
const canvasContainerRef = ref<HTMLDivElement | null>(null)
const fileInputRef = ref<HTMLInputElement | null>(null)
const savePresetDialogVisible = ref(false)
const newPresetName = ref('')
const textImportDialogVisible = ref(false)
const textImportPresetName = ref('')
const pendingTextImportGains = ref<EqualizerGains | null>(null)
const selectedBandIndex = ref(-1)

let animationId = 0
let unlisten: UnlistenFn | null = null
let resizeObserver: ResizeObserver | null = null
let spectrumData = new Array<number>(SPECTRUM_BAND_COUNT).fill(SILENCE_DB)
let displaySpectrumData = new Array<number>(SPECTRUM_BAND_COUNT).fill(SILENCE_DB)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const formatFreq = (freq: number) => freq >= 1000 ? `${freq / 1000}k` : `${freq}`

const isGainDisabled = (type: FilterType) => GAIN_DISABLED_TYPES.has(type)

const filterTypeLabel = (type: FilterType) => t(FILTER_TYPE_I18N_KEYS[type])

const freqToSliderValue = (freq: number): number => {
  return Math.log10(freq / EQ_FREQ_MIN) / Math.log10(EQ_FREQ_MAX / EQ_FREQ_MIN)
}

const sliderValueToFreq = (val: number): number => {
  return EQ_FREQ_MIN * Math.pow(EQ_FREQ_MAX / EQ_FREQ_MIN, val)
}

// ---------------------------------------------------------------------------
// Frequency / dB mapping
// ---------------------------------------------------------------------------

const freqToX = (freq: number, width: number): number => {
  const minLog = Math.log10(20)
  const maxLog = Math.log10(20000)
  return PLOT_INSET_X + ((Math.log10(freq) - minLog) / (maxLog - minLog)) * Math.max(1, width - PLOT_INSET_X * 2)
}

const dbToY = (db: number, height: number): number => {
  return PLOT_INSET_Y + ((24 - db) / 48) * Math.max(1, height - PLOT_INSET_Y * 2)
}

const drawRoundedRect = (
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number
) => {
  if (typeof ctx.roundRect === 'function') {
    ctx.beginPath()
    ctx.roundRect(x, y, width, height, radius)
    ctx.fill()
    return
  }
  ctx.fillRect(x, y, width, height)
}

const prefersReducedMotion = (): boolean => {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

// ---------------------------------------------------------------------------
// Biquad math
// ---------------------------------------------------------------------------

interface BiquadCoeffs {
  b0: number
  b1: number
  b2: number
  a0: number
  a1: number
  a2: number
}

function calcBiquadCoeffs(
  type: FilterType,
  freq: number,
  gainDb: number,
  q: number,
  sampleRate: number
): BiquadCoeffs {
  const A = Math.pow(10, gainDb / 40)
  const omega = 2 * Math.PI * freq / sampleRate
  const cosW = Math.cos(omega)
  const sinW = Math.sin(omega)
  const alpha = sinW / (2 * q)

  switch (type) {
    case 'peak': {
      const b0 = 1 + alpha * A
      const b1 = -2 * cosW
      const b2 = 1 - alpha * A
      const a0 = 1 + alpha / A
      const a1 = -2 * cosW
      const a2 = 1 - alpha / A
      return { b0, b1, b2, a0, a1, a2 }
    }
    case 'lowshelf': {
      const sqrtA = Math.sqrt(A)
      const b0 = A * ((A + 1) - (A - 1) * cosW + 2 * sqrtA * alpha)
      const b1 = 2 * A * ((A - 1) - (A + 1) * cosW)
      const b2 = A * ((A + 1) - (A - 1) * cosW - 2 * sqrtA * alpha)
      const a0 = (A + 1) + (A - 1) * cosW + 2 * sqrtA * alpha
      const a1 = -2 * ((A - 1) + (A + 1) * cosW)
      const a2 = (A + 1) + (A - 1) * cosW - 2 * sqrtA * alpha
      return { b0, b1, b2, a0, a1, a2 }
    }
    case 'highshelf': {
      const sqrtA = Math.sqrt(A)
      const b0 = A * ((A + 1) + (A - 1) * cosW + 2 * sqrtA * alpha)
      const b1 = -2 * A * ((A - 1) + (A + 1) * cosW)
      const b2 = A * ((A + 1) + (A - 1) * cosW - 2 * sqrtA * alpha)
      const a0 = (A + 1) - (A - 1) * cosW + 2 * sqrtA * alpha
      const a1 = 2 * ((A - 1) - (A + 1) * cosW)
      const a2 = (A + 1) - (A - 1) * cosW - 2 * sqrtA * alpha
      return { b0, b1, b2, a0, a1, a2 }
    }
    case 'lowpass': {
      const b0 = (1 - cosW) / 2
      const b1 = 1 - cosW
      const b2 = (1 - cosW) / 2
      const a0 = 1 + alpha
      const a1 = -2 * cosW
      const a2 = 1 - alpha
      return { b0, b1, b2, a0, a1, a2 }
    }
    case 'highpass': {
      const b0 = (1 + cosW) / 2
      const b1 = -(1 + cosW)
      const b2 = (1 + cosW) / 2
      const a0 = 1 + alpha
      const a1 = -2 * cosW
      const a2 = 1 - alpha
      return { b0, b1, b2, a0, a1, a2 }
    }
    case 'notch': {
      const b0 = 1
      const b1 = -2 * cosW
      const b2 = 1
      const a0 = 1 + alpha
      const a1 = -2 * cosW
      const a2 = 1 - alpha
      return { b0, b1, b2, a0, a1, a2 }
    }
  }
}

function calcFilterResponseDb(
  coeffs: BiquadCoeffs,
  freq: number,
  sampleRate: number
): number {
  const omega = 2 * Math.PI * freq / sampleRate
  const cosW = Math.cos(omega)
  const sinW = Math.sin(omega)
  const { b0, b1, b2, a0, a1, a2 } = coeffs
  const cos2W = 2 * cosW * cosW - 1
  const realNum = b0 + b1 * cosW + b2 * cos2W
  const imagNum = -(b1 * sinW + b2 * 2 * sinW * cosW)
  const realDen = a0 + a1 * cosW + a2 * cos2W
  const imagDen = -(a1 * sinW + a2 * 2 * sinW * cosW)
  const magNum = Math.sqrt(realNum * realNum + imagNum * imagNum)
  const magDen = Math.sqrt(realDen * realDen + imagDen * imagDen)
  return 20 * Math.log10(magNum / Math.max(magDen, 1e-10))
}

// Pre-compute log-spaced frequency points (20Hz - 20kHz)
const freqPoints = (() => {
  const points: number[] = []
  const minLog = Math.log10(20)
  const maxLog = Math.log10(20000)
  for (let i = 0; i < RESPONSE_POINTS; i++) {
    const logFreq = minLog + (i / (RESPONSE_POINTS - 1)) * (maxLog - minLog)
    points.push(Math.pow(10, logFreq))
  }
  return points
})()

function calcTotalResponseDb(bands: { frequency: number; gain: number; q: number; type: FilterType; enabled: boolean }[], globalGain: number): number[] {
  return freqPoints.map(freq => {
    let totalDb = 0
    for (const band of bands) {
      if (!band.enabled) continue
      const coeffs = calcBiquadCoeffs(band.type, band.frequency, band.gain, band.q, SAMPLE_RATE)
      totalDb += calcFilterResponseDb(coeffs, freq, SAMPLE_RATE)
    }
    return totalDb + globalGain
  })
}

// ---------------------------------------------------------------------------
// Theme helpers
// ---------------------------------------------------------------------------

const readThemeValue = (name: string, fallback: string): string => {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return value || fallback
}

const clamp = (value: number, min: number, max: number): number => Math.min(max, Math.max(min, value))

const withAlpha = (color: string, alpha: number, fallback = 'rgba(0, 167, 77, 1)'): string => {
  const normalizedAlpha = clamp(alpha, 0, 1)
  const trimmed = color.trim()

  const fallbackMatch = fallback.match(/rgba?\(([^)]+)\)/i)
  const fallbackParts = fallbackMatch?.[1]
    .split(',')
    .map((part) => Number.parseFloat(part.trim())) ?? [0, 167, 77]
  const fallbackRgb = fallbackParts.slice(0, 3).map((part, index) => clamp(Number.isFinite(part) ? part : [0, 167, 77][index], 0, 255))

  const hexMatch = trimmed.match(/^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i)
  if (hexMatch) {
    const hex = hexMatch[1]
    const fullHex = hex.length === 3
      ? hex.split('').map((char) => char + char).join('')
      : hex.slice(0, 6)
    const rgb = [0, 2, 4].map((index) => Number.parseInt(fullHex.slice(index, index + 2), 16))
    return `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${normalizedAlpha})`
  }

  const rgbMatch = trimmed.match(/^rgba?\(([^)]+)\)$/i)
  if (rgbMatch) {
    const parts = rgbMatch[1].split(',').map((part) => Number.parseFloat(part.trim()))
    if (parts.length >= 3 && parts.slice(0, 3).every(Number.isFinite)) {
      const rgb = parts.slice(0, 3).map((part) => clamp(part, 0, 255))
      return `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${normalizedAlpha})`
    }
  }

  return `rgba(${fallbackRgb[0]}, ${fallbackRgb[1]}, ${fallbackRgb[2]}, ${normalizedAlpha})`
}

// ---------------------------------------------------------------------------
// Preset / display
// ---------------------------------------------------------------------------

const displayPresetName = (preset: EqualizerPreset) => {
  const key = PRESET_LABEL_KEYS[preset.id]
  return preset.isDefault && key ? t(key) : preset.name
}

const currentPresetDisplayName = computed(() => {
  const preset = currentPresetDetail.value
  return preset ? displayPresetName(preset) : t(PRESET_LABEL_KEYS[EQ_FLAT_PRESET_ID])
})

const selectedPresetId = computed({
  get: () => currentPresetId.value,
  set: (val: string) => {
    currentPresetId.value = val
  }
})

const canModifyCurrentPreset = computed(() => {
  const preset = currentPresetDetail.value
  return Boolean(preset && !preset.isDefault)
})

const localizedDefaultPresetNames = computed(() => new Set(
  presets.value
    .filter((preset) => preset.isDefault)
    .map((preset) => displayPresetName(preset).trim().toLocaleLowerCase())
))

// ---------------------------------------------------------------------------
// Event handlers
// ---------------------------------------------------------------------------

const handleEnabledChange = (val: boolean) => {
  eqStore.setEnabled(val)
}

const handlePresetChange = (val: string) => {
  if (!eqStore.applyPreset(val)) {
    MessagePlugin.error(t('settings.equalizer.presetNotExist'))
  }
}

const confirmDeletePreset = () => {
  if (!canModifyCurrentPreset.value) {
    MessagePlugin.warning(t('settings.equalizer.builtinNoDelete'))
    return
  }

  const dialog = DialogPlugin.confirm({
    header: t('settings.equalizer.deletePresetTitle'),
    body: t('settings.equalizer.deletePresetBody', { name: currentPresetDisplayName.value }),
    confirmBtn: { theme: 'danger', content: t('settings.equalizer.deletePreset') },
    onConfirm: () => {
      deleteCurrentPreset()
      dialog.destroy()
    }
  })
}

const deleteCurrentPreset = () => {
  const presetName = currentPresetDisplayName.value
  const result = eqStore.deleteUserPreset(currentPresetId.value)
  if (!result.success) {
    MessagePlugin.warning(result.error || t('settings.equalizer.builtinNoDelete'))
    return
  }
  MessagePlugin.success(t('settings.equalizer.presetDeleted', { name: presetName }))
}

const saveCurrentToPreset = () => {
  const presetName = currentPresetDisplayName.value
  const result = eqStore.updateUserPreset(currentPresetId.value)
  if (!result.success) {
    MessagePlugin.warning(result.error || t('settings.equalizer.builtinNoModify'))
    return
  }
  MessagePlugin.success(t('settings.equalizer.savedToPreset', { name: presetName }))
}

const onGlobalGainChange = (val: number) => {
  eqStore.setGlobalGain(val)
}

const onBandGainChange = (index: number, val: number) => {
  eqStore.setBandGain(index, val)
}

const onFreqSliderChange = (event: Event) => {
  const input = event.target as HTMLInputElement
  const freq = sliderValueToFreq(parseFloat(input.value))
  eqStore.setBandFrequency(selectedBandIndex.value, freq)
  updateSliderTrack(input)
}

const onQSliderChange = (event: Event) => {
  const input = event.target as HTMLInputElement
  eqStore.setBandQ(selectedBandIndex.value, parseFloat(input.value))
  updateSliderTrack(input)
}

/**
 * Update the native range slider background to show a filled track effect.
 * The left portion (from start to current value) is rendered in brand color,
 * the right portion remains in the component background color.
 */
const updateSliderTrack = (el: HTMLInputElement) => {
  const min = parseFloat(el.min)
  const max = parseFloat(el.max)
  const val = parseFloat(el.value)
  const pct = ((val - min) / (max - min)) * 100
  el.style.background =
    `linear-gradient(to right, var(--td-brand-color) ${pct}%, var(--td-bg-color-component) ${pct}%)`
}

/**
 * Initialize filled track styles on all param-slider elements inside the panel.
 */
const initAllSliderTracks = () => {
  const panel = document.querySelector('.band-params-panel')
  if (!panel) return
  const sliders = panel.querySelectorAll<HTMLInputElement>('.param-slider')
  sliders.forEach(updateSliderTrack)
}

const onTypeChange = (type: FilterType) => {
  eqStore.setBandType(selectedBandIndex.value, type)
}

const resetToCurrentPreset = () => {
  if (eqStore.resetToCurrentPreset()) {
    MessagePlugin.success(t('settings.equalizer.resetToPreset', { name: currentPresetDisplayName.value }))
    return
  }

  eqStore.applyPreset(EQ_FLAT_PRESET_ID)
  MessagePlugin.success(t('settings.equalizer.resetToFlat'))
}

const saveNewPreset = () => {
  const name = newPresetName.value.trim()
  if (!name) return

  if (localizedDefaultPresetNames.value.has(name.toLocaleLowerCase())) {
    MessagePlugin.warning(t('settings.equalizer.builtinNameConflict', { name }))
    return
  }

  const result = eqStore.createUserPreset(name)
  if (!result.success) {
    MessagePlugin.warning(result.error || t('settings.equalizer.presetExists', { name }))
    return
  }

  savePresetDialogVisible.value = false
  newPresetName.value = ''
  MessagePlugin.success(t('settings.equalizer.presetSaveSuccess'))
}

const exportConfig = () => {
  const data = JSON.stringify(eqStore.exportConfig(), null, 2)
  const blob = new Blob([data], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'mio-eq-config.json'
  a.click()
  URL.revokeObjectURL(url)
  eqStore.addLog('Exported configuration')
}

const triggerImport = () => {
  fileInputRef.value?.click()
}

const openPresetVault = async () => {
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener')
    await openUrl(PRESET_VAULT_URL)
  } catch {
    window.open(PRESET_VAULT_URL, '_blank')
  }
}

const getFilePresetName = (fileName: string): string => {
  const lastDotIndex = fileName.lastIndexOf('.')
  const baseName = lastDotIndex > 0 ? fileName.slice(0, lastDotIndex) : fileName
  return baseName.trim()
}

const resetTextImportDialog = () => {
  textImportDialogVisible.value = false
  textImportPresetName.value = ''
  pendingTextImportGains.value = null
}

const openTextImportDialog = (sourceName: string, importedGains: EqualizerGains) => {
  pendingTextImportGains.value = importedGains
  textImportPresetName.value = sourceName
  textImportDialogVisible.value = true
}

const saveTextImportPreset = () => {
  const importedGains = pendingTextImportGains.value
  if (!importedGains) {
    resetTextImportDialog()
    return
  }

  const name = textImportPresetName.value.trim()
  if (!name) {
    MessagePlugin.warning(t('settings.equalizer.textImportNameRequired'))
    return
  }

  if (localizedDefaultPresetNames.value.has(name.toLocaleLowerCase())) {
    MessagePlugin.warning(t('settings.equalizer.builtinNameConflict', { name }))
    return
  }

  if (eqStore.isPresetNameTaken(name)) {
    MessagePlugin.warning(t('settings.equalizer.presetExists', { name }))
    return
  }

  const result = eqStore.createUserPresetFromGains(name, importedGains)
  if (!result.success) {
    MessagePlugin.warning(result.error || t('settings.equalizer.presetExists', { name }))
    return
  }

  resetTextImportDialog()
  MessagePlugin.success(t('settings.equalizer.textImportSuccess', { name }))
}

const handleFileImport = async (event: Event) => {
  const input = event.target as HTMLInputElement
  if (!input.files?.length) return

  const file = input.files[0]
  try {
    const text = await file.text()
    try {
      const data = JSON.parse(text)
      const result = eqStore.importConfig(data)
      if (!result.success) {
        MessagePlugin.error(result.error || t('settings.equalizer.importFailed'))
        return
      }
      MessagePlugin.success(t('settings.equalizer.importSuccess'))
      return
    } catch {
      const result = eqStore.parseTextPreset(text)
      if (!result.success || !result.gains) {
        MessagePlugin.error(t('settings.equalizer.textImportParseFailed'))
        return
      }
      openTextImportDialog(getFilePresetName(file.name), result.gains)
    }
  } catch {
    MessagePlugin.error(t('settings.equalizer.importFailed'))
  } finally {
    input.value = ''
  }
}

// ---------------------------------------------------------------------------
// Spectrum listener
// ---------------------------------------------------------------------------

interface SpectrumPayload {
  bands?: unknown
}

// ---------------------------------------------------------------------------
// Canvas rendering
// ---------------------------------------------------------------------------

const resizeCanvas = () => {
  if (!canvasRef.value || !canvasContainerRef.value) return
  const rect = canvasContainerRef.value.getBoundingClientRect()
  const dpr = window.devicePixelRatio || 1
  const cssHeight = window.matchMedia('(max-width: 768px)').matches ? 180 : CANVAS_HEIGHT
  canvasRef.value.width = Math.max(1, Math.floor(rect.width * dpr))
  canvasRef.value.height = Math.max(1, Math.floor(cssHeight * dpr))
  if (rect.width > 0) {
    canvasRef.value.style.width = `${rect.width}px`
  }
  canvasRef.value.style.height = `${cssHeight}px`
}

const setupVisualizer = async () => {
  if (!canvasRef.value) return
  resizeCanvas()

  // Set up spectrum listener (non-blocking, failures don't prevent visualizer)
  try {
    unlisten = await listen<SpectrumPayload>('player:spectrum', (event) => {
      const { bands } = event.payload
      if (!Array.isArray(bands)) return

      const nextSpectrumData = new Array<number>(SPECTRUM_BAND_COUNT).fill(SILENCE_DB)
      const len = Math.min(bands.length, SPECTRUM_BAND_COUNT)
      for (let i = 0; i < len; i++) {
        const band = bands[i]
        nextSpectrumData[i] = typeof band === 'number' && Number.isFinite(band) ? band : SILENCE_DB
      }
      spectrumData = nextSpectrumData
    })
  } catch {
    // Spectrum data won't be available, but visualizer still works
  }

  // Draw loop (independent of spectrum listener)
  const ctx = canvasRef.value.getContext('2d')
  if (!ctx) return

  const drawLayer = (name: string, fn: () => void) => {
    try {
      fn()
    } catch (error) {
      console.warn(`[EqualizerSettings] Failed to draw ${name} layer`, error)
    }
  }

  const draw = () => {
    if (!ctx || !canvasRef.value) return
    animationId = requestAnimationFrame(draw)

    const dpr = window.devicePixelRatio || 1
    const width = canvasRef.value.width / dpr
    const height = canvasRef.value.height / dpr

    ctx.save()
    try {
      ctx.scale(dpr, dpr)

      // --- Layer 0: Background + Grid ---
      drawLayer('background', () => drawBackground(ctx, width, height))

      // --- Layer 1: Spectrum Analyzer ---
      drawLayer('spectrum', () => drawSpectrum(ctx, width, height))

      // --- Layer 2: EQ Response Curve ---
      drawLayer('eq-curve', () => drawEQCurve(ctx, width, height))

      // --- Layer 3: Band Markers ---
      drawLayer('band-markers', () => drawBandMarkers(ctx, width, height))
    } finally {
      ctx.restore()
    }
  }
  draw()
}

function drawBackground(ctx: CanvasRenderingContext2D, width: number, height: number) {
  const bgColor = readThemeValue('--settings-eq-visualizer-bg', readThemeValue('--td-bg-color-page', '#101418'))
  const brandColor = readThemeValue('--td-brand-color', '#00a74d')
  const textColor = readThemeValue('--td-text-color-primary', '#ffffff')
  const gridColor = withAlpha(textColor, 0.055, 'rgba(255,255,255,1)')
  const subGridColor = withAlpha(textColor, 0.035, 'rgba(255,255,255,1)')
  const zeroLineColor = withAlpha(brandColor, 0.34)
  const labelColor = withAlpha(textColor, 0.38, 'rgba(255,255,255,1)')
  const reducedMotion = prefersReducedMotion()

  const bgGradient = ctx.createLinearGradient(0, 0, 0, height)
  bgGradient.addColorStop(0, withAlpha(bgColor, 1, '#101418'))
  bgGradient.addColorStop(0.48, withAlpha(bgColor, 0.94, '#101418'))
  bgGradient.addColorStop(1, withAlpha(brandColor, 0.045))
  ctx.fillStyle = bgGradient
  ctx.fillRect(0, 0, width, height)

  ctx.save()
  ctx.lineCap = 'butt'
  ctx.font = '10px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace'
  ctx.textBaseline = 'middle'

  const top = PLOT_INSET_Y
  const bottom = height - PLOT_INSET_Y
  const left = PLOT_INSET_X
  const right = width - PLOT_INSET_X

  for (const freq of GRID_FREQS) {
    const x = Math.round(freqToX(freq, width)) + 0.5
    ctx.beginPath()
    ctx.moveTo(x, top)
    ctx.lineTo(x, bottom)
    ctx.strokeStyle = freq === 1000 || freq === 10000 ? gridColor : subGridColor
    ctx.lineWidth = freq === 1000 || freq === 10000 ? 0.8 : 0.5
    ctx.stroke()

    ctx.fillStyle = labelColor
    ctx.textAlign = 'center'
    ctx.fillText(formatFreq(freq), x, height - 7)
  }

  for (const db of GRID_DB) {
    const y = Math.round(dbToY(db, height)) + 0.5
    ctx.beginPath()
    ctx.moveTo(left, y)
    ctx.lineTo(right, y)

    if (db === 0) {
      ctx.save()
      if (!reducedMotion) {
        ctx.shadowColor = withAlpha(brandColor, 0.22)
        ctx.shadowBlur = 8
      }
      ctx.setLineDash([6, 7])
      ctx.strokeStyle = zeroLineColor
      ctx.lineWidth = 1.15
      ctx.stroke()
      ctx.restore()
    } else {
      ctx.strokeStyle = db % 12 === 0 ? gridColor : subGridColor
      ctx.lineWidth = db % 12 === 0 ? 0.7 : 0.5
      ctx.stroke()
    }

    if (db % 6 === 0) {
      ctx.fillStyle = labelColor
      ctx.textAlign = 'left'
      ctx.fillText(db > 0 ? `+${db}` : `${db}`, 5, y)
    }
  }

  ctx.strokeStyle = withAlpha(textColor, 0.045, 'rgba(255,255,255,1)')
  ctx.lineWidth = 1
  ctx.strokeRect(PLOT_INSET_X + 0.5, PLOT_INSET_Y + 0.5, Math.max(1, width - PLOT_INSET_X * 2 - 1), Math.max(1, height - PLOT_INSET_Y * 2 - 1))
  ctx.restore()
}

function drawSpectrum(ctx: CanvasRenderingContext2D, width: number, height: number) {
  const barCount = SPECTRUM_BAND_COUNT
  const plotWidth = Math.max(1, width - PLOT_INSET_X * 2)
  const barWidth = plotWidth / barCount
  const visibleBarWidth = Math.max(1, barWidth * 0.55)
  const baseline = height - PLOT_INSET_Y
  const maxBarHeight = Math.max(1, (height - PLOT_INSET_Y * 2) * 0.48)

  const barStartColor = readThemeValue('--settings-eq-visualizer-bar-start', readThemeValue('--td-brand-color-5', '#00a74d'))
  const barEndColor = readThemeValue('--settings-eq-visualizer-bar-end', readThemeValue('--td-brand-color-7', '#03de6d'))
  const hasSignal = spectrumData.some((value) => value > SILENCE_DB + 3)

  for (let i = 0; i < barCount; i++) {
    const target = typeof spectrumData[i] === 'number' && Number.isFinite(spectrumData[i]) ? spectrumData[i] : SILENCE_DB
    displaySpectrumData[i] += (target - displaySpectrumData[i]) * 0.18

    const normalized = clamp((displaySpectrumData[i] - SILENCE_DB) / 80, 0, 1)
    const floorAmount = hasSignal ? 0 : 0.018 + (Math.sin(i * 1.73) + 1) * 0.007
    const barHeight = Math.max(floorAmount * maxBarHeight, Math.pow(normalized, 0.68) * maxBarHeight)
    if (barHeight < 0.6) continue

    const x = PLOT_INSET_X + i * barWidth + (barWidth - visibleBarWidth) / 2
    const y = baseline - barHeight
    const gradient = ctx.createLinearGradient(0, baseline, 0, y)
    gradient.addColorStop(0, withAlpha(barStartColor, hasSignal ? 0.02 : 0.025))
    gradient.addColorStop(0.3, withAlpha(barStartColor, hasSignal ? 0.11 : 0.04))
    gradient.addColorStop(1, withAlpha(barEndColor, hasSignal ? 0.28 : 0.07))
    ctx.fillStyle = gradient
    drawRoundedRect(ctx, x, y, visibleBarWidth, barHeight, Math.min(2.5, visibleBarWidth / 2))
  }
}

function drawEQCurve(ctx: CanvasRenderingContext2D, width: number, height: number) {
  const response = calcTotalResponseDb(gains.value.bands, gains.value.global)
  const curveColor = readThemeValue('--td-brand-color', '#00a74d')
  const reducedMotion = prefersReducedMotion()
  const path = new Path2D()
  const fillPath = new Path2D()
  const zeroY = dbToY(0, height)
  const firstX = freqToX(freqPoints[0], width)
  const lastX = freqToX(freqPoints[freqPoints.length - 1], width)

  let started = false
  for (let i = 0; i < freqPoints.length; i++) {
    const x = freqToX(freqPoints[i], width)
    const clampedDb = clamp(response[i], -24, 24)
    const y = dbToY(clampedDb, height)
    if (!started) {
      path.moveTo(x, y)
      fillPath.moveTo(x, zeroY)
      fillPath.lineTo(x, y)
      started = true
    } else {
      path.lineTo(x, y)
      fillPath.lineTo(x, y)
    }
  }
  fillPath.lineTo(lastX, zeroY)
  fillPath.lineTo(firstX, zeroY)
  fillPath.closePath()

  const fillGradient = ctx.createLinearGradient(0, PLOT_INSET_Y, 0, height - PLOT_INSET_Y)
  fillGradient.addColorStop(0, withAlpha(curveColor, 0.16))
  fillGradient.addColorStop(0.48, withAlpha(curveColor, 0.055))
  fillGradient.addColorStop(1, withAlpha(curveColor, 0.012))
  ctx.fillStyle = fillGradient
  ctx.fill(fillPath)

  ctx.save()
  ctx.lineJoin = 'round'
  ctx.lineCap = 'round'
  if (!reducedMotion) {
    ctx.shadowColor = withAlpha(curveColor, 0.24)
    ctx.shadowBlur = 10
  }
  ctx.strokeStyle = withAlpha(curveColor, reducedMotion ? 0.18 : 0.24)
  ctx.lineWidth = reducedMotion ? 5.5 : 8
  ctx.stroke(path)
  ctx.restore()

  ctx.save()
  ctx.lineJoin = 'round'
  ctx.lineCap = 'round'
  ctx.strokeStyle = withAlpha(curveColor, 0.92)
  ctx.lineWidth = 2.2
  ctx.stroke(path)
  ctx.restore()
}

function drawBandMarkers(ctx: CanvasRenderingContext2D, width: number, height: number) {
  const curveColor = readThemeValue('--td-brand-color', '#00a74d')
  const textColor = readThemeValue('--td-text-color-primary', '#ffffff')
  const bands = gains.value.bands
  const reducedMotion = prefersReducedMotion()

  for (let i = 0; i < bands.length; i++) {
    const band = bands[i]
    const x = freqToX(band.frequency, width)
    const effectiveGain = GAIN_DISABLED_TYPES.has(band.type) ? 0 : band.gain
    const y = dbToY(clamp(effectiveGain, -24, 24), height)
    const isSelected = i === selectedBandIndex.value

    if (isSelected) {
      ctx.save()
      if (!reducedMotion) {
        ctx.shadowColor = withAlpha(curveColor, 0.32)
        ctx.shadowBlur = 12
      }
      ctx.beginPath()
      ctx.arc(x, y, 11, 0, Math.PI * 2)
      ctx.strokeStyle = withAlpha(curveColor, 0.34)
      ctx.lineWidth = 3
      ctx.stroke()
      ctx.restore()

      ctx.beginPath()
      ctx.arc(x, y, 6.2, 0, Math.PI * 2)
      ctx.fillStyle = withAlpha(curveColor, 0.86)
      ctx.fill()
      ctx.strokeStyle = withAlpha(textColor, 0.86, 'rgba(255,255,255,1)')
      ctx.lineWidth = 1.6
      ctx.stroke()

      ctx.beginPath()
      ctx.arc(x, y, 2.2, 0, Math.PI * 2)
      ctx.fillStyle = withAlpha(textColor, 0.7, 'rgba(255,255,255,1)')
      ctx.fill()
    } else {
      ctx.beginPath()
      ctx.arc(x, y, 6, 0, Math.PI * 2)
      ctx.strokeStyle = withAlpha(curveColor, 0.28)
      ctx.lineWidth = 1.2
      ctx.stroke()

      ctx.beginPath()
      ctx.arc(x, y, 3.2, 0, Math.PI * 2)
      ctx.fillStyle = withAlpha(curveColor, 0.42)
      ctx.fill()
      ctx.strokeStyle = withAlpha(textColor, 0.18, 'rgba(255,255,255,1)')
      ctx.lineWidth = 0.8
      ctx.stroke()
    }
  }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

onMounted(() => {
  setupVisualizer()
  if (canvasContainerRef.value) {
    resizeObserver = new ResizeObserver(() => {
      resizeCanvas()
    })
    resizeObserver.observe(canvasContainerRef.value)
  }
  window.addEventListener('resize', resizeCanvas)
  initAllSliderTracks()
})

// Re-initialize slider tracks when the selected band changes (panel re-renders)
watch(selectedBandIndex, () => {
  nextTick(initAllSliderTracks)
})

onUnmounted(() => {
  if (animationId) cancelAnimationFrame(animationId)
  if (unlisten) {
    unlisten()
    unlisten = null
  }
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
  window.removeEventListener('resize', resizeCanvas)
})
</script>

<style scoped>
.equalizer-settings {
  color: var(--td-text-color-primary);
}
.settings-inline-header {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 14px;
}
.eq-mobile-summary {
  display: none;
}
.mobile-action-grid {
  display: none;
}
.eq-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* --- Canvas visualizer --- */
.visualizer-container {
  width: 100%;
  background: var(--settings-eq-visualizer-bg, var(--td-bg-color-page));
  border: 1px solid color-mix(in srgb, var(--td-brand-color), var(--td-border-level-1-color) 76%);
  border-radius: 12px;
  overflow: hidden;
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, var(--td-text-color-anti), transparent 94%),
    inset 0 -18px 42px color-mix(in srgb, var(--td-brand-color), transparent 94%),
    0 10px 28px color-mix(in srgb, var(--td-brand-color), transparent 92%);
}
.visualizer-container canvas {
  width: 100%;
  height: 250px;
  display: block;
}

/* --- Band sliders --- */
.sliders-container {
  display: flex;
  justify-content: space-between;
  height: 280px;
  padding: 20px 0;
  gap: 4px;
}
.slider-group {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
  padding: 8px 2px;
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.2s;
}
.slider-group:hover {
  background: color-mix(in srgb, var(--td-bg-color-container), var(--td-brand-color) 8%);
}
.slider-group--selected {
  background: color-mix(in srgb, var(--td-bg-color-container), var(--td-brand-color) 12%);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--td-brand-color), transparent 60%);
}
.slider-wrapper {
  height: 210px;
  display: flex;
  justify-content: center;
}
.freq-label {
  margin-top: 10px;
  font-size: 12px;
  color: var(--td-text-color-secondary);
}
.gain-label {
  margin-top: 4px;
  font-size: 10px;
  color: var(--td-text-color-secondary);
}

/* --- Band params panel --- */
.band-params-panel {
  padding: 16px 20px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--td-bg-color-container), var(--td-brand-color) 5%);
  border: 1px solid color-mix(in srgb, var(--td-border-level-1-color), var(--td-brand-color) 12%);
}
.band-params-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 14px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--td-border-level-1-color);
}
.band-params-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--td-text-color-primary);
}
.band-params-freq {
  font-size: 12px;
  color: var(--td-text-color-secondary);
  font-variant-numeric: tabular-nums;
}
.band-params-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.param-row {
  display: flex;
  align-items: center;
  gap: 14px;
}
/* Add a subtle divider between param rows, except after the last */
.param-row + .param-row {
  padding-top: 12px;
  border-top: 1px solid color-mix(in srgb, var(--td-component-border), transparent 40%);
}
.param-label {
  width: 64px;
  flex-shrink: 0;
  font-size: 12px;
  color: var(--td-text-color-secondary);
  user-select: none;
}
.param-control {
  flex: 1;
  min-width: 0;
}
.param-value {
  width: 56px;
  flex-shrink: 0;
  font-size: 12px;
  text-align: right;
  color: var(--td-text-color-primary);
  font-variant-numeric: tabular-nums;
}
/* --- Custom range slider (param-slider) --- */
.param-slider {
  width: 100%;
  height: 6px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--td-bg-color-component);
  border-radius: 3px;
  outline: none;
  cursor: pointer;
  transition: opacity 0.2s;
}
.param-slider::-webkit-slider-runnable-track {
  height: 6px;
  border-radius: 3px;
}
.param-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--td-brand-color);
  cursor: pointer;
  border: 2px solid var(--td-bg-color-container);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
  margin-top: -5px;
  transition: box-shadow 0.15s, transform 0.15s;
}
.param-slider::-webkit-slider-thumb:hover {
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--td-brand-color), transparent 75%);
  transform: scale(1.1);
}
.param-slider::-webkit-slider-thumb:active {
  transform: scale(1.15);
}
.param-slider::-moz-range-track {
  height: 6px;
  border-radius: 3px;
  background: var(--td-bg-color-component);
}
.param-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--td-brand-color);
  cursor: pointer;
  border: 2px solid var(--td-bg-color-container);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
}
.param-slider::-moz-range-thumb:hover {
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--td-brand-color), transparent 75%);
}
.param-slider:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.param-slider:disabled::-webkit-slider-thumb {
  cursor: not-allowed;
}

/* Filter type select — match param-row visual weight */
.band-params-body .param-control :deep(.t-select) {
  width: 100%;
}
.band-params-body .param-control :deep(.t-input--small) {
  height: 32px;
  border-radius: 6px;
  font-size: 12px;
}

/* --- Controls row --- */
.controls-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
}
.preset-controls {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.preset-select {
  width: 180px;
}
.action-buttons {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

/* --- Global gain --- */
.global-gain-control {
  padding: 12px 16px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--td-bg-color-container), var(--td-brand-color) 5%);
  border: 1px solid var(--td-border-level-1-color);
}
.global-gain-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}
.global-gain-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--td-text-color-primary);
}
.global-gain-value {
  font-size: 12px;
  color: var(--td-text-color-secondary);
}

@media (max-width: 768px) {
  .equalizer-settings {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .settings-inline-header {
    align-items: stretch;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 0;
    padding: 14px;
    border: 1px solid var(--mobile-glass-border);
    border-radius: var(--mobile-card-radius-small);
    background: color-mix(in srgb, var(--td-bg-color-container) 80%, transparent);
  }

  .eq-mobile-summary {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    justify-content: center;
    gap: 4px;
  }

  .eq-mobile-title {
    overflow: hidden;
    color: var(--td-text-color-primary);
    font-size: 15px;
    font-weight: 600;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .eq-mobile-status {
    color: var(--td-text-color-secondary);
    font-size: 12px;
    line-height: 1.3;
  }

  .eq-header-actions {
    flex: 0 0 auto;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .eq-header-actions :deep(.t-button) {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
  }

  .eq-content {
    gap: 12px;
  }

  .visualizer-container,
  .sliders-container,
  .band-params-panel,
  .controls-row,
  .global-gain-control {
    border: 1px solid var(--mobile-glass-border);
    border-radius: var(--mobile-card-radius-small);
    background: color-mix(in srgb, var(--td-bg-color-container) 82%, transparent);
  }

  .visualizer-container {
    box-shadow:
      inset 0 1px 0 color-mix(in srgb, var(--td-text-color-anti), transparent 95%),
      0 8px 18px color-mix(in srgb, var(--td-brand-color), transparent 94%);
  }

  .visualizer-container canvas {
    height: 180px;
  }

  .controls-row {
    display: block;
    padding: 12px;
  }

  .preset-controls {
    display: grid;
    grid-template-columns: 1fr;
    gap: 10px;
  }

  .preset-select {
    width: 100%;
  }

  .controls-row .preset-controls > .t-button,
  .action-buttons {
    display: none;
  }

  .mobile-action-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .mobile-action-grid :deep(.t-button) {
    min-width: 0;
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    white-space: normal;
    touch-action: manipulation;
  }

  .sliders-container {
    height: 232px;
    gap: 2px;
    padding: 14px 6px 12px;
    overflow: hidden;
  }

  .slider-group {
    min-width: 0;
    min-height: var(--mobile-touch-target);
    padding: 8px 1px 6px;
    border-radius: 12px;
    touch-action: manipulation;
  }

  .slider-wrapper {
    height: 168px;
  }

  .freq-label {
    margin-top: 8px;
    font-size: 10px;
    line-height: 1.2;
    white-space: nowrap;
  }

  .gain-label {
    margin-top: 3px;
    font-size: 9px;
    line-height: 1.2;
    white-space: nowrap;
  }

  .band-params-panel,
  .global-gain-control {
    padding: 14px;
  }

  .band-params-header {
    margin-bottom: 12px;
    padding-bottom: 10px;
  }

  .param-row {
    display: grid;
    grid-template-columns: minmax(72px, auto) 1fr minmax(48px, auto);
    gap: 10px;
  }

  .param-label,
  .param-value {
    width: auto;
  }

  .param-slider {
    height: 8px;
  }

  .param-slider::-webkit-slider-runnable-track {
    height: 8px;
  }

  .param-slider::-webkit-slider-thumb {
    width: 22px;
    height: 22px;
    margin-top: -7px;
  }

  .param-slider::-moz-range-track {
    height: 8px;
  }

  .param-slider::-moz-range-thumb {
    width: 22px;
    height: 22px;
  }

  .band-params-body .param-control :deep(.t-input--small) {
    min-height: var(--mobile-touch-target);
    border-radius: 12px;
  }

  .global-gain-header {
    margin-bottom: 12px;
  }
}
</style>
