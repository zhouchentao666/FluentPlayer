<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import { Version } from '@bridge/app'
import {
  type AppSettings,
  HOTKEY_ACTIONS,
  type HotkeyAction,
  type DesktopLyricConfig,
  type AudioQuality,
  AUDIO_QUALITY_OPTIONS,
} from '../composables/useConfig'
import { useFontList } from '../composables/useFontList'
import SettingCard from './settings/SettingCard.vue'
import SettingRow from './settings/SettingRow.vue'
import SegmentedControl from './settings/SegmentedControl.vue'
import ColorPicker from './settings/ColorPicker.vue'
import ToggleSwitch from './settings/ToggleSwitch.vue'
import SettingSlider from './settings/SettingSlider.vue'
import WindowEffectSettings from './settings/WindowEffectSettings.vue'
import HotkeyInput from './settings/HotkeyInput.vue'
import DesktopLyricSettings from './settings/DesktopLyricSettings.vue'

const props = defineProps<{
  settings: AppSettings
}>()

const emit = defineEmits<{
  (e: 'update:settings', settings: AppSettings): void
  (e: 'close'): void
}>()

const appVersion = ref('')

onMounted(async () => {
  try {
    appVersion.value = await Version()
  } catch {
    appVersion.value = ''
  }
})

function update(partial: Partial<AppSettings>) {
  emit('update:settings', { ...props.settings, ...partial })
}

function updateHotkey(action: HotkeyAction, key: string | undefined) {
  const next = { ...props.settings.hotkeys }
  if (key) {
    next[action] = key
  } else {
    delete next[action]
  }
  update({ hotkeys: next })
}

function updateDesktopLyric(config: DesktopLyricConfig) {
  emit('update:settings', { ...props.settings, desktopLyric: { ...config, enabled: props.settings.desktopLyric.enabled } })
}

function updateDesktopLyricEnabled(enabled: boolean) {
  emit('update:settings', { ...props.settings, desktopLyric: { ...props.settings.desktopLyric, enabled } })
}

const themes = [
  { value: 'system', label: '跟随系统' },
  { value: 'light', label: '浅色' },
  { value: 'dark', label: '深色' },
] as const

const accentColors = ['#0078d4', '#107c10', '#ff8c00', '#d13438', '#881798', '#00b7c3']

const fullScreenBackgrounds = [
  { value: 'static', label: '静态' },
  { value: 'dynamic', label: '动态' },
] as const

const coverTransitions = [
  { value: 'fade', label: '淡入淡出' },
  { value: 'slide-left', label: '左边滑入滑出' },
  { value: 'slide-both', label: '左右滑入滑出' },
] as const

const { fonts } = useFontList()
const fontOptions = computed(() => {
  const list = [...fonts.value]
  const cur = props.settings.lyricFontFamily
  if (cur && !list.includes(cur)) list.unshift(cur)
  return list
})
</script>

