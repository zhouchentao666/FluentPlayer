import { defineStore } from 'pinia'
import { computed, ref, watch, toRaw } from 'vue'
import { ControlAudioStore } from './ControlAudio'
import type { SongList } from '../types/audio'
import type { MusicSource, UserInfo } from '../types/userInfo'
import i18n from '@/locales'

const SUBSONIC_SOURCE: MusicSource = {
  name: 'Subsonic',
  type: i18n.global.t('plugin.builtInSource'),
  qualitys: ['128k', '320k', 'flac'],
}

function hasValidSubsonicConfig(userInfo: UserInfo): boolean {
  const config = userInfo.subsonicConfig
  return !!(
    config?.enabled &&
    config.baseUrl?.trim() &&
    config.username?.trim() &&
    config.password
  )
}

function mergeBuiltInSources(userInfo: UserInfo, sources: Record<string, MusicSource> = {}): Record<string, MusicSource> {
  const merged = { ...sources }
  if (hasValidSubsonicConfig(userInfo)) {
    merged.subsonic = SUBSONIC_SOURCE
  } else {
    delete merged.subsonic
  }
  return merged
}

function ensureBuiltInSources(userInfo: UserInfo): void {
  userInfo.supportedSources = mergeBuiltInSources(userInfo, userInfo.supportedSources || {})
}

function debounce<T extends (...args: any[]) => void>(fn: T, ms: number): T {
  let timer: ReturnType<typeof setTimeout> | null = null
  return ((...args: any[]) => {
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => { fn(...args); timer = null }, ms)
  }) as T
}

export interface PlaylistRow {
  id: string
  name: string
  description: string
  coverImgUrl: string
  source: string
  meta: string
  createTime: string
  updateTime: string
  songCount: number
}

export interface PlaylistSongRow {
  playlist_id: string
  songmid: string
  position: number
  data: string
  name: string
  singer: string
  albumName: string
  img: string
}

