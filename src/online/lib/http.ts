import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

/**
 * Tauri 的 plugin-http 响应体是 ReadableStream，调用方若对某些接口（如 QQ 评论）
 * 直接 `await res.json()` 偶发 `Failed to execute 'close' on
 * 'ReadableStreamDefaultController': Unexpected end of JSON input`——这是 Tauri
 * 在 body 流被重复 close 时的已知问题。
 *
 * 这里对 Tauri 返回的 Response 做一次包装：覆写 `json()`，改为先 `text()` 再解析，
 * 既避开流二次 close，又对空/损坏响应给出安全的兜底，而不是直接抛错。
 */
function safeResponse(res: Response): Response {
  return new Proxy(res, {
    get(target, prop, receiver) {
      if (prop === 'json') {
        return async () => {
          try {
            const text = await target.text()
            if (!text) return {}
            return JSON.parse(text)
          } catch {
            return {}
          }
        }
      }
      const value = Reflect.get(target, prop, receiver)
      return typeof value === 'function' ? value.bind(target) : value
    },
  })
}

export async function httpFetch(
  url: string,
  options: RequestInit & { timeout?: number; responseType?: 'json' | 'text' } = {},
): Promise<Response> {
  const { responseType, ...rest } = options
  const res = await tauriFetch(url, {
    ...rest,
    responseType: responseType === 'text' ? 'Text' : undefined,
  } as RequestInit)
  return safeResponse(res)
}
