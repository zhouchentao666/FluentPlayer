<script setup lang="ts">
import { onMounted, onUnmounted, provide, watch } from 'vue'
import { ControlAudioStore } from '@/store/ControlAudio'
import { useEqualizerStore } from '@/store/Equalizer'
import { useAudioEffectsStore } from '@/store/AudioEffects'
import { invoke } from '@tauri-apps/api/core'
import { installShortDurationGuard, onCrossfadeSwap, playNext, syncAndroidCurrentPlaybackState, uninstallShortDurationGuard } from '@/utils/audio/globaPlayList'
import createLogger from '@/utils/logger'

const log = createLogger('GlobalAudio')

const audioStore = ControlAudioStore()
const eqStore = useEqualizerStore()
const effectStore = useAudioEffectsStore()
let unsubscribeEnded: (() => void) | null = null
let unsubscribeSlotSwap: (() => void) | null = null

const handleVisibilityChange = () => {
  if (document.visibilityState === 'visible') {
    syncAndroidCurrentPlaybackState()
  }
}

provide('audioSubscribe', audioStore.subscribe)

// EQ 同步集中在 Equalizer store；此处仅在全局播放器初始化后恢复一次快照。
const applyGlobalEQ = () => {
  void eqStore.syncToBackend()
}

// 音效变化时同步到 Rust 后端
const applyGlobalEffects = () => {
  const { balance } = effectStore
  invoke('player__set_balance', { value: balance.enabled ? balance.value : 0 })
}

onMounted(async () => {
  await audioStore.init()
  unsubscribeEnded = audioStore.subscribe('ended', () => {
    playNext()
  })
  unsubscribeSlotSwap = audioStore.subscribe('slotSwap', (payload) => {
    onCrossfadeSwap(payload)
    syncAndroidCurrentPlaybackState()
  })
  document.addEventListener('visibilitychange', handleVisibilityChange)
  installShortDurationGuard()
  log.debug('Rust 原生音频引擎初始化完成')

  // 恢复 EQ 和音效设置到 Rust 后端
  applyGlobalEQ()
  applyGlobalEffects()
})

watch(
  [() => effectStore.surround, () => effectStore.balance],
  () => { applyGlobalEffects() },
  { deep: true }
)

onUnmounted(() => {
  unsubscribeEnded?.()
  unsubscribeEnded = null
  unsubscribeSlotSwap?.()
  unsubscribeSlotSwap = null
  document.removeEventListener('visibilitychange', handleVisibilityChange)
  uninstallShortDurationGuard()
  audioStore.destroy()
})
</script>

<template>
  <div />
</template>
