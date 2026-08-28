import { computed, ref, type Ref } from 'vue'
import type { LyricLine, LyricWord } from '@applemusic-like-lyrics/core'
import type { DesktopLyricConfig } from './useConfig'

export type RenderLine = {
  line: LyricLine
  index: number
  key: string
  active: boolean
}

export type LyricData = {
  playName: string
  artistName: string
  playStatus: boolean
  lrcData: LyricLine[]
  yrcData: LyricLine[]
}

const LYRIC_LOOKAHEAD = 300
const SCROLL_START_AT_PROGRESS = 0.3
const END_MARGIN_SEC = 2

function getPlainText(words: LyricWord[]): string {
  if (!Array.isArray(words)) return ''
  return words.map((w) => w?.word || '').join('')
}

function getSafeEndTime(lyrics: LyricLine[], idx: number): number {
  const cur = lyrics?.[idx]
  const next = lyrics?.[idx + 1]
  const curEnd = Number(cur?.endTime)
  const curStart = Number(cur?.startTime)
  if (Number.isFinite(curEnd) && curEnd > curStart) return curEnd
  const nextStart = Number(next?.startTime)
  if (Number.isFinite(nextStart) && nextStart > curStart) return nextStart
  return 0
}

function placeholder(text: string): RenderLine[] {
  return [{
    line: {
      startTime: 0,
      endTime: 0,
      words: [{ word: text, startTime: 0, endTime: 0, romanWord: '' }],
      translatedLyric: '',
      romanLyric: '',
      isBG: false,
      isDuet: false,
    } as LyricLine,
    index: -1,
    key: 'placeholder',
    active: true,
  }]
}

