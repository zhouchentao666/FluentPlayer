<template>
  <div>
    <div class="section">
      <div class="setting-item" style="margin-top: 0">
        <div class="item-info">
          <div class="item-title">{{ t('settings.hotkey.enableGlobal') }}</div>
          <div class="item-desc">{{ t('settings.hotkey.enableGlobalDesc') }}</div>
        </div>
        <t-switch v-model="enabled" :loading="saving" @change="save" />
      </div>

      <div class="hotkey-list" :class="{ disabled: !enabled }">
        <div class="hotkey-toolbar">
          <t-button size="small" theme="default" variant="outline" :loading="saving" @click="resetToDefault">
            {{ t('settings.hotkey.restoreDefault') }}
          </t-button>
        </div>
        <div v-for="it in hotkeyActions" :key="it.id" class="hotkey-row">
          <div class="hotkey-meta">
            <div class="hotkey-title">{{ it.title }}</div>
            <div class="hotkey-desc">{{ it.desc }}</div>
          </div>
          <div class="hotkey-actions">
            <t-tag v-if="failedActions.has(it.id)" theme="danger" variant="light">
              {{ displayBinding(it.id) }}
            </t-tag>
            <t-tag v-else theme="primary" variant="light">
              {{ displayBinding(it.id) }}
            </t-tag>
            <t-button size="small" theme="primary" variant="outline" :disabled="!enabled" @click="beginRecord(it.id)">
              {{ t('settings.hotkey.set') }}
            </t-button>
            <t-button size="small" theme="default" variant="text" :disabled="!enabled" @click="clearHotkey(it.id)">
              {{ t('settings.hotkey.clear') }}
            </t-button>
          </div>
        </div>
      </div>
    </div>

    <div class="section">
      <div class="section-title">{{ t('settings.hotkey.inAppTitle') }}</div>
      <div class="section-desc">{{ t('settings.hotkey.inAppDesc') }}</div>
      <div class="hotkey-list">
        <div v-for="it in inAppShortcuts" :key="it.keys + it.title" class="hotkey-row">
          <div class="hotkey-meta">
            <div class="hotkey-title">{{ it.title }}</div>
            <div class="hotkey-desc">{{ it.desc }}</div>
          </div>
          <div class="hotkey-actions">
            <t-tag theme="default" variant="light">{{ it.keys }}</t-tag>
          </div>
        </div>
      </div>
    </div>

    <t-dialog
      v-model:visible="recording.visible"
      :header="recordTitle"
      :close-btn="true"
      :close-on-esc-keydown="false"
      :close-on-overlay-click="false"
    >
      <div class="recording">
        <div class="preview">{{ recordPreview }}</div>
        <div class="preview sub">
          {{ recording.captured ? acceleratorToDisplay(recording.captured) : t('settings.hotkey.waitForInput') }}
        </div>
        <div class="tips">
          <div>{{ t('settings.hotkey.pressEscToCancel') }}</div>
          <div>{{ t('settings.hotkey.pressBackspaceToClear') }}</div>
        </div>
      </div>
      <template #footer>
        <t-button theme="default" @click="cancelRecord">{{ t('common.cancel') }}</t-button>
        <t-button theme="primary" :loading="saving" :disabled="!recordCanSave" @click="confirmRecord">
          {{ t('common.save') }}
        </t-button>
      </template>
    </t-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, toRaw, watch } from 'vue'
import { MessagePlugin } from 'tdesign-vue-next'
import type { HotkeyAction, HotkeyConfig } from '@/types/hotkeys'
import { defaultHotkeyConfig } from '@/types/hotkeys'
import { acceleratorToDisplay, createHotkeyRecorder, isCompleteAccelerator } from '@/utils/hotkeys/recording'

const { t } = useI18n()

