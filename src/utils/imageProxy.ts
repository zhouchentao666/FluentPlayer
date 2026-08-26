/**
 * 图片 URL 代理工具。
 *
 * 桌面端：保留原始 HTTP(S) 链接，加载失败时通过 imgproxy:// 兜底。
 * 移动端（Android）：主动将所有外部图片转为 http://imgproxy.localhost/ 代理 URL，
 *   因为 Android WebView 不支持自定义 scheme（ERR_UNKNOWN_URL_SCHEME），
 *   需使用 WRY 的 workaround 格式让 shouldInterceptRequest 拦截后路由到 Rust 处理器。
 */

const PROXY_PREFIX = 'imgproxy://localhost/'
const PROXY_PREFIX_ANDROID = 'http://imgproxy.localhost/'
const FALLBACK_MARK = 'imageProxyFallbackApplied'
const isMobile = /android|iphone|ipad/i.test(navigator.userAgent)

export function isProxiedImageUrl(url: string): boolean {
  return url.startsWith(PROXY_PREFIX) || url.startsWith('imgproxy://') || url.startsWith(PROXY_PREFIX_ANDROID)
}

export function isRemoteHttpImageUrl(url: string): boolean {
  return url.startsWith('http://') || url.startsWith('https://')
}

export function canProxyImageUrl(url: string): boolean {
  if (!url) return false
  if (isProxiedImageUrl(url)) return false
  // On Android, the workaround prefix is http://imgproxy.localhost/ which startsWith http://,
  // so we must exclude it before checking isRemoteHttpImageUrl
  return isRemoteHttpImageUrl(url)
}

export function getOriginalImageUrl(url: string): string {
  if (url.startsWith(PROXY_PREFIX)) {
    const encoded = url.slice(PROXY_PREFIX.length)
    try { return decodeURIComponent(encoded) } catch { return url }
  }
  if (url.startsWith(PROXY_PREFIX_ANDROID)) {
    const encoded = url.slice(PROXY_PREFIX_ANDROID.length)
    try { return decodeURIComponent(encoded) } catch { return url }
  }
  return url
}

/**
 * 将外部图片 URL 转为代理 URL。
 * 桌面端：imgproxy://localhost/{encoded}
 * Android：http://imgproxy.localhost/{encoded}（WRY workaround，Android WebView 不支持自定义 scheme）
 */
export function proxyImageUrl(url: string): string {
  if (!canProxyImageUrl(url)) return url
  const prefix = isMobile ? PROXY_PREFIX_ANDROID : PROXY_PREFIX
  return `${prefix}${encodeURIComponent(url)}`
}

/**
 * direct-first 语义：保留原始 URL，仅规范化历史已代理 URL 以外的非远程值。
 */
export function directImageUrl(url: string): string {
  return getOriginalImageUrl(url)
}

/**
 * 根据平台解析图片 URL：
 * - 移动端：外部图片主动转为 imgproxy:// 协议
 * - 桌面端：返回原始直链
 */
export function resolveImageUrl(url: string): string {
  if (!isMobile) return directImageUrl(url)
  if (isProxiedImageUrl(url)) return url
  if (canProxyImageUrl(url)) return proxyImageUrl(url)
  return url
}

const IMAGE_FIELDS = ['img', 'avatar', 'cover', 'pic', 'imgUrl', 'coverUrl', 'picUrl', 'albumImg'] as const

export function rewriteImageUrls(obj: unknown): unknown {
  if (!obj || typeof obj !== 'object') return obj

  if (Array.isArray(obj)) {
    for (const item of obj) rewriteImageUrls(item)
    return obj
  }

  const record = obj as Record<string, unknown>
  for (const field of IMAGE_FIELDS) {
    if (typeof record[field] === 'string') {
      record[field] = resolveImageUrl(record[field] as string)
    }
  }
  for (const key of Object.keys(record)) {
    if (typeof record[key] === 'object' && record[key] !== null) {
      rewriteImageUrls(record[key])
    }
  }
  return obj
}

function handleImageError(event: Event): void {
  const target = event.target
  if (!(target instanceof HTMLImageElement)) return

  const declaredSrc = target.getAttribute('src') || ''
  if (declaredSrc && !canProxyImageUrl(declaredSrc)) return

  const currentSrc = target.currentSrc || target.src || declaredSrc
  const fallbackSource = declaredSrc || currentSrc
  if (!canProxyImageUrl(fallbackSource)) return
  if (target.dataset[FALLBACK_MARK] === fallbackSource) return

  const fallbackUrl = proxyImageUrl(fallbackSource)
  if (fallbackUrl === currentSrc) return

  target.dataset[FALLBACK_MARK] = fallbackSource
  target.src = fallbackUrl
}

let imageFallbackInstalled = false

/**
 * 安装全局 <img> direct-first 失败兜底。
 * 使用捕获阶段监听 error，因为 img 的 error 事件不会冒泡。
 */
export function installImageProxyFallback(): () => void {
  if (imageFallbackInstalled) return () => {}

  window.addEventListener('error', handleImageError, true)
  imageFallbackInstalled = true

  return () => {
    window.removeEventListener('error', handleImageError, true)
    imageFallbackInstalled = false
  }
}
