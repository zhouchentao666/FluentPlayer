import { httpFetch as tauriFetch } from "@online/lib/http"
import { eapi } from "@online/lib/platforms/wy/eapi"

// Ported from lx-music-desktop: src/renderer/utils/musicSdk/wy/hotSearch.js
//
// 网易云热搜有多个网关，返回结构各不相同且时常变动（eapi/batch 会把结果挂在
// 以接口路径为 key 的字段上，而不是固定的 data.itemList），这是此前"热搜数据
// 缺失"的根因。这里改为「多接口依次兜底 + 结构无关的递归提取」。

const UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36"

/** 关键词可能出现的字段名（不同网关命名不同）。 */
const KEY_FIELDS = ["searchWord", "first", "keyword", "searchword", "word"]

/**
 * 结构无关地把响应里的热搜词收集出来。
 * 网易返回体可能是 `data.itemList[].searchWord`、`result.hots[].first`、
 * 或 eapi/batch 的 `["/api/search/chart/detail"].data.itemList[]`，
 * 递归提取可一次覆盖全部形态。
 */
function collectKeywords(node: unknown, out: string[], depth = 0): void {
  if (node == null || depth > 8) return
  if (Array.isArray(node)) {
    for (const item of node) collectKeywords(item, out, depth + 1)
    return
  }
  if (typeof node !== "object") return
  const obj = node as Record<string, unknown>
  for (const field of KEY_FIELDS) {
    const v = obj[field]
    if (typeof v === "string" && v.trim()) {
      out.push(v.trim())
      return
    }
  }
  for (const v of Object.values(obj)) collectKeywords(v, out, depth + 1)
}

async function postJson(url: string, body: string): Promise<unknown> {
  const res = await tauriFetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
      "User-Agent": UA,
      origin: "https://music.163.com",
      referer: "https://music.163.com/",
    },
    body,
  })
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  return res.json()
}

/** eapi/batch 热搜榜（词条最全，含 30 条）。 */
async function viaEapiChart(): Promise<string[]> {
  const form = eapi("/api/search/chart/detail", { id: "HOT_SEARCH_SONG#@#" })
  const data = await postJson(
    "http://interface.music.163.com/eapi/batch",
    new URLSearchParams(form).toString(),
  )
  const out: string[] = []
  collectKeywords(data, out)
  return out
}

/** 网页端热搜详情接口（`result.hots[].first`）。 */
async function viaHotDetail(): Promise<string[]> {
  const data = await postJson(
    "https://music.163.com/api/search/hot/detail",
    new URLSearchParams({ type: "1111" }).toString(),
  )
  const out: string[] = []
  collectKeywords(data, out)
  return out
}

/** 最老的简版热搜接口，作为最后兜底。 */
async function viaHotSimple(): Promise<string[]> {
  const data = await postJson(
    "https://music.163.com/api/search/hot",
    new URLSearchParams({ type: "1111" }).toString(),
  )
  const out: string[] = []
  collectKeywords(data, out)
  return out
}

export async function getWyHotSearch(): Promise<string[]> {
  const providers = [viaEapiChart, viaHotDetail, viaHotSimple]
  let lastError: unknown = null
  for (const provider of providers) {
    try {
      const list = await provider()
      if (list.length) return list
    } catch (e) {
      lastError = e
    }
  }
  if (lastError) throw new Error(`网易云热搜获取失败：${(lastError as Error)?.message ?? lastError}`)
  return []
}