const hotkeyActions = computed<Array<{ id: HotkeyAction; title: string; desc: string }>>(() => [
  { id: 'toggle', title: t('settings.hotkey.actionToggle'), desc: t('settings.hotkey.actionToggleDesc') },
  { id: 'playPrev', title: t('settings.hotkey.actionPlayPrev'), desc: t('settings.hotkey.actionPlayPrevDesc') },
  { id: 'playNext', title: t('settings.hotkey.actionPlayNext'), desc: t('settings.hotkey.actionPlayNextDesc') },
  { id: 'seekBackward', title: t('settings.hotkey.actionSeekBackward'), desc: t('settings.hotkey.actionSeekBackwardDesc') },
  { id: 'seekForward', title: t('settings.hotkey.actionSeekForward'), desc: t('settings.hotkey.actionSeekForwardDesc') },
  { id: 'toggleDesktopLyric', title: t('settings.hotkey.actionToggleDesktopLyric'), desc: t('settings.hotkey.actionToggleDesktopLyricDesc') },
  { id: 'volumeDown', title: t('settings.hotkey.actionVolumeDown'), desc: t('settings.hotkey.actionVolumeDownDesc') },
  { id: 'volumeUp', title: t('settings.hotkey.actionVolumeUp'), desc: t('settings.hotkey.actionVolumeUpDesc') },
  { id: 'setPlayModeSequence', title: t('settings.hotkey.actionSetPlayModeSequence'), desc: t('settings.hotkey.actionSetPlayModeSequenceDesc') },
  { id: 'setPlayModeRandom', title: t('settings.hotkey.actionSetPlayModeRandom'), desc: t('settings.hotkey.actionSetPlayModeRandomDesc') },
  { id: 'togglePlayModeSingle', title: t('settings.hotkey.actionTogglePlayModeSingle'), desc: t('settings.hotkey.actionTogglePlayModeSingleDesc') }
])

const inAppShortcuts = computed<Array<{ keys: string; title: string; desc: string }>>(() => [
  { keys: 'Space', title: t('settings.hotkey.inAppToggle'), desc: t('settings.hotkey.inAppToggleDesc') },
  { keys: '↑', title: t('settings.hotkey.inAppVolumeUp'), desc: t('settings.hotkey.inAppVolumeUpDesc') },
  { keys: '↓', title: t('settings.hotkey.inAppVolumeDown'), desc: t('settings.hotkey.inAppVolumeDownDesc') },
  { keys: '←', title: t('settings.hotkey.inAppSeekBackward'), desc: t('settings.hotkey.inAppSeekBackwardDesc') },
  { keys: '→', title: t('settings.hotkey.inAppSeekForward'), desc: t('settings.hotkey.inAppSeekForwardDesc') },
  { keys: 'F', title: t('settings.hotkey.inAppFullscreen'), desc: t('settings.hotkey.inAppFullscreenDesc') }
])

const loading = ref(false)
const saving = ref(false)
const enabled = ref(true)
const bindings = ref<HotkeyConfig['bindings']>({})
const failedActions = ref<Set<HotkeyAction>>(new Set())

const displayBinding = (action: HotkeyAction) => {
  return acceleratorToDisplay(bindings.value[action] || '')
}

