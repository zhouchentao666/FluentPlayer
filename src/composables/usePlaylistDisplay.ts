import type { Song } from '../types'
import {
  displayTitle as localDisplayTitle,
  displayArtist as localDisplayArtist,
  displayAlbum as localDisplayAlbum,
} from './useLocalMetadata'

export function formatDuration(seconds: number): string {
  if (!seconds || seconds < 0) return '--:--'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

export function displayTitle(song: Song): string {
  return localDisplayTitle(song)
}

export function displayArtist(song: Song): string {
  return localDisplayArtist(song)
}

export function displayAlbum(song: Song): string {
  return localDisplayAlbum(song)
}

export function displayDuration(song: Song): string {
  return formatDuration(song.metadata?.duration ?? 0)
}
