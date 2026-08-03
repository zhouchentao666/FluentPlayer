import { ref } from 'vue'

export interface ToastItem {
  id: number
  message: string
  type: 'info' | 'success' | 'error' | 'warning'
}

export const toasts = ref<ToastItem[]>([])

let nextId = 1

export function toast(message: string, type: ToastItem['type'] = 'info', durationMs = 3200) {
  const id = nextId++
  toasts.value = [...toasts.value, { id, message, type }]
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }, durationMs)
}
