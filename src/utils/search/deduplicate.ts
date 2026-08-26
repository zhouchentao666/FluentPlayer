import type { MusicItem, PlaylistItem } from '@/services/musicSdk'
import {
  normalizeSearchText,
  normalizeArtistText,
  parseDurationSeconds,
  MULTI_ARTIST_SEPARATOR,
  DUPLICATE_DURATION_TOLERANCE_SECONDS,
} from './normalize'
import { sortBySimilarity } from './similarity'

// ─── Song deduplication ────────────────────────────────────

interface AggregateSongCandidate {
  song: MusicItem
  sourceOrder: number
  itemOrder: number
  relevanceRank: number
  duplicateKey: string
  albumKey: string
  durationSeconds: number | null
}

const hasStableSongIdentity = (song: MusicItem) => Boolean(
  song.songmid ||
  song.hash ||
  (song.typeUrl && Object.keys(song.typeUrl).length > 0)
)

const buildSongDuplicateKey = (
  song: MusicItem,
  sourceOrder: number,
  itemOrder: number
): string => {
  const titleKey = normalizeSearchText(song.name)
  const singerKey = normalizeArtistText(song.singer)
  if (titleKey && singerKey && hasStableSongIdentity(song)) {
    return `${titleKey}|${singerKey}`
  }
  return `unique:${sourceOrder}:${itemOrder}:${song.source || ''}:${song.songmid || song.hash || ''}`
}

const getRelevanceRank = (song: MusicItem, normalizedQuery: string): number => {
  if (!normalizedQuery) return 5

  const title = normalizeSearchText(song.name)
  const singer = normalizeSearchText(song.singer)
  const album = normalizeSearchText(song.albumName)

  if (title && title === normalizedQuery) return 0
  if (title && title.startsWith(normalizedQuery)) return 1
  if (title && title.includes(normalizedQuery)) return 2
  if (singer && singer.includes(normalizedQuery)) return 3
  if (album && album.includes(normalizedQuery)) return 4
  return 5
}

const createSongCandidate = (
  song: MusicItem,
  normalizedQuery: string,
  sourceOrder: number,
  itemOrder: number
): AggregateSongCandidate => {
  const titleKey = normalizeSearchText(song.name)
  const singerKey = normalizeArtistText(song.singer)
  const albumKey = normalizeSearchText(song.albumName)
  const durationSeconds = parseDurationSeconds(song.interval)
  const duplicateKey = titleKey && singerKey && hasStableSongIdentity(song)
    ? `${titleKey}|${singerKey}`
    : `unique:${sourceOrder}:${itemOrder}:${song.source || ''}:${song.songmid || song.hash || ''}`

  return {
    song,
    sourceOrder,
    itemOrder,
    relevanceRank: getRelevanceRank(song, normalizedQuery),
    duplicateKey,
    albumKey,
    durationSeconds,
  }
}

const compareSongCandidates = (a: AggregateSongCandidate, b: AggregateSongCandidate) => {
  if (a.relevanceRank !== b.relevanceRank) return a.relevanceRank - b.relevanceRank
  if (a.sourceOrder !== b.sourceOrder) return a.sourceOrder - b.sourceOrder
  return a.itemOrder - b.itemOrder
}

const areLikelyDuplicateCandidates = (a: AggregateSongCandidate, b: AggregateSongCandidate) => {
  if (a.duplicateKey !== b.duplicateKey || a.duplicateKey.startsWith('unique:')) return false
  if (a.albumKey && b.albumKey && a.albumKey !== b.albumKey) return false

  if (a.durationSeconds !== null && b.durationSeconds !== null) {
    return Math.abs(a.durationSeconds - b.durationSeconds) <= DUPLICATE_DURATION_TOLERANCE_SECONDS
  }

  return Boolean(a.albumKey && b.albumKey)
}

/**
 * Deduplicate and sort aggregate song results.
 * Groups by duplicateKey, keeps best candidate per group,
 * sorts by (relevanceRank, sourceOrder, itemOrder),
 * then applies Levenshtein similarity sorting within each relevance tier.
 */
export const dedupeAndSortSongs = (
  results: Array<{ source?: string; list: MusicItem[] }>,
  query: string
): MusicItem[] => {
  const normalizedQuery = normalizeSearchText(query)
  const candidates = results.flatMap((result, sourceOrder) => {
    const source = result?.source || ''
    const list = Array.isArray(result?.list) ? result.list : []
    return list.map((song, itemOrder) => createSongCandidate(
      { ...song, source: song.source || source },
      normalizedQuery,
      sourceOrder,
      itemOrder
    ))
  })

  const buckets = new Map<string, AggregateSongCandidate[]>()
  candidates.forEach(candidate => {
    const bucket = buckets.get(candidate.duplicateKey)
    if (!bucket) {
      buckets.set(candidate.duplicateKey, [candidate])
      return
    }
    const dupIdx = bucket.findIndex(item => areLikelyDuplicateCandidates(item, candidate))
    if (dupIdx === -1) {
      bucket.push(candidate)
      return
    }
    if (compareSongCandidates(candidate, bucket[dupIdx]) < 0) {
      bucket[dupIdx] = candidate
    }
  })

  const deduped = Array.from(buckets.values()).flat()

  // Primary sort by relevance tier + source + position
  deduped.sort(compareSongCandidates)

  // Secondary sort: within same relevance rank, sort by similarity to query
  const finalList: MusicItem[] = []
  let i = 0
  while (i < deduped.length) {
    const rank = deduped[i].relevanceRank
    let j = i
    while (j < deduped.length && deduped[j].relevanceRank === rank) j++

    const tier = deduped.slice(i, j)
    if (tier.length > 1) {
      const sorted = sortBySimilarity(tier, query, c => `${c.song.name} ${c.song.singer}`)
      finalList.push(...sorted.map(c => c.song))
    } else {
      finalList.push(tier[0].song)
    }
    i = j
  }

  return finalList
}

