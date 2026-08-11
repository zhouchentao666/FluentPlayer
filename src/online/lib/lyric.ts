import { httpFetch as tauriFetch } from "@online/lib/http"
import * as pako from "pako"
import type { LyricInfo, MusicInfo } from "@online/types/music"
import { getTxLyric, getWyLyric, getKgLyric, getMgLyric } from "@online/lib/lyric/extra"

// Built-in lyric fetching, fetched directly from each platform's public API
// (like built-in search) rather than relying on source scripts — most lx-music
// source scripts only implement `musicUrl` and reply "lyric not support".

function fmtLrcTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) seconds = 0
  const m = Math.floor(seconds / 60)
  const s = (seconds % 60).toFixed(2).padStart(5, "0")
  return `${String(m).padStart(2, "0")}:${s}`
}

interface KwLrcResponse {
  status?: number
  data?: {
    lrclist?: { time: string; lineLyric: string }[]
  }
}

// --- kw mobile H5 endpoint -----------------------------------------------
// Returns a plain {time(seconds), lineLyric} list. Fast but unreliable — it
// intermittently rate-limits with `{status:301}` and no lyric, so it's only the
// first try; getKwLyricEncrypted below is the fallback.
async function getKwLyricH5(songId: string): Promise<LyricInfo | null> {
  const res = await tauriFetch(`http://m.kuwo.cn/newh5/singles/songinfoandlrc?musicId=${songId}`, {
    method: "GET",
    headers: { "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36" },
  })
  if (!res.ok) return null
  const data = (await res.json()) as KwLrcResponse
  const list = data.data?.lrclist
  if (!list?.length) return null
  const lyric = list
    .map((l) => `[${fmtLrcTime(parseFloat(l.time))}]${l.lineLyric ?? ""}`)
    .join("\n")
  if (!lyric.trim()) return null
  return { lyric }
}

// --- kw official encrypted endpoint --------------------------------------
// Ported from lx-music-desktop kw/lyric.js (active getLyric) + kw/util.js
// (decodeLyric) + main/.../kw_decodeLyric.ts. The request params are XOR-ed
// byte-by-byte with the 7-byte key "yeelion" then base64-encoded; the response
// is a binary blob: `tp=content...\r\n\r\n<zlib-deflated body>`. We inflate the
// body and (for the non-lyricx form) gb18030-decode it directly into plain LRC.

const KW_KEY = new TextEncoder().encode("yeelion") // 7 bytes

// XOR `params` with the rolling "yeelion" key, base64-encode (buildParams).
function kwBuildParams(id: string): string {
  // 必须带 isGetLyricx=1，否则 kuwo 只返回纯 LRC，没有逐字歌词
  // （这正是之前酷我永远不显示逐字的根因：不带该参数端点不返回 lyricx）。
  const params = `user=12345,web,web,web&requester=localhost&req=1&rid=MUSIC_${id}&isGetLyricx=1`
  const src = new TextEncoder().encode(params)
  const out = new Uint8Array(src.length)
  let i = 0
  while (i < src.length) {
    let j = 0
    while (j < KW_KEY.length && i < src.length) {
      out[i] = KW_KEY[j] ^ src[i]
      i++
      j++
    }
  }
  // base64-encode raw bytes
  let binary = ""
  for (let k = 0; k < out.length; k++) binary += String.fromCharCode(out[k])
  return btoa(binary)
}