export const LocalUserDetailStore = defineStore('Local', () => {
  const list = ref<SongList[]>([])
  const userInfo = ref<UserInfo>({})
  const initialization = ref(false)
  const isWatchStarted = ref(false)

  // Persisted playlists from Rust SQLite
  const playlists = ref<PlaylistRow[]>([])
  const playlistsLoaded = ref(false)

  function init(): void {
    const UserInfoLocal = localStorage.getItem('userInfo')
    const ListLocal = localStorage.getItem('songList')
    if (UserInfoLocal) {
      userInfo.value = JSON.parse(UserInfoLocal) as UserInfo
      if (!userInfo.value.sourceQualityMap) userInfo.value.sourceQualityMap = {}
      ensureBuiltInSources(userInfo.value)
    } else {
      userInfo.value = {
        lastPlaySongId: null, topBarStyle: false, mainColor: '#00DAC0',
        volume: 80, currentTime: 0, selectSources: 'wy', sourceQualityMap: {}, hasGuide: false
      }
      ensureBuiltInSources(userInfo.value)
      localStorage.setItem('userInfo', JSON.stringify(userInfo.value))
    }
    if (ListLocal) {
      list.value = JSON.parse(ListLocal) as SongList[]
    } else {
      list.value = []
      localStorage.setItem('songList', JSON.stringify([]))
    }
    initialization.value = true
    const Audio = ControlAudioStore()
    startWatch()
    Audio.setVolume(userInfo.value.volume as number)
    loadPlaylists()
  }

  function startWatch() {
    if (isWatchStarted.value) return
    isWatchStarted.value = true
    const saveList = debounce((val: SongList[]) => {
      const cleaned = toRaw(val).map(s => {
        if ((s as any).source === 'local' && (s as any).img && String((s as any).img).startsWith('data:')) {
          const { img, ...rest } = s as any
          return rest as SongList
        }
        return s
      })
      localStorage.setItem('songList', JSON.stringify(cleaned))
    }, 500)
    const saveUserInfo = debounce((val: UserInfo) => {
      localStorage.setItem('userInfo', JSON.stringify(toRaw(val)))
    }, 500)
    watch(list, (newVal) => saveList(newVal), { deep: true })
    watch(userInfo, (newVal) => saveUserInfo(newVal), { deep: true })
  }

  function addSong(song: SongList) {
    if (!list.value.find((item) => item.songmid === song.songmid)) list.value.push(song)
    return list.value
  }

  function addSongToFirst(song: SongList) {
    const existingIndex = list.value.findIndex((item) => item.songmid === song.songmid)
    if (existingIndex !== -1) {
      const existingSong = list.value.splice(existingIndex, 1)[0]
      list.value.unshift(existingSong)
    } else {
      list.value.unshift(song)
    }
    return list.value
  }

  function removeSong(songId: number | string) {
    const index = list.value.findIndex((item) => item.songmid === songId)
    if (index !== -1) {
      const newList = [...list.value]
      newList.splice(index, 1)
      list.value = newList
    }
  }

  function clearList() { list.value = [] }

  function replaceSongList(songs: SongList[]) {
    const seen = new Set<string | number>()
    const deduped: SongList[] = []
    for (const s of songs) {
      const mid = (s as any).songmid
      if (!seen.has(mid)) { seen.add(mid); deduped.push(s) }
    }
    list.value = deduped
    return list.value
  }

  const userSource = computed(() => ({
    pluginId: userInfo.value.pluginId,
    source: userInfo.value.selectSources,
    quality: (userInfo.value.sourceQualityMap || {})[userInfo.value.selectSources as string] || userInfo.value.selectQuality
  }))

  // --- Playlist IPC (Rust SQLite backend) ---

  async function loadPlaylists(): Promise<void> {
    try {
      const res = await (window as any).api?.songList?.getAll?.()
      if (res?.success && Array.isArray(res.data)) {
        playlists.value = res.data
      }
    } catch (e) {
      console.warn('[Playlist] loadPlaylists failed:', e)
    }
    playlistsLoaded.value = true
  }

  async function createPlaylist(name: string, description?: string, source?: string): Promise<PlaylistRow | null> {
    try {
      const res = await (window as any).api?.songList?.create?.(name, description, source)
      if (res?.success && res.data) {
        playlists.value.push(res.data)
        return res.data
      }
    } catch (e) {
      console.warn('[Playlist] createPlaylist failed:', e)
    }
    return null
  }

  async function deletePlaylist(id: string): Promise<boolean> {
    try {
      const res = await (window as any).api?.songList?.delete?.(id)
      if (res?.success) {
        playlists.value = playlists.value.filter(p => p.id !== id)
        return true
      }
    } catch (e) {
      console.warn('[Playlist] deletePlaylist failed:', e)
    }
    return false
  }

  async function batchDeletePlaylists(ids: string[]): Promise<boolean> {
    try {
      const res = await (window as any).api?.songList?.batchDelete?.(ids)
      if (res?.success) {
        const idSet = new Set(ids)
        playlists.value = playlists.value.filter(p => !idSet.has(p.id))
        return true
      }
    } catch (e) {
      console.warn('[Playlist] batchDeletePlaylists failed:', e)
    }
    return false
  }

  async function updatePlaylist(id: string, name: string, description: string): Promise<boolean> {
    try {
      const res = await (window as any).api?.songList?.edit?.(id, { name, description })
      if (res?.success) {
        const pl = playlists.value.find(p => p.id === id)
        if (pl) { pl.name = name; pl.description = description }
        return true
      }
    } catch (e) {
      console.warn('[Playlist] updatePlaylist failed:', e)
    }
    return false
  }

  async function updatePlaylistCover(id: string, coverUrl: string): Promise<boolean> {
    try {
      const res = await (window as any).api?.songList?.updateCover?.(id, coverUrl)
      if (res?.success) {
        const pl = playlists.value.find(p => p.id === id)
        if (pl) pl.coverImgUrl = coverUrl
        return true
      }
    } catch (e) {
      console.warn('[Playlist] updatePlaylistCover failed:', e)
    }
    return false
  }

  function songToPlaylistSong(playlistId: string, song: SongList, position: number): PlaylistSongRow {
    return {
      playlist_id: playlistId,
      songmid: String(song.songmid),
      position,
      data: JSON.stringify(song),
      name: song.name || '',
      singer: song.singer || '',
      albumName: song.albumName || '',
      img: song.img || '',
    }
  }

  async function addSongsToPlaylist(playlistId: string, songs: SongList[]): Promise<number> {
    try {
      const rows = songs.map((s, i) => songToPlaylistSong(playlistId, s, i))
      const res = await (window as any).api?.songList?.addSongs?.(playlistId, rows)
      return res?.success ? res.data : 0
    } catch (e) {
      console.warn('[Playlist] addSongsToPlaylist failed:', e)
      return 0
    }
  }

  async function removeSongFromPlaylist(playlistId: string, songmid: string): Promise<boolean> {
    try {
      const res = await (window as any).api?.songList?.removeSong?.(playlistId, songmid)
      return res?.success ?? false
    } catch (e) {
      console.warn('[Playlist] removeSongFromPlaylist failed:', e)
      return false
    }
  }

  async function removeSongsFromPlaylist(playlistId: string, songmids: string[]): Promise<boolean> {
    try {
      const res = await (window as any).api?.songList?.removeSongs?.(playlistId, songmids)
      return res?.success ?? false
    } catch (e) {
      console.warn('[Playlist] removeSongsFromPlaylist failed:', e)
      return false
    }
  }

  async function getSongsForPlaylist(playlistId: string): Promise<PlaylistSongRow[]> {
    try {
      const res = await (window as any).api?.songList?.getSongs?.(playlistId)
      return res?.success ? (res.data ?? []) : []
    } catch (e) {
      console.warn('[Playlist] getSongsForPlaylist failed:', e)
      return []
    }
  }

  async function searchSongsInPlaylist(playlistId: string, keyword: string): Promise<PlaylistSongRow[]> {
    try {
      const res = await (window as any).api?.songList?.search?.(playlistId, keyword)
      return res?.success ? (res.data ?? []) : []
    } catch (e) {
      console.warn('[Playlist] searchSongsInPlaylist failed:', e)
      return []
    }
  }

  async function moveSongInPlaylist(playlistId: string, songmid: string, toIndex: number): Promise<boolean> {
    try {
      const res = await (window as any).api?.songList?.moveSong?.(playlistId, songmid, toIndex)
      return res?.success ?? false
    } catch (e) {
      console.warn('[Playlist] moveSongInPlaylist failed:', e)
      return false
    }
  }

  async function getFavoritesId(): Promise<string | null> {
    try {
      const res = await (window as any).api?.songList?.getFavoritesId?.()
      return res?.success ? res.data : null
    } catch { return null }
  }

  async function setFavoritesId(id: string): Promise<boolean> {
    try {
      const res = await (window as any).api?.songList?.setFavoritesId?.(id)
      return res?.success ?? false
    } catch { return false }
  }

  return {
    list, userInfo, initialization, playlists, playlistsLoaded,
    init, addSong, addSongToFirst, removeSong, clearList, replaceSongList, userSource,
    loadPlaylists, createPlaylist, deletePlaylist, batchDeletePlaylists,
    updatePlaylist, updatePlaylistCover, addSongsToPlaylist, removeSongFromPlaylist,
    removeSongsFromPlaylist, getSongsForPlaylist, searchSongsInPlaylist,
    moveSongInPlaylist, getFavoritesId, setFavoritesId,
    mergeBuiltInSources,
    ensureBuiltInSources,
    hasValidSubsonicConfig
  }
}, { persist: false })
