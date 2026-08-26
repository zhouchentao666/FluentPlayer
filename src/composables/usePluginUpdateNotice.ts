import { ref } from 'vue'
import PluginRunner, { type PluginUpdateNotice } from '@/utils/plugin/PluginRunner'

export type { PluginUpdateNotice }

const notices = ref<PluginUpdateNotice[]>([])
let listenerAttached = false
let detachListener: (() => void) | null = null

function ensureListener() {
  if (listenerAttached) return
  listenerAttached = true
  detachListener = PluginRunner.onUpdateNotice((notice) => {
    notices.value.push({ ...notice })
  })
}

function removeNotice(index: number) {
  notices.value.splice(index, 1)
}

function clearNotices() {
  notices.value = []
}

export function usePluginUpdateNotice() {
  ensureListener()

  return { notices, removeNotice, clearNotices }
}