// ─── Playlist deduplication ────────────────────────────────

export interface PlaylistCardItem {
  id: string | number
  title: string
  description: string
  cover: string
  playCount?: number | string | null
  author?: string
  total?: number | string | null
  source: string
}

const PLAYLIST_SONG_COUNT_FIELDS = [
  'songCount', 'trackCount', 'song_count', 'track_count',
  'songNum', 'songnum', 'songsCount', 'songs_count', 'total',
] as const

const parsePlaylistSongCount = (value: unknown): number | null => {
  if (typeof value === 'number') return Number.isFinite(value) && value >= 0 ? value : null
  if (typeof value !== 'string') return null
  const text = value.trim().replace(/,/g, '')
  if (!/^\d+(?:\.\d+)?$/.test(text)) return null
  const count = Number(text)
  return Number.isFinite(count) && count >= 0 ? count : null
}

const getKnownPlaylistSongCount = (item: PlaylistItem): number | null => {
  const record = item as PlaylistItem & Record<string, unknown>
  for (const field of PLAYLIST_SONG_COUNT_FIELDS) {
    const count = parsePlaylistSongCount(record[field])
    if (count !== null) return count
  }
  return null
}

const hasKnownEmptyPlaylistSongCount = (item: PlaylistItem) => getKnownPlaylistSongCount(item) === 0

export const mapPlaylistItem = (item: PlaylistItem, fallbackSource = ''): PlaylistCardItem => {
  const raw = item as PlaylistItem & { play_count?: number | string | null }
  return {
    id: item.id,
    title: item.name || '',
    description: item.desc || '',
    cover: item.img || '',
    playCount: item.playCount ?? raw.play_count ?? null,
    author: item.author || '',
    total: item.total ?? null,
    source: item.source || fallbackSource,
  }
}

const getPlaylistRelevanceRank = (playlist: PlaylistCardItem, normalizedQuery: string): number => {
  if (!normalizedQuery) return 5
  const title = normalizeSearchText(playlist.title)
  const author = normalizeSearchText(playlist.author)
  const description = normalizeSearchText(playlist.description)

  if (title && title === normalizedQuery) return 0
  if (title && title.startsWith(normalizedQuery)) return 1
  if (title && title.includes(normalizedQuery)) return 2
  if (author && author.includes(normalizedQuery)) return 3
  if (description && description.includes(normalizedQuery)) return 4
  return 5
}

interface AggregatePlaylistCandidate {
  playlist: PlaylistCardItem
  sourceOrder: number
  itemOrder: number
  relevanceRank: number
  duplicateKey: string
}

const createPlaylistDuplicateKey = (
  playlist: PlaylistCardItem,
  sourceOrder: number,
  itemOrder: number
): string => {
  const sourceKey = normalizeSearchText(playlist.source)
  const idKey = String(playlist.id ?? '').trim()
  if (sourceKey && idKey) return `id:${sourceKey}:${idKey}`

  const titleKey = normalizeSearchText(playlist.title)
  const authorKey = normalizeSearchText(playlist.author)
  if (sourceKey && titleKey && authorKey) return `meta:${sourceKey}:${titleKey}:${authorKey}`

  return `unique:${sourceOrder}:${itemOrder}`
}

const comparePlaylistCandidates = (a: AggregatePlaylistCandidate, b: AggregatePlaylistCandidate) => {
  if (a.relevanceRank !== b.relevanceRank) return a.relevanceRank - b.relevanceRank
  if (a.sourceOrder !== b.sourceOrder) return a.sourceOrder - b.sourceOrder
  return a.itemOrder - b.itemOrder
}

/**
 * Deduplicate and sort aggregate playlist results.
 * Keeps best candidate per duplicate key, sorted by relevance.
 */
export const dedupeAndSortPlaylists = (
  results: Array<{ source?: string; list: PlaylistItem[] }>,
  query: string,
  sourceOrderMap?: Map<string, number>
): PlaylistCardItem[] => {
  const normalizedQuery = normalizeSearchText(query)
  const candidates = results.flatMap((result, resultOrder) => {
    const source = result?.source || ''
    const sourceOrder = sourceOrderMap?.get(source) ?? resultOrder
    const list = Array.isArray(result?.list)
      ? result.list.filter(item => !hasKnownEmptyPlaylistSongCount(item))
      : []

    return list.map((item, itemOrder): AggregatePlaylistCandidate => {
      const playlist = mapPlaylistItem(item, source)
      return {
        playlist,
        sourceOrder,
        itemOrder,
        relevanceRank: getPlaylistRelevanceRank(playlist, normalizedQuery),
        duplicateKey: createPlaylistDuplicateKey(playlist, sourceOrder, itemOrder),
      }
    })
  })

  const bestByKey = new Map<string, AggregatePlaylistCandidate>()
  candidates.forEach(candidate => {
    const current = bestByKey.get(candidate.duplicateKey)
    if (!current || comparePlaylistCandidates(candidate, current) < 0) {
      bestByKey.set(candidate.duplicateKey, candidate)
    }
  })

  return Array.from(bestByKey.values())
    .sort(comparePlaylistCandidates)
    .map(candidate => candidate.playlist)
}
