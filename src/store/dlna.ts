import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface DlnaDevice {
  usn: string
  name: string
  location: string
  address: string
}

export interface DlnaPositionInfo {
  position: number
  duration: number
}

interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export const useDlnaStore = defineStore('dlna', () => {
  const devices = ref<DlnaDevice[]>([])
  const currentDevice = ref<DlnaDevice | null>(null)
  const isSearching = ref(false)
  const errorMessage = ref<string | null>(null)
  const api = window.api.dlna
  let searchRequest: Promise<DlnaDevice[]> | null = null

  const fail = (action: string, error: unknown): Error => {
    const message = error instanceof Error ? error.message : String(error)
    const failure = new Error(message || `DLNA ${action} failed`)
    errorMessage.value = failure.message
    console.error(`DLNA ${action} error`, failure)
    return failure
  }

  const request = async <T>(action: string, invoke: () => Promise<ApiResponse<T>>): Promise<T> => {
    try {
      const response = await invoke()
      if (!response?.success) throw new Error(response?.error || `DLNA ${action} failed`)
      errorMessage.value = null
      return response.data as T
    } catch (error) {
      throw fail(action, error)
    }
  }

  const startSearch = async (): Promise<DlnaDevice[]> => {
    if (searchRequest) return searchRequest
    isSearching.value = true
    searchRequest = request<DlnaDevice[]>('start search', () => api.startSearch()).then(foundDevices => {
      const selectedUsn = currentDevice.value?.usn
      devices.value = foundDevices
      currentDevice.value = selectedUsn
        ? foundDevices.find(device => device.usn === selectedUsn) || null
        : null
      return foundDevices
    })
    try {
      return await searchRequest
    } finally {
      searchRequest = null
      isSearching.value = false
    }
  }

  const stopSearch = async () => {
    try {
      return await request('stop search', () => api.stopSearch())
    } finally {
      isSearching.value = false
    }
  }

  const selectDevice = (usn: string | null) => {
    currentDevice.value = usn ? devices.value.find(device => device.usn === usn) || null : null
    return currentDevice.value
  }

  const play = async (url: string, title: string): Promise<void> => {
    const device = currentDevice.value
    if (!device) throw fail('play', new Error('No DLNA device selected'))
    await request('play', () => api.play({ url, location: device.location, title }))
  }

  const pause = () => request('pause', () => api.pause())
  const resume = () => request('resume', () => api.resume())
  const stop = () => request('stop', () => api.stop())
  const seek = (seconds: number) => request('seek', () => api.seek(seconds))
  const setVolume = (volume: number) => request('set volume', () => api.setVolume(volume))
  const getPosition = () => request<DlnaPositionInfo>('get position', () => api.getPosition())

  return {
    devices,
    currentDevice,
    isSearching,
    errorMessage,
    startSearch,
    stopSearch,
    selectDevice,
    play,
    pause,
    resume,
    stop,
    seek,
    setVolume,
    getPosition
  }
})
