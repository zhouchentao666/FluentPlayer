// 设备 / 平台运行时检测：一份代码双端（desktop / mobile）运行（双端共用）
//
// 设计：
// - isMobile / platform / isAndroid / isIOS 在前端运行时判断，区分桌面与移动端。
// - 优先用 Tauri 的 app.getPlatform() 精确判断；非 Tauri 环境（纯 Web / 预览）退化为 userAgent 判断。
// - 全部为惰性异步初始化 + 同步快照，业务代码可直接读 isMobile.value 这种响应式值，
//   也可调用 isMobileSync() 做同步分支（如 import 流程）。
import { ref } from 'vue'
import type { Platform } from '@tauri-apps/api/os'

const mobile = ref(false)
const platform = ref<Platform | 'web'>('web')
let initialized = false

function uaIsMobile(): boolean {
  if (typeof navigator === 'undefined') return false
  const ua = navigator.userAgent || ''
  return /Android|iPhone|iPad|iPod|Mobile|Windows Phone/i.test(ua)
}

// 是否在 Tauri 运行时内（含 desktop 与 mobile）
function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

/** 异步初始化平台信息，应在 app 启动时调用一次。 */
export async function initDevice(): Promise<void> {
  if (initialized) return
  initialized = true
  try {
    if (inTauri()) {
      const { getPlatform } = await import('@tauri-apps/api/os')
      const p = await getPlatform()
      platform.value = p
      mobile.value = p === 'android' || p === 'ios'
    } else {
      platform.value = 'web'
      mobile.value = uaIsMobile()
    }
  } catch {
    platform.value = 'web'
    mobile.value = uaIsMobile()
  }
}

/** 同步读取移动端状态（初始化完成前用 UA 兜底）。 */
export function isMobileSync(): boolean {
  if (initialized) return mobile.value
  // 未初始化时先做一次同步兜底，避免 SSR/首帧误判
  return uaIsMobile()
}

export function isAndroidSync(): boolean {
  return platform.value === 'android'
}
export function isIOSSync(): boolean {
  return platform.value === 'ios'
}

export const device = {
  mobile,
  platform,
  isMobileSync,
  isAndroidSync,
  isIOSSync,
  initDevice,
}
