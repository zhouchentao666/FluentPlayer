import { ref } from 'vue'
import type { Song } from '../types'

export interface LocalSongMetadata {
  title?: string
  artist?: string
  album?: string
  cover?: string
  lyrics?: string
  lyricsFormat?: 'auto' | 'lrc' | 'lrc-a2' | 'yrc' | 'qrc' | 'eslrc' | 'ttml'
}

export const localMetadata = ref<Record<string, LocalSongMetadata>>({})

export function getLocalMetadata(path: string): LocalSongMetadata | undefined {
  return localMetadata.value[path]
}

export function setLocalMetadata(path: string, meta: LocalSongMetadata) {
  localMetadata.value[path] = { ...localMetadata.value[path], ...meta }
}

export function clearLocalMetadata(path: string) {
  delete localMetadata.value[path]
}

export function displayTitle(song: Song): string {
  return localMetadata.value[song.path]?.title || song.metadata?.title || song.title
}

export function displayArtist(song: Song): string {
  return localMetadata.value[song.path]?.artist || song.metadata?.artist || '未知艺术家'
}

export function displayAlbum(song: Song): string {
  return localMetadata.value[song.path]?.album || song.metadata?.album || ''
}

export function displayCover(song: Song): string | undefined {
  return localMetadata.value[song.path]?.cover
}

export function displayLyrics(song: Song): string | undefined {
  return localMetadata.value[song.path]?.lyrics
}
