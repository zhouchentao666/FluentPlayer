<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import { Version, OpenMusicFolder, DefaultMusicFolder } from '@bridge/app'
import {
  type AppSettings,
  HOTKEY_ACTIONS,
  type HotkeyAction,
  type DesktopLyricConfig,
  type AudioQuality,
  AUDIO_QUALITY_OPTIONS,
} from '../composables/useConfig'
import { useFontList } from '../composables/useFontList'
import { useUpdater } from '../composables/useUpdater'
import SettingCard from './settings/SettingCard.vue'
import SettingRow from './settings/SettingRow.vue'
import SegmentedControl from './settings/SegmentedControl.vue'
import ColorPicker from './settings/ColorPicker.vue'
import ToggleSwitch from './settings/ToggleSwitch.vue'
import ComboBox, { type ComboBoxOption } from './settings/ComboBox.vue'
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
  (e: 'open-sources'): void
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

// 下载目录：选择自定义文件夹
async function selectDownloadFolder() {
  try {
    const dir = await OpenMusicFolder()
    if (dir) update({ downloadFolder: dir })
  } catch {
    // 用户取消或失败
  }
}

// 下载目录：恢复默认（系统音乐文件夹）
function resetDownloadFolder() {
  update({ downloadFolder: '' })
}

// 显示用：当前下载目录（为空时显示默认音乐文件夹）
const downloadFolderDisplay = computed(() => {
  const saved = props.settings.downloadFolder
  return saved && saved.trim() ? saved.trim() : '（默认：系统音乐文件夹）'
})

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

const qualityOptions = computed<ComboBoxOption[]>(() =>
  AUDIO_QUALITY_OPTIONS.map(q => ({ value: q.value, label: q.label }))
)

const fontComboOptions = computed<ComboBoxOption[]>(() => [
  { value: '', label: '跟随系统' },
  ...fontOptions.value.map(f => ({ value: f, label: f })),
])

// 手动检查更新（复用 App 共享的更新单例，带 toast 反馈）
const { checking, checkForUpdates } = useUpdater()
function manualCheckUpdate() {
  checkForUpdates(true)
}
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

      <SettingCard title="在线设置">
        <SettingRow label="播放音质" description="在线播放优先请求的音质，获取失败时自动向下降级">
          <ComboBox
            width="160px"
            aria-label="播放音质"
            :options="qualityOptions"
            :model-value="settings.playQuality"
            @update:model-value="value => update({ playQuality: value as AudioQuality })"
          />
        </SettingRow>
        <SettingRow label="下载音质" description="下载在线歌曲时优先请求的音质，获取失败时自动向下降级">
          <ComboBox
            width="160px"
            aria-label="下载音质"
            :options="qualityOptions"
            :model-value="settings.downloadQuality"
            @update:model-value="value => update({ downloadQuality: value as AudioQuality })"
          />
        </SettingRow>
        <SettingRow label="下载保存目录" description="在线歌曲下载后保存到的文件夹，未设置时使用系统音乐文件夹，下载时不再弹出选择框">
          <div class="folder-row">
            <div class="folder-path" :title="downloadFolderDisplay">{{ downloadFolderDisplay }}</div>
            <button class="fluent-btn" @click="selectDownloadFolder">选择文件夹</button>
            <button
              class="fluent-btn"
              :disabled="!settings.downloadFolder"
              @click="resetDownloadFolder"
            >恢复默认</button>
          </div>
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
          <ComboBox
            width="220px"
            aria-label="歌词字体"
            :options="fontComboOptions"
            :model-value="settings.lyricFontFamily"
            @update:model-value="value => update({ lyricFontFamily: value })"
          />
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
        <SettingRow label="开机自动启动" description="登录系统后自动运行 fluentplayer">
          <ToggleSwitch
            :model-value="settings.autoStart"
            @update:model-value="value => update({ autoStart: value })"
          />
        </SettingRow>
        <SettingRow label="启动时检查更新" description="每次启动自动检查新版本，发现更新时弹出提示">
          <ToggleSwitch
            :model-value="settings.checkUpdateOnLaunch"
            @update:model-value="value => update({ checkUpdateOnLaunch: value })"
          />
        </SettingRow>
        <SettingRow label="检查更新" description="手动检查是否有新版本发布">
          <button class="fluent-btn" :disabled="checking" @click="manualCheckUpdate">
            {{ checking ? '检查中…' : '检查更新' }}
          </button>
        </SettingRow>
        <SettingRow label="启用系统托盘" description="在任务栏托盘显示 fluentplayer 图标">
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
        <SettingRow label="音源管理" description="导入 / 启停 / 排序自定义在线音源（LX 格式）">
          <button class="sources-entry" @click="emit('open-sources')">打开音源管理</button>
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
        <SettingRow label="fluentplayer" description="一个简洁的纯离线本地音乐播放器">
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
.sources-entry {
  padding: 7px 14px;
  border-radius: 10px;
  border: 1px solid var(--fluent-border, #2a2a2a);
  background: var(--fluent-bg-card, rgba(255, 255, 255, 0.04));
  color: var(--fluent-text, #e6e6e6);
  font-size: 13px;
  cursor: pointer;
}
.sources-entry:hover {
  background: var(--fluent-accent, #3b82f6);
  border-color: var(--fluent-accent, #3b82f6);
  color: #fff;
}
.folder-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.folder-path {
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  color: var(--fluent-text-secondary, #888);
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
.setting-value {
  font-size: 13px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
}
.fluent-btn {
  height: 30px;
  padding: 0 14px;
  border: 1px solid var(--fluent-border);
  border-radius: 6px;
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
  font-size: 13px;
  line-height: 1;
  cursor: pointer;
  transition: background 0.18s ease, border-color 0.18s ease;
}
.fluent-btn:hover:not(:disabled) {
  background: var(--fluent-bg-active);
  border-color: var(--fluent-input-border);
}
.fluent-btn:active:not(:disabled) {
  background: var(--fluent-bg-active);
}
.fluent-btn:disabled {
  opacity: 0.45;
  cursor: default;
}
</style>