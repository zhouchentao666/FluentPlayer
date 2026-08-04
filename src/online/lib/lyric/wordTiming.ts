// Word-by-word (逐字) lyric helpers.
//
// FluentPlayer's `useLyrics.parseLyric` already auto-detects and renders
// AMLL-style timed lyrics — TTML, YRC, QRC, ESLRC (via @applemusic-like-lyrics).
// The only thing we need from each platform's built-in lyric fetcher is to
// return the *raw timed text* instead of flattening it into plain LRC.
//
// Different platforms expose different timed formats:
//   - NetEase   : TTML (preferred) / YRC-like `[ms,dur] (off,dur,0)word`
//   - QQ        : QRC `[ti:][ar:]...<off,dur,dur>word`  (native decode not portable)
//   - KuGou     : KRC `[ms,dur]<off,dur,dur>word`
//   - KuWo      : KRC-like `[ms,dur]<off,dur>word` (encrypted endpoint)
//   - Migu      : MRC `[ms,dur]<off,dur>word` (encrypted)
//
// To keep a single, well-supported code path, we normalize every platform's
// word-timed text into the **YRC** format that parseYrc understands:
//
//     [lineStartMs,lineDurMs] (wordStartMs,wordDurMs,0)char...
//
// Lines without any per-word timing are emitted as plain `[mm:ss.xx]text`
// (still parseable by parseLrc, so nothing regresses).

// mm:ss.xx / mm:ss.xxx -> milliseconds
export function lrcTimeToMs(time: string): number {
  if (!time) return 0
  const parts = time.split(":")
  let total = 0
  let unit = 1
  while (parts.length) {
    total += parseFloat(parts.pop()!) * unit
    unit *= 60
  }
  return Math.trunc(total * 1000)
}

// milliseconds -> "mm:ss.xx" (2-digit centiseconds, parseYrc/lrc compatible)
export function msToLrcTime(ms: number): string {
  const clamped = Math.max(0, Math.trunc(ms))
  const m = Math.floor(clamped / 60000)
  const s = Math.floor((clamped % 60000) / 1000)
  const cs = Math.floor((clamped % 1000) / 10)
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}.${String(cs).padStart(2, "0")}`
}

// milliseconds -> "[mm:ss.xx]" bracket tag
export function msToLrcTag(ms: number): string {
  return `[${msToLrcTime(ms)}]`
}

// --- KuWo / Migu (KRC-like `<off,dur>word`, 2 fields) ------------------
// Input line: `[lineMs,lineDur]<wordOff,wordDur>char...`
// Output     : YRC `[lineMs,lineDur] (wordOff,wordDur,0)char...`
export function kwLikeToYrc(text: string): string {
  const lines = text.replace(/\r/g, "").split("\n")
  const out: string[] = []
  for (const line of lines) {
    if (!line.trim()) continue
    const m = /^\[(\d+),(\d+)\]([\s\S]*)$/.exec(line)
    if (!m) {
      // Not a timed line — keep as-is (likely plain LRC already).
      out.push(line)
      continue
    }
    const lineStart = parseInt(m[1])
    const lineDur = parseInt(m[2])
    const body = m[3]
    // replace each `<off,dur>` (2 fields) with `(off,dur,0)`
    const words = body.replace(/<(-?\d+),(-?\d+)>/g, (_s, off, dur) => `(${off},${dur},0)`)
    out.push(`[${lineStart},${lineDur}]${words}`)
  }
  return out.join("\n")
}

// --- KuGou KRC `<off,dur,dur>word` (3 fields) --------------------------
// Input line: `[lineMs,lineDur]<wordOff,wordDur,wordLineDur>char...`
// Output     : YRC `[lineMs,lineDur] (wordOff,wordDur,0)char...`
export function krcToYrc(text: string): string {
  const lines = text.replace(/\r/g, "").split("\n")
  const out: string[] = []
  for (const line of lines) {
    if (!line.trim()) continue
    const m = /^\[(\d+),(\d+)\]([\s\S]*)$/.exec(line)
    if (!m) {
      out.push(line)
      continue
    }
    const lineStart = parseInt(m[1])
    const lineDur = parseInt(m[2])
    const body = m[3]
    const words = body.replace(/<(-?\d+),(-?\d+),(-?\d+)>/g, (_s, off, dur) => `(${off},${dur},0)`)
    out.push(`[${lineStart},${lineDur}]${words}`)
  }
  return out.join("\n")
}

// --- Migu MRC (already `<off,dur>` 2 fields, same as KuWo) -------------
export function mrcToYrc(text: string): string {
  return kwLikeToYrc(text)
}

// --- QQ QRC `<off,dur,dur>word`, but with leading [ti:][ar:][al:] tags -
// QRC header tags are harmless for parseQrc but NOT for parseYrc, so strip
// them and keep only the timed word groups. Output is YRC.
export function qrcToYrc(text: string): string {
  const lines = text.replace(/\r/g, "").split("\n")
  const out: string[] = []
  for (const line of lines) {
    // Drop metadata header lines like [ti:...] / [ar:...] / [al:...] (no word groups)
    if (/^\[(ti|ar|al|by|offset|length|re|ve):/i.test(line)) continue
    if (!line.trim()) continue
    // QRC can carry `[\d+,...]` line timers like KRC; convert if present.
    const m = /^\[(\d+),(\d+)\]([\s\S]*)$/.exec(line)
    if (m) {
      const lineStart = parseInt(m[1])
      const lineDur = parseInt(m[2])
      const body = m[3].replace(/<(-?\d+),(-?\d+),(-?\d+)>/g, (_s, off, dur) => `(${off},${dur},0)`)
      out.push(`[${lineStart},${lineDur}]${body}`)
      continue
    }
    // Fallback: convert bare `<off,dur,dur>` groups into `(off,dur,0)` and
    // wrap the line with a zero timestamp so it still renders.
    if (/<\d+,\d+,\d+>/.test(line)) {
      const body = line.replace(/<(-?\d+),(-?\d+),(-?\d+)>/g, (_s, off, dur) => `(${off},${dur},0)`)
      out.push(`[0,0]${body}`)
      continue
    }
    out.push(line)
  }
  return out.join("\n")
}

// Take the platform-specific timed text and normalize to YRC (or pass TTML
// straight through). Returns text that `useLyrics.parseLyric` renders as
// word-by-word lyrics.
export type WordFormat = "ttml" | "yrc" | "qrc" | "krc" | "mrc" | "kw"

export function toWordTimed(text: string, format: WordFormat): string {
  switch (format) {
    case "ttml":
      return text // parseTTML handles it directly
    case "qrc":
      return qrcToYrc(text)
    case "krc":
      return krcToYrc(text)
    case "mrc":
    case "kw":
      return kwLikeToYrc(text)
    case "yrc":
    default:
      return text
  }
}