<template>
  <div class="settings">
    <div class="settings-header">
      <h1>设置</h1>
      <button class="close-btn" @click="emit('close')">✕</button>
    </div>
    <div class="settings-content">
      <SettingCard title="外观">
        <SettingRow label="应用主题" description="选择应用使用的颜色模式">
          <SegmentedControl
            :options="themes"
            :model-value="settings.theme"
            @update:model-value="value => update({ theme: value as AppSettings['theme'] })"
          />
        </SettingRow>
        <SettingRow label="强调色" description="选择应用使用的强调色">
          <ColorPicker
            :colors="accentColors"
            :model-value="settings.accentColor"
            @update:model-value="value => update({ accentColor: value })"
          />
        </SettingRow>
      </SettingCard>

      <SettingCard title="播放">
        <SettingRow label="打开后自动播放音乐" description="启动应用后自动继续播放">
          <ToggleSwitch
            :model-value="settings.autoplay"
            @update:model-value="value => update({ autoplay: value })"
          />
        </SettingRow>
        <SettingRow label="重启后保存播放列表和当前音乐" description="退出时记住当前播放的列表、歌曲和进度">
          <ToggleSwitch
            :model-value="settings.savePlaylistAndSong"
            @update:model-value="value => update({ savePlaylistAndSong: value })"
          />
        </SettingRow>
      </SettingCard>

      <SettingCard title="在线音质">
        <SettingRow label="播放音质" description="在线播放优先请求的音质，获取失败时自动向下降级">
          <select
            class="fluent-select"
            :value="settings.playQuality"
            @change="e => update({ playQuality: (e.target as HTMLSelectElement).value as AudioQuality })"
          >
            <option v-for="q in AUDIO_QUALITY_OPTIONS" :key="q.value" :value="q.value">{{ q.label }}</option>
          </select>
        </SettingRow>
        <SettingRow label="下载音质" description="下载在线歌曲时优先请求的音质，获取失败时自动向下降级">
          <select
            class="fluent-select"
            :value="settings.downloadQuality"
            @change="e => update({ downloadQuality: (e.target as HTMLSelectElement).value as AudioQuality })"
          >
            <option v-for="q in AUDIO_QUALITY_OPTIONS" :key="q.value" :value="q.value">{{ q.label }}</option>
          </select>
        </SettingRow>
      </SettingCard>

      <WindowEffectSettings :settings="settings" @update="update" />

      <SettingCard title="全屏播放器">
        <SettingRow label="背景效果" description="选择全屏播放器的背景效果">
          <SegmentedControl
            :options="fullScreenBackgrounds"
            :model-value="settings.fullScreenBackground"
            @update:model-value="value => update({ fullScreenBackground: value as AppSettings['fullScreenBackground'] })"
          />
        </SettingRow>
        <SettingRow label="封面切换动画" description="切换歌曲时封面的过渡效果">
          <SegmentedControl
            :options="coverTransitions"
            :model-value="settings.coverTransition"
            @update:model-value="value => update({ coverTransition: value as AppSettings['coverTransition'] })"
          />
        </SettingRow>
        <SettingRow label="沉浸式播放栏" description="鼠标移开时淡化播放栏中间与右侧，移入时恢复显示">
          <ToggleSwitch
            :model-value="settings.immersivePlayerBar"
            @update:model-value="value => update({ immersivePlayerBar: value })"
          />
        </SettingRow>
        <SettingRow label="歌词文字大小" description="全屏播放器歌词字号（像素）">
          <SettingSlider
            :min="12"
            :max="72"
            :model-value="settings.lyricFontSize"
            @update:model-value="value => update({ lyricFontSize: value })"
          />
        </SettingRow>
        <SettingRow label="歌词大小自适应" description="开启后，非全屏状态下歌词字号自动缩小，便于小窗阅读">
          <ToggleSwitch
            :model-value="settings.lyricFontSizeAdaptive"
            @update:model-value="value => update({ lyricFontSizeAdaptive: value })"
          />
        </SettingRow>
        <SettingRow label="歌词字体" description="全屏播放器歌词使用的字体（下拉框列出系统全部字体）">
          <select
            class="fluent-select"
            :value="settings.lyricFontFamily"
            @change="e => update({ lyricFontFamily: (e.target as HTMLSelectElement).value })"
          >
            <option value="">跟随系统</option>
            <option v-for="f in fontOptions" :key="f" :value="f">{{ f }}</option>
          </select>
        </SettingRow>
        <SettingRow label="歌词对齐位置" description="歌词在播放器中的垂直位置（0=顶部，1=底部）">
          <SettingSlider
            :min="0"
            :max="1"
            :step="0.01"
            :model-value="settings.lyricAlignPosition"
            @update:model-value="value => update({ lyricAlignPosition: value })"
          />
        </SettingRow>
        <SettingRow label="歌词行模糊效果" description="为当前/已播放歌词行添加模糊特效">
          <ToggleSwitch
            :model-value="settings.lyricBlur"
            @update:model-value="value => update({ lyricBlur: value })"
          />
        </SettingRow>
        <SettingRow label="物理弹簧动画" description="使用弹簧算法实现歌词动画；性能不足时可关闭以回退到 transition 过渡">
          <ToggleSwitch
            :model-value="settings.lyricSpring"
            @update:model-value="value => update({ lyricSpring: value })"
          />
        </SettingRow>
        <SettingRow label="背景流动速度" description="动态背景的流动速度">
          <SettingSlider
            :min="0.5"
            :max="6"
            :step="0.5"
            :model-value="settings.lyricFlowSpeed"
            @update:model-value="value => update({ lyricFlowSpeed: value })"
          />
        </SettingRow>
        <SettingRow label="背景动画帧率" description="动态背景的渲染帧率">
          <SettingSlider
            :min="10"
            :max="360"
            :step="5"
            :model-value="settings.lyricFps"
            @update:model-value="value => update({ lyricFps: value })"
          />
        </SettingRow>
      </SettingCard>

      <SettingCard title="快捷键">
        <SettingRow
          v-for="action in HOTKEY_ACTIONS"
          :key="action.value"
          :label="action.label"
          description="点击输入框后按下想要的按键"
        >
          <HotkeyInput
            :model-value="settings.hotkeys[action.value]"
            @update:model-value="value => updateHotkey(action.value, value)"
          />
        </SettingRow>
      </SettingCard>

      <SettingCard title="系统">
        <SettingRow label="开机自动启动" description="登录系统后自动运行 tideaudio">
          <ToggleSwitch
            :model-value="settings.autoStart"
            @update:model-value="value => update({ autoStart: value })"
          />
        </SettingRow>
        <SettingRow label="启用系统托盘" description="在任务栏托盘显示 tideaudio 图标">
          <ToggleSwitch
            :model-value="settings.trayEnabled"
            @update:model-value="value => update({ trayEnabled: value })"
          />
        </SettingRow>
        <SettingRow label="关闭按钮最小化到托盘" description="点击标题栏关闭按钮时隐藏到托盘而不是退出">
          <ToggleSwitch
            :model-value="settings.closeToTray"
            :disabled="!settings.trayEnabled"
            @update:model-value="value => update({ closeToTray: value })"
          />
        </SettingRow>
        <SettingRow label="系统媒体控制" description="把歌曲信息与封面同步到系统媒体面板，并支持键盘媒体键 / 系统悬浮控件">
          <ToggleSwitch
            :model-value="settings.systemMediaControl"
            @update:model-value="value => update({ systemMediaControl: value })"
          />
        </SettingRow>
      </SettingCard>

      <SettingCard title="桌面歌词">
        <DesktopLyricSettings
          :config="settings.desktopLyric"
          :enabled="settings.desktopLyric.enabled"
          @update:config="updateDesktopLyric"
          @update:enabled="updateDesktopLyricEnabled"
        />
      </SettingCard>

      <SettingCard title="关于">
        <SettingRow label="tideaudio" description="一个简洁的纯离线本地音乐播放器">
          <span class="setting-value">v{{ appVersion || '0.0.1' }}</span>
        </SettingRow>
      </SettingCard>
    </div>
  </div>
</template>

<style scoped>
.settings {
  height: 100%;
  padding: 28px 32px;
  color: var(--fluent-text);
  overflow-y: auto;
  box-sizing: border-box;
}
.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 28px;
}
.settings-header h1 {
  margin: 0;
  font-size: 28px;
  font-weight: 600;
  letter-spacing: -0.2px;
}
.close-btn {
  width: 34px;
  height: 34px;
  border: none;
  border-radius: 8px;
  background: var(--fluent-bg-hover);
  color: inherit;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.18s ease;
}
.close-btn:hover {
  background: var(--fluent-bg-active);
}
.settings-content {
  display: flex;
  flex-direction: column;
  gap: 20px;
}
.fluent-select {
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
  font-size: 13px;
  outline: none;
  cursor: pointer;
}
.setting-value {
  font-size: 13px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
}
.fluent-btn {
  padding: 6px 14px;
  border: 1px solid var(--fluent-border);
  border-radius: 6px;
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.18s ease;
}
.fluent-btn:hover {
  background: var(--fluent-bg-active);
}
.fluent-btn:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>