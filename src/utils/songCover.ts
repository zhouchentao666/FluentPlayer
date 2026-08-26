import { musicSdk, type MusicItem } from '@/services/musicSdk'
import type { SongList } from '@/types/audio'
import { resolveImageUrl } from '@/utils/imageProxy'

type CoverSong = Pick<SongList, 'songmid' | 'source'> & { img?: string; name?: string; singer?: string }

const COVER_BATCH_SIZE = 12
const coverUrlCache = new Map<string, string>()
const coverUrlInflight = new Map<string, Promise<string | null>>()

function coverKey(song: CoverSong): string {
  const id = song.songmid ? String(song.songmid) : `${song.name || ''}:${song.singer || ''}`
  return `${song.source || ''}:${id}`
}

async function resolveCoverUrl<T extends CoverSong>(song: T, loadCover: (song: T) => Promise<string | null | undefined>): Promise<string | null> {
  const key = coverKey(song)
  if (coverUrlCache.has(key)) return coverUrlCache.get(key) || null

  const existing = coverUrlInflight.get(key)
  if (existing) return existing

  const request = (async () => {
    try {
      const url = await loadCover(song)
      const normalized = typeof url === 'string' && url ? resolveImageUrl(url) : null
      if (normalized) coverUrlCache.set(key, normalized)
      return normalized
    } catch (error) {
      console.warn('获取歌曲封面失败:', error)
      return null
    } finally {
      coverUrlInflight.delete(key)
    }
  })()

  coverUrlInflight.set(key, request)
  return request
}

interface FillMissingCoverOptions<T extends CoverSong> {
  onBatchComplete?: () => void
  resolver: (song: T) => Promise<string | null | undefined>
}

async function fillMissingCovers<T extends CoverSong>(songs: T[], options: FillMissingCoverOptions<T>) {
  const songsNeedCover = songs.filter(song => !song.img)
  for (let index = 0; index < songsNeedCover.length; index += COVER_BATCH_SIZE) {
    const batch = songsNeedCover.slice(index, index + COVER_BATCH_SIZE)
    await Promise.all(batch.map(async (song) => {
      const url = await resolveCoverUrl(song, options.resolver)
      if (url && !song.img) song.img = url
    }))
    options.onBatchComplete?.()
  }
}

export async function fillMissingSongCovers(songs: MusicItem[], options: { onBatchComplete?: () => void } = {}) {
  return fillMissingCovers(songs, {
    onBatchComplete: options.onBatchComplete,
    resolver: song => musicSdk.getPic(song)
  })
}

export async function fillMissingCoversWithResolver<T extends CoverSong>(songs: T[], options: FillMissingCoverOptions<T>) {
  return fillMissingCovers(songs, options)
}
