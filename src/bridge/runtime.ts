// Wails 运行时 API 兼容层（Events / Window / Application / System）→ Tauri 实现
import { emit, listen } from '@tauri-apps/api/event'
import { getCurrentWindow, PhysicalPosition, PhysicalSize } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'

// ---------- 事件（跨窗口广播，与 Wails Events 语义一致） ----------
export interface BridgeEvent<T = unknown> {
  name: string
  data: T
}

export const Events = {
  Emit(name: string, data?: unknown): Promise<void> {
    return emit(name, data)
  },
  On(name: string, callback: (event: BridgeEvent<any>) => void): () => void {
    let unlisten: (() => void) | null = null
    let disposed = false
    listen(name, (e) => {
      callback({ name, data: e.payload })
    }).then((fn) => {
      if (disposed) fn()
      else unlisten = fn
    })
    return () => {
      disposed = true
      unlisten?.()
      unlisten = null
    }
  },
}

// ---------- 当前窗口控制 ----------
export const Window = {
  Minimise(): Promise<void> {
    return getCurrentWindow().minimize()
  },
  ToggleMaximise(): Promise<void> {
    return getCurrentWindow().toggleMaximize()
  },
  IsMaximised(): Promise<boolean> {
    return getCurrentWindow().isMaximized()
  },
  Hide(): Promise<void> {
    return getCurrentWindow().hide()
  },
  Show(): Promise<void> {
    return getCurrentWindow().show()
  },
  Close(): Promise<void> {
    return getCurrentWindow().close()
  },
  Fullscreen(): Promise<void> {
    return getCurrentWindow().setFullscreen(true)
  },
  UnFullscreen(): Promise<void> {
    return getCurrentWindow().setFullscreen(false)
  },
  SetAlwaysOnTop(onTop: boolean): Promise<void> {
    return getCurrentWindow().setAlwaysOnTop(onTop)
  },
  async Position(): Promise<{ x: number; y: number }> {
    const pos = await getCurrentWindow().outerPosition()
    return { x: pos.x, y: pos.y }
  },
  async Size(): Promise<{ width: number; height: number }> {
    const size = await getCurrentWindow().outerSize()
    return { width: size.width, height: size.height }
  },
  SetPosition(x: number, y: number): Promise<void> {
    return getCurrentWindow().setPosition(new PhysicalPosition(Math.round(x), Math.round(y)))
  },
  SetSize(width: number, height: number): Promise<void> {
    return getCurrentWindow().setSize(new PhysicalSize(Math.round(width), Math.round(height)))
  },
  StartDragging(): Promise<void> {
    return getCurrentWindow().startDragging()
  },
}

// ---------- 应用 ----------
export const Application = {
  Quit(): void {
    invoke('quit_app').catch(() => {})
  },
}

// ---------- 系统 ----------
export const System = {
  IsWindows(): boolean {
    return navigator.userAgent.includes('Windows')
  },
  IsMac(): boolean {
    return navigator.userAgent.includes('Mac')
  },
  IsLinux(): boolean {
    return navigator.userAgent.includes('Linux')
  },
}
