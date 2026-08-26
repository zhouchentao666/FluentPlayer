import { defineStore } from 'pinia'
import type { SongList } from '@/types/audio'
import { LocalUserDetailStore } from './LocalUserDetail'
import PluginRunner from '@/utils/plugin/PluginRunner'
import { playSetting } from './playSetting'
import { reactive, watch, toRaw, markRaw } from 'vue'
import { analyzeImageColors, type Color } from '@/utils/color/colorExtractor'
import { resolveImageUrl } from '@/utils/imageProxy'
import { type LyricLine } from '@/types/lyric'
import {
  parseLrc as amllParseLrc,
  parseLrcA2,
  parseYrc,
  parseQrc,
  parseTTML
} from '@applemusic-like-lyrics/lyric'
import i18n from '@/locales'

function parseLyricByFormat(raw: string): LyricLine[] {
  if (!raw) return []
  const trimmed = raw.trim()
  try {
    if (/^\[(\d+),\d+]/m.test(trimmed) || /\((\d+),\d+,\d+\)/.test(trimmed)) return parseYrc(trimmed)
  } catch {}
  try {
    if (trimmed.startsWith('<?xml') || trimmed.startsWith('<Qrc')) return parseQrc(trimmed)
  } catch {}
  try {
    if (/<tt[\s>]/.test(trimmed)) return parseTTML(trimmed).lines
  } catch {}
  try {
    if (/<\d{2}:\d{2}\.\d{3}>/.test(trimmed)) return parseLrcA2(trimmed)
    return amllParseLrc(trimmed)
  } catch {
    return []
  }
}

const sanitizeLyricLines = (lines: LyricLine[]): LyricLine[] => {
  const defaultLineDuration = 3000
  const toFiniteNumber = (v: any, fallback: number) => {
    const n = typeof v === 'number' ? v : Number(v)
    return Number.isFinite(n) ? n : fallback
  }

  const cleaned: LyricLine[] = []
  for (const rawLine of lines || []) {
    const rawWords = Array.isArray((rawLine as any).words) ? (rawLine as any).words : []
    const fixedWords: any[] = []
    let prevEnd = -1

    for (const rawWord of rawWords) {
      const rawStart = toFiniteNumber(rawWord?.startTime, Number.NaN)
      const rawEnd = toFiniteNumber(rawWord?.endTime, Number.NaN)
      if (!Number.isFinite(rawStart)) continue
      let startTime = Math.max(0, rawStart)
      if (startTime < prevEnd) startTime = prevEnd
      let endTime = Number.isFinite(rawEnd) ? rawEnd : startTime + 1
      if (endTime <= startTime) endTime = startTime + 1
      prevEnd = endTime
      fixedWords.push({ ...rawWord, startTime, endTime })
    }

    if (fixedWords.length === 0) continue

    const firstWordStart = fixedWords[0].startTime
    const lastWordEnd = fixedWords[fixedWords.length - 1].endTime
    let startTime = toFiniteNumber((rawLine as any).startTime, firstWordStart)
    startTime = Math.max(0, startTime)
    let endTime = toFiniteNumber((rawLine as any).endTime, lastWordEnd)
    if (!Number.isFinite(endTime) || endTime <= startTime) endTime = startTime + defaultLineDuration
    if (endTime < lastWordEnd) endTime = lastWordEnd

    cleaned.push({
      ...(rawLine as any),
      startTime,
      endTime,
      words: fixedWords,
      translatedLyric: (rawLine as any).translatedLyric ?? '',
      romanLyric: (rawLine as any).romanLyric ?? '',
      isBG: (rawLine as any).isBG ?? false,
      isDuet: (rawLine as any).isDuet ?? false,
    })
  }

  cleaned.sort((a: any, b: any) => (a?.startTime ?? 0) - (b?.startTime ?? 0))
  return cleaned
}

const parseCrLyricBySource = (source: string, text: string): LyricLine[] => {
  return source === 'tx' ? (parseQrc(text) as any) : (parseYrc(text) as any)
}

