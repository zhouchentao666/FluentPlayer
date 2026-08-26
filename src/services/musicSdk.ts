import { invoke } from '@tauri-apps/api/core'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { rewriteImageUrls } from '@/utils/imageProxy'

export interface MusicItem {
  songmid: string | number
  singer: string
  name: string
  albumName: string
  albumId: string | number
  source: string
  interval: string
  img: string
  lrc: string | null
  types?: string[]
  _types?: Record<string, any>
  typeUrl?: Record<string, any>
  hash?: string
  singerId?: string
}

export interface SearchResult {
  list: MusicItem[]
  allPage: number
  limit: number
  total: number
  source: string
}

export interface PlaylistItem {
  id: string | number
  name: string
  img: string
  source: string
  desc?: string
  playCount?: number | string | null
  author?: string
  total?: number | string | null
}

export interface PlaylistResult {
  list: PlaylistItem[]
  allPage: number
  limit: number
  total: number
  source: string
}

export interface PlaylistDetailResult {
  list: MusicItem[]
  info: any
  allPage: number
  limit: number
  total: number
  source: string
}

export interface SingerInfo {
  id: string | number
  source: string
  info: {
    name: string
    desc: string
    avatar: string
    gender?: string
  }
  count: {
    music: number
    album: number
  }
}

export interface SingerAlbumItem {
  id: string | number
  count: number
  info: {
    name: string
    author: string
    img: string
    desc?: string
  }
}

export interface SingerAlbumListResult {
  list: SingerAlbumItem[]
  allPage: number
  limit: number
  total: number
  source: string
}

function getSource(): string {
  const store = LocalUserDetailStore()
  return store.userSource.source || 'kw'
}

function getSubsonicConfig(): Record<string, any> | undefined {
  const store = LocalUserDetailStore()
  const config = store.userInfo.subsonicConfig
  if (!config) return undefined
  return {
    baseUrl: config.baseUrl || '',
    username: config.username || '',
    password: config.password || '',
    apiVersion: config.apiVersion || '1.16.1',
    clientName: config.clientName || 'Mio',
  }
}

function withSourceConfig(source: string, args: Record<string, any>): Record<string, any> {
  if (source !== 'subsonic') return { ...args, source }
  return { ...args, source, subsonicConfig: args.subsonicConfig || getSubsonicConfig() }
}

function parsePlaylistPlayCount(raw: unknown): number {
  if (typeof raw === 'number') return Number.isFinite(raw) ? raw : 0
  if (typeof raw !== 'string') return 0

  const text = raw.trim().replace(/,/g, '')
  if (!text) return 0

  const unit = text.endsWith('亿') ? 100000000 : text.endsWith('万') ? 10000 : 1
  const numericText = text.replace(/[亿万]/g, '')
  const num = Number(numericText)
  return Number.isFinite(num) ? num * unit : 0
}

function comparePlaylistIdAsc(a: unknown, b: unknown): number {
  const idA = String(a ?? '')
  const idB = String(b ?? '')
  return idA.localeCompare(idB, undefined, { numeric: true, sensitivity: 'base' })
}

