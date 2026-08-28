import { fetch } from '@tauri-apps/plugin-http'

const cache = new Map<string, string>()

function guessMime(url: string): string {
  const ext = url.split('?')[0].split('.').pop()?.toLowerCase() ?? ''
  switch (ext) {
    case 'png':
      return 'image/png'
    case 'webp':
      return 'image/webp'
    case 'gif':
      return 'image/gif'
    case 'jpg':
    case 'jpeg':
      return 'image/jpeg'
    default:
      return 'image/jpeg'
  }
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  const chunk = 0x8000
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk))
  }
  return btoa(binary)
}

/**
 * 在线封面通常是跨域远程地址，Pixi 的 WebGL 纹理加载会在 CORS 校验失败时失败，
 * 导致全屏播放器的动态背景变黑。这里通过 Tauri 的 http 插件（走 Rust，不受浏览器
 * CORS 限制）把图片抓取成本地 data URL，再交给动态背景使用。
 *
 * 已为 base64/data URL 的本地封面直接返回，避免无谓请求。
 */
export async function toDisplayableCover(url: string | null | undefined): Promise<string | null> {
  if (!url) return null
  if (url.startsWith('data:') || url.startsWith('asset:') || url.startsWith('file://') || url.startsWith('http://asset')) {
    return url
  }
  if (cache.has(url)) return cache.get(url)!

  try {
    const res = await fetch(url, { method: 'GET' })
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    const buf = await res.arrayBuffer()
    const mime = res.headers.get('content-type')?.split(';')[0] || guessMime(url)
    const dataUrl = `data:${mime};base64,${bytesToBase64(new Uint8Array(buf))}`
    cache.set(url, dataUrl)
    return dataUrl
  } catch {
    // 代理失败时退回原 URL（动态背景可能变黑，但封面图本身仍可用）
    return url
  }
}