const mergeTranslation = (base: LyricLine[], tlyric?: string): LyricLine[] => {
  if (!tlyric || base.length === 0) return base

  const translated = amllParseLrc(tlyric)
  if (!translated || translated.length === 0) return base

  const joinWords = (line: LyricLine) => (line.words || []).map((w) => w.word).join('')
  const translatedSorted = translated.slice().sort((a, b) => a.startTime - b.startTime)

  const baseTolerance = 300
  const ratioTolerance = 0.4
  const firstBase = base[0]
  const firstDuration = Math.max(1, firstBase.endTime - firstBase.startTime)
  const firstTol = Math.min(baseTolerance, firstDuration * ratioTolerance)

  let anchorIndex: number | null = null
  let bestDiff = Number.POSITIVE_INFINITY
  for (let i = 0; i < translatedSorted.length; i++) {
    const diff = Math.abs(translatedSorted[i].startTime - firstBase.startTime)
    if (diff <= firstTol && diff < bestDiff) {
      bestDiff = diff
      anchorIndex = i
    }
  }

  if (anchorIndex === null) return base

  let j = anchorIndex
  for (let i = 0; i < base.length && j < translatedSorted.length; i++, j++) {
    const bl = base[i]
    const tl = translatedSorted[j] as LyricLine
    const tlText = joinWords(tl)
    const blText = joinWords(bl)
    if (!tlText || tlText === '//' || !blText) continue
    ;(bl as any).translatedLyric = tlText
  }
  return base
}

const extractServiceLyricText = (serviceResult: any): string => {
  if (!serviceResult) return ''
  if (typeof serviceResult === 'string') return serviceResult
  if (typeof serviceResult?.data === 'string') return serviceResult.data
  const obj = typeof serviceResult === 'object' ? serviceResult : typeof serviceResult?.data === 'object' ? serviceResult.data : null
  if (obj) {
    if (typeof obj.lxlyric === 'string' && obj.lxlyric) return obj.lxlyric
    if (typeof obj.lyric === 'string' && obj.lyric) return obj.lyric
    if (typeof obj.tlyric === 'string' && obj.tlyric) return obj.tlyric
    if (typeof obj.rlyric === 'string' && obj.rlyric) return obj.rlyric
  }
  return ''
}

const getCleanSongInfo = (songInfo: any) => JSON.parse(JSON.stringify(toRaw(songInfo)))

const getLikelyServicePluginId = (songInfo: any): string | undefined => {
  const keys = ['_servicePluginId', 'servicePluginId']
  for (const key of keys) {
    const val = songInfo?.[key]
    if (typeof val === 'string' && val) return val
  }
  return undefined
}

const fetchTtmlLyrics = async (
  source: string,
  songId: string | number,
  _signal: AbortSignal
): Promise<LyricLine[] | null> => {
  const ttmlSource = source === 'wy' ? 'ncm' : source === 'tx' ? 'qq' : ''
  if (!ttmlSource) return null

  const url = `https://amll-ttml-db.stevexmh.net/${ttmlSource}/${songId}`
  const proxyResponse = await (window as any).api?.httpProxy?.(url, { method: 'GET', timeout: 10000 })
  const statusCode = Number(proxyResponse?.statusCode || 0)
  if (statusCode >= 400) throw new Error(`TTML request failed with status ${statusCode}`)
  const body = proxyResponse?.body
  const content = typeof body === 'string' ? body : ''

  if (!content || content.length < 100) throw new Error('TTML empty')
  const lines = parseTTML(content).lines as LyricLine[]
  if (!lines || lines.length === 0) throw new Error('TTML parsed empty')
  return lines
}