export const musicSdk = {
  async request(method: string, args: Record<string, any> = {}): Promise<any> {
    try {
      const source = args.source || getSource()
      const result = await invoke('service_music_sdk_request', {
        method,
        args: withSourceConfig(source, args)
      })
      return rewriteImageUrls(result)
    } catch (error) {
      console.error(`[musicSdk] request('${method}') failed:`, error)
      throw error
    }
  },

  async search(keyword: string, page = 1, limit = 30, source?: string): Promise<SearchResult> {
    return this.request('search', { keyword, page, limit, ...(source ? { source } : {}) })
  },

  async aggregateSearch(keyword: string, limit = 20): Promise<SearchResult[]> {
    try {
      const res = await invoke<SearchResult[]>('service_music_search_music', {
        name: keyword,
        singer: '',
        source: '',
        limit
      })
      const normalized = rewriteImageUrls(res) as SearchResult[]
      return (Array.isArray(normalized) ? normalized : []).filter(r => Array.isArray(r?.list) && r.list.length > 0)
    } catch (error) {
      console.error('[musicSdk] aggregateSearch failed:', error)
      throw error
    }
  },

  async tipSearch(keyword: string): Promise<any> {
    const source = getSource()
    return invoke('service_music_tip_search', { source, keyword })
  },

  async getHotSonglist(): Promise<PlaylistResult> {
    return this.request('getHotSonglist')
  },

  async getPlaylistTags(): Promise<any> {
    return this.request('getPlaylistTags')
  },

  async getCategoryPlaylists(sortId?: string, tagId?: string, page = 1, limit = 30): Promise<PlaylistResult> {
    const source = getSource()
    const res = await this.request('getCategoryPlaylists', { sortId, tagId, page, limit, source })

    if (source !== 'kw' || !Array.isArray(res?.list)) return res

    const list = [...res.list].sort((a: any, b: any) => {
      const aCount = parsePlaylistPlayCount(a?.play_count ?? a?.playCount)
      const bCount = parsePlaylistPlayCount(b?.play_count ?? b?.playCount)
      if (aCount !== bCount) return bCount - aCount
      return comparePlaylistIdAsc(a?.id, b?.id)
    })

    return { ...res, list }
  },

  async getPlaylistDetail(id: string | number, page = 1, source?: string): Promise<PlaylistDetailResult> {
    return this.request('getPlaylistDetail', { id, page, ...(source ? { source } : {}) })
  },

  async getLeaderboards(): Promise<any> {
    return this.request('getLeaderboards')
  },

  async getLeaderboardDetail(id: string | number, page = 1): Promise<PlaylistDetailResult> {
    return this.request('getLeaderboardDetail', { id, page })
  },

  async getMusicUrl(songInfo: MusicItem, quality?: string): Promise<string> {
    const res = await this.request('getMusicUrl', { songInfo, quality, source: songInfo.source })
    return res?.url || ''
  },

  async getPic(songInfo: MusicItem): Promise<string> {
    const res = await this.request('getPic', { songInfo, source: songInfo.source })
    return res?.url || ''
  },

  async getLyric(songInfo: MusicItem): Promise<string> {
    const res = await this.request('getLyric', { songInfo, source: songInfo.source })
    return res?.lyric || res?.lrc || ''
  },

  async searchPlaylist(keyword: string, page = 1, limit = 30, source?: string): Promise<PlaylistResult> {
    return this.request('searchPlaylist', { keyword, page, limit, ...(source ? { source } : {}) })
  },

  async aggregateSearchPlaylists(keyword: string, limit = 20, sources: string[] = []): Promise<PlaylistResult[]> {
    const settled = await Promise.allSettled(
      sources.map(source => this.searchPlaylist(keyword, 1, limit, source))
    )

    settled.forEach((result, index) => {
      if (result.status === 'rejected') {
        console.warn(`[musicSdk] searchPlaylist('${sources[index]}') failed:`, result.reason)
      }
    })

    return settled.flatMap((result) => {
      if (result.status !== 'fulfilled') return []
      const normalized = rewriteImageUrls(result.value) as PlaylistResult
      return Array.isArray(normalized?.list) && normalized.list.length > 0 ? [normalized] : []
    })
  },

  async getComment(songInfo: MusicItem, page = 1, limit = 30): Promise<any> {
    return this.request('getComment', { songInfo, page, limit })
  },

  async getSingerInfo(id: string | number, source?: string): Promise<SingerInfo> {
    return this.request('getSingerInfo', { id, source })
  },

  async getSingerSongList(id: string | number, page = 1, limit = 30, source?: string): Promise<SearchResult> {
    return this.request('getSingerSongList', { id, page, limit, source })
  },

  async getSingerAlbumList(id: string | number, page = 1, limit = 30, source?: string): Promise<SingerAlbumListResult> {
    return this.request('getSingerAlbumList', { id, page, limit, source })
  }
}
