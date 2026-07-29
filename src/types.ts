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
}

export interface Playlist {
  id: string
  name: string
  songs: Song[]
  folders: string[]
}
