import type { OnlineSource } from '@online/types/music'
import { searchWangyi } from '@online/lib/search/wy'
import { searchKuwo } from '@online/lib/search/kuwo'
import { searchKugou } from '@online/lib/search/kg'
import { searchTx } from '@online/lib/search/tx'
import { searchMigu } from '@online/lib/search/mg'
import type { SearchResult } from '@online/types/music'

/** Browsing (playlists / albums / hot / hot-search) supports these 5 platforms. */
export type BrowseSource = 'wy' | 'kw' | 'kg' | 'tx' | 'mg'

export interface BrowsePlatform {
  id: BrowseSource
  name: string
}

export const BROWSE_PLATFORMS: BrowsePlatform[] = [
  { id: 'wy', name: '网易云' },
  { id: 'tx', name: 'QQ音乐' },
  { id: 'kg', name: '酷狗' },
  { id: 'kw', name: '酷我' },
  { id: 'mg', name: '咪咕' },
]

export interface SinglePlatform {
  id: BrowseSource
  name: string
  search: (query: string, page?: number, limit?: number) => Promise<SearchResult>
}

/** Single-track search. All built-in platforms return `SearchResult`. */
export const SINGLE_PLATFORMS: SinglePlatform[] = [
  { id: 'wy', name: '网易云', search: searchWangyi },
  { id: 'tx', name: 'QQ音乐', search: searchTx },
  { id: 'kg', name: '酷狗', search: searchKugou },
  { id: 'kw', name: '酷我', search: searchKuwo },
  { id: 'mg', name: '咪咕', search: searchMigu },
]

const NAME_MAP: Record<string, string> = {
  wy: '网易云',
  tx: 'QQ音乐',
  kw: '酷我',
  kg: '酷狗',
  mg: '咪咕',
  local: '本地',
}

export function platformName(source: string | undefined): string {
  if (!source) return ''
  return NAME_MAP[source] ?? source
}

export const BROWSE_SOURCE_LIST: BrowseSource[] = BROWSE_PLATFORMS.map((p) => p.id)