// Decode the raw response bytes -> plain LRC text (with per-word timing when
// isGetLyricx=1). Mirrors kw_decodeLyric.ts:
//   non-lyricx: `tp=content\r\n\r\n` + zlib-deflated GB18030 LRC
//   lyricx:     `tp=content\r\n\r\n` + base64 text -> XOR 0x64 -> GB18030 LRC
//               (the LRC carries <-?\d+,-?\d+> word-time marks for karaoke lyrics)
function kwDecodeLyric(buf: Uint8Array): string {
  if (buf.length < 10) return ""
  // First 10 bytes must be the ASCII marker "tp=content".
  const head = new TextDecoder("utf-8").decode(buf.subarray(0, 10))
  if (head !== "tp=content") return ""

  // Read the rest as UTF-8 text so we can locate the "\r\n\r\n" boundary; the
  // lyricx body is pure ASCII/base64 (safe to inspect), and the non-lyricx body
  // is binary but the separator is still an ASCII 0x0d 0x0a 0x0d 0x0a run.
  const tail = new TextDecoder("utf-8").decode(buf.subarray(10))
  const sepIdx = tail.indexOf("\r\n\r\n")
  if (sepIdx < 0) return ""
  const headerPart = tail.slice(0, sepIdx)
  const afterSep = buf.subarray(10 + sepIdx + 4)

  // lyricx form: header contains `tp=content`, body is base64 (after sep).
  if (/tp=content/.test(headerPart) && afterSep.every((b) => b < 0x80)) {
    let b64 = ""
    for (let i = 0; i < afterSep.length; i++) b64 += String.fromCharCode(afterSep[i])
    try {
      const bytes = b64ToUint8(b64.replace(/\s+/g, ""))
      const xored = new Uint8Array(bytes.length)
      for (let i = 0; i < bytes.length; i++) xored[i] = bytes[i] ^ 0x64
      for (const enc of ["gb18030", "gbk", "utf-8"]) {
        try {
          return new TextDecoder(enc).decode(xored)
        } catch {
          // try next encoding
        }
      }
    } catch {
      // not valid base64 -> fall through to zlib path
    }
  }

  // Non-lyricx form: zlib-inflate the binary body, then gb18030-decode.
  let inflated: Uint8Array
  try {
    inflated = pako.inflate(afterSep)
  } catch {
    return ""
  }
  for (const enc of ["gb18030", "gbk", "utf-8"]) {
    try {
      return new TextDecoder(enc).decode(inflated)
    } catch {
      // try next encoding
    }
  }
  return ""
}

// base64 -> raw bytes (shared helper, mirrors lyric/extra.ts b64ToUint8).
function b64ToUint8(str: string): Uint8Array {
  const binary = atob(str)
  const buf = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) buf[i] = binary.charCodeAt(i)
  return buf
}

// Split parsed LRC lines into lyric + translation. Mirrors kw/lyric.js
// sortLrcArr: lines that repeat an already-seen timestamp are treated as the
// translation of the previous line. Throws on a failed heuristic (mirrors ref).
function kwSortLrc(arr: { time: string; text: string }[]): {
  lrc: { time: string; text: string }[]
  lrcT: { time: string; text: string }[]
} {
  const seen = new Set<string>()
  const lrc: { time: string; text: string }[] = []
  const lrcT: { time: string; text: string }[] = []
  let isLyricx = false
  const lyricxTag = /^<-?\d+,-?\d+>/
  for (const item of arr) {
    if (seen.has(item.time)) {
      if (lrc.length < 2) continue
      const t = lrc.pop()!
      t.time = lrc[lrc.length - 1].time
      lrcT.push(t)
      lrc.push(item)
    } else {
      lrc.push(item)
      seen.add(item.time)
    }
    if (!isLyricx && lyricxTag.test(item.text)) isLyricx = true
  }
  if (!isLyricx && lrcT.length > lrc.length * 0.3 && lrc.length - lrcT.length > 6) {
    throw new Error("failed")
  }
  return { lrc, lrcT }
}

const KW_TIME_EXP = /^\[([\d:.]*)\]/
const KW_EXIST_TIME_EXP = /\[\d{1,2}:.*\d{1,4}\]/
const KW_WORD_TIME_ALL = /<(-?\d+),(-?\d+)(?:,-?\d+)?>/g

/**
 * 解析酷我 LRC，产出 { lyric, tlyric, lxlyric }。对应 kw/lyric.js 的
 * parseLrc + transformLrc。
 *
 * 酷我逐字标记形如 `[00:28.480]<0,160>我<160,420>带`，即行内 `<相对偏移,时长>`
 * 位于字【之前】，偏移相对行首。转成 AMLL 的 A2 语法 `<mm:ss.SSS>` 绝对时间后，
 * 交给 parseLrcA2 渲染。没有字级时间的歌曲则只返回普通 LRC。
 */
