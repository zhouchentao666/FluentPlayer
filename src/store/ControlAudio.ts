import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { LocalUserDetailStore } from './LocalUserDetail'
import { isLoadingSong } from '@/utils/audio/loadingState'

import type {
  AudioEventCallback,
  AudioEventType,
  AudioSlot,
  AudioSubscriber,
  PlayerSnapshotPayload,
  UnsubscribeFunction,
  ControlAudioState
} from '../types/audio'

let userInfo: any
export const ControlAudioStore = defineStore('controlAudio', () => {
  const Audio = reactive<ControlAudioState>({
    primarySlot: 'A',
    secondaryUrl: '',
    isPlay: false,
    currentTime: 0,
    duration: 0,
    volume: 80,
    url: '',
    eventInit: false
  })

  const subscribers = reactive<Record<AudioEventType, AudioSubscriber[]>>({
    ended: [], seeked: [], timeupdate: [], play: [], pause: [], error: [], canplay: [], slotSwap: []
  })

  const generateId = (): string => `${Date.now()}_${Math.random().toString(36).substr(2, 9)}`

  const subscribe = (eventType: AudioEventType, callback: AudioEventCallback): UnsubscribeFunction => {
    const id = generateId()
    subscribers[eventType].push({ id, callback })
    return () => {
      const index = subscribers[eventType].findIndex((sub) => sub.id === id)
      if (index > -1) subscribers[eventType].splice(index, 1)
    }
  }

  const publish = (eventType: AudioEventType, payload?: PlayerSnapshotPayload): void => {
    subscribers[eventType].forEach((subscriber) => {
      try { subscriber.callback(payload) } catch (error) { console.error(`音频事件回调执行错误 [${eventType}]:`, error) }
    })
  }

  const clearAllSubscribers = (): void => {
    Object.keys(subscribers).forEach((eventType) => { subscribers[eventType as AudioEventType] = [] })
  }

  const clearEventSubscribers = (eventType: AudioEventType) => { subscribers[eventType] = [] }

  // --- Tauri 事件监听 ---

  let unlisteners: UnlistenFn[] = []

  const init = async () => {
    userInfo = LocalUserDetailStore()
    if (Audio.eventInit) {
      // 已初始化时确保旧监听器清理干净后重新绑定
      unlisteners.forEach((un) => { try { un() } catch {} })
      unlisteners = []
    }
    Audio.eventInit = true

    const un1 = await listen('player:state', (event: any) => {
      const { state, position, duration, volume, url, isPlaying } = event.payload
      Audio.isPlay = isPlaying
      Audio.currentTime = position
      Audio.duration = duration
      Audio.volume = volume
      Audio.url = url || ''
      if (state === 'Playing') {
        isLoadingSong.value = false
        publish('play')
      } else if (state === 'Paused') {
        publish('pause')
      }
    })

    const un2 = await listen('player:time', (event: any) => {
      const { position, duration } = event.payload
      Audio.currentTime = position
      Audio.duration = duration
      publish('timeupdate')
    })

    const un3 = await listen('player:ended', () => {
      Audio.isPlay = false
      publish('ended')
    })

    const un4 = await listen<PlayerSnapshotPayload>('player:crossfade_swap', (event) => {
      publish('slotSwap', event.payload)
    })

    const un5 = await listen('player:error', (event: any) => {
      console.error('[Player] 异步播放错误:', event.payload?.error)
      Audio.isPlay = false
      isLoadingSong.value = false
      publish('error')
    })

    const un6 = await listen('player:auto_advanced', () => {
      publish('slotSwap')
    })

    unlisteners = [un1, un2, un3, un4, un5, un6]
  }

  const swapPrimarySlot = () => {
    Audio.primarySlot = Audio.primarySlot === 'A' ? 'B' : 'A'
    invoke('player__swap_slot')
    publish('slotSwap')
  }

  const setCurrentTime = (time: number) => {
    Audio.currentTime = time
  }

  const setDuration = (duration: number) => {
    Audio.duration = duration
  }

  const setVolume = (volume: number, _transition: boolean = false) => {
    if (typeof volume === 'number' && volume >= 0 && volume <= 100) {
      Audio.volume = volume
      userInfo.userInfo.volume = volume
      invoke('player__set_volume', { volume })
    }
  }

  const start = async () => {
    await invoke('player__resume')
    Audio.isPlay = true
    publish('play')
  }

  const stop = async () => {
    await invoke('player__pause')
    Audio.isPlay = false
    publish('pause')
  }

  const destroy = () => {
    unlisteners.forEach((un) => { try { un() } catch {} })
    unlisteners = []
    Audio.eventInit = false
  }

  return {
    Audio, init, setCurrentTime, setVolume,
    swapPrimarySlot, start, stop,
    subscribe, publish, clearAllSubscribers, clearEventSubscribers, setDuration,
    destroy
  }
}, { persist: false })

export type { AudioSlot }
