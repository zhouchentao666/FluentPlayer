export interface SongMetadata {
  title: string
  artist: string
  album: string
  genre: string
  year: string
  duration: number
  bitrate: number
  sample_rate?: number
  // Note: Go binds uint as number in JS bindings
}

export interface Song {
  id: string
  path: string
  title: string
  cover?: string
  metadata?: SongMetadata
  /** 在线歌曲附带的源信息（本地歌曲为空） */
  online?: import('@online/types/music').MusicInfo
}

export interface Playlist {
  id: string
  name: string
  songs: Song[]
  folders: string[]
}
