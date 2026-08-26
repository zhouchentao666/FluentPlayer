export type { LyricLine, LyricWord } from '@applemusic-like-lyrics/lyric'

export function getLineText(line: { words: { word: string }[] }): string {
  return line.words.map((w) => w.word).join('')
}

export function findCurrentLine(
  lines: { startTime: number }[],
  currentTimeMs: number
): number {
  if (!lines.length) return -1
  for (let i = lines.length - 1; i >= 0; i--) {
    if (currentTimeMs >= lines[i].startTime) return i
  }
  return -1
}
