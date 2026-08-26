/**
 * 性能监控器
 * 监控 FPS 并在性能不足时触发降级
 */
export class PerformanceMonitor {
  private fps: number = 60
  private lowFpsCount: number = 0
  private lastTime: number = performance.now()
  private frameCount: number = 0
  private lastFpsUpdate: number = performance.now()

  private readonly fpsThreshold: number = 20
  private readonly lowFpsFrames: number = 300
  private readonly degradeCallback: () => void
  private readonly enabled: boolean

  constructor(options: {
    fpsThreshold?: number
    lowFpsFrames?: number
    onDegrade: () => void
    enabled?: boolean
  }) {
    this.fpsThreshold = options.fpsThreshold ?? 20
    this.lowFpsFrames = options.lowFpsFrames ?? 300
    this.degradeCallback = options.onDegrade
    this.enabled = options.enabled ?? true
  }

  tick(): 'ok' | 'degrade' | 'disabled' {
    if (!this.enabled) return 'disabled'

    const now = performance.now()
    this.lastTime = now
    this.frameCount++

    if (now - this.lastFpsUpdate >= 500) {
      this.fps = (this.frameCount * 1000) / (now - this.lastFpsUpdate)
      this.frameCount = 0
      this.lastFpsUpdate = now
      performanceTelemetry.recordFps(this.fps)
    }

    if (this.fps < this.fpsThreshold) {
      this.lowFpsCount++
      if (this.lowFpsCount >= this.lowFpsFrames) {
        this.degradeCallback()
        return 'degrade'
      }
    } else {
      this.lowFpsCount = 0
    }

    return 'ok'
  }

  getFPS(): number {
    return this.fps
  }

  reset(): void {
    this.lowFpsCount = 0
    this.frameCount = 0
    this.lastFpsUpdate = performance.now()
    this.lastTime = performance.now()
  }

  isLowPerformance(): boolean {
    return this.lowFpsCount > this.lowFpsFrames * 0.5
  }
}

export class PerformanceDegrader {
  private hasDegraded: boolean = false
  private monitor: PerformanceMonitor | null = null
  private rafId: number | null = null
  private onDegradeCallback?: (newConfig: any) => void

  start(options: {
    onTick?: (fps: number) => void
    onDegrade: (newConfig: any) => void
    enabled?: boolean
  }) {
    this.onDegradeCallback = options.onDegrade

    this.monitor = new PerformanceMonitor({
      onDegrade: () => this.handleDegrade(),
      enabled: options.enabled ?? true
    })

    const tick = () => {
      if (!this.monitor) return

      const result = this.monitor.tick()

      if (options.onTick) {
        options.onTick(this.monitor.getFPS())
      }

      if (result === 'degrade') {
        this.handleDegrade()
      }

      this.rafId = requestAnimationFrame(tick)
    }

    this.rafId = requestAnimationFrame(tick)
  }

  stop() {
    if (this.rafId) {
      cancelAnimationFrame(this.rafId)
      this.rafId = null
    }
    this.monitor = null
  }

  private handleDegrade() {
    if (this.hasDegraded || !this.onDegradeCallback) return

    this.hasDegraded = true

    const degradedConfig = {
      renderScale: 0.3,
      fps: 15,
      audioResponse: false,
      flowSpeed: 0.5,
      staticMode: true,
      preset: 'custom' as const
    }

    this.onDegradeCallback(degradedConfig)
    this.stop()
  }

  reset() {
    this.hasDegraded = false
    this.monitor?.reset()
  }

  hasDegradedConfig(): boolean {
    return this.hasDegraded
  }
}

export interface PerfSnapshot {
  fps: number
  ipcAvgMs: number
  ipcP95Ms: number
  renderAvgMs: number
  memoryMb: number | null
  ipcCount: number
  renderCount: number
}

class PerformanceTelemetry {
  private ipcDurations: number[] = []
  private renderDurations: number[] = []
  private fpsHistory: number[] = []
  private memoryHistory: number[] = []
  private memoryTimer: number | null = null
  private panelTimer: number | null = null
  private panelEl: HTMLDivElement | null = null

  recordIpc(durationMs: number) {
    this.ipcDurations.push(durationMs)
    if (this.ipcDurations.length > 500) this.ipcDurations.shift()
  }

