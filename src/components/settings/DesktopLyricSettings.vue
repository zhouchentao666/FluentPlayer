<script setup lang="ts">
import { computed } from 'vue'
import { Events } from '@bridge/runtime'
import { SetDesktopLyricBounds } from '@bridge/app'
import type { DesktopLyricConfig } from '../../composables/useConfig'
import SettingRow from './SettingRow.vue'
import ToggleSwitch from './ToggleSwitch.vue'
import SegmentedControl from './SegmentedControl.vue'
import SettingSlider from './SettingSlider.vue'

const props = defineProps<{
  config: DesktopLyricConfig
  enabled: boolean
}>()

const emit = defineEmits<{
  (e: 'update:config', value: DesktopLyricConfig): void
  (e: 'update:enabled', value: boolean): void
}>()

const positions = [
  { value: 'left', label: '居左' },
  { value: 'center', label: '居中' },
  { value: 'right', label: '居右' },
  { value: 'both', label: '双行' },
] as const

const weights = [
  { value: '400', label: '400' },
  { value: '500', label: '500' },
  { value: '600', label: '600' },
  { value: '700', label: '700' },
  { value: '800', label: '800' },
] as const

function patch(partial: Partial<DesktopLyricConfig>) {
  const next = { ...props.config, ...partial }
  emit('update:config', next)
  Events.Emit('desktop-lyric:config', next).catch(() => {})
}

function heightForFontSize(fontSize: number): number {
  return Math.round(120 + (fontSize - 12) * (180 / 84))
}

function patchFontSize(fontSize: number) {
  const height = heightForFontSize(fontSize)
  const next = { ...props.config, fontSize, height }
  emit('update:config', next)
  Events.Emit('desktop-lyric:config', next).catch(() => {})
  SetDesktopLyricBounds(next.x, next.y, next.width, height).catch(() => {})
}

function setEnabled(value: boolean) {
  emit('update:enabled', value)
}

function rgbaToHex(input: string): string {
  const m = input.match(/rgba?\((\d+)[,\s]+(\d+)[,\s]+(\d+)/i)
  if (!m) return '#ffffff'
  const toHex = (n: number) => n.toString(16).padStart(2, '0')
  return `#${toHex(Number(m[1]))}${toHex(Number(m[2]))}${toHex(Number(m[3]))}`
}

function setRgbaFromHex(key: 'unplayedColor' | 'shadowColor' | 'backgroundMaskColor', hex: string, alpha: number) {
  const clean = hex.replace('#', '')
  if (clean.length !== 6) return
  const r = parseInt(clean.slice(0, 2), 16)
  const g = parseInt(clean.slice(2, 4), 16)
  const b = parseInt(clean.slice(4, 6), 16)
  patch({ [key]: `rgba(${r}, ${g}, ${b}, ${alpha})` })
}

function parseAlpha(input: string): number {
  const m = input.match(/rgba?\([^)]+,\s*([\d.]+)\)/i)
  return m ? Number(m[1]) : 1
}

const previewStyle = computed(() => ({
  fontSize: props.config.fontSize + 'px',
  fontWeight: props.config.fontWeight,
  color: props.config.mainColor,
  textShadow: `0 0 6px ${props.config.shadowColor}`,
}))
</script>

