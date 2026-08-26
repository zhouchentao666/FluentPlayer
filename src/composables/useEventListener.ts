import { onUnmounted } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'

type Target = EventTarget | null | undefined

interface Registration {
  dispose: () => void
}

export function useLifecycle() {
  const registrations: Registration[] = []

  const onCleanup = () => {
    for (const reg of registrations) {
      try { reg.dispose() } catch {}
    }
    registrations.length = 0
  }

  onUnmounted(onCleanup)

  function addEventListener(
    target: Target,
    event: string,
    handler: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions
  ) {
    target?.addEventListener(event, handler, options)
    registrations.push({
      dispose: () => target?.removeEventListener(event, handler, options)
    })
  }

  function addTauriListen(unlisten: Promise<UnlistenFn>) {
    let fn: UnlistenFn | null = null
    unlisten.then((f) => { fn = f }).catch(() => {})
    registrations.push({ dispose: () => fn?.() })
  }

  function addIpcListener(channel: string, handler: (...args: any[]) => void) {
    const ipc = (window as any).electron?.ipcRenderer
    try { ipc?.on?.(channel, handler) } catch {}
    registrations.push({
      dispose: () => { try { ipc?.removeListener?.(channel, handler) } catch {} }
    })
  }

  function addInterval(callback: () => void, ms: number) {
    const id = setInterval(callback, ms)
    registrations.push({ dispose: () => clearInterval(id) })
    return () => clearInterval(id)
  }

  function addTimeout(callback: () => void, ms: number) {
    let cleared = false
    const id = setTimeout(() => { cleared = true; callback() }, ms)
    registrations.push({ dispose: () => { if (!cleared) clearTimeout(id) } })
    return () => { cleared = true; clearTimeout(id) }
  }

  function addCleanup(fn: () => void) {
    registrations.push({ dispose: fn })
  }

  return {
    addEventListener,
    addTauriListen,
    addIpcListener,
    addInterval,
    addTimeout,
    addCleanup,
    dispose: onCleanup
  }
}
