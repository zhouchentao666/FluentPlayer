import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { MessagePlugin } from 'tdesign-vue-next'
import i18n from '@/locales'

export interface AudioOutputDevice {
  deviceId: string
  kind: MediaDeviceKind
  label: string
  groupId: string
}

export interface RustAudioDevice {
  id: number
  name: string
  is_default: boolean
  sample_rate: number
  channels: number
  volume: number
  volume_supported: boolean
}

export interface DeviceStats {
  sampleRate: number
  channelCount: number
  latency: number
}

export const useAudioOutputStore = defineStore(
  'audioOutput',
  () => {
    const supported = ref(typeof navigator !== 'undefined' && !!navigator.mediaDevices?.enumerateDevices)
    const rustSupported = ref(false)
    const devices = ref<AudioOutputDevice[]>([])
    const rustDevices = ref<RustAudioDevice[]>([])
    const currentDeviceId = ref<string>('default')
    const currentRustDeviceId = ref<number>(0)
    const isLoading = ref(false)
    const error = ref<string | null>(null)
    const deviceStats = ref<DeviceStats>({
      sampleRate: 0,
      channelCount: 0,
      latency: 0
    })
    let rustDeviceChangeUnlisten: UnlistenFn | null = null

    // For A/B testing
    const primaryDeviceId = ref<string>('default')
    const secondaryDeviceId = ref<string>('')
    const activeABChannel = ref<'A' | 'B'>('A')

    // Combined device list (Rust backend takes priority)
    const allDevices = computed(() => {
      if (rustSupported.value && rustDevices.value.length > 0) {
        return rustDevices.value.map((d) => ({
          id: d.id,
          name: d.name,
          is_default: d.is_default,
          sample_rate: d.sample_rate,
          channels: d.channels,
          volume: d.volume,
          volume_supported: d.volume_supported
        }))
      }
      return devices.value.map((d) => ({
        id: d.deviceId,
        name: d.label,
        is_default: d.deviceId === 'default',
        sample_rate: 0,
        channels: 0,
        volume: 0,
        volume_supported: false
      }))
    })

    const sortedDevices = computed(() => {
      return [...allDevices.value].sort((a, b) => a.name.localeCompare(b.name))
    })

    const currentDeviceLabel = computed(() => {
      const id = rustSupported.value ? String(currentRustDeviceId.value) : currentDeviceId.value
      const device = allDevices.value.find((d) => String(d.id) === id)
      return device ? device.name : 'Default'
    })

    const scanRustDevices = async () => {
      try {
        const result = await invoke<RustAudioDevice[]>('audio__enumerate_devices')
        rustDevices.value = result
        rustSupported.value = true

        // Sync current selection
        const defaultDevice = result.find((d) => d.is_default)
        if (defaultDevice) {
          currentRustDeviceId.value = defaultDevice.id
        }
      } catch (err: any) {
        console.warn('Rust audio device enumeration not available:', err)
        rustSupported.value = false
      }
    }

    const scanDevices = async () => {
      isLoading.value = true
      error.value = null

      // Try Rust backend first
      await scanRustDevices()

      // Fallback to Web API if Rust not available
      if (!rustSupported.value && navigator.mediaDevices?.enumerateDevices) {
        try {
          const allDevices = await navigator.mediaDevices.enumerateDevices()
          devices.value = allDevices
            .filter((device) => device.kind === 'audiooutput')
            .map((device) => ({
              deviceId: device.deviceId,
              kind: device.kind,
              label: device.label || `Speaker (${device.deviceId.slice(0, 5)}...)`,
              groupId: device.groupId
            }))

          if (
            currentDeviceId.value !== 'default' &&
            !devices.value.find((d) => d.deviceId === currentDeviceId.value)
          ) {
            console.warn(
              `Previously selected device ${currentDeviceId.value} not found, reverting to default.`
            )
            currentDeviceId.value = 'default'
            MessagePlugin.warning(i18n.global.t('settings.audioOutput.prevDeviceNotFound'))
          }
        } catch (err: any) {
          console.error('Failed to enumerate audio devices:', err)
          if (err.name === 'NotAllowedError') {
            error.value = i18n.global.t('settings.audioOutput.permissionDenied')
          } else if (err.name === 'NotFoundError') {
            error.value = i18n.global.t('settings.audioOutput.deviceNotFound')
          } else {
            error.value = err.message || i18n.global.t('settings.audioOutput.cannotGetDevices')
          }
          MessagePlugin.error(error.value || i18n.global.t('settings.audioOutput.getDevicesFailed'))
        }
      }

      if (!rustSupported.value && devices.value.length === 0 && !error.value) {
        error.value = i18n.global.t('settings.audioOutput.deviceNotFound')
      }

      isLoading.value = false
    }

    const simulateDevices = (count: number = 100) => {
      const fakeDevices: AudioOutputDevice[] = []
      for (let i = 0; i < count; i++) {
        fakeDevices.push({
          deviceId: `fake-device-${i}`,
          kind: 'audiooutput',
          label: `Simulated Speaker ${i + 1} - High Definition Audio Device`,
          groupId: `fake-group-${i}`
        })
      }
      devices.value = [...devices.value, ...fakeDevices]
      MessagePlugin.info(i18n.global.t('settings.audioOutput.virtualDevicesGenerated', { count }))
    }

    const setDevice = async (deviceId: string) => {
      if (deviceId === currentDeviceId.value) return

      try {
        currentDeviceId.value = deviceId

        if (activeABChannel.value === 'A') {
          primaryDeviceId.value = deviceId
        } else {
          secondaryDeviceId.value = deviceId
        }

        MessagePlugin.success(i18n.global.t('settings.audioOutput.switchedTo', { name: currentDeviceLabel.value }))
      } catch (err: any) {
        console.error('Failed to set audio device:', err)
        error.value = err.message
        MessagePlugin.error(i18n.global.t('settings.audioOutput.switchFailed'))
      }
    }

    const setRustDevice = async (deviceId: number) => {
      if (deviceId === currentRustDeviceId.value) return

      try {
        await invoke('audio__set_output_device', { deviceId })
        currentRustDeviceId.value = deviceId

        // Update A/B state
        const device = rustDevices.value.find((d) => d.id === deviceId)
        if (activeABChannel.value === 'A') {
          primaryDeviceId.value = String(deviceId)
        } else {
          secondaryDeviceId.value = String(deviceId)
        }

        // Update is_default flags
        rustDevices.value = rustDevices.value.map((d) => ({
          ...d,
          is_default: d.id === deviceId
        }))

        MessagePlugin.success(i18n.global.t('settings.audioOutput.switchedTo', { name: device?.name || deviceId }))
      } catch (err: any) {
        console.error('Failed to set Rust audio device:', err)
        error.value = err.message || i18n.global.t('settings.audioOutput.switchFailed')
        MessagePlugin.error(i18n.global.t('settings.audioOutput.switchFailed'))
      }
    }

    const setDeviceVolume = async (deviceId: number, volume: number) => {
      try {
        await invoke('audio__set_device_volume', { deviceId, volume })
        // Update local state
        rustDevices.value = rustDevices.value.map((d) =>
          d.id === deviceId ? { ...d, volume } : d
        )
      } catch (err: any) {
        console.error('Failed to set device volume:', err)
      }
    }

    const getDeviceVolume = async (deviceId: number): Promise<number | null> => {
      try {
        const vol = await invoke<number>('audio__get_device_volume', { deviceId })
        return vol
      } catch {
        return null
      }
    }

    const toggleAB = () => {
      if (activeABChannel.value === 'A') {
        if (secondaryDeviceId.value && secondaryDeviceId.value !== primaryDeviceId.value) {
          activeABChannel.value = 'B'
          setDevice(secondaryDeviceId.value)
        } else {
          MessagePlugin.info(i18n.global.t('settings.audioOutput.setCompareDevice'))
        }
      } else {
        activeABChannel.value = 'A'
        setDevice(primaryDeviceId.value)
      }
    }

    const handleDeviceChange = () => {
      scanDevices()
    }

    const init = async () => {
      if (!rustDeviceChangeUnlisten) {
        try {
          rustDeviceChangeUnlisten = await listen('audio-device-changed', () => {
            scanRustDevices()
          })
        } catch {
          // Event system not available in non-Tauri environment
        }
      }

      // Listen for Web API device change
      if (navigator.mediaDevices) {
        navigator.mediaDevices.removeEventListener('devicechange', handleDeviceChange)
        navigator.mediaDevices.addEventListener('devicechange', handleDeviceChange)
      }

      await scanDevices()
    }

    const destroy = () => {
      if (rustDeviceChangeUnlisten) {
        rustDeviceChangeUnlisten()
        rustDeviceChangeUnlisten = null
      }
      navigator.mediaDevices?.removeEventListener('devicechange', handleDeviceChange)
    }

    const playTestSound = (deviceId: string) => {
      try {
        const AudioContextClass = window.AudioContext || (window as any).webkitAudioContext
        const ctx = new AudioContextClass()

        if ((ctx as any).setSinkId && deviceId !== 'default') {
          try {
            ;(ctx as any).setSinkId(deviceId)
          } catch (e) {
            console.warn('Test sound routing failed', e)
          }
        }

        const osc = ctx.createOscillator()
        const gain = ctx.createGain()

        osc.connect(gain)
        gain.connect(ctx.destination)

        osc.type = 'sine'
        osc.frequency.setValueAtTime(440, ctx.currentTime)
        osc.frequency.exponentialRampToValueAtTime(880, ctx.currentTime + 0.1)

        gain.gain.setValueAtTime(0.1, ctx.currentTime)
        gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.5)

        osc.start()
        osc.stop(ctx.currentTime + 0.5)

        setTimeout(() => {
          ctx.close()
        }, 600)
      } catch (err: any) {
        console.error('Test sound failed', err)
        MessagePlugin.error(i18n.global.t('settings.audioOutput.testPlayFailed'))
      }
    }

    return {
      supported,
      rustSupported,
      devices,
      rustDevices,
      allDevices,
      sortedDevices,
      currentDeviceId,
      currentRustDeviceId,
      isLoading,
      error,
      deviceStats,
      currentDeviceLabel,
      primaryDeviceId,
      secondaryDeviceId,
      activeABChannel,
      scanDevices,
      setDevice,
      setRustDevice,
      setDeviceVolume,
      getDeviceVolume,
      toggleAB,
      init,
      destroy,
      playTestSound,
      simulateDevices
    }
  },
  {
    persist: {
      paths: ['currentDeviceId', 'currentRustDeviceId', 'primaryDeviceId', 'secondaryDeviceId']
    } as any
  }
)
