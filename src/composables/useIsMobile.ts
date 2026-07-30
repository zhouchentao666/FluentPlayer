import { ref } from 'vue'

// 移动端判定阈值（竖屏手机 / 小屏平板）
const MOBILE_QUERY = '(max-width: 820px)'

// 模块级单例，保证所有组件共享同一状态
const isMobile = ref(false)
let media: MediaQueryList | null = null
let removeListener: (() => void) | null = null

function evaluate() {
  isMobile.value = media ? media.matches : window.matchMedia(MOBILE_QUERY).matches
}

export function useIsMobile() {
  if (!media) {
    media = window.matchMedia(MOBILE_QUERY)
    evaluate()
    const handler = () => evaluate()
    if (typeof media.addEventListener === 'function') {
      media.addEventListener('change', handler)
      removeListener = () => media?.removeEventListener('change', handler)
    } else {
      // 兼容旧版 Safari
      media.addListener(handler)
      removeListener = () => media?.removeListener(handler)
    }
  }
  return { isMobile }
}
