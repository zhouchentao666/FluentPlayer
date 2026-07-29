import { ref, computed, watch, type Ref } from 'vue'
import type { Playlist, Song } from '../types'

export type SortMode = 'custom' | 'title' | 'filename' | 'artist' | 'album' | 'duration' | 'year' | 'folder'
export type SortOrder = 'asc' | 'desc'

const sortLabels: Record<SortMode, string> = {
  custom: '自定义',
  title: '标题',
  filename: '文件名',
  artist: '艺术家',
  album: '专辑',
  duration: '时长',
  year: '年份',
  folder: '文件夹',
}

function filename(path: string) {
  return path.replace(/\\/g, '/').split('/').pop() || path
}

function folder(path: string) {
  const parts = path.replace(/\\/g, '/').split('/')
  parts.pop()
  return parts.join('/') || path
}

function sortValue(song: Song, mode: SortMode): string | number {
  switch (mode) {
    case 'title':
      return song.metadata?.title || song.title || ''
    case 'filename':
      return filename(song.path)
    case 'artist':
      return song.metadata?.artist || ''
    case 'album':
      return song.metadata?.album || ''
    case 'duration':
      return song.metadata?.duration ?? 0
    case 'year':
      return song.metadata?.year || ''
    case 'folder':
      return folder(song.path)
    default:
      return ''
  }
}

export function usePlaylistView(playlist: Ref<Playlist>) {
  const searchQuery = ref('')
  const sortMode = ref<SortMode>('custom')
  const sortOrder = ref<SortOrder>('asc')
  const batchMode = ref(false)
  const selectedIds = ref<Set<string>>(new Set())

  const displaySongs = computed(() => {
    let songs = [...playlist.value.songs]
    const query = searchQuery.value.trim().toLowerCase()
    if (query) {
      songs = songs.filter((song) => {
        const text = [
          song.metadata?.title,
          song.metadata?.artist,
          song.metadata?.album,
          song.title,
          filename(song.path),
        ]
          .filter(Boolean)
          .join(' ')
          .toLowerCase()
        return text.includes(query)
      })
    }
    if (sortMode.value !== 'custom') {
      const order = sortOrder.value === 'asc' ? 1 : -1
      songs.sort((a, b) => {
        const av = sortValue(a, sortMode.value)
        const bv = sortValue(b, sortMode.value)
        if (typeof av === 'number' && typeof bv === 'number') {
          return (av - bv) * order
        }
        return String(av).localeCompare(String(bv), undefined, { numeric: true }) * order
      })
    }
    return songs
  })

  const selectedSongs = computed(() =>
    displaySongs.value.filter(song => selectedIds.value.has(song.id))
  )

  const allSelected = computed(() =>
    displaySongs.value.length > 0 && displaySongs.value.every(song => selectedIds.value.has(song.id))
  )

  function toggleSelection(id: string) {
    const next = new Set(selectedIds.value)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    selectedIds.value = next
  }

  function selectAll() {
    selectedIds.value = new Set(displaySongs.value.map(song => song.id))
  }

  function clearSelection() {
    selectedIds.value = new Set()
  }

  function exitBatchMode() {
    batchMode.value = false
    clearSelection()
  }

  watch(() => playlist.value.id, exitBatchMode)

  return {
    searchQuery,
    sortMode,
    sortOrder,
    batchMode,
    selectedIds,
    displaySongs,
    selectedSongs,
    allSelected,
    sortLabels,
    toggleSelection,
    selectAll,
    clearSelection,
    exitBatchMode,
  }
}