const fetchSdkLyrics = async (
  source: string,
  songInfo: any,
  grepLyricInfo: boolean,
  useStrictMode: boolean
): Promise<LyricLine[] | null> => {
  let lyricData: any
  const cleanInfo = getCleanSongInfo(songInfo)
  try {
    lyricData = await (window as any).api?.music?.requestSdk?.('getLyric', {
      source,
      songInfo: cleanInfo,
      grepLyricInfo,
      useStrictMode
    })
  } catch (e) {
    console.warn('[Lyrics] SDK getLyric IPC failed:', e)
    return null
  }

  if (!lyricData) {
    console.warn('[Lyrics] SDK getLyric returned empty for source:', source)
    return null
  }

  const crlyric = typeof lyricData?.crlyric === 'string' ? lyricData.crlyric.trim() : ''
  const lyric = typeof lyricData?.lyric === 'string' ? lyricData.lyric.trim() : ''
  const lrc = typeof lyricData?.lrc === 'string' ? lyricData.lrc.trim() : ''
  const hasLyricField =
    Object.prototype.hasOwnProperty.call(lyricData, 'crlyric') ||
    Object.prototype.hasOwnProperty.call(lyricData, 'lyric') ||
    Object.prototype.hasOwnProperty.call(lyricData, 'lrc')

  let lyrics: LyricLine[] = []

  if (crlyric) {
    try {
      lyrics = parseCrLyricBySource(source, crlyric)
    } catch (e) {
      console.warn('[Lyrics] parse crlyric failed, fallback to lyric/lrc:', source, e)
      lyrics = []
    }
  }

  if (!lyrics.length && lyric) {
    lyrics = parseLyricByFormat(lyric)
  }

  if (!lyrics.length && lrc) {
    lyrics = parseLyricByFormat(lrc)
  }

  if (!lyrics.length) {
    if (hasLyricField) {
      console.warn('[Lyrics] SDK getLyric parsed empty:', source, {
        keys: Object.keys(lyricData),
        hasCrlyric: !!crlyric,
        hasLyric: !!lyric,
        hasLrc: !!lrc
      })
    } else {
      console.warn('[Lyrics] SDK getLyric response has no lyric-related fields:', source, Object.keys(lyricData))
    }
  }

  lyrics = mergeTranslation(lyrics, lyricData?.tlyric)
  return lyrics.length ? lyrics : null
}

const fetchServicePluginLyrics = async (servicePluginId: string, songInfo: any): Promise<LyricLine[] | null> => {
  if (!servicePluginId) return null
  try {
    const source = songInfo?.source || 'kw'
    const result = await PluginRunner.getLyric(servicePluginId, source, getCleanSongInfo(songInfo))
    const text = extractServiceLyricText(result)
    if (!text) return null
    return parseLyricByFormat(text)
  } catch (e) {
    console.warn('[Lyrics] Service plugin getLyric failed:', e)
    return null
  }
}

const fetchLocalLyrics = async (songInfo: any): Promise<LyricLine[] | null> => {
  let text: string | null = songInfo?.lrc as string | null
  if (!text) {
    try {
      const res = await (window as any).api?.localMusic?.getLyric?.(String(songInfo?.songmid ?? ''))
      text = res?.success ? res.data : null
    } catch {
      text = null
    }
  }
  if (!text) return null
  return parseLyricByFormat(text)
}

const resolveLyrics = async (
  source: string,
  songId: string,
  songInfo: any,
  options: { grepLyricInfo: boolean; useStrictMode: boolean },
  abort: AbortSignal
): Promise<LyricLine[]> => {
  if (source === 'wy' || source === 'tx') {
    const sdkPromise = fetchSdkLyrics(source, songInfo, options.grepLyricInfo, options.useStrictMode)
    try {
      const ttml = await fetchTtmlLyrics(source, songId, abort)
      if (ttml?.length) return sanitizeLyricLines(ttml)
    } catch (e: any) {
      if (e?.name === 'AbortError') throw e
    }
    const sdk = await sdkPromise
    if (!sdk?.length) console.warn('[Lyrics] resolveLyrics: TTML and SDK both empty for', source, songId)
    return sdk?.length ? sanitizeLyricLines(sdk) : []
  }

  if (source === 'local') {
    const localLyrics = await fetchLocalLyrics(songInfo)
    if (!localLyrics?.length) console.warn('[Lyrics] resolveLyrics: local lyrics empty for', songInfo?.songmid)
    return localLyrics?.length ? sanitizeLyricLines(localLyrics) : []
  }

  const servicePluginId = getLikelyServicePluginId(songInfo)
  if (servicePluginId) {
    const serviceLyrics = await fetchServicePluginLyrics(servicePluginId, songInfo)
    if (serviceLyrics?.length) return sanitizeLyricLines(serviceLyrics)
    console.warn('[Lyrics] resolveLyrics: service plugin lyrics empty for', source, songId)
    return []
  }

  const sdk = await fetchSdkLyrics(source, songInfo, options.grepLyricInfo, options.useStrictMode)
  if (!sdk?.length) console.warn('[Lyrics] resolveLyrics: SDK empty for', source, songId)
  return sdk?.length ? sanitizeLyricLines(sdk) : []
}


