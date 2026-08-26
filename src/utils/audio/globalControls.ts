import { ControlAudioStore } from '@/store/ControlAudio'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

let installed = false
let globalKeyDownHandler: ((e: KeyboardEvent) => void) | null = null
let removeHotkeyListener: (() => void) | null = null

function dispatch(name: string, val?: any) {
  window.dispatchEvent(new CustomEvent('global-music-control', { detail: { name, val } }))
}

export function installGlobalMusicControls() {
  if (installed) return
  installed = true

  // 键盘快捷键
  let keyThrottle = false
  const throttle = (cb: () => void, delay: number) => {
    if (keyThrottle) return
    keyThrottle = true
    setTimeout(() => { try { cb() } finally { keyThrottle = false } }, delay)
  }

  globalKeyDownHandler = (e: KeyboardEvent) => {
    const target = e.target as HTMLElement | null
    const tag = (target?.tagName || '').toLowerCase()
    const isEditable = target?.hasAttribute('contenteditable') || ['input','textarea'].includes(tag)
    throttle(() => {
      if (isEditable) return
      if (e.code === 'Space') { e.preventDefault(); dispatch('toggle') }
      else if (e.code === 'ArrowUp') { e.preventDefault(); dispatch('volumeDelta', 5) }
      else if (e.code === 'ArrowDown') { e.preventDefault(); dispatch('volumeDelta', -5) }
      else if (e.code === 'ArrowLeft') dispatch('seekDelta', -5)
      else if (e.code === 'ArrowRight') dispatch('seekDelta', 5)
      else if (e.code === 'KeyF') { e.preventDefault(); dispatch('toggleFullPlay') }
    }, 100)
  }
  document.addEventListener('keydown', globalKeyDownHandler)

  // 媒体键（耳机/键盘上的播放控制按钮）
  try { (window as any).api?.onMusicCtrl?.(() => dispatch('toggle')) } catch {}

  // 监听 Rust 后台全局快捷键事件
  listen<string>('hotkey-triggered', (event) => {
    const action = event.payload
    if (!action) return
    const actionMap: Record<string, string> = {
      toggle: 'toggle',
      playPrev: 'playPrev',
      playNext: 'playNext',
      seekBackward: 'seekDelta',
      seekForward: 'seekDelta',
      volumeDown: 'volumeDelta',
      volumeUp: 'volumeDelta',
      toggleDesktopLyric: 'toggleDesktopLyric',
      setPlayModeSequence: 'setPlayModeSequence',
      setPlayModeRandom: 'setPlayModeRandom',
      togglePlayModeSingle: 'togglePlayModeSingle',
    }
    const mapped = actionMap[action]
    if (!mapped) return
    if (action === 'seekBackward') dispatch('seekDelta', -5)
    else if (action === 'seekForward') dispatch('seekDelta', 5)
    else if (action === 'volumeDown') dispatch('volumeDelta', -5)
    else if (action === 'volumeUp') dispatch('volumeDelta', 5)
    else dispatch(mapped)
  }).then((unlisten) => { removeHotkeyListener = unlisten })

  // 监听 Rust 媒体控制事件（系统媒体键 → 前端切歌）
  listen('media:next', () => dispatch('playNext'))
  listen('media:previous', () => dispatch('playPrev'))
}

export function uninstallGlobalMusicControls() {
  if (!installed) return
  installed = false
  if (globalKeyDownHandler) { document.removeEventListener('keydown', globalKeyDownHandler); globalKeyDownHandler = null }
  if (removeHotkeyListener) { removeHotkeyListener(); removeHotkeyListener = null }
}
