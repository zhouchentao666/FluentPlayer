import { httpFetch as tauriFetch } from "@online/lib/http"
import type { LyricInfo, MusicInfo } from "@online/types/music"

// Third-party lyric source: AMLL TTML Database (https://ttml.db.ceru.dev).
// It indexes user-contributed translated/word-by-word lyrics keyed by
// (song name, artist, album), so it works across platforms without any login.
// We query by metadata and (when available) prefer the TTML payload, falling
// back to the plain `lrc` field. Returns null on miss / error.

const TTML_API = "https://api.ttml.db.ceru.dev/api/lyrics"

interface TtmlSearchItem {
  id: string
  name: string
  artist: string
  album?: string
  length?: number
  lyric?: string
  ttml?: string
  providers?: { name: string; id: string }[]
}

interface TtmlSearchResponse {
  result?: TtmlSearchItem[]
}

interface TtmlDetailResponse {
  id?: string
  name?: string
  artist?: string
  album?: string
  length?: number
  lyric?: string
  ttml?: string
}

function lrcTimestamp(ms: number): string {
  const total = Math.max(0, Math.floor(ms))
  const m = Math.floor(total / 60000)
  const s = Math.floor((total % 60000) / 1000)
  const cs = Math.floor((total % 1000) / 10)
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}.${String(cs).padStart(2, "0")}`
}

// TTML (XML) -> plain LRC. Parses <p begin="mm:ss.mmm" end="...">text</p>
// and emits one [time]line per <p>. Translation (amll <span> inside main<p>)
// is appended on the same line for simplicity.
function ttmlToLrc(ttml: string): string | null {
  const lines: { time: number; text: string }[] = []
  const pRe = /<p\s+begin="([\d:.]+)"[^>]*>([\s\S]*?)<\/p>/g
  let m: RegExpExecArray | null
  while ((m = pRe.exec(ttml))) {
    const begin = m[1]
    const body = m[2]
    const text = body
      .replace(/<span[^>]*>/g, "")
      .replace(/<\/span>/g, "")
      .replace(/<br\s*\/?>/g, " / ")
      .replace(/<[^>]+>/g, "")
      .replace(/\s+/g, " ")
      .trim()
    // begin form: mm:ss.mmm or hh:mm:ss.mmm
    const parts = begin.split(":").map(parseFloat)
    let ms = 0
    if (parts.length === 3) ms = (parts[0] * 3600 + parts[1] * 60 + parts[2]) * 1000
    else if (parts.length === 2) ms = (parts[0] * 60 + parts[1]) * 1000
    lines.push({ time: ms, text })
  }
  if (!lines.length) return null
  return lines.map((l) => `[${lrcTimestamp(l.time)}]${l.text}`).join("\n")
}

export async function getTtmlLyric(song: MusicInfo): Promise<LyricInfo | null> {
  const name = song.name?.trim()
  const artist = song.singer?.trim()
  if (!name || !artist) return null
  const q = new URLSearchParams({ name, artist })
  if (song.albumName) q.set("album", song.albumName.trim())
  try {
    const res = await tauriFetch(`${TTML_API}?${q.toString()}`, {
      method: "GET",
      headers: { "User-Agent": "Mozilla/5.0 FluentPlayer" },
    })
    if (!res.ok) return null
    const data = (await res.json()) as TtmlSearchResponse
    const list = data.result
    if (!list?.length) return null
    // pick the closest match (first is usually best; prefer one with ttml)
    const item = list.find((it) => it.ttml) ?? list[0]
    let ttml = item.ttml
    let lrc = item.lyric
    // If only an id is returned, fetch the detail endpoint.
    if (!ttml && !lrc && item.id) {
      try {
        const dRes = await tauriFetch(`${TTML_API}/${encodeURIComponent(item.id)}`, {
          method: "GET",
          headers: { "User-Agent": "Mozilla/5.0 FluentPlayer" },
        })
        if (dRes.ok) {
          const det = (await dRes.json()) as TtmlDetailResponse
          ttml = det.ttml ?? ttml
          lrc = det.lyric ?? lrc
        }
      } catch {
        // ignore detail fetch error
      }
    }
    let lyric: string = ttml ? (ttmlToLrc(ttml) ?? "") : (lrc ?? "")
    lyric = lyric.trim()
    if (!lyric) return null
    return { lyric }
  } catch {
    return null
  }
}