export interface Comment {
  id: number | string
  text: string
  time: number
  timeStr: string
  location: string
  userName: string
  avatar: string
  userId: number | string
  likedCount: number
  images: string[]
  reply: Comment[]
}

interface CommentsState {
  hotList: Comment[]
  latestList: Comment[]
  hotTotal: number
  hotPage: number
  hotMaxPage: number
  latestTotal: number
  latestPage: number
  latestMaxPage: number
  limit: number
  type: 'hot' | 'latest'
  hotIsLoading: boolean
  latestIsLoading: boolean
}

const COMMENT_SOURCE_WHITELIST = new Set(['wy', 'tx', 'mg', 'kw', 'bd'])

function canFetchComments(source?: string) {
  return !!source && COMMENT_SOURCE_WHITELIST.has(source)
}

function resetCommentState(player: PlayerState) {
  player.comments.hotList = []
  player.comments.latestList = []
  player.comments.hotPage = 0
  player.comments.hotTotal = 0
  player.comments.hotMaxPage = 0
  player.comments.latestPage = 0
  player.comments.latestTotal = 0
  player.comments.latestMaxPage = 0
}


type PlayerSongInfo = Omit<SongList, 'songmid'> & { songmid: null | number | string }

interface PlayerState {
  songId?: string
  songInfo?: PlayerSongInfo
  playbackSongInfo?: PlayerSongInfo
  cover?: string
  songName: string
  singer: string
  coverDetail: {
    ColorObject?: Color
    mainColor?: string
    lightMainColor?: string
    contrastColor?: string
    textColor?: string
    hoverColor?: string
    playBg?: string
    playBgHover?: string
    useBlackText?: boolean
  }
  lyrics: { lines: LyricLine[]; trans?: string; source?: string }
  isLoading: boolean
  isFullPlayOpen: boolean
  comments: CommentsState
  _lyricsTrigger: number
}

const DEFAULT_SONG_INFO = {
  songmid: null,
  hash: '',
  name: i18n.global.t('play.welcomeSong'),
  singer: i18n.global.t('play.welcomeSinger'),
  albumName: '',
  albumId: '0',
  source: '',
  interval: '00:00',
  img: '',
  lrc: null,
  types: [],
  _types: {},
  typeUrl: {}
}

