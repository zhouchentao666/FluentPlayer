import type { MusicInfo, OnlineSource } from "@online/types/music"
import { createAsyncCache } from "@online/lib/cache"
import { searchAlbums as searchAlbumsImpl } from "./search"
import { getWyAlbumDetail, getWyAlbumTags, getWyHotAlbums } from "./wy"
import { getTxAlbumDetail, getTxAlbumTags, getTxHotAlbums } from "./tx"
import { getKwAlbumDetail, getKwAlbumTags, getKwHotAlbums } from "./kw"
import { getKgAlbumDetail, getKgAlbumTags, getKgHotAlbums } from "./kg"
import { getMgAlbumDetail, getMgAlbumTags, getMgHotAlbums } from "./mg"

export interface Album {
  id: string
  name: string
  img: string | null
  author?: string
  /** ISO-ish date string from search (e.g. 2020-06-12). */
  publishTime?: string
  songCount?: number
  source: OnlineSource
}

/** Category chip for filtering the hot-album grid (mirrors PlaylistTag). */
export interface AlbumTag {
  id: string
  name: string
}

export interface AlbumDetailInfo {
  name: string
  img: string | null
  author?: string
}

export interface AlbumDetail {
  info: AlbumDetailInfo
  list: MusicInfo[]
}

function fetchAlbumDetail(source: OnlineSource, id: string, page: number): Promise<AlbumDetail> {
  switch (source) {
    case "wy":
      return getWyAlbumDetail(id, page)
    case "tx":
      return getTxAlbumDetail(id, page)
    case "kw":
      return getKwAlbumDetail(id, page)
    case "kg":
      return getKgAlbumDetail(id, page)
    case "mg":
      return getMgAlbumDetail(id, page)
    default:
      return Promise.resolve({ info: { name: "", img: null }, list: [] })
  }
}

function fetchHotAlbums(
  source: OnlineSource,
  page: number,
  tagId?: string | null
): Promise<Album[]> {
  switch (source) {
    case "wy":
      return getWyHotAlbums(page, tagId)
    case "tx":
      return getTxHotAlbums(page, tagId)
    case "kw":
      return getKwHotAlbums(page, tagId)
    case "kg":
      return getKgHotAlbums(page, tagId)
    case "mg":
      return getMgHotAlbums(page, tagId)
    default:
      return Promise.resolve([])
  }
}

function fetchAlbumTags(source: OnlineSource): Promise<AlbumTag[]> {
  switch (source) {
    case "wy":
      return getWyAlbumTags()
    case "tx":
      return getTxAlbumTags()
    case "kw":
      return getKwAlbumTags()
    case "kg":
      return getKgAlbumTags()
    case "mg":
      return getMgAlbumTags()
    default:
      return Promise.resolve([])
  }
}

const searchCache = createAsyncCache<Album[]>(3 * 60_000)
const detailCache = createAsyncCache<AlbumDetail>(5 * 60_000)
const hotCache = createAsyncCache<Album[]>(5 * 60_000)
const tagsCache = createAsyncCache<AlbumTag[]>(10 * 60_000)

/** Keyword album search; soft-fails to [] on errors (cached 3 min). */
export function searchAlbums(
  source: OnlineSource,
  query: string,
  page = 1,
  limit = 30
): Promise<Album[]> {
  return searchCache(`v4:${source}:${query.trim()}:${page}:${limit}`, () =>
    searchAlbumsImpl(source, query, page, limit)
  )
}

// 网易云 / QQ 的专辑详情接口忽略 `page`，一次性返回完整曲目列表。
// 若「加载更多」仍按页重新请求，会把同一份完整列表重复追加，造成重复。
// 因此在 wrapper 层只拉一次完整列表（缓存），再按固定页大小切片，
// 保证「加载更多」拿到的是后续片段而非重复内容。
const FULL_LIST_ALBUM_SOURCES = new Set<string>(['wy', 'tx'])
const DETAIL_PAGE_SIZE = 30

/** Album metadata + tracks; may throw on network / bad response (cached 5 min). */
export function getAlbumDetail(
  source: OnlineSource,
  id: string,
  page = 1
): Promise<AlbumDetail> {
  const p = Math.max(1, Math.floor(page) || 1)
  if (FULL_LIST_ALBUM_SOURCES.has(source)) {
    return detailCache(`${source}:${id}`, () => fetchAlbumDetail(source, id, 1)).then((full) => {
      const start = (p - 1) * DETAIL_PAGE_SIZE
      const list = full.list.slice(start, start + DETAIL_PAGE_SIZE)
      return { info: full.info, list }
    })
  }
  return detailCache(`${source}:${id}:${page}`, () => fetchAlbumDetail(source, id, page))
}

/** Platform album plaza / 新碟 list (cached 5 min). */
export function getHotAlbums(
  source: OnlineSource,
  page = 1,
  tagId?: string | null
): Promise<Album[]> {
  return hotCache(`v2:${source}:${page}:${tagId ?? ""}`, () => fetchHotAlbums(source, page, tagId))
}

/** Category chips for the album plaza (cached 10 min). */
export function getAlbumTags(source: OnlineSource): Promise<AlbumTag[]> {
  return tagsCache(`v2:${source}`, () => fetchAlbumTags(source))
}
