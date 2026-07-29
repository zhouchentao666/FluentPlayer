import { computed, onBeforeUnmount, onMounted, ref, unref, type MaybeRef } from 'vue'
import { Window, Application } from '@bridge/runtime'
import { staggerPhase } from './useSharedTransition'

const STAGGER_DELAYS = [0.45, 0.6, 0.8] // phase 1/2/3 的延迟系数（相对 500ms）

export function usePlayerDetail(autoHideTopChrome: MaybeRef<boolean> = false) {
  const isMaximised = ref(false)
  const isFullscreen = ref(false)
  const isAlwaysOnTop = ref(false)

  async function updateMaximizeState() {
    isMaximised.value = await Window.IsMaximised()
  }

  async function toggleMaximize() {
    Window.ToggleMaximise()
    await updateMaximizeState()
  }

  async function toggleFullscreen() {
    if (isFullscreen.value) {
      Window.UnFullscreen()
    } else {
      Window.Fullscreen()
    }
    isFullscreen.value = !isFullscreen.value
  }

  async function toggleAlwaysOnTop() {
    isAlwaysOnTop.value = !isAlwaysOnTop.value
    await Window.SetAlwaysOnTop(isAlwaysOnTop.value)
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'F11') {
      e.preventDefault()
      toggleFullscreen()
    }
  }

  const TOP_CHROME_HIDE_DELAY = 2500
  const isTopChromeVisible = ref(false)
  let topChromeHideTimer: ReturnType<typeof setTimeout> | null = null

  const clearTopChromeHideTimer = () => {
    if (topChromeHideTimer) {
      clearTimeout(topChromeHideTimer)
      topChromeHideTimer = null
    }
  }

  const scheduleTopChromeHide = () => {
    clearTopChromeHideTimer()
    topChromeHideTimer = setTimeout(() => {
      isTopChromeVisible.value = false
      topChromeHideTimer = null
    }, TOP_CHROME_HIDE_DELAY)
  }

  const showTopChrome = () => {
    clearTopChromeHideTimer()
    isTopChromeVisible.value = true
  }

  const handleTopChromeLeave = () => {
    if (unref(autoHideTopChrome)) {
      scheduleTopChromeHide()
    }
  }

  // 配角元素交错渐入/渐出
  let staggerTimers: ReturnType<typeof setTimeout>[] = []
  const clearStaggerTimers = () => {
    staggerTimers.forEach(t => clearTimeout(t))
    staggerTimers = []
  }

  function runStaggerEnter() {
    clearStaggerTimers()
    staggerPhase.value = 0
    STAGGER_DELAYS.forEach((delay, i) => {
      staggerTimers.push(
        setTimeout(() => {
          staggerPhase.value = i + 1
        }, 500 * delay),
      )
    })
  }

  function runStaggerLeave() {
    clearStaggerTimers()
    staggerPhase.value = 0
  }

  const staggerStyle = (show: boolean, phase: number, translateDir: 'Y' | 'X' = 'Y', distance = 20) => {
    const visible = show || staggerPhase.value >= phase
    const translate = translateDir === 'Y' ? `translateY(${distance}px)` : `translateX(${distance}px)`

    return {
      opacity: visible ? 1 : 0,
      transform: visible ? 'translate(0, 0)' : translate,
      transition: `opacity 400ms cubic-bezier(0.22,1,0.36,1) ${show ? phase * 100 : 0}ms, transform 400ms cubic-bezier(0.22,1,0.36,1) ${show ? phase * 100 : 0}ms`,
    }
  }

  const minimize = () => Window.Minimise()
  const closeApp = () => Application.Quit()

  onMounted(() => {
    window.addEventListener('keydown', onKeydown)
  })

  onBeforeUnmount(() => {
    clearTopChromeHideTimer()
    clearStaggerTimers()
    window.removeEventListener('keydown', onKeydown)
  })

  return {
    staggerPhase: computed(() => staggerPhase.value),
    isTopChromeVisible,
    isMaximised,
    isFullscreen,
    isAlwaysOnTop,
    showTopChrome,
    handleTopChromeLeave,
    runStaggerEnter,
    runStaggerLeave,
    updateMaximizeState,
    staggerStyle,
    minimize,
    toggleMaximize,
    toggleFullscreen,
    toggleAlwaysOnTop,
    closeApp,
  }
}