const load = async () => {
  loading.value = true
  try {
    const res = await (window as any).api.hotkeys.get()
    const cfg = res?.data as HotkeyConfig | undefined
    if (!cfg) return
    enabled.value = !!cfg.enabled
    bindings.value = { ...(cfg.bindings || {}) }
    const status = res?.status as any
    failedActions.value = new Set(status?.failedActions || [])
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
}

const save = async () => {
  saving.value = true
  try {
    const res = await (window as any).api.hotkeys.set({
      enabled: enabled.value,
      bindings: { ...(toRaw(bindings.value) || {}) }
    })
    if (!res?.success) {
      MessagePlugin.warning(res?.error || t('settings.hotkey.saveFailed'))
      return
    }
    const cfg = res?.data as HotkeyConfig | undefined
    if (cfg) {
      enabled.value = !!cfg.enabled
      bindings.value = { ...(cfg.bindings || {}) }
    }
    const status = res?.status as any
    failedActions.value = new Set(status?.failedActions || [])
    MessagePlugin.success(t('settings.hotkey.saveSuccess'))
  } catch {
    MessagePlugin.error(t('settings.hotkey.saveFailed'))
  } finally {
    saving.value = false
  }
}

const resetToDefault = async () => {
  enabled.value = !!defaultHotkeyConfig.enabled
  bindings.value = { ...(defaultHotkeyConfig.bindings || {}) }
  await save()
}

const clearHotkey = async (action: HotkeyAction) => {
  bindings.value[action] = ''
  await save()
}

const recording = ref<{ visible: boolean; action: HotkeyAction | null; preview: string; captured: string }>({
  visible: false, action: null, preview: '', captured: ''
})
let recorder: ReturnType<typeof createHotkeyRecorder> | null = null

const beginRecord = (action: HotkeyAction) => {
  recording.value = { visible: true, action, preview: '', captured: bindings.value[action] || '' }
  recorder?.unmount()
  recorder = createHotkeyRecorder({
    onPreviewChange: (preview) => { recording.value.preview = preview },
    onCapture: (acc) => {
      if (recording.value.action) {
        const conflict = Object.entries(bindings.value).find(
          ([k, v]) => k !== recording.value.action && (v || '').toLowerCase() === (acc || '').toLowerCase() && !!acc
        )
        if (conflict) {
          MessagePlugin.warning(t('settings.hotkey.hotkeyConflict'))
          return
        }
      }
      recording.value.captured = acc
    },
    onCancel: cancelRecord
  })
  recorder.mount()
}

const cancelRecord = () => {
  recording.value = { visible: false, action: null, preview: '', captured: '' }
  recorder?.unmount()
  recorder = null
}

const confirmRecord = async () => {
  const action = recording.value.action
  if (!action) return
  const acc = recording.value.captured || ''
  if (acc && !isCompleteAccelerator(acc)) return
  bindings.value[action] = acc
  cancelRecord()
  await save()
}

const recordTitle = computed(() => {
  if (!recording.value.action) return t('settings.hotkey.recordHotkey')
  const meta = hotkeyActions.value.find(a => a.id === recording.value.action)
  return meta ? t('settings.hotkey.setWith', { name: meta.title }) : t('settings.hotkey.recordHotkey')
})

const recordPreview = computed(() => {
  if (!recording.value.preview) return t('settings.hotkey.pressCombo')
  return acceleratorToDisplay(recording.value.preview)
})

const recordCanSave = computed(() => {
  if (!recording.value.visible || !recording.value.action) return false
  if (!recording.value.captured) return true
  return isCompleteAccelerator(recording.value.captured)
})

onMounted(load)

watch(() => recording.value.visible, (v) => {
  if (!v) { recorder?.unmount(); recorder = null }
})

onBeforeUnmount(() => { recorder?.unmount(); recorder = null })
</script>

<style scoped>
.section { display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px; color: var(--td-text-color-primary); }
.section-title { font-weight: 700; color: var(--td-text-color-primary); }
.section-desc { font-size: 12px; color: var(--td-text-color-secondary); }
.setting-item {
  display: flex; align-items: center; justify-content: space-between;
  padding: 0.875rem 1rem; border: 1px solid var(--settings-feature-border, var(--td-border-level-1-color));
  background: var(--settings-feature-bg, var(--td-bg-color-container)); border-radius: 0.5rem; margin-top: 0.75rem;
}
.setting-item .item-info { display: flex; flex-direction: column; gap: 0.25rem; }
.setting-item .item-title { font-weight: 600; font-size: 0.95rem; color: var(--td-text-color-primary); }
.setting-item .item-desc { color: var(--td-text-color-secondary); font-size: 0.8rem; }
.hotkey-list { margin-top: 12px; display: flex; flex-direction: column; gap: 10px; }
.hotkey-list.disabled { opacity: 0.6; }
.hotkey-toolbar { display: flex; justify-content: flex-end; }
.hotkey-row {
  display: flex; align-items: center; justify-content: space-between;
  padding: 12px; border: 1px solid var(--settings-feature-border, var(--td-border-level-1-color));
  background: var(--settings-feature-bg, var(--td-bg-color-container)); border-radius: 10px; gap: 12px;
}
.hotkey-meta { display: flex; flex-direction: column; gap: 4px; }
.hotkey-title { font-weight: 600; color: var(--td-text-color-primary); }
.hotkey-desc { font-size: 12px; color: var(--td-text-color-secondary); }
.hotkey-actions { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
.recording { display: flex; flex-direction: column; gap: 12px; color: var(--td-text-color-primary); }
.preview { padding: 12px; border: 1px dashed var(--td-border-level-1-color); border-radius: 8px; font-weight: 700; text-align: center; color: var(--td-text-color-primary); }
.preview.sub { font-weight: 600; color: var(--td-text-color-secondary); }
.tips { display: flex; justify-content: space-between; color: var(--td-text-color-secondary); font-size: 12px; }
</style>
