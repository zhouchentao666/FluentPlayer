import { Events } from '@bridge/runtime'
import { watch, type Ref } from 'vue'
import { ToggleDesktopLyric, CloseDesktopLyric } from '@bridge/app'
import type { AppSettings } from './useConfig'

export interface UseDesktopLyricOptions {
  settings: Ref<AppSettings>
}

export function useDesktopLyric(options: UseDesktopLyricOptions) {
  const { settings } = options

  async function applyEnabled(value: boolean) {
    await ToggleDesktopLyric(value).catch(() => {})
    if (!value) {
      await CloseDesktopLyric().catch(() => {})
    }
  }

  function setEnabled(value: boolean) {
    settings.value.desktopLyric.enabled = value
  }

  function toggle() {
    settings.value.desktopLyric.enabled = !settings.value.desktopLyric.enabled
  }

  const unwatchEnabled = watch(
    () => settings.value.desktopLyric.enabled,
    (value) => {
      applyEnabled(value)
    }
  )

  const offClose = Events.On('desktop-lyric:close', () => {
    settings.value.desktopLyric.enabled = false
  })

  const offLock = Events.On('desktop-lyric:lock-changed', (event: any) => {
    settings.value.desktopLyric.isLock = !!event?.data?.locked
  })

  async function openIfEnabled() {
    if (settings.value.desktopLyric.enabled) {
      await applyEnabled(true)
    }
  }

  function dispose() {
    unwatchEnabled()
    offClose?.()
    offLock?.()
  }

  return { setEnabled, toggle, openIfEnabled, dispose }
}