export const useGlobalPlayStatusStore = defineStore(
  'globalPlayStatus',
  () => {
    const localUserStore = LocalUserDetailStore()
    const playSettingStore = playSetting()
    const player = reactive<PlayerState>({
      songId: void 0,
      songInfo: DEFAULT_SONG_INFO,
      playbackSongInfo: void 0,
      cover: void 0,
      songName: '',
      singer: '',
      coverDetail: {
        ColorObject: void 0,
        mainColor: 'var(--td-brand-color-5)',
        lightMainColor: 'rgba(255,255,255,0.9)',
        contrastColor: 'var(--player-text-idle)',
        textColor: 'var(--player-text-idle)',
        hoverColor: 'var(--player-text-hover-idle)',
        playBg: 'var(--player-btn-bg-idle)',
        playBgHover: 'var(--player-btn-bg-hover-idle)',
        useBlackText: false
      },
      lyrics: { lines: [] },
      isLoading: false,
      isFullPlayOpen: false,
      _lyricsTrigger: 0,
      comments: {
        hotList: [],
        latestList: [],
        hotTotal: 0,
        hotPage: 0,
        hotMaxPage: 0,
        latestTotal: 0,
        latestPage: 0,
        latestMaxPage: 0,
        limit: 20,
        type: 'hot',
        hotIsLoading: false,
        latestIsLoading: false
      }
    })

    watch(
      () => localUserStore.userInfo.lastPlaySongId,
      (newId) => {
        if (newId && newId !== player.songId) {
          player.songId = String(newId)
          const song = localUserStore.list.find((s: any) => s.songmid === newId)
          if (song) updatePlayerInfo(song)
        }
      },
      { immediate: true }
    )

    let currentBlobUrl: string | null = null

    watch(
      () => player.songInfo?.img,
      async (newImg, _oldVal, onCleanup) => {
        let active = true
        onCleanup(() => { active = false })

        if (currentBlobUrl) {
          URL.revokeObjectURL(currentBlobUrl)
          currentBlobUrl = null
        }
        const info: any = player.songInfo
        const snapshotSongmid = info?.songmid
        if (!newImg && info?.source === 'local' && info?.hasCover !== false && info?.songmid) {
          try {
            const res = await (window as any).api?.localMusic?.getCoverBase64?.(String(info.songmid))
            if (!active) return
            const coverData = res?.success ? res.data : null
            if (
              coverData &&
              player.songInfo &&
              String(player.songInfo.songmid) === String(snapshotSongmid) &&
              !player.songInfo.img
            ) {
              player.songInfo.img = coverData
            }
          } catch {}
        }
        if (!active) return
        // For MG source, call backend SDK directly to bypass plugin CORS issue
        if (info?.source === 'mg') {
          try {
            const res = await (window as any).api?.music?.requestSdk?.('getPic', {
              source: 'mg',
              songInfo: getCleanSongInfo(info)
            })
            if (!active) return
            const picUrl = res?.url || res
            if (picUrl && typeof picUrl === 'string' && picUrl.length > 0) {
              player.cover = resolveImageUrl(picUrl)
              return
            }
          } catch {
          }
          player.cover = '/default-cover.png'
          return
        }
        if (!newImg && info?.source && info?.source !== 'local') {
          const pluginId = getLikelyServicePluginId(info) || LocalUserDetailStore().userSource.pluginId
          if (pluginId) {
            try {
              const picUrl = await PluginRunner.getPic(pluginId, info.source, getCleanSongInfo(info))
              if (!active) return
              if (picUrl && player.songInfo && !player.songInfo.img) {
                newImg = resolveImageUrl(picUrl)
              }
            } catch {
            }
          }
        }
        if (!active) return
        player.cover = resolveImageUrl(newImg) || '/default-cover.png'
      },
      { immediate: true }
    )

    watch(
      () => player.cover,
      async (newCover, _oldVal, onCleanup) => {
        if (!newCover) return
        let active = true
        onCleanup(() => { active = false })
        try {
          const { dominantColor, useBlackText } = await analyzeImageColors(newCover)
          if (!active) return
          player.coverDetail.ColorObject = dominantColor
          player.coverDetail.mainColor = `rgba(${dominantColor.r},${dominantColor.g},${dominantColor.b},1)`
          const base = useBlackText ? '0,0,0' : '255,255,255'
          player.coverDetail.textColor = `rgba(${base},0.6)`
          player.coverDetail.hoverColor = `rgba(${base},1)`
          player.coverDetail.contrastColor = player.coverDetail.textColor
          player.coverDetail.playBg = 'rgba(255,255,255,0.2)'
          player.coverDetail.playBgHover = 'rgba(255,255,255,0.33)'
          let r = dominantColor.r
          let g = dominantColor.g
          let b = dominantColor.b
          r = Math.min(255, r + (255 - r) * 0.8)
          g = Math.min(255, g + (255 - g) * 0.8)
          b = Math.min(255, b + (255 - b) * 0.8)
          player.coverDetail.lightMainColor = `rgba(${Math.round(r)},${Math.round(g)},${Math.round(b)},0.9)`
          player.coverDetail.useBlackText = useBlackText
        } catch {
          if (active) resetColors()
        }
      }
    )

    function resetColors() {
      player.coverDetail.mainColor = 'var(--td-brand-color-5)'
      player.coverDetail.lightMainColor = 'rgba(255,255,255,0.9)'
      player.coverDetail.contrastColor = 'var(--player-text-idle)'
      player.coverDetail.textColor = 'var(--player-text-idle)'
      player.coverDetail.hoverColor = 'var(--player-text-hover-idle)'
      player.coverDetail.playBg = 'var(--player-btn-bg-idle)'
      player.coverDetail.playBgHover = 'var(--player-btn-bg-hover-idle)'
    }

    watch(
      [
        () => player.songInfo?.songmid,
        () => (player.songInfo as any)?.source,
        () => player.playbackSongInfo?.songmid,
        () => (player.playbackSongInfo as any)?.source,
        () => player._lyricsTrigger
      ],
      async ([_newSongmid, _newSource, playbackSongmid, playbackSource], _oldVals, onCleanup) => {
        const lyricSongInfo = player.playbackSongInfo || player.songInfo
        const lyricSongmid = playbackSongmid || lyricSongInfo?.songmid
        if (!lyricSongmid || !lyricSongInfo) {
          player.lyrics.lines = []
          return
        }

        // 让播放状态先渲染一帧再加载歌词
        await new Promise(r => requestAnimationFrame(r))

        player.isLoading = true
        const targetSongId = String(lyricSongmid)
        const source = playbackSource || (lyricSongInfo as any)?.source || 'kg'

        let active = true
        const abort = new AbortController()
        onCleanup(() => {
          active = false
          abort.abort()
        })

        try {
          const parsedLyrics = await resolveLyrics(
            source,
            targetSongId,
            toRaw(lyricSongInfo),
            {
              grepLyricInfo: playSettingStore.getIsGrepLyricInfo,
              useStrictMode: playSettingStore.getStrictGrep
            },
            abort.signal
          )
          if (active) player.lyrics.lines = markRaw(parsedLyrics)
        } catch {
          if (active) player.lyrics.lines = []
        } finally {
          if (active) player.isLoading = false
        }
      },
      { immediate: true }
    )

    function updatePlayerInfo(songInfo: SongList) {
      const changed = player.songInfo?.songmid !== songInfo.songmid
      if (changed) {
        player.songInfo = songInfo
      }
      player.playbackSongInfo = songInfo
      player.songName = songInfo.name || ''
      player.singer = songInfo.singer || ''
      if (changed) {
        updateComments(songInfo)
      }
    }

    function updatePlaybackSongInfo(songInfo: SongList) {
      player.playbackSongInfo = songInfo as PlayerSongInfo
      player._lyricsTrigger++
    }

    async function fetchComments(page = 1, type: 'hot' | 'latest' = 'hot') {
      const currentSongInfo = toRaw(player.songInfo)
      if (!currentSongInfo || !currentSongInfo.songmid) return
      if (!canFetchComments((currentSongInfo as any).source)) return

      if (type === 'hot') {
        player.comments.hotIsLoading = true
      } else {
        player.comments.latestIsLoading = true
      }

      try {
        const method = type === 'hot' ? 'getHotComment' : 'getComment'
        const res = await (window as any).api?.music?.requestSdk?.(method, {
          source: (currentSongInfo as any).source || 'wy',
          songInfo: JSON.parse(JSON.stringify(currentSongInfo)),
          page,
          limit: player.comments.limit
        })

        if (type === 'hot') {
          if (page === 1) {
            player.comments.hotList = res?.comments || []
          } else {
            player.comments.hotList.push(...(res?.comments || []))
          }
          player.comments.hotTotal = res?.total || 0
          player.comments.hotPage = page
          player.comments.hotMaxPage = res?.maxPage || 1
        } else {
          if (page === 1) {
            player.comments.latestList = res?.comments || []
          } else {
            player.comments.latestList.push(...(res?.comments || []))
          }
          player.comments.latestTotal = res?.total || 0
          player.comments.latestPage = page
          player.comments.latestMaxPage = res?.maxPage || 1
        }
        player.comments.type = type
      } catch (err) {
        console.warn('[Comments] fetch failed:', err)
      } finally {
        if (type === 'hot') {
          player.comments.hotIsLoading = false
        } else {
          player.comments.latestIsLoading = false
        }
      }
    }

    function updateComments(songInfo: SongList) {
      resetCommentState(player)
      if (!canFetchComments((songInfo as any).source)) return
      setTimeout(() => { fetchComments(1, 'hot'); fetchComments(1, 'latest') }, 500)
    }

    function setFullPlayOpen(open: boolean) {
      player.isFullPlayOpen = open
    }

    return { player, updatePlayerInfo, updatePlaybackSongInfo, fetchComments, setFullPlayOpen }
  },
  { persist: false }
)
