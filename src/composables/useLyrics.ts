import { ref, watch, type Ref } from 'vue'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import { parseLrc, parseLrcA2, parseYrc, parseQrc, parseEslrc, parseTTML } from '@applemusic-like-lyrics/lyric'
import { ReadLyrics } from '@bridge/app'
import {
  convertLrcFormat,
  isA2Format,
  isBracketA2Format,
  convertBracketA2,
  isEnhancedLrc,
  isYrcFormat,
  sanitizeLyricLines,
} from '../utils/lyricConverter'
import { localMetadata } from './useLocalMetadata'
import { resolveOnlineLyric } from '@online/player'
import type { MusicInfo } from '@online/types/music'

export type { LyricLine }

function normalizeLrcA2(lrc: string): string {
  return lrc
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    .replace(/<00:00\.000>(?=\s*<\d{2}:\d{2}(?:\.\d+)?>)/g, '')
}

function isTtmlFormat(lrc: string): boolean {
  return /<tt\s+xmlns=/i.test(lrc) || /<tt\s+>/i.test(lrc)
}

function isQrcFormat(lrc: string): boolean {
  return /^\[ti:.*\]\[ar:.*\]\[al:.*\]/i.test(lrc) && /<\d+,\d+,\d+>/.test(lrc)
}

function isEslrcFormat(lrc: string): boolean {
  return /^\[\d+,\d+\]/m.test(lrc) && /^\d+/.test(lrc.trimStart())
}

function parseLyric(lrc: string, format?: string): LyricLine[] {
  let parsed: LyricLine[] = []

  // 如果指定了格式，按指定格式解析
  if (format && format !== 'auto') {
    switch (format) {
      case 'ttml':
        const ttmlResult = parseTTML(lrc)
        parsed = ttmlResult.lines
        break
      case 'qrc':
        parsed = parseQrc(lrc) as LyricLine[]
        break
      case 'eslrc':
        parsed = parseEslrc(lrc) as LyricLine[]
        break
      case 'yrc':
        parsed = parseYrc(lrc) as LyricLine[]
        break
      case 'lrc-a2':
        parsed = parseLrcA2(normalizeLrcA2(lrc)) as LyricLine[]
        break
      case 'lrc':
        parsed = parseLrc(lrc) as LyricLine[]
        break
      default:
        parsed = parseLrc(lrc) as LyricLine[]
    }
    return sanitizeLyricLines(parsed)
  }

  // 自动检测格式
  if (isTtmlFormat(lrc)) {
    const ttmlResult = parseTTML(lrc)
    parsed = ttmlResult.lines
  } else if (isQrcFormat(lrc)) {
    parsed = parseQrc(lrc) as LyricLine[]
  } else if (isEslrcFormat(lrc)) {
    parsed = parseEslrc(lrc) as LyricLine[]
  } else if (isYrcFormat(lrc)) {
    parsed = parseYrc(lrc) as LyricLine[]
  } else if (isEnhancedLrc(lrc)) {
    parsed = parseLrcA2(convertLrcFormat(lrc)) as LyricLine[]
  } else if (isA2Format(lrc)) {
    parsed = parseLrcA2(normalizeLrcA2(lrc)) as LyricLine[]
  } else if (isBracketA2Format(lrc)) {
    parsed = parseLrcA2(normalizeLrcA2(convertBracketA2(lrc))) as LyricLine[]
  } else {
    parsed = parseLrc(lrc) as LyricLine[]
  }

  return sanitizeLyricLines(parsed)
}

export function useLyrics(currentSong: Ref<{ path: string; online?: MusicInfo } | null>) {
  const lyrics = ref<LyricLine[]>([])
  const hasLyrics = ref(false)
  let loadSeq = 0

  async function loadOnlineLyrics(info: MusicInfo) {
    const seq = ++loadSeq
    lyrics.value = []
    hasLyrics.value = false
    const lyricInfo = await resolveOnlineLyric(info).catch(() => null)
    if (seq !== loadSeq) return
    if (!lyricInfo?.lyric) return
    // 优先使用逐字歌词（酷狗 KRC 的 crlyric / 网易云 TTML 已包含在 lyric 中），
    // 否则回退到纯 LRC。
    const parsed = lyricInfo.crlyric
      ? parseLyric(lyricInfo.crlyric, 'eslrc')
      : parseLyric(lyricInfo.lyric)
    lyrics.value = parsed
    hasLyrics.value = parsed.length > 0
  }

  async function loadLyrics(path: string | null) {
    const seq = ++loadSeq
    if (!path) {
      lyrics.value = []
      hasLyrics.value = false
      return
    }

    const meta = localMetadata.value[path]
    if (seq !== loadSeq) return
    const override = meta?.lyrics
    const format = meta?.lyricsFormat
    const lrc = (override && override.trim() !== '') ? override : await ReadLyrics(path).catch(() => '')
    if (!lrc) {
      lyrics.value = []
      hasLyrics.value = false
      return
    }
    const parsed = parseLyric(lrc, format)
    lyrics.value = parsed
    hasLyrics.value = parsed.length > 0
  }

  watch(currentSong, (song) => {
    if (song?.online) {
      loadOnlineLyrics(song.online)
    } else {
      loadLyrics(song?.path || null)
    }
  }, { immediate: true })

  watch(() => localMetadata.value[currentSong.value?.path ?? '']?.lyrics, () => {
    if (currentSong.value?.online) return
    loadLyrics(currentSong.value?.path || null)
  })

  watch(() => localMetadata.value[currentSong.value?.path ?? '']?.lyricsFormat, () => {
    if (currentSong.value?.online) return
    loadLyrics(currentSong.value?.path || null)
  })

  return {
    lyrics,
    hasLyrics,
  }
}