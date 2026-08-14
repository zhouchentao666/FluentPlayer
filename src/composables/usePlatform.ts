import { computed } from 'vue'

export type Platform = 'windows' | 'macos' | 'linux' | 'android' | 'ios'

/**
 * 读取 Tauri 运行时注入的平台标识。
 * window.__TAURI_INTERNALS__.platform 在桌面与移动端都会被正确填充：
 *   windows / macos / linux / android / ios
 * 在纯浏览器（非 Tauri 环境，如 vite dev 预览）下返回 null。
 */
function readPlatform(): Platform | null {
  const internals = (window as unknown as {
    __TAURI_INTERNALS__?: { platform?: Platform }
  }).__TAURI_INTERNALS__
  return internals?.platform ?? null
}

const platform = readPlatform()

/** 当前平台（null 表示非 Tauri 环境 / 未知）。 */
export const currentPlatform = platform

/** 是否为桌面平台（windows / macos / linux）。非 Tauri 环境默认按桌面处理，避免误隐藏功能。 */
export const isDesktop = computed(
  () => platform === null || platform === 'windows' || platform === 'macos' || platform === 'linux'
)

/** 是否为移动平台（android / ios）。 */
export const isMobile = computed(() => platform === 'android' || platform === 'ios')
