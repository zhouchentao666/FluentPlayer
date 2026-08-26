import type playList from './playList'
export type Songs = playList

export type SongList = {
  id: string
  name: string
  createTime: string
  updateTime: string
  description: string
  coverImgUrl: string
  source: 'local' | 'wy' | 'tx' | 'mg' | 'kg' | 'kw' | 'bd' | (string & {})
  meta: Record<string, any>
}

export interface IPCResponse<T = any> {
  success: boolean
  data?: T
  error?: string
  message?: string
  code?: string
}

export interface SongListAPI {
  create(name: string, description?: string, source?: SongList['source']): Promise<IPCResponse<{ id: string }>>
  getAll(): Promise<IPCResponse<SongList[]>>
  getById(hashId: string): Promise<IPCResponse<SongList | null>>
  delete(hashId: string): Promise<IPCResponse>
  batchDelete(hashIds: string[]): Promise<IPCResponse<BatchOperationResult>>
  edit(hashId: string, updates: Partial<Omit<SongList, 'id' | 'createTime'>>): Promise<IPCResponse>
  updateCover(hashId: string, coverImgUrl: string): Promise<IPCResponse>
  search(keyword: string, source?: SongList['source']): Promise<IPCResponse<SongList[]>>
  getStatistics(): Promise<IPCResponse<SongListStatistics>>
  exists(hashId: string): Promise<IPCResponse<boolean>>
  addSongs(hashId: string, songs: Songs[]): Promise<IPCResponse>
  removeSong(hashId: string, songmid: string | number): Promise<IPCResponse<boolean>>
  removeSongs(hashId: string, songmids: (string | number)[]): Promise<IPCResponse<RemoveSongsResult>>
  clearSongs(hashId: string): Promise<IPCResponse>
  getSongs(hashId: string): Promise<IPCResponse<readonly Songs[]>>
  getSongCount(hashId: string): Promise<IPCResponse<number>>
  hasSong(hashId: string, songmid: string | number): Promise<IPCResponse<boolean>>
  getSong(hashId: string, songmid: string | number): Promise<IPCResponse<Songs | null>>
  searchSongs(hashId: string, keyword: string): Promise<IPCResponse<Songs[]>>
  getSongStatistics(hashId: string): Promise<IPCResponse<SongStatistics>>
  validateIntegrity(hashId: string): Promise<IPCResponse<IntegrityCheckResult>>
  repairData(hashId: string): Promise<IPCResponse<RepairResult>>
  forceSave(hashId: string): Promise<IPCResponse>
  reorderSongs(hashId: string, songmids: (string | number)[]): Promise<IPCResponse<{ updated: number }>>
  moveSong(hashId: string, songmid: string | number, toIndex: number): Promise<IPCResponse<boolean>>
}

export interface BatchOperationResult {
  success: string[]
  failed: string[]
}

export interface RemoveSongsResult {
  removed: number
  notFound: number
}

export interface SongListStatistics {
  total: number
  bySource: Record<string, number>
  lastUpdated: string
}

export interface SongStatistics {
  total: number
  bySinger: Record<string, number>
  byAlbum: Record<string, number>
  lastModified: string
}

export interface IntegrityCheckResult {
  isValid: boolean
  issues: string[]
}

export interface RepairResult {
  fixed: boolean
  changes: string[]
}