<template>
  <div class="desktop-lyric-settings">
    <SettingRow label="启用桌面歌词" description="在桌面上显示悬浮歌词窗口">
      <ToggleSwitch :model-value="enabled" @update:model-value="setEnabled" />
    </SettingRow>

    <div class="settings-grid">
      <div class="field">
        <label>字体大小</label>
        <SettingSlider :min="12" :max="96" :model-value="config.fontSize" @update:model-value="patchFontSize" />
      </div>

      <div class="field">
        <label>字重</label>
        <SegmentedControl :options="weights" :model-value="String(config.fontWeight)" @update:model-value="v => patch({ fontWeight: Number(v) })" />
      </div>

      <div class="field">
        <label>主颜色</label>
        <div class="color-inputs">
          <input type="color" :value="config.mainColor" @input="e => patch({ mainColor: (e.target as HTMLInputElement).value })" />
          <input class="color-text" type="text" :value="config.mainColor" @change="e => patch({ mainColor: (e.target as HTMLInputElement).value })" />
        </div>
      </div>

      <div class="field">
        <label>未播放颜色</label>
        <div class="color-inputs">
          <input type="color" :value="rgbaToHex(config.unplayedColor)" @input="e => setRgbaFromHex('unplayedColor', (e.target as HTMLInputElement).value, parseAlpha(config.unplayedColor))" />
          <input class="color-text" type="text" :value="config.unplayedColor" @change="e => patch({ unplayedColor: (e.target as HTMLInputElement).value })" />
        </div>
      </div>

      <div class="field">
        <label>阴影颜色</label>
        <div class="color-inputs">
          <input type="color" :value="rgbaToHex(config.shadowColor)" @input="e => setRgbaFromHex('shadowColor', (e.target as HTMLInputElement).value, parseAlpha(config.shadowColor))" />
          <input class="color-text" type="text" :value="config.shadowColor" @change="e => patch({ shadowColor: (e.target as HTMLInputElement).value })" />
        </div>
      </div>

      <div class="field">
        <label>背景遮罩颜色</label>
        <div class="color-inputs">
          <input type="color" :value="rgbaToHex(config.backgroundMaskColor)" @input="e => setRgbaFromHex('backgroundMaskColor', (e.target as HTMLInputElement).value, parseAlpha(config.backgroundMaskColor))" />
          <input class="color-text" type="text" :value="config.backgroundMaskColor" @change="e => patch({ backgroundMaskColor: (e.target as HTMLInputElement).value })" />
        </div>
      </div>

      <div class="field">
        <label>对齐方式</label>
        <SegmentedControl :options="positions" :model-value="config.position" @update:model-value="v => patch({ position: v as DesktopLyricConfig['position'] })" />
      </div>
    </div>

    <div class="toggles-row">
      <SettingRow label="背景遮罩" description="歌词文字背后显示半透明遮罩">
        <ToggleSwitch :model-value="config.textBackgroundMask" @update:model-value="v => patch({ textBackgroundMask: v })" />
      </SettingRow>
      <SettingRow label="动画" description="歌词切换时使用过渡动画">
        <ToggleSwitch :model-value="config.animation" @update:model-value="v => patch({ animation: v })" />
      </SettingRow>
      <SettingRow label="逐字歌词" description="逐字高亮显示（需歌曲支持）">
        <ToggleSwitch :model-value="config.showYrc" @update:model-value="v => patch({ showYrc: v })" />
      </SettingRow>
      <SettingRow label="显示翻译" description="同时显示歌词翻译">
        <ToggleSwitch :model-value="config.showTran" @update:model-value="v => patch({ showTran: v })" />
      </SettingRow>
      <SettingRow label="双行显示" description="同时显示当前行与下一行">
        <ToggleSwitch :model-value="config.isDoubleLine" @update:model-value="v => patch({ isDoubleLine: v })" />
      </SettingRow>
      <SettingRow label="总是显示歌曲信息" description="锁定后也保留歌曲标题">
        <ToggleSwitch :model-value="config.alwaysShowPlayInfo" @update:model-value="v => patch({ alwaysShowPlayInfo: v })" />
      </SettingRow>
    </div>

    <div class="preview">
      <div class="preview-lyric" :style="previewStyle">这是桌面歌词预览</div>
    </div>
  </div>
</template>

<style scoped>
.desktop-lyric-settings {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field label {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.color-inputs {
  display: flex;
  align-items: center;
  gap: 8px;
}
.color-inputs input[type='color'] {
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--fluent-border);
  border-radius: 6px;
  background: transparent;
  cursor: pointer;
}
.color-text {
  flex: 1;
  min-width: 0;
  padding: 6px 8px;
  border-radius: 6px;
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
  font-size: 13px;
  outline: none;
}
.toggles-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 0 16px;
}
.toggles-row :deep(.setting-row) {
  padding: 10px 0;
}
.preview {
  padding: 24px;
  border-radius: 10px;
  border: 1px dashed var(--fluent-border);
  background: var(--fluent-bg-hover);
  text-align: center;
}
</style>
