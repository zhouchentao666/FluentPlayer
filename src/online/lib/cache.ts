// A tiny in-memory TTL cache for read-only third-party calls (search, charts,
// playlists, lyrics, play URLs, covers). It serves two purposes for reducing
// request frequency:
//
//   1. Result caching — within `ttlMs`, an identical key returns the previous
//      result instead of hitting the network (e.g. switching back to a chart
//      board, re-selecting a search platform, or replaying a song).
//   2. In-flight de-duplication — because we cache the *promise*, several rapid
//      identical calls (double-clicks, React effect re-runs) share one request.
//
// Failures are NOT cached: a rejected promise is evicted so a manual retry (or
// the next visit) re-fetches. A simple LRU bound keeps memory in check.

// 全局缓存注册表：用于「清理缓存」一次性清空所有在线数据缓存，
// 以及按「最大缓存」设置统一调整各缓存实例的容量上限。
interface CacheInstance {
  map: Map<string, { promise: Promise<unknown>; expires: number }>
  getLimit: () => number
}
const registry = new Set<CacheInstance>()
let globalCacheLimit = 60

/** 设置全局缓存容量上限（单位：条目数），立即作用于所有已注册/将来的缓存实例。 */
export function setGlobalCacheLimit(limit: number) {
  globalCacheLimit = Math.max(1, Math.floor(limit))
  for (const inst of registry) {
    // 仅当实例当前数量超过新上限时裁剪，避免无谓抖动。
    while (inst.map.size > inst.getLimit()) {
      const oldest = inst.map.keys().next().value
      if (oldest === undefined) break
      inst.map.delete(oldest)
    }
  }
}

/** 清空所有已注册的在线数据缓存（下次访问重新拉取）。 */
export function clearAllCaches() {
  for (const inst of registry) inst.map.clear()
}

export function createAsyncCache<T>(ttlMs: number, max?: number) {
  const map = new Map<string, { promise: Promise<T>; expires: number }>()
  const getLimit = () => max ?? globalCacheLimit

  registry.add({ map: map as CacheInstance['map'], getLimit })

  return function cached(key: string, fn: () => Promise<T>): Promise<T> {
    const hit = map.get(key)
    if (hit && Date.now() < hit.expires) {
      // Refresh recency (Map keeps insertion order → re-insert = most recent).
      map.delete(key)
      map.set(key, hit)
      return hit.promise
    }

    const promise = fn().catch((err) => {
      // Don't keep a failed request cached — allow retry to re-fetch.
      if (map.get(key)?.promise === promise) map.delete(key)
      throw err
    })

    map.set(key, { promise, expires: Date.now() + ttlMs })

    // Evict the oldest entry if we're over the bound.
    if (map.size > getLimit()) {
      const oldest = map.keys().next().value
      if (oldest !== undefined && oldest !== key) map.delete(oldest)
    }

    return promise
  }
}
