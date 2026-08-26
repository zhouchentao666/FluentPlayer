import type {
  SongListAPI,
  IPCResponse,
  BatchOperationResult,
  RemoveSongsResult,
  SongListStatistics,
  SongStatistics,
  IntegrityCheckResult,
  RepairResult
} from '@/types/songList'
import type { SongList, Songs } from '@/types/songList'

const isElectron = typeof window !== 'undefined' && window.api && window.api.songList

class SongListService implements SongListAPI {
  private get songListAPI() {
    if (!isElectron) {
      throw new Error('当前环境不支持 Electron API 调用')
    }
    return window.api.songList
  }

  async create(
    name: string,
    description: string = '',
    source: SongList['source'] = 'local',
    meta: Record<string, any> = {}
  ): Promise<IPCResponse<{ id: string }>> {
    try {
      return await this.songListAPI.create(name, description, source, meta)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '创建歌单失败'
      }
    }
  }

  async getAll(): Promise<IPCResponse<SongList[]>> {
    try {
      return await this.songListAPI.getAll()
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '获取歌单列表失败'
      }
    }
  }

  async getById(hashId: string): Promise<IPCResponse<SongList | null>> {
    try {
      return await this.songListAPI.getById(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '获取歌单信息失败'
      }
    }
  }

  async delete(hashId: string): Promise<IPCResponse> {
    try {
      return await this.songListAPI.delete(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '删除歌单失败'
      }
    }
  }

  async batchDelete(hashIds: string[]): Promise<IPCResponse<BatchOperationResult>> {
    try {
      return await this.songListAPI.batchDelete(hashIds)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '批量删除歌单失败'
      }
    }
  }

  async edit(
    hashId: string,
    updates: Partial<Omit<SongList, 'id' | 'createTime'>>
  ): Promise<IPCResponse> {
    try {
      return await this.songListAPI.edit(hashId, updates)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '编辑歌单失败'
      }
    }
  }

  async updateCover(hashId: string, coverImgUrl: string): Promise<IPCResponse> {
    try {
      return await this.songListAPI.updateCover(hashId, coverImgUrl)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '更新封面失败'
      }
    }
  }

  async search(keyword: string, source?: SongList['source']): Promise<IPCResponse<SongList[]>> {
    try {
      return await this.songListAPI.search(keyword, source)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '搜索歌单失败'
      }
    }
  }

  async getStatistics(): Promise<IPCResponse<SongListStatistics>> {
    try {
      return await this.songListAPI.getStatistics()
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '获取统计信息失败'
      }
    }
  }

  async exists(hashId: string): Promise<IPCResponse<boolean>> {
    try {
      return await this.songListAPI.exists(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '检查歌单存在性失败'
      }
    }
  }

  async addSongs(hashId: string, songs: Songs[]): Promise<IPCResponse> {
    try {
      return await this.songListAPI.addSongs(hashId, songs)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '添加歌曲失败'
      }
    }
  }

  async removeSong(hashId: string, songmid: string | number): Promise<IPCResponse<boolean>> {
    try {
      return await this.songListAPI.removeSong(hashId, songmid)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '移除歌曲失败'
      }
    }
  }

  async removeSongs(
    hashId: string,
    songmids: (string | number)[]
  ): Promise<IPCResponse<RemoveSongsResult>> {
    try {
      return await this.songListAPI.removeSongs(hashId, songmids)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '批量移除歌曲失败'
      }
    }
  }

  async clearSongs(hashId: string): Promise<IPCResponse> {
    try {
      return await this.songListAPI.clearSongs(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '清空歌单失败'
      }
    }
  }

  async getSongs(hashId: string): Promise<IPCResponse<readonly Songs[]>> {
    try {
      return await this.songListAPI.getSongs(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '获取歌曲列表失败'
      }
    }
  }

  async getSongCount(hashId: string): Promise<IPCResponse<number>> {
    try {
      return await this.songListAPI.getSongCount(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '获取歌曲数量失败'
      }
    }
  }

  async hasSong(hashId: string, songmid: string | number): Promise<IPCResponse<boolean>> {
    try {
      return await this.songListAPI.hasSong(hashId, songmid)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '检查歌曲存在性失败'
      }
    }
  }

  async getSong(hashId: string, songmid: string | number): Promise<IPCResponse<Songs | null>> {
    try {
      return await this.songListAPI.getSong(hashId, songmid)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '获取歌曲信息失败'
      }
    }
  }

  async searchSongs(hashId: string, keyword: string): Promise<IPCResponse<Songs[]>> {
    try {
      return await this.songListAPI.searchSongs(hashId, keyword)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '搜索歌曲失败'
      }
    }
  }

  async getSongStatistics(hashId: string): Promise<IPCResponse<SongStatistics>> {
    try {
      return await this.songListAPI.getSongStatistics(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '获取歌曲统计信息失败'
      }
    }
  }

  async validateIntegrity(hashId: string): Promise<IPCResponse<IntegrityCheckResult>> {
    try {
      return await this.songListAPI.validateIntegrity(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '验证数据完整性失败'
      }
    }
  }

  async repairData(hashId: string): Promise<IPCResponse<RepairResult>> {
    try {
      return await this.songListAPI.repairData(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '修复数据失败'
      }
    }
  }

  async forceSave(hashId: string): Promise<IPCResponse> {
    try {
      return await this.songListAPI.forceSave(hashId)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '强制保存失败'
      }
    }
  }

  async reorderSongs(
    hashId: string,
    songmids: (string | number)[]
  ): Promise<IPCResponse<{ updated: number }>> {
    try {
      return await this.songListAPI.reorderSongs(hashId, songmids)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '重排歌单失败'
      }
    }
  }

  async moveSong(
    hashId: string,
    songmid: string | number,
    toIndex: number
  ): Promise<IPCResponse<boolean>> {
    try {
      return await this.songListAPI.moveSong(hashId, songmid, toIndex)
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : '移动歌曲失败'
      }
    }
  }

  async createLocal(name: string, description?: string): Promise<IPCResponse<{ id: string }>> {
    return this.create(name, description, 'local')
  }

  async getPlaylistDetail(hashId: string): Promise<{
    playlist: SongList | null
    songs: readonly Songs[]
    success: boolean
    error?: string
  }> {
    try {
      const [playlistRes, songsRes] = await Promise.all([
        this.getById(hashId),
        this.getSongs(hashId)
      ])

      if (!playlistRes.success) {
        return {
          playlist: null,
          songs: [],
          success: false,
          error: playlistRes.error
        }
      }

      return {
        playlist: playlistRes.data || null,
        songs: songsRes.success ? songsRes.data || [] : [],
        success: true
      }
    } catch (error) {
      return {
        playlist: null,
        songs: [],
        success: false,
        error: error instanceof Error ? error.message : '获取歌单详情失败'
      }
    }
  }

  async safeDelete(hashId: string, confirmCallback?: () => Promise<boolean>): Promise<IPCResponse> {
    if (confirmCallback) {
      const confirmed = await confirmCallback()
      if (!confirmed) {
        return { success: false, error: '用户取消删除操作' }
      }
    }
    return this.delete(hashId)
  }

  async checkAndRepair(hashId: string): Promise<{
    needsRepair: boolean
    repairResult?: RepairResult
    success: boolean
    error?: string
  }> {
    try {
      const integrityRes = await this.validateIntegrity(hashId)
      if (!integrityRes.success) {
        return { needsRepair: false, success: false, error: integrityRes.error }
      }

      const { isValid } = integrityRes.data!
      if (isValid) {
        return { needsRepair: false, success: true }
      }

      const repairRes = await this.repairData(hashId)
      return {
        needsRepair: true,
        repairResult: repairRes.data,
        success: repairRes.success,
        error: repairRes.error
      }
    } catch (error) {
      return {
        needsRepair: false,
        success: false,
        error: error instanceof Error ? error.message : '检查修复失败'
      }
    }
  }
}

export const songListAPI = new SongListService()
export default songListAPI
export type { SongListAPI, IPCResponse }
