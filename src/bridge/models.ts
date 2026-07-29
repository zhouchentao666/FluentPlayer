// 与 Rust 后端约定的数据模型（纯本地播放器）

export interface SongMetadata {
  title: string
  artist: string
  album: string
  genre: string
  year: string
  duration: number
  bitrate: number
  sample_rate?: number
}

export interface AppConfig {
  playlists?: unknown[]
  settings?: Record<string, unknown> & {
    theme?: string
    accentColor?: string
    autoplay?: boolean
    savePlaylistAndSong?: boolean
    saveWindowPosition?: boolean
    windowEffect?: string
    customImagePath?: string
    customImageOpacity?: number
    customImageBlur?: number
    songColorOpacity?: number
    songColorBlur?: number
    fullScreenBackground?: string
    immersivePlayerBar?: boolean
    selectedPlaylistId?: string
    playlistSorts?: unknown
    localMetadata?: unknown
  }
  playback?: {
    playlistId?: string
    songIndex?: number
    time?: number
  }
  window?: {
    x?: number
    y?: number
    width?: number
    height?: number
  }
}
