/**
 * 全局 fetch 拦截器：图片请求先尝试 WebView 原生 fetch，
 * 失败后再通过 Tauri Rust 后端代理兜底（PixiJS 等库内部使用 fetch 加载图片）。
 */

import { canProxyImageUrl } from '@/utils/imageProxy'

const IMAGE_RE = /\.(jpg|jpeg|png|gif|webp|bmp|avif)(\?.*)?$/i
const MIME: Record<string, string> = {
  '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.png': 'image/png',
  '.gif': 'image/gif', '.webp': 'image/webp', '.bmp': 'image/bmp', '.avif': 'image/avif'
}

const _fetch = window.fetch.bind(window)

function b64toBlob(b64: string, mime: string): Blob {
  const bin = atob(b64)
  const arr = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i)
  return new Blob([arr], { type: mime })
}

function getRequestUrl(input: RequestInfo | URL): string {
  return typeof input === 'string' ? input
    : input instanceof Request ? input.url
    : input instanceof URL ? input.href
    : String(input)
}

function isImageRequest(url: string): boolean {
  return IMAGE_RE.test(url) && canProxyImageUrl(url)
}

function getMimeFromUrl(url: string): string {
  const ext = (url.match(/\.(\w+)(?:\?|$)/)?.[1] || 'jpeg').toLowerCase()
  return MIME[`.${ext}`] || 'image/jpeg'
}

async function fetchImageViaProxy(url: string): Promise<Response> {
  const proxy = (window as any).api?.httpProxy
  if (!proxy) throw new Error('HTTP proxy unavailable')

  const res = await proxy(url, { raw: true, timeout: 15000 })
  const statusCode = Number(res?.statusCode || 0)
  if (statusCode >= 400) throw new Error(`HTTP proxy failed with status ${statusCode}`)
  if (!res?.isBase64 || !res?.body) throw new Error('HTTP proxy returned empty image body')

  const mime = getMimeFromUrl(url)
  const blob = b64toBlob(res.body, mime)
  return new Response(blob, { status: statusCode || 200, headers: { 'Content-Type': mime } })
}

window.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
  const url = getRequestUrl(input)
  if (!isImageRequest(url)) return _fetch(input, init)

  let directResponse: Response | null = null
  try {
    directResponse = await _fetch(input, init)
    if (directResponse.ok) return directResponse
  } catch (e) {
    console.warn('[CORS Proxy] 图片直连失败，尝试代理兜底:', url.substring(0, 80), e)
  }

  try {
    return await fetchImageViaProxy(url)
  } catch (e) {
    console.warn('[CORS Proxy] 图片代理兜底失败:', url.substring(0, 80), e)
    if (directResponse) return directResponse
    throw e
  }
}
