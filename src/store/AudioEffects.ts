import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface AudioEffectsState {
  surround: {
    enabled: boolean
    mode: 'off' | 'small' | 'medium' | 'large'
  }
  balance: {
    enabled: boolean
    value: number
  }
}

export const useAudioEffectsStore = defineStore(
  'audioEffects',
  () => {
    const surround = ref({
      enabled: false,
      mode: 'off' as const
    })

    const balance = ref({
      enabled: true,
      value: 0
    })

    const resetEffects = () => {
      surround.value = { enabled: false, mode: 'off' }
      balance.value = { enabled: true, value: 0 }
    }

    return {
      surround,
      balance,
      resetEffects
    }
  },
  {
    persist: true
  }
)