function kwParseLrc(text: string): LyricInfo | null {
  const lines = text.split(/\r\n|\r|\n/)
  const lrcArr: { time: string; text: string }[] = []
  for (const raw of lines) {
    const line = raw.trim()
    const m = KW_TIME_EXP.exec(line)
    if (!m) continue
    let time = m[1]
    if (/\.\d\d$/.test(time)) time += "0"
    const body = line.replace(KW_TIME_EXP, "").trim()
    lrcArr.push({ time, text: body })
  }
  if (!lrcArr.length) return null

  let parts: ReturnType<typeof kwSortLrc>
  try {
    parts = kwSortLrc(lrcArr)
  } catch {
    return null
  }

  const toText = (list: { time: string; text: string }[]): string =>
    list.map((l) => `[${l.time}]${l.text}`).join("\n")

  const rawLyric = toText(parts.lrc)
  let lyric = rawLyric.replace(KW_WORD_TIME_ALL, "")
  if (!KW_EXIST_TIME_EXP.test(lyric)) return null
  let tlyric = parts.lrcT.length ? toText(parts.lrcT).replace(KW_WORD_TIME_ALL, "") : ""
  lyric = lyric.trim()
  tlyric = tlyric.trim()
  return {
    lyric,
    tlyric: tlyric || null,
    lxlyric: kwToA2(rawLyric),
  }
}

/** `[mm:ss.SSS]` -> 毫秒；解析失败返回 null。 */
function kwTagToMs(tag: string): number | null {
  const m = /^(\d+):(\d+)(?:\.(\d+))?$/.exec(tag)
  if (!m) return null
  const frac = m[3] ? parseInt(m[3].padEnd(3, "0").slice(0, 3)) : 0
  return parseInt(m[1]) * 60000 + parseInt(m[2]) * 1000 + frac
}

/**
 * 把酷我的 `[mm:ss.SSS]<相对偏移,时长>字` 转成 AMLL ESLRC 的
 * `[行起始,行时长](绝对偏移,时长,0)字`，与酷狗逐字歌词格式一致，
 * 交给 parseLyric -> parseYrc 渲染。无字级时间时返回 null。
 */
function kwToA2(text: string): string | null {
  const lines: { base: number; body: string }[] = []
  let hasWordTime = false
  for (const raw of text.split("\n")) {
    const line = raw.trim()
    const m = KW_TIME_EXP.exec(line)
    if (!m) continue
    const base = kwTagToMs(m[1])
    const body = line.replace(KW_TIME_EXP, "")
    if (base == null) continue
    if (!/<-?\d+,-?\d+(?:,-?\d+)?>/.test(body)) {
      lines.push({ base, body })
      continue
    }
    hasWordTime = true
    lines.push({ base, body })
  }
  if (!hasWordTime) return null
  const out: string[] = []
  for (let i = 0; i < lines.length; i++) {
    const { base, body } = lines[i]
    const lineDur = (i + 1 < lines.length ? lines[i + 1].base : base + 4000) - base
    const converted = body.replace(
      /<(-?\d+),(-?\d+)(?:,-?\d+)?>/g,
      (_all, off: string, dur: string) => {
        const abs = Math.max(0, base + parseInt(off))
        const d = parseInt(dur)
        return `(${abs},${d},0)`
      }
    )
    out.push(`[${base},${lineDur}]${converted}`)
  }
  return out.join("\n")
}

async function getKwLyricEncrypted(songId: string): Promise<LyricInfo | null> {
  const res = await tauriFetch(`http://newlyric.kuwo.cn/newlyric.lrc?${kwBuildParams(songId)}`, {
    method: "GET",
    headers: {
      "User-Agent":
        "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/69.0.3497.100 Safari/537.36",
    },
  })
  if (!res.ok) return null
  const buf = new Uint8Array(await res.arrayBuffer())
  const text = kwDecodeLyric(buf)
  if (!text) return null
  return kwParseLrc(text)
}

// KuWo lyric: try the fast (but rate-limited) H5 endpoint first, then fall back
// to the official encrypted endpoint lx-music uses. song.meta.songId is the id.
async function getKwLyric(songId: string): Promise<LyricInfo | null> {
  if (!songId) return null
  try {
    const h5 = await getKwLyricH5(songId)
    if (h5) return h5
  } catch {
    // ignore -> fall through to encrypted endpoint
  }
  try {
    return await getKwLyricEncrypted(songId)
  } catch {
    return null
  }
}

/**
 * Fetch lyrics for a song directly from its platform. Returns null when the
 * platform isn't supported yet or has no lyrics — callers can then fall back
 * to a source script.
 */
export async function getBuiltinLyric(song: MusicInfo): Promise<LyricInfo | null> {
  try {
    switch (song.source) {
      case "kw":
        return await getKwLyric(song.meta.songId)
      case "tx":
        return await getTxLyric(song)
      case "wy":
        return await getWyLyric(song)
      case "kg":
        return await getKgLyric(song)
      case "mg":
        return await getMgLyric(song)
      default:
        return null
    }
  } catch {
    return null
  }
}
