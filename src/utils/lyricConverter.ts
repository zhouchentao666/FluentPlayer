import type { LyricLine, LyricWord } from '@applemusic-like-lyrics/core'

function formatTimestamp(timeMs: number): string {
  const t = Math.max(0, Math.floor(timeMs))
  const minutes = Math.floor(t / 60000)
  const seconds = Math.floor((t % 60000) / 1000)
  const milliseconds = t % 1000
  return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}.${milliseconds.toString().padStart(3, '0')}`
}

function convertNewFormat(baseTimeMs: number, content: string): string | null {
  const baseTimestamp = formatTimestamp(baseTimeMs)
  let convertedContent = `<${formatTimestamp(0)}>`
  const charPattern = /\((\d+),(\d+),(\d+)\)([^\(]*?)(?=\(|$)/g
  let match: RegExpExecArray | null
  let isFirstChar = true
  let lastConsumedIndex = 0

  while ((match = charPattern.exec(content)) !== null) {
    const [, charStartMs] = match
    const charTimeMs = parseInt(charStartMs, 10)
    const charTimestamp = formatTimestamp(charTimeMs)
    const char = match[4] ?? ''

    if (match.index > lastConsumedIndex) {
      convertedContent += content.substring(lastConsumedIndex, match.index)
    }
    lastConsumedIndex = match.index + match[0].length

    if (isFirstChar) {
      if (charTimeMs !== 0) {
        convertedContent += `<${charTimestamp}>${char}`
      } else {
        convertedContent += char
      }
      isFirstChar = false
    } else {
      convertedContent += `<${charTimestamp}>${char}`
    }
  }

  if (isFirstChar) return null

  if (lastConsumedIndex < content.length) {
    const remainingText = content.substring(lastConsumedIndex)
    if (remainingText) convertedContent += remainingText
  }

  return `[${baseTimestamp}]${convertedContent}`
}

function convertOldFormat(timestamp: string, content: string): string | null {
  let convertedContent = `<${formatTimestamp(0)}>`
  const charPattern = /([^()]*?)\((\d+),(\d+)\)/g
  let match: RegExpExecArray | null
  let lastIndex = 0
  let isFirstChar = true
  let matched = false

  while ((match = charPattern.exec(content)) !== null) {
    const [fullMatch, char, offsetMs] = match
    const charTimeMs = parseInt(offsetMs, 10)
    const charTimestamp = formatTimestamp(charTimeMs)
    matched = true

    if (match.index > lastIndex) {
      convertedContent += content.substring(lastIndex, match.index)
    }

    if (isFirstChar) {
      if (charTimeMs !== 0) {
        convertedContent += `<${charTimestamp}>${char}`
      } else {
        convertedContent += char
      }
      isFirstChar = false
    } else {
      convertedContent += `<${charTimestamp}>${char}`
    }
    lastIndex = match.index + fullMatch.length
  }

  if (!matched) return null

  if (lastIndex < content.length) {
    const remainingText = content.substring(lastIndex)
    convertedContent += remainingText
  }

  return `[${timestamp}]${convertedContent}`
}

export function isYrcFormat(lrc: string): boolean {
  return /^\[(\d+),\d+\]/m.test(lrc)
}

export function isEnhancedLrc(lrc: string): boolean {
  return /^\[\d{2}:\d{2}(?:\.\d+)?\].*\(\d+,\d+\)/m.test(lrc)
}

export function isA2Format(lrc: string): boolean {
  return /^\[\d{2}:\d{2}(?:\.\d+)?\].*<\d{2}:\d{2}(?:\.\d+)?>/m.test(lrc)
}

// LRC A2 的变体：逐字时间戳使用方括号而非尖括号，例如
//   [00:00.00] [00:00.04] When [00:00.16] the [00:00.82] truth
// 每个方括号表示其后续文本片段（一个词）的开始时间。
export function isBracketA2Format(lrc: string): boolean {
  // 逐字时间戳的典型结构是同一行内「[时间] 单词 [时间] 单词…」，即一个
  // [mm:ss.xx] 之后紧跟真实单词字符、再紧跟另一个 [mm:ss.xx]。
  // 必须逐行、且用 [^\n] 限定，否则 \s* 会跨行把「上一行歌词 + 换行 +
  // 下一行行首时间戳」连起来误判成逐字结构，导致普通 LRC 被错判。
  const inlinePair =
    /\[\d{2}:\d{2}(?:\.\d+)?\][^\n]*[^\s\[\]]+[^\n]*\[\d{2}:\d{2}(?:\.\d+)?\]/
  return lrc.split('\n').some((line) => inlinePair.test(line))
}

// 将中括号版 A2 转换为 AMLL 支持的尖括号 A2：保留行首第一个 [mm:ss.xx]
// 作为行时间戳，其余 [mm:ss.xx] 逐字时间戳改写为 <mm:ss.xx>。
export function convertBracketA2(lrc: string): string {
  if (!lrc) return ''
  return lrc
    .split('\n')
    .map((line) => {
      const headMatch = line.match(/^\[\d{2}:\d{2}(?:\.\d+)?\]/)
      if (!headMatch) return line
      const headEnd = headMatch.index! + headMatch[0].length
      const head = line.slice(0, headEnd)
      const tail = line.slice(headEnd)
      const convertedTail = tail.replace(
        /\[\d{2}:\d{2}(?:\.\d+)?\]/g,
        (t) => `<${t.slice(1, -1)}>`,
      )
      return head + convertedTail
    })
    .join('\n')
}

export function convertLrcFormat(lrcContent: string): string {
  if (!lrcContent) return ''
  const lines = lrcContent.split('\n')
  const convertedLines: string[] = []

  for (const line of lines) {
    if (!line.trim()) {
      convertedLines.push(line)
      continue
    }

    const newFormatMatch = line.match(/^\[(\d+),(\d+)\](.*)$/)
    if (newFormatMatch) {
      const [, startTimeMs, , content] = newFormatMatch
      const baseTimeMs = parseInt(startTimeMs, 10)
      if (!/\(\d+,\d+,\d+\)/.test(content)) {
        convertedLines.push(`[${formatTimestamp(baseTimeMs)}]${content}`)
        continue
      }
      const convertedLine = convertNewFormat(baseTimeMs, content)
      convertedLines.push(convertedLine ?? `[${formatTimestamp(baseTimeMs)}]${content}`)
      continue
    }

    const oldFormatMatch = line.match(/^\[(\d{2}:\d{2}\.\d{3})\](.*)$/)
    if (oldFormatMatch) {
      const [, timestamp, content] = oldFormatMatch
      if (!/\(\d+,\d+\)/.test(content)) {
        convertedLines.push(line)
        continue
      }
      const convertedLine = convertOldFormat(timestamp, content)
      convertedLines.push(convertedLine ?? line)
      continue
    }

    convertedLines.push(line)
  }

  return convertedLines.join('\n')
}

export function convertToStandardLrc(lrc: string): string {
  if (!lrc) return ''
  const lines = lrc.replace(/\\n/g, '\n').split('\n')
  const resultLines: string[] = []

  for (const line of lines) {
    const enhancedMatch = line.match(/^\[(\d+),(\d+)\](.*)/)
    if (enhancedMatch) {
      const startTime = parseInt(enhancedMatch[1], 10)
      const textContent = enhancedMatch[3] || ''

      const mm = Math.floor(startTime / 60000)
        .toString()
        .padStart(2, '0')
      const ss = Math.floor((startTime % 60000) / 1000)
        .toString()
        .padStart(2, '0')
      const xx = Math.floor((startTime % 1000) / 10)
        .toString()
        .padStart(2, '0')

      const pureText = textContent.replace(/\(\d+,\d+,\d+\)/g, '')
      resultLines.push(`[${mm}:${ss}.${xx}]${pureText}`)
      continue
    }

    const standardMatch = line.match(/^(\[\d{2}:\d{2}(?:\.\d{1,3})?\])(.*)/)
    if (standardMatch) {
      const timestamp = standardMatch[1]
      const textContent = standardMatch[2]
      const pureText = textContent.replace(/\(\d+,\d+\)/g, '')
      const normalizedTimestamp = timestamp.length > 9 ? timestamp.substring(0, 9) + ']' : timestamp
      resultLines.push(`${normalizedTimestamp}${pureText}`)
      continue
    }

    resultLines.push(line)
  }

  return resultLines.join('\n')
}

export function sanitizeLyricLines(lines: LyricLine[]): LyricLine[] {
  const defaultLineDuration = 3000
  const toFiniteNumber = (v: unknown, fallback: number): number => {
    const n = typeof v === 'number' ? v : Number(v)
    return Number.isFinite(n) ? n : fallback
  }

  const cleaned: LyricLine[] = []
  for (const rawLine of lines || []) {
    const rawWords = Array.isArray(rawLine.words) ? rawLine.words : []
    const fixedWords: LyricWord[] = []
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
    let startTime = toFiniteNumber(rawLine.startTime, firstWordStart)
    startTime = Math.max(0, startTime)
    let endTime = toFiniteNumber(rawLine.endTime, lastWordEnd)
    if (!Number.isFinite(endTime) || endTime <= startTime) endTime = startTime + defaultLineDuration
    if (endTime < lastWordEnd) endTime = lastWordEnd

    cleaned.push({ ...rawLine, startTime, endTime, words: fixedWords })
  }

  cleaned.sort((a, b) => (a?.startTime ?? 0) - (b?.startTime ?? 0))
  return cleaned
}
