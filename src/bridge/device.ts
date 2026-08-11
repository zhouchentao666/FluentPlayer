// 设备 / 平台运行时检测：一份代码双端（desktop / mobile）运行（双端共用）
//
// 设计：
// - isMobile / platform / isAndroid / isIOS 在前端运行时判断，区分桌面与移动端。
// - 不依赖 @tauri-apps/api/os（v2 未内置 os 子模块），改用 navigator.userAgent 判断，
//   移动端 webview 的 UA 稳定包含 "Android" / "iPhone" / "iPad" / "iPod"。
// - 全部为同步判断，业务代码可直接读 isMobileSync()，或在需要时调用 initDevice() 做一次性初始化。
import { ref } from 'vue'

const mobile = ref(false)
const platform = ref<'android' | 'ios' | 'windows' | 'macos' | 'linux' | 'web'>('web')

function detect(): { isMobile: boolean; platform: typeof platform.value } {
  if (typeof navigator === 'undefined') {
    return { isMobile: false, platform: 'web' }
  }
  const ua = navigator.userAgent || ''
  if (/Android/i.test(ua)) return { isMobile: true, platform: 'android' }
  if (/iPhone|iPad|iPod/i.test(ua)) return { isMobile: true, platform: 'ios' }
  if (/Windows/i.test(ua)) return { isMobile: false, platform: 'windows' }
  if (/Mac OS|Macintosh/i.test(ua)) return { isMobile: false, platform: 'macos' }
  if (/Linux/i.test(ua)) return { isMobile: false, platform: 'linux' }
  return { isMobile: false, platform: 'web' }
}

/** 应用启动时调用一次，完成平台初始化（同步，无外部依赖）。 */
export function initDevice(): void {
  const d = detect()
  mobile.value = d.isMobile
  platform.value = d.platform
}

/** 同步读取移动端状态（首次调用前也会基于 UA 即时判断）。 */
export function isMobileSync(): boolean {
  if (typeof navigator === 'undefined') return false
  // 若尚未初始化，先做即时判断（避免首帧误判为桌面）
  if (platform.value === 'web' && !mobile.value) {
    const d = detect()
    mobile.value = d.isMobile
    platform.value = d.platform
  }
  return mobile.value
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