export function useDesktopLyricRenderer(
  lyricData: Ref<LyricData>,
  config: Ref<DesktopLyricConfig>,
  playSeekMs: Ref<number>,
) {
  const lineRefs = ref<Record<string, HTMLElement>>({})
  const contentRefs = ref<Record<string, HTMLElement>>({})

  function setLineRef(el: Element | null, key: string) {
    if (el) lineRefs.value[key] = el as HTMLElement
    else delete lineRefs.value[key]
  }

  function setContentRef(el: Element | null, key: string) {
    if (el) contentRefs.value[key] = el as HTMLElement
    else delete contentRefs.value[key]
  }

  const currentLyricIndex = computed(() => {
    const lyrics = config.value.showYrc && lyricData.value.yrcData.length ? lyricData.value.yrcData : lyricData.value.lrcData
    if (!lyrics.length) return -1
    const seek = playSeekMs.value
    const idx = lyrics.findIndex((v) => (v.startTime || 0) > seek)
    if (idx === -1) return lyrics.length - 1
    if (idx > 0) return idx - 1
    return -1
  })

  const hasYrc = computed(() => {
    return lyricData.value.yrcData.length > 0
  })

  const renderLyricLines = computed<RenderLine[]>(() => {
    const lyrics = config.value.showYrc && lyricData.value.yrcData.length ? lyricData.value.yrcData : lyricData.value.lrcData
    if (!lyricData.value.playName && !lyrics.length) {
      return placeholder('Sugar Desktop Lyric')
    }
    if (!lyrics.length) return placeholder('纯音乐，请欣赏')
    const idx = currentLyricIndex.value
    if (idx < 0) {
      return placeholder(`${lyricData.value.playName} - ${lyricData.value.artistName}`)
    }
    const current = lyrics[idx]
    const next = lyrics[idx + 1]
    if (!current) return []
    const safeEnd = getSafeEndTime(lyrics, idx)

    if (config.value.showTran && current.translatedLyric) {
      return [
        { line: { ...current, endTime: safeEnd }, index: idx, key: `${idx}-orig`, active: true },
        {
          line: {
            startTime: current.startTime,
            endTime: safeEnd,
            words: [{ word: current.translatedLyric, startTime: current.startTime, endTime: safeEnd, romanWord: '' }],
            translatedLyric: '',
            romanLyric: '',
            isBG: false,
            isDuet: false,
          } as LyricLine,
          index: idx,
          key: `${idx}-tran`,
          active: false,
        },
      ]
    }

    if (config.value.isDoubleLine) {
      const lines: RenderLine[] = [{
        line: { ...current, endTime: safeEnd },
        index: idx,
        key: `${idx}-orig`,
        active: true,
      }]
      if (next) {
        lines.push({
          line: next,
          index: idx + 1,
          key: `${idx + 1}-orig`,
          active: false,
        })
      }
      return lines
    }

    return [{ line: { ...current, endTime: safeEnd }, index: idx, key: `${idx}-orig`, active: true }]
  })

  function getYrcStyle(wordData: LyricWord, lyricIndex: number): { backgroundPositionX: string } {
    const currentLine = lyricData.value.yrcData?.[lyricIndex]
    if (!currentLine) return { backgroundPositionX: '100%' }
    const seekSec = playSeekMs.value + LYRIC_LOOKAHEAD
    const startSec = currentLine.startTime || 0
    const endSec = currentLine.endTime || 0
    const isLineActive = (seekSec >= startSec && seekSec < endSec) || currentLyricIndex.value === lyricIndex
    if (!isLineActive) {
      const hasPlayed = seekSec >= (wordData.endTime || 0)
      return { backgroundPositionX: hasPlayed ? '0%' : '100%' }
    }
    const durationSec = Math.max((wordData.endTime || 0) - (wordData.startTime || 0), 0.001)
    const progress = Math.max(Math.min((seekSec - (wordData.startTime || 0)) / durationSec, 1), 0)
    return { backgroundPositionX: `${100 - progress * 100}%` }
  }

  function getScrollStyle(line: RenderLine): { transform: string; willChange?: string } {
    const container = lineRefs.value[line.key]
    const content = contentRefs.value[line.key]
    if (!container || !content || !line.line) return { transform: 'translateX(0px)' }
    const padL = parseFloat(getComputedStyle(container).paddingLeft) || 0
    const padR = parseFloat(getComputedStyle(container).paddingRight) || 0
    const marginL = parseFloat(getComputedStyle(content).marginLeft) || 0
    const marginR = parseFloat(getComputedStyle(content).marginRight) || 0
    const borderL = parseFloat(getComputedStyle(content).borderLeftWidth) || 0
    const borderR = parseFloat(getComputedStyle(content).borderRightWidth) || 0
    const visibleWidth = Math.max(0, container.clientWidth - padL - padR)
    const contentFullWidth = Math.max(0, content.scrollWidth + marginL + marginR + borderL + borderR)
    const overflow = Math.max(0, contentFullWidth - visibleWidth)
    if (overflow <= 0) return { transform: 'translateX(0px)' }
    const start = Number(line.line.startTime ?? 0)
    const endRaw = Number(line.line.endTime)
    if (!Number.isFinite(endRaw) || endRaw <= 0 || endRaw <= start) return { transform: 'translateX(0px)' }
    const end = Math.max(start + 0.001, endRaw - END_MARGIN_SEC)
    const duration = Math.max(end - start, 0.001)
    const progress = Math.max(Math.min((playSeekMs.value - start) / duration, 1), 0)
    if (progress <= SCROLL_START_AT_PROGRESS) return { transform: 'translateX(0px)' }
    const ratio = (progress - SCROLL_START_AT_PROGRESS) / (1 - SCROLL_START_AT_PROGRESS)
    const offset = Math.round(overflow * ratio)
    return { transform: `translateX(-${offset}px)`, willChange: 'transform' }
  }

  return {
    renderLyricLines,
    currentLyricIndex,
    hasYrc,
    getPlainText,
    getYrcStyle,
    getScrollStyle,
    setLineRef,
    setContentRef,
  }
}