  recordRender(durationMs: number) {
    this.renderDurations.push(durationMs)
    if (this.renderDurations.length > 500) this.renderDurations.shift()
  }

  recordFps(fps: number) {
    this.fpsHistory.push(fps)
    if (this.fpsHistory.length > 120) this.fpsHistory.shift()
  }

  startMemorySampling(intervalMs: number = 3000) {
    if (this.memoryTimer) return
    this.memoryTimer = window.setInterval(async () => {
      let mb: number | null = null

      const mem = (performance as any).memory
      if (mem && typeof mem.usedJSHeapSize === 'number') {
        mb = mem.usedJSHeapSize / 1024 / 1024
      } else {
        try {
          const res = await (window as any).api?.performance?.getMemory?.()
          if (res && typeof res.rss_mb === 'number') {
            mb = res.rss_mb
          }
        } catch {}
      }

      if (typeof mb === 'number' && Number.isFinite(mb)) {
        this.memoryHistory.push(mb)
        if (this.memoryHistory.length > 120) this.memoryHistory.shift()
      }
    }, intervalMs)
  }

  stopMemorySampling() {
    if (this.memoryTimer) {
      clearInterval(this.memoryTimer)
      this.memoryTimer = null
    }
  }

  getSnapshot(): PerfSnapshot {
    const ipc = this.ipcDurations
    const render = this.renderDurations
    const fps = this.fpsHistory

    const ipcAvgMs = ipc.length ? ipc.reduce((a, b) => a + b, 0) / ipc.length : 0
    const renderAvgMs = render.length ? render.reduce((a, b) => a + b, 0) / render.length : 0
    const fpsAvg = fps.length ? fps.reduce((a, b) => a + b, 0) / fps.length : 60

    const sortedIpc = [...ipc].sort((a, b) => a - b)
    const p95Index = sortedIpc.length ? Math.floor(sortedIpc.length * 0.95) : 0
    const ipcP95Ms = sortedIpc.length ? sortedIpc[p95Index] : 0

    const memoryMb = this.memoryHistory.length
      ? this.memoryHistory[this.memoryHistory.length - 1]
      : null

    return {
      fps: fpsAvg,
      ipcAvgMs,
      ipcP95Ms,
      renderAvgMs,
      memoryMb,
      ipcCount: ipc.length,
      renderCount: render.length
    }
  }

  startDevPanel() {
    if (!import.meta.env.DEV || typeof document === 'undefined' || this.panelEl) return

    const el = document.createElement('div')
    el.style.position = 'fixed'
    el.style.right = '12px'
    el.style.bottom = '12px'
    el.style.zIndex = '999999'
    el.style.background = 'rgba(0,0,0,0.72)'
    el.style.color = '#fff'
    el.style.padding = '8px 10px'
    el.style.borderRadius = '8px'
    el.style.fontSize = '12px'
    el.style.fontFamily = 'ui-monospace, SFMono-Regular, Menlo, monospace'
    el.style.lineHeight = '1.5'
    el.style.pointerEvents = 'none'
    document.body.appendChild(el)
    this.panelEl = el

    this.panelTimer = window.setInterval(() => {
      const s = this.getSnapshot()
      if (!this.panelEl) return
      this.panelEl.textContent = `FPS ${s.fps.toFixed(1)} | IPC ${s.ipcAvgMs.toFixed(1)}ms (p95 ${s.ipcP95Ms.toFixed(1)}ms) | Render ${s.renderAvgMs.toFixed(1)}ms | Mem ${s.memoryMb?.toFixed(1) ?? '-'}MB`
    }, 1000)
  }

  stopDevPanel() {
    if (this.panelTimer) {
      clearInterval(this.panelTimer)
      this.panelTimer = null
    }
    if (this.panelEl?.parentNode) {
      this.panelEl.parentNode.removeChild(this.panelEl)
    }
    this.panelEl = null
  }
}

export const performanceTelemetry = new PerformanceTelemetry()

export async function withIpcPerformance<T>(fn: () => Promise<T>): Promise<T> {
  const start = performance.now()
  try {
    return await fn()
  } finally {
    performanceTelemetry.recordIpc(performance.now() - start)
  }
}

export function markRenderPerformance(startTime: number) {
  performanceTelemetry.recordRender(performance.now() - startTime)
}
