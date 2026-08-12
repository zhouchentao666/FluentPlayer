import { ref } from 'vue'
import {
  OpenMusicFiles,
  OpenMusicFolder,
  ScanMusicFolder,
  WatchMusicFolder,
  ReadMetadata,
} from '@bridge/app'
import { type Playlist, type Song } from '../types'

const AUDIO_EXT = /\.(mp3|flac|wav|ogg|m4a|aac|opus|wma|ape|tta|ac3|dts|mp2|mid|midi)$/i

/** 把任意路径集合（文件 + 文件夹）展开为音频文件路径列表。 */
async function collectAudioPaths(paths: string[]): Promise<string[]> {
  const files: string[] = []
  const folders: string[] = []
  for (const p of paths) {
    if (AUDIO_EXT.test(p)) files.push(p)
    else folders.push(p)
  }
  const result = [...files]
  for (const folder of folders) {
    try {
      const scanned = await ScanMusicFolder(folder)
      if (scanned) result.push(...scanned)
    } catch {
      // 文件夹扫描失败忽略（可能不是目录或无可读音频）
    }
  }
  return result
}

export function usePlaylists() {
  const playlists = ref<Playlist[]>([
    { id: 'favorites', name: '我的喜欢', songs: [], folders: [] },
  ])
  const selectedId = ref<string>('favorites')

  function updatePlaylists(updated: Playlist[]) {
    playlists.value = updated
  }

  function updatePlaylist(updated: Playlist) {
    updatePlaylists(playlists.value.map(p => (p.id === updated.id ? updated : p)))
  }

  function selectPlaylist(id: string) {
    selectedId.value = id
  }

  function pathToTitle(path: string): string {
    const name = path.replace(/\\/g, '/').split('/').pop() || path
    return name.replace(/\.[^.]+$/, '')
  }

  async function createSongFromPath(path: string): Promise<Song> {
    try {
      const metadata = await ReadMetadata(path)
      return {
        id: crypto.randomUUID(),
        path,
        title: metadata.title || pathToTitle(path),
        metadata,
      }
    } catch {
      return {
        id: crypto.randomUUID(),
        path,
        title: pathToTitle(path),
      }
    }
  }

  async function uniqueSongs(existing: Song[], paths: string[]): Promise<Song[]> {
    const seen = new Set(existing.map(s => s.path))
    const added: Song[] = []
    for (const path of paths) {
      if (!seen.has(path)) {
        seen.add(path)
        added.push(await createSongFromPath(path))
      }
    }
    return [...existing, ...added]
  }

  async function addMusicFiles(playlistId: string) {
    const playlist = playlists.value.find(p => p.id === playlistId)
    if (!playlist) return
    const paths = await OpenMusicFiles()
    if (!paths || paths.length === 0) return
    const validPaths = paths.filter((p): p is string => p !== null)
    updatePlaylist({
      ...playlist,
      songs: await uniqueSongs(playlist.songs, validPaths),
    })
  }

  /**
   * 批量导入路径（文件 / 文件夹混合，常见于拖拽）：自动过滤音频文件，
   * 文件夹递归扫描后一并导入，跳过已存在路径。返回实际新增的歌曲数。
   */
  async function importPaths(playlistId: string, rawPaths: string[]): Promise<number> {
    const playlist = playlists.value.find(p => p.id === playlistId)
    if (!playlist || rawPaths.length === 0) return 0
    const audioPaths = await collectAudioPaths(rawPaths)
    if (audioPaths.length === 0) return 0
    const seen = new Set(playlist.songs.map(s => s.path))
    const added: Song[] = []
    for (const path of audioPaths) {
      if (seen.has(path)) continue
      seen.add(path)
      added.push(await createSongFromPath(path))
    }
    if (added.length === 0) return 0
    updatePlaylist({ ...playlist, songs: [...playlist.songs, ...added] })
    return added.length
  }

  async function addMusicFolder(playlistId: string) {
    const playlist = playlists.value.find(p => p.id === playlistId)
    if (!playlist) return
    const folder = await OpenMusicFolder()
    if (!folder) return
    const paths = await ScanMusicFolder(folder)
    await WatchMusicFolder(folder)
    const folders = playlist.folders.includes(folder)
      ? playlist.folders
      : [...playlist.folders, folder]
    const validPaths = paths ?? []
    updatePlaylist({
      ...playlist,
      folders,
      songs: await uniqueSongs(playlist.songs, validPaths),
    })
  }

  async function refreshFolder(folder: string) {
    const playlist = playlists.value.find(p => p.folders.includes(folder))
    if (!playlist) return
    const allPaths: string[] = []
    for (const f of playlist.folders) {
      const paths = await ScanMusicFolder(f)
      if (paths) allPaths.push(...paths)
    }
    const seen = new Set<string>()
    const uniquePaths: string[] = []
    for (const path of allPaths) {
      if (seen.has(path)) continue
      seen.add(path)
      uniquePaths.push(path)
    }
    updatePlaylist({ ...playlist, songs: await uniqueSongs([], uniquePaths) })
  }

  async function rewatchFolders() {
    for (const playlist of playlists.value) {
      for (const folder of playlist.folders) {
        try {
          await WatchMusicFolder(folder)
        } catch {
          // ignore watcher errors on startup
        }
      }
    }
  }

  function addSongs(playlistId: string, songs: Song[]) {
    const playlist = playlists.value.find(p => p.id === playlistId)
    if (!playlist) return
    const seen = new Set(playlist.songs.map(s => s.path))
    const merged = [...playlist.songs]
    for (const song of songs) {
      if (!seen.has(song.path)) {
        seen.add(song.path)
        merged.push({ ...song, id: song.id || crypto.randomUUID() })
      }
    }
    updatePlaylist({ ...playlist, songs: merged })
  }

  function replaceSongs(playlistId: string, songs: Song[]) {
    const playlist = playlists.value.find(p => p.id === playlistId)
    if (!playlist) return
    updatePlaylist({
      ...playlist,
      songs: songs.map(song => ({ ...song, id: crypto.randomUUID() })),
    })
  }

  return {
    playlists,
    selectedId,
    updatePlaylists,
    updatePlaylist,
    selectPlaylist,
    addMusicFiles,
    importPaths,
    addMusicFolder,
    refreshFolder,
    rewatchFolders,
    addSongs,
    replaceSongs,
  }
}
