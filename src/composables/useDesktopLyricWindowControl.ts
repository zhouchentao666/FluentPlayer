import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { SetDesktopLyricBounds, SetDesktopLyricIgnoreMouseEvents } from '@bridge/app'
import { Window } from '@bridge/runtime'
import type { DesktopLyricConfig } from './useConfig'

const RESIZE_BORDER = 10
const MIN_WIDTH = 440
const MIN_HEIGHT = 120
const MAX_WIDTH = 1600
const MAX_HEIGHT = 300

type ResizeEdge = '' | 'left' | 'right' | 'top' | 'bottom' | 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'

export function useDesktopLyricWindowControl(config: { value: DesktopLyricConfig }, locked: { value: boolean }) {
  const cursorStyle = ref('')
  const isHovered = ref(false)

  const resizeState = reactive({
    isResizing: false,
    resizeEdge: '' as ResizeEdge,
    startX: 0,
    startY: 0,
    startWinX: 0,
    startWinY: 0,
    winWidth: 0,
    winHeight: 0,
  })

  function computeEdge(clientX: number, clientY: number): ResizeEdge {
    const w = window.innerWidth
    const h = window.innerHeight
    const b = RESIZE_BORDER
    const nearLeft = clientX <= b
    const nearRight = clientX >= w - b
    const nearTop = clientY <= b
    const nearBottom = clientY >= h - b
    if (nearLeft && nearTop) return 'top-left'
    if (nearRight && nearTop) return 'top-right'
    if (nearLeft && nearBottom) return 'bottom-left'
    if (nearRight && nearBottom) return 'bottom-right'
    if (nearLeft) return 'left'
    if (nearRight) return 'right'
    if (nearTop) return 'top'
    if (nearBottom) return 'bottom'
    return ''
  }

  function edgeCursor(edge: ResizeEdge): string {
    if (edge === 'top-left' || edge === 'bottom-right') return 'nwse-resize'
    if (edge === 'top-right' || edge === 'bottom-left') return 'nesw-resize'
    if (edge === 'left' || edge === 'right') return 'ew-resize'
    if (edge === 'top' || edge === 'bottom') return 'ns-resize'
    return ''
  }

  function onPointerDown(event: PointerEvent) {
    if (locked.value || event.button !== 0) return
    const target = event.target as HTMLElement | null
    if (target?.closest('.dl-btn, .dl-play-title')) return

    const edge = computeEdge(event.clientX, event.clientY)
    if (!edge) {
      // 中心区域：发起原生窗口拖拽
      Window.StartDragging().catch(() => {})
      return
    }

    resizeState.isResizing = true
    resizeState.resizeEdge = edge
    cursorStyle.value = edgeCursor(edge)

    resizeState.startX = event.screenX ?? 0
    resizeState.startY = event.screenY ?? 0
    resizeState.startWinX = config.value.x
    resizeState.startWinY = config.value.y
    resizeState.winWidth = config.value.width || 800
    resizeState.winHeight = config.value.height || 180

    event.preventDefault()
  }

  function clampSize(val: number, min: number, max: number): number {
    return Math.max(min, Math.min(max, val))
  }

  function onPointerMove(event: PointerEvent) {
    if (locked.value) return

    if (!resizeState.isResizing) {
      const edge = computeEdge(event.clientX, event.clientY)
      cursorStyle.value = edge ? edgeCursor(edge) : ''
      return
    }

    const dx = (event.screenX ?? 0) - resizeState.startX
    const dy = (event.screenY ?? 0) - resizeState.startY

    let newX = resizeState.startWinX
    let newY = resizeState.startWinY
    let newWidth = resizeState.winWidth
    let newHeight = resizeState.winHeight
    const edge = resizeState.resizeEdge
    if (edge.includes('right')) newWidth = resizeState.winWidth + dx
    if (edge.includes('bottom')) newHeight = resizeState.winHeight + dy
    if (edge.includes('left')) {
      newX = resizeState.startWinX + dx
      newWidth = resizeState.winWidth - dx
    }
    if (edge.includes('top')) {
      newY = resizeState.startWinY + dy
      newHeight = resizeState.winHeight - dy
    }
    newWidth = clampSize(newWidth, MIN_WIDTH, MAX_WIDTH)
    newHeight = clampSize(newHeight, MIN_HEIGHT, MAX_HEIGHT)
    if (edge.includes('left')) newX = resizeState.startWinX + resizeState.winWidth - newWidth
    if (edge.includes('top')) newY = resizeState.startWinY + resizeState.winHeight - newHeight
    pushBounds(newX, newY, newWidth, newHeight)
  }

  let pendingBounds: { x: number; y: number; width: number; height: number } | null = null
  let boundsRaf = 0

  function pushBounds(x: number, y: number, width: number, height: number) {
    pendingBounds = { x, y, width, height }
    config.value.x = x
    config.value.y = y
    config.value.width = width
    config.value.height = height
    if (boundsRaf) return
    boundsRaf = requestAnimationFrame(() => {
      boundsRaf = 0
      if (!pendingBounds) return
      const b = pendingBounds
      pendingBounds = null
      SetDesktopLyricBounds(b.x, b.y, b.width, b.height).catch(() => {})
    })
  }

  function onPointerUp() {
    if (!resizeState.isResizing) return
    resizeState.isResizing = false
    resizeState.resizeEdge = ''
    cursorStyle.value = ''
    if (boundsRaf) {
      cancelAnimationFrame(boundsRaf)
      boundsRaf = 0
    }
    if (pendingBounds) {
      const b = pendingBounds
      pendingBounds = null
      SetDesktopLyricBounds(b.x, b.y, b.width, b.height).catch(() => {})
    }
  }

  function onMouseMove() {
    isHovered.value = true
  }

  function onMouseLeave() {
    isHovered.value = false
  }

  async function setLocked(value: boolean) {
    config.value.isLock = value
    await SetDesktopLyricIgnoreMouseEvents(value).catch(() => {})
  }

  onMounted(() => {
    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('pointermove', onPointerMove)
    document.addEventListener('pointerup', onPointerUp)
    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseleave', onMouseLeave)
    SetDesktopLyricIgnoreMouseEvents(config.value.isLock).catch(() => {})
  })

  onBeforeUnmount(() => {
    document.removeEventListener('pointerdown', onPointerDown)
    document.removeEventListener('pointermove', onPointerMove)
    document.removeEventListener('pointerup', onPointerUp)
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseleave', onMouseLeave)
  })

  return {
    cursorStyle,
    isHovered,
    setLocked,
  }
}
