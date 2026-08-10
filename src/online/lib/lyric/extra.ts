import { httpFetch } from "@online/lib/http"
import { eapi } from "@online/lib/platforms/wy/eapi"
import * as pako from "pako"
import * as md5Lib from "js-md5"
import createQrcDecrypt from "@online/lib/lyric/qrcDecrypt"
import type { LyricInfo, MusicInfo } from "@online/types/music"

// QRC 解码器构建一次即可（内部会预生成 DES 密钥表）。
let qrcDecoder: ((hex: string) => string) | null = null
function getQrcDecoder(): (hex: string) => string {
  if (!qrcDecoder) qrcDecoder = createQrcDecrypt()
  return qrcDecoder
}

// js-md5 CommonJS/ESM interop (same pattern as src/lib/search/mg.ts).
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const md5 = ((md5Lib as any).default ?? md5Lib) as (str: string) => string

// Built-in lyric fetching for tx / wy / kg / mg, fetched directly from each
// platform's public API (same approach as src/lib/search/*.ts and the KuWo
// implementation in src/lib/lyric.ts). Ported from lx-music-desktop:
//   tx: src/renderer/utils/musicSdk/tx/lyric.js
//   wy: src/renderer/utils/musicSdk/wy/lyric.js (+ wy/utils/crypto.js)
//   kg: src/renderer/utils/musicSdk/kg/lyric.js (+ common/utils/lyricUtils/kg.ts)
//   mg: src/renderer/utils/musicSdk/mg/lyric.js (+ mg/musicInfo.js)
//
// Each exported fn returns { lyric, tlyric? } (LRC text) or null on
// failure / no-lyric. Everything is wrapped in try/catch -> null.

// --- shared helpers ------------------------------------------------------

// base64 -> raw bytes
function b64ToUint8(str: string): Uint8Array {
  const binary = atob(str)
  const buf = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) buf[i] = binary.charCodeAt(i)
  return buf
}

// Decode a UTF-8 base64 string (Buffer-free: bytes -> TextDecoder).
function b64DecodeUtf8(str: string): string {
  if (!str) return ""
  return new TextDecoder().decode(b64ToUint8(str))
}

// HTML-entity decode, mirroring lx-music common/utils/lyricUtils/util.ts.
const ENTITY_MAP: Record<string, string> = {
  "&nbsp;": " ",
  "&amp;": "&",
  "&lt;": "<",
  "&gt;": ">",
  "&quot;": '"',
  "&apos;": "'",
  "&#039;": "'",
}
function decodeName(str: string | null | undefined): string {
  return (
    str?.replace(/(?:&amp;|&lt;|&gt;|&quot;|&apos;|&#039;|&nbsp;)/gm, (s) => ENTITY_MAP[s]) ?? ""
  )
}

// --- tx (QQ音乐) ---------------------------------------------------------

interface TxLyricResponse {
  code?: number
  lyric?: string
  trans?: string
}

/**
 * QQ 音乐逐字歌词（QRC）。
 *
 * musicu.fcg 的 `crypt:1` 返回自定义 S 盒 Triple-DES + zlib 加密的 QRC，
 * 已由 `qrcDecrypt.ts` 纯 JS 解出（无需原生插件）。该接口要求【数字 songID】，
 * 而 song.meta.songId 存的是 songmid，故先用 songmid 换取数字 id。
 *
 * 解出的是 XML，逐字正文在 `LyricContent` 属性里，形如：
 *   `[19349,2531]今(19349,362)天(19711,376)我(20087,1793)`
 * 该格式可直接交给 AMLL 的 parseQrc。
 *
 * 任一环节失败都回落到 fcg_query_lyric_new.fcg 的纯 LRC。
 */
async function getTxSongId(songmid: string): Promise<number | null> {
  try {
    const body = {
      comm: { ct: "19", cv: "1859", uin: "0" },
      req: {
        method: "get_song_detail_yqq",
        module: "music.pf_song_detail_svr",
        param: { song_mid: songmid },
      },
    }
    const res = await httpFetch("https://u.y.qq.com/cgi-bin/musicu.fcg", {
      method: "POST",
      headers: { Referer: "https://y.qq.com", "Content-Type": "application/json" },
      body: JSON.stringify(body),
    })
    if (!res.ok) return null
    const data = (await res.json()) as { req?: { data?: { track_info?: { id?: number } } } }
    return data?.req?.data?.track_info?.id ?? null
  } catch {
    return null
  }
}

/** 取出 QRC XML 中 LyricContent 属性内的正文。 */
function unwrapQrcXml(str: string): string {
  if (!str) return ""
  if (!/LyricContent="/.test(str)) return str
  return decodeName(str.replace(/^[\S\s]*?LyricContent="/, "").replace(/"\/>[\S\s]*?$/, ""))
}

async function getTxQrc(songmid: string): Promise<LyricInfo | null> {
  const songID = await getTxSongId(songmid)
  if (!songID) return null
  const body = {
    comm: { ct: "19", cv: "1859", uin: "0" },
    req: {
      method: "GetPlayLyricInfo",
      module: "music.musichallSong.PlayLyricInfo",
      param: {
        format: "json",
        crypt: 1,
        ct: 19,
        cv: 1873,
        interval: 0,
        lrc_t: 0,
        qrc: 1,
        qrc_t: 0,
        roma: 0,
        roma_t: 0,
        songID,
        trans: 1,
        trans_t: 0,
        type: -1,
      },
    },
  }
  const res = await httpFetch("https://u.y.qq.com/cgi-bin/musicu.fcg", {
    method: "POST",
    headers: { Referer: "https://y.qq.com", "Content-Type": "application/json" },
    body: JSON.stringify(body),
  })
  if (!res.ok) return null
  const data = (await res.json()) as {
    code?: number
    req?: { code?: number; data?: { lyric?: string; trans?: string } }
  }
  if (data.code !== 0 || data.req?.code !== 0) return null
  const raw = data.req?.data
  if (!raw?.lyric) return null

  const decrypt = getQrcDecoder()
  const qrc = unwrapQrcXml(decrypt(raw.lyric))
  if (!qrc.trim()) return null
  // 翻译同样是加密的，但为普通 LRC（无字级时间）。
  let tlyric = ""
  if (raw.trans) {
    try {
      tlyric = unwrapQrcXml(decrypt(raw.trans))
    } catch {
      tlyric = ""
    }
  }

  // 纯 LRC 回退版：行首 [ms,dur] 转 [mm:ss.SSS]，并去掉 (off,dur,0) 字标记。
  const lrcLines: string[] = []
  for (const line of qrc.split("\n")) {
    const m = /^\[(\d+),\d+\]/.exec(line.trim())
    if (!m) continue
    let t = parseInt(m[1])
    const ms = t % 1000
    t = Math.floor(t / 1000)
    const mm = String(Math.floor(t / 60)).padStart(2, "0")
    const ss = String(t % 60).padStart(2, "0")
    const words = line
      .trim()
      .replace(/^\[\d+,\d+\]/, "")
      .replace(/\(\d+,\d+,\d+\)/g, "")
    lrcLines.push(`[${mm}:${ss}.${String(ms).padStart(3, "0")}]${words}`)
  }
  if (!lrcLines.length) return null

  return {
    lyric: lrcLines.join("\n"),
    tlyric: tlyric.trim() || null,
    lxlyric: qrc,
  }
}

// 先试逐字 QRC，失败再退回旧的 base64 纯 LRC 接口。
// song.meta.songId 为 songmid（形如 "0039MnYb0qxYhV"）。
export async function getTxLyric(song: MusicInfo): Promise<LyricInfo | null> {
  try {
    const songmid = song.meta.songId
    if (!songmid) return null
    try {
      const qrc = await getTxQrc(songmid)
      if (qrc) return qrc
    } catch {
      // 逐字失败 -> 走下面的纯 LRC
    }
    const url =
      `https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg?songmid=${encodeURIComponent(songmid)}` +
      `&g_tk=5381&loginUin=0&hostUin=0&format=json&inCharset=utf8&outCharset=utf-8` +
      `&platform=yqq&notice=0&needNewCode=0`
    const res = await httpFetch(url, {
      method: "GET",
      headers: {
        Referer: "https://y.qq.com/portal/player.html",
        "User-Agent":
          "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/86.0.4240.198 Safari/537.36",
      },
    })
    if (!res.ok) return null

    // The endpoint sometimes wraps JSON in a `MusicJsonCallback(...)` JSONP
    // shell depending on params; strip it defensively before parsing.
    let text = await res.text()
    const jsonpMatch = /^[^{]*?(\{[\s\S]*\})[^}]*$/.exec(text)
    if (jsonpMatch) text = jsonpMatch[1]
    const data = JSON.parse(text) as TxLyricResponse

    if (data.code !== 0 || !data.lyric) return null
    const lyric = decodeName(b64DecodeUtf8(data.lyric))
    if (!lyric.trim()) return null
    const tlyric = data.trans ? decodeName(b64DecodeUtf8(data.trans)) : ""
    return { lyric, tlyric: tlyric || null }
  } catch {
    return null
  }
}

// --- wy (网易云) ---------------------------------------------------------
interface WyLyricResponse {
  lrc?: { lyric?: string }
  tlyric?: { lyric?: string }
  yrc?: { lyric?: string }
  ytlrc?: { lyric?: string }
  data?: { lrc?: { lyric?: string }; tlyric?: { lyric?: string } }
}

/**
 * 网易云歌词。用 `/api/song/lyric/v1` 并带上 `yv/ytv/yrv` 参数，
 * 除普通 LRC 外还会返回 `yrc` 逐字歌词（部分歌曲才有），格式为
 * `[行起始,行时长](起始,时长,0)字...`，可直接交给 AMLL 的 parseYrc。
 * 该接口是 GET、无需 weapi 签名。没有 yrc 时自动退回纯 LRC。
 */
export async function getWyLyric(song: MusicInfo): Promise<LyricInfo | null> {
  try {
    const id = song.meta.songId
    if (!id) return null

    const res = await httpFetch(
      `https://music.163.com/api/song/lyric/v1?id=${encodeURIComponent(id)}` +
        `&cp=false&lv=0&kv=0&tv=0&rv=0&yv=0&ytv=0&yrv=0`,
      {
        method: "GET",
        headers: {
          "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
          Referer: "https://music.163.com/",
        },
      }
    )
    if (!res.ok) return null
    const data = (await res.json()) as WyLyricResponse
    const lrc = data.lrc?.lyric ?? null
    const yrc = data.yrc?.lyric ?? null
    // 有逐字时优先用逐字对应的翻译（ytlrc），否则用普通翻译。
    const tlyric = (yrc ? (data.ytlrc?.lyric ?? data.tlyric?.lyric) : data.tlyric?.lyric) ?? null
    if (!lrc?.trim() && !yrc?.trim()) return null
    return {
      lyric: lrc ?? "",
      tlyric: tlyric || null,
      lxlyric: yrc?.trim() ? yrc : null,
    }
  } catch {
    return null
  }
}

// --- kg (酷狗) -----------------------------------------------------------
// Multi-step: searchLyric (by name+hash+timelength) -> candidate {id,
// accesskey, fmt} -> download -> base64 (lrc) or krc decode (krc). No signing,
// just static client headers. song.meta.hash is the file hash.

const KG_HEADERS: Record<string, string> = {
  "KG-RC": "1",
  "KG-THash": "expand_search_manager.cpp:852736169:451",
  "User-Agent": "KuGou2012-9020-ExpandSearchManager",
}

interface KgSearchCandidate {
  id: number | string
  accesskey: string
  krctype?: number
  contenttype?: number
}
interface KgSearchLyricResponse {
  candidates?: KgSearchCandidate[]
}
interface KgDownloadResponse {
  fmt?: string
  content?: string
}

// "mm:ss" / "hh:mm:ss" -> seconds. Mirrors kg/lyric.js getIntv.
function kgIntervalToSeconds(interval: string): number {
  if (!interval) return 0
  const parts = interval.split(":")
  let total = 0
  let unit = 1
  while (parts.length) {
    total += Number(parts.pop()) * unit
    unit *= 60
  }
  return Math.trunc(total)
}

// krc decode: base64 -> drop 4-byte magic -> XOR with rolling key -> zlib
// inflate -> parse word-timed lyric + embedded translation. Ported from
// common/utils/lyricUtils/kg.ts (uses pako.inflate in place of node zlib).
const KG_KRC_KEY = new Uint8Array([
  0x40, 0x47, 0x61, 0x77, 0x5e, 0x32, 0x74, 0x47, 0x51, 0x36, 0x31, 0x2d, 0xce, 0xd2, 0x6e, 0x69,
])

interface KgKrcTransContentItem {
  type?: number
  lyricContent?: string[][]
}
interface KgKrcTrans {
  content?: KgKrcTransContentItem[]
}

function decodeKrc(content: string): LyricInfo | null {
  const raw = b64ToUint8(content)
  if (!raw.length) return null
  const body = raw.subarray(4)
  for (let i = 0; i < body.length; i++) body[i] = body[i] ^ KG_KRC_KEY[i % 16]
  const inflated = pako.inflate(body)
  let str = new TextDecoder().decode(inflated).replace(/\r/g, "")

  const headExp = /^.*\[id:\$\w+\]\n/
  if (headExp.test(str)) str = str.replace(headExp, "")

  // Translation lines are stored as a base64 JSON blob in a [language:...] tag.
  let tlyricLines: string[] | null = null
  const transMatch = /\[language:([\w=\\/+]+)\]/.exec(str)
  if (transMatch) {
    str = str.replace(/\[language:[\w=\\/+]+\]\n/, "")
    try {
      const json = JSON.parse(b64DecodeUtf8(transMatch[1])) as KgKrcTrans
      for (const item of json.content ?? []) {
        // type 1 == translation, type 0 == romaji/pinyin.
        if (item.type === 1) tlyricLines = (item.lyricContent ?? []).map((g) => g.join(""))
      }
    } catch {
      tlyricLines = null
    }
  }

  // Each line is `[<start>,<dur>]<(off,dur,wd)word...>`, where each word's
  // `off` is relative to the line start.
  //
  // We emit two things:
  //   - `lyric`   : plain LRC (`[mm:ss.SSS]text`) with word timings stripped,
  //                 used as a fallback and for the compact/mini views.
  //   - `lxlyric` : the raw `[startMs,durMs]` + `(absOff,dur,0)` word form.
  //                 `isYrcFormat` detects the `[digits,digits]` line head and
  //                 `convertLrcFormat` rewrites it into AMLL's `<mm:ss.SSS>`
  //                 A2 syntax, giving karaoke-style per-character highlighting.
  //                 Note KRC word offsets are relative to the line start, while
  //                 the converter expects absolute times, so we add the line
  //                 start to each offset here.
  const lrcLines: string[] = []
  const tlrcLines: string[] = []
  const lxLines: string[] = []
  let idx = 0
  for (const line of str.split("\n")) {
    const m = /^\[(\d+),(\d+)\].*/.exec(line)
    if (!m) continue
    const lineStart = parseInt(m[1])
    const lineDur = parseInt(m[2])
    let time = parseInt(m[1])
    const ms = time % 1000
    time = Math.floor(time / 1000)
    const mm = String(Math.floor(time / 60)).padStart(2, "0")
    const ss = String(time % 60).padStart(2, "0")
    const tag = `[${mm}:${ss}.${String(ms).padStart(3, "0")}]`
    const body = line.replace(/^\[\d+,\d+\]/, "")
    const decoded = decodeName(body)
    // 纯 LRC：去掉所有字时间标记。
    const words = decoded.replace(/<\d+,\d+,\d+>/g, "")
    lrcLines.push(`${tag}${words}`)
    // 逐字：把 KRC 的 `<相对偏移,时长,0>字` 转成 YRC 的 `(绝对时间,时长,0)字`。
    // 注意标记在字【之前】，且偏移要加上行起始时间转成绝对时间。
    if (/<\d+,\d+,\d+>/.test(decoded)) {
      const lxBody = decoded.replace(
        /<(\d+),(\d+),\d+>/g,
        (_all, off: string, dur: string) => `(${lineStart + parseInt(off)},${dur},0)`,
      )
      lxLines.push(`[${lineStart},${lineDur}]${lxBody}`)
    }
    if (tlyricLines) tlrcLines.push(`${tag}${tlyricLines[idx] ?? ""}`)
    idx++
  }

  const lyric = lrcLines.join("\n")
  if (!lyric.trim()) return null
  return {
    lyric,
    tlyric: tlrcLines.length ? tlrcLines.join("\n") : null,
    lxlyric: lxLines.length ? lxLines.join("\n") : null,
  }
}

async function kgSearchLyric(
  name: string,
  hash: string,
  time: number
): Promise<{ id: string; accessKey: string; fmt: string } | null> {
  const url =
    `https://lyrics.kugou.com/search?ver=1&man=yes&client=pc` +
    `&keyword=${encodeURIComponent(name)}&hash=${encodeURIComponent(hash)}` +
    `&timelength=${time}&lrctxt=1`
  const res = await httpFetch(url, { method: "GET", headers: KG_HEADERS })
  if (!res.ok) return null
  const data = (await res.json()) as KgSearchLyricResponse
  const info = data.candidates?.[0]
  if (!info) return null
  const fmt = info.krctype === 1 && info.contenttype !== 1 ? "krc" : "lrc"
  return { id: String(info.id), accessKey: info.accesskey, fmt }
}

async function kgDownloadLyric(
  id: string,
  accessKey: string,
  fmt: string
): Promise<LyricInfo | null> {
  const url =
    `https://lyrics.kugou.com/download?ver=1&client=pc&id=${encodeURIComponent(id)}` +
    `&accesskey=${encodeURIComponent(accessKey)}&fmt=${fmt}&charset=utf8`
  const res = await httpFetch(url, { method: "GET", headers: KG_HEADERS })
  if (!res.ok) return null
  const data = (await res.json()) as KgDownloadResponse
  if (!data.content) return null
  switch (data.fmt) {
    case "krc":
      return decodeKrc(data.content)
    case "lrc": {
      const lyric = b64DecodeUtf8(data.content)
      if (!lyric.trim()) return null
      return { lyric, tlyric: null }
    }
    default:
      return null
  }
}

// KuGou lyric. song.meta.hash is the file hash; song.meta.songId is the
// album_audio_id. Both krc (word-timed, with optional translation) and plain
// lrc candidate formats are handled.
export async function getKgLyric(song: MusicInfo): Promise<LyricInfo | null> {
  try {
    const hash = song.meta.hash
    if (!hash) return null
    const time = kgIntervalToSeconds(song.interval)
    const candidate = await kgSearchLyric(song.name, hash, time)
    if (!candidate) return null
    return await kgDownloadLyric(candidate.id, candidate.accessKey, candidate.fmt)
  } catch {
    return null
  }
}

// --- mg (咪咕) -----------------------------------------------------------
// Migu lyric URLs (lrcUrl / trcUrl / mrcUrl) live on the song resource. Two
// ways to obtain them, mirroring lx-music's mg/lyric.js + mg/musicSearch.js:
//
//   1. PRIMARY — the signed v3 `searchAll` endpoint (same signature scheme as
//      src/lib/search/mg.ts). Each search hit already carries `lrcUrl`/`trcUrl`/
//      `mrcurl`, which is exactly what lx-music reads off its search results.
//   2. FALLBACK — `resourceinfo.do?resourceType=2` with `resourceId=<copyrightId>`.
//      lx-music's createGetMusicInfosTask sends this with NO custom headers, so it
//      inherits the global DESKTOP Chrome User-Agent (request.js defaultHeaders).
//      The previous Museek port sent a *mobile* UA + Referer/channel here, which
//      is why resourceinfo.do returned an empty `resource:[]` (the endpoint is
//      UA-sensitive). We now send the desktop UA to match the reference.
//
// We prefer the plain `lrcUrl` (download as-is). When a song only exposes the
// encrypted `mrcUrl`, we download it and decrypt it: the payload is a hex string
// that lx-music (mg/utils/mrc.js) treats as an array of signed 64-bit longs and
// runs XXTEA (Corrected Block TEA) decryption over, then reinterprets the long
// array as UTF-16LE text. The decrypted MRC text is word-timed
// (`[lineMs,dur]<wordOff,wordDur>word...`) and we collapse it to plain
// `[mm:ss.xx]` LRC (mrcParseLyric). In both cases `trcUrl` is fetched for the
// translation.

// Desktop UA matching lx-music renderer/utils/request.js defaultHeaders — the
// resourceinfo.do endpoint only fills `resource` for this UA.
const MG_DESKTOP_UA =
  "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/69.0.3497.100 Safari/537.36"

interface MgLyricUrls {
  lrcUrl?: string
  trcUrl?: string
  mrcUrl?: string
}

// --- mg encrypted MRC decryption (XXTEA on 64-bit longs) -----------------
// Faithful port of lx-music mg/utils/mrc.js. The encrypted lyric is a hex
// string; we slice it into 16-hex-char (= 64-bit) chunks, decrypt the resulting
// long array with XXTEA (Corrected Block TEA) using MG_MRC_KEY/MG_MRC_DELTA, and
// reinterpret each long as 8 little-endian bytes decoded as UTF-16LE.
//
// All arithmetic is signed-64-bit with wraparound. lx-music achieves this with a
// `toLong` helper that folds a BigInt into [-2^63, 2^63); BigInt.asIntN(64, x) is
// exactly that operation, so it stands in for every `toLong(...)` below.

const MG_MRC_DELTA = 2654435769n
const MG_MRC_MIN_LENGTH = 32
const MG_MRC_KEY = [
  27303562373562475n,
  18014862372307051n,
  22799692160172081n,
  34058940340699235n,
  30962724186095721n,
  27303523720101991n,
  27303523720101998n,
  31244139033526382n,
  28992395054481524n,
]

// Signed 64-bit fold — the BigInt equivalent of lx-music's `toLong`.
const toLong = (n: bigint): bigint => BigInt.asIntN(64, n)

// XXTEA decryption over an array of signed 64-bit longs (in place), mirroring
// mg/utils/mrc.js teaDecrypt. `y`/`z`/`sum`/`e`/`p` follow the canonical XXTEA
// naming; every intermediate is folded back to signed 64-bit via toLong.
function mgTeaDecrypt(data: bigint[], key: bigint[]): bigint[] {
  const length = data.length
  if (length < 1) return data
  const n = BigInt(length)

  let y = data[0]
  let sum = toLong((6n + 52n / n) * MG_MRC_DELTA)
  while (sum !== 0n) {
    const e = toLong(3n & toLong(sum >> 2n))
    let p = n
    while (true) {
      p--
      if (p > 0n) {
        const idx = Number(p)
        const z = data[idx - 1]
        y = toLong(
          data[idx] -
            (toLong(toLong(y ^ sum) + toLong(z ^ key[Number(toLong(toLong(3n & p) ^ e))])) ^
              toLong(toLong(toLong(z >> 5n) ^ toLong(y << 2n)) + toLong(toLong(y >> 3n) ^ toLong(z << 4n))))
        )
        data[idx] = y
      } else break
    }
    const z0 = data[length - 1]
    // Wrap branch (p == 0). Source wraps the whole MX in an extra toLong before
    // subtracting; kept here verbatim though it is a no-op (XOR of two signed-64
    // values is already signed-64).
    y = toLong(
      data[0] -
        toLong(
          toLong(toLong(key[Number(toLong(toLong(p & 3n) ^ e))] ^ z0) + toLong(y ^ sum)) ^
            toLong(toLong(toLong(z0 >> 5n) ^ toLong(y << 2n)) + toLong(toLong(y >> 3n) ^ toLong(z0 << 4n)))
        )
    )
    data[0] = y
    sum = toLong(sum - MG_MRC_DELTA)
  }
  return data
}

// Slice the hex string into 16-char (64-bit) chunks parsed as signed longs.
function mgHexToLongArray(data: string): bigint[] {
  const length = Math.floor(data.length / 16)
  const arr = new Array<bigint>(length)
  for (let i = 0; i < length; i++) {
    arr[i] = toLong(BigInt("0x" + data.substring(i * 16, i * 16 + 16)))
  }
  return arr
}

// Each long -> 8 little-endian bytes (two's-complement low 64 bits) -> UTF-16LE.
function mgLongArrayToString(data: bigint[]): string {
  const bytes = new Uint8Array(data.length * 8)
  for (let i = 0; i < data.length; i++) {
    let l = BigInt.asUintN(64, data[i])
    const base = i * 8
    for (let b = 0; b < 8; b++) {
      bytes[base + b] = Number(l & 0xffn)
      l >>= 8n
    }
  }
  return new TextDecoder("utf-16le").decode(bytes)
}

// decrypt(text): mg/utils/mrc.js — short payloads are returned unchanged.
function mgDecryptMrc(data: string): string {
  if (data == null || data.length < MG_MRC_MIN_LENGTH) return data
  return mgLongArrayToString(mgTeaDecrypt(mgHexToLongArray(data), MG_MRC_KEY))
}

/**
 * 解密后的咪咕 MRC 逐字文本 -> 普通 LRC + 逐字歌词。
 * MRC 行形如 `[行起始,行时长]字(起始,时长)字(起始,时长)`，字标记在字【之后】，
 * 时间已是绝对毫秒。补上第三个占位参数即成 QRC 风格，可交给 AMLL 的 parseQrc。
 */
function mgParseLyric(str: string): { lyric: string; lxlyric: string | null } {
  str = str.replace(/\r/g, "")
  const lineTime = /^\s*\[(\d+),\d+\]/
  const wordTime = /\(\d+,\d+\)/
  const wordTimeAll = /(\(\d+,\d+\))/g
  const lrcLines: string[] = []
  const lxLines: string[] = []
  for (const line of str.split("\n")) {
    if (line.length < 6) continue
    const result = lineTime.exec(line)
    if (!result) continue

    let time = parseInt(result[1])
    const ms = time % 1000
    time = Math.floor(time / 1000)
    const m = String(Math.floor(time / 60)).padStart(2, "0")
    const s = String(time % 60).padStart(2, "0")
    const tag = `[${m}:${s}.${ms}]`

    const words = line.replace(lineTime, "")
    lrcLines.push(`${tag}${words.replace(wordTimeAll, "")}`)
    // `(起始,时长)` -> `(起始,时长,0)`，行头沿用原始 `[起始,时长]`。
    if (wordTime.test(words)) {
      lxLines.push(
        `${(/^\s*(\[\d+,\d+\])/.exec(line) ?? [])[1] ?? tag}` +
          words.replace(/\((\d+),(\d+)\)/g, "($1,$2,0)")
      )
    }
  }
  return {
    lyric: lrcLines.join("\n"),
    lxlyric: lxLines.length ? lxLines.join("\n") : null,
  }
}

// Ported from mg/musicSearch.js createSignature (static device id + salts).
function mgCreateSignature(time: string, str: string): { sign: string; deviceId: string } {
  const deviceId = "963B7AA0D21511ED807EE5846EC87D20"
  const signatureMd5 = "6cdc72a439cef99a3418d2a78aa28c73"
  const sign = md5(`${str}${signatureMd5}yyapp2d16148780a1dcc7408e06336b98cfd50${deviceId}${time}`)
  return { sign, deviceId }
}

interface MgSearchSong {
  songId?: string
  copyrightId?: string
  lrcUrl?: string
  trcUrl?: string
  mrcurl?: string
}
interface MgSearchResponse {
  code?: string
  songResultData?: { resultList?: MgSearchSong[][] }
}

// PRIMARY: look the song up via the signed v3 search and read its lyric URLs
// straight off the matching result (mirrors lx-music's mg/musicSearch.filterData
// which exposes data.lrcUrl / data.trcUrl / data.mrcurl).
async function mgSearchLyricUrls(song: MusicInfo): Promise<MgLyricUrls | null> {
  const keyword = [song.name, song.singer].filter(Boolean).join(" ").trim()
  if (!keyword) return null
  const time = Date.now().toString()
  const { sign, deviceId } = mgCreateSignature(time, keyword)

  const searchSwitch =
    "%7B%22song%22%3A1%2C%22album%22%3A0%2C%22singer%22%3A0%2C%22tagSong%22%3A1" +
    "%2C%22mvSong%22%3A0%2C%22bestShow%22%3A1%2C%22songlist%22%3A0%2C%22lyricSong%22%3A0%7D"
  const url =
    `https://jadeite.migu.cn/music_search/v3/search/searchAll?isCorrect=0&isCopyright=1` +
    `&searchSwitch=${searchSwitch}&pageSize=30&text=${encodeURIComponent(keyword)}` +
    `&pageNo=1&sort=0&sid=USS`

  const res = await httpFetch(url, {
    method: "GET",
    headers: {
      uiVersion: "A_music_3.6.1",
      deviceId,
      timestamp: time,
      sign,
      channel: "0146921",
      "User-Agent":
        "Mozilla/5.0 (Linux; U; Android 11.0.0; zh-cn; MI 11 Build/OPR1.170623.032) AppleWebKit/534.30 (KHTML, like Gecko) Version/4.0 Mobile Safari/534.30",
    },
  })
  if (!res.ok) return null
  const data = (await res.json()) as MgSearchResponse
  if (data.code !== "000000") return null

  const groups = data.songResultData?.resultList ?? []
  const flat: MgSearchSong[] = []
  for (const g of groups) for (const s of g) flat.push(s)

  // Prefer the exact resource (copyrightId, then songId); else the first hit
  // for this name that actually carries a lyric URL.
  const byCopyright = song.meta.copyrightId
    ? flat.find((s) => s.copyrightId === song.meta.copyrightId)
    : undefined
  const bySongId = song.meta.songId
    ? flat.find((s) => s.songId === song.meta.songId)
    : undefined
  const match =
    byCopyright ??
    bySongId ??
    flat.find((s) => s.lrcUrl || s.mrcurl)
  if (!match) return null
  return { lrcUrl: match.lrcUrl, trcUrl: match.trcUrl, mrcUrl: match.mrcurl }
}

interface MgResourceItem {
  lrcUrl?: string
  trcUrl?: string
  mrcUrl?: string
}
interface MgResourceResponse {
  code?: string
  resource?: MgResourceItem[]
}

// FALLBACK: resourceinfo.do keyed by copyrightId. Sent with the desktop UA (see
// note above) — no Referer/channel, matching lx-music createGetMusicInfosTask.
async function mgResourceLyricUrls(copyrightId: string): Promise<MgLyricUrls | null> {
  const res = await httpFetch(
    "https://c.musicapp.migu.cn/MIGUM2.0/v1.0/content/resourceinfo.do?resourceType=2",
    {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        "User-Agent": MG_DESKTOP_UA,
      },
      body: new URLSearchParams({ resourceId: copyrightId }).toString(),
    }
  )
  if (!res.ok) return null
  const data = (await res.json()) as MgResourceResponse
  const item = data.resource?.[0]
  if (!item) return null
  return { lrcUrl: item.lrcUrl, trcUrl: item.trcUrl, mrcUrl: item.mrcUrl }
}

async function mgFetchText(url: string): Promise<string | null> {
  const res = await httpFetch(url, {
    method: "GET",
    headers: {
      Referer: "https://app.c.nf.migu.cn/",
      channel: "0146921",
      "User-Agent":
        "Mozilla/5.0 (Linux; Android 5.1.1; Nexus 6 Build/LYZ28E) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.115 Mobile Safari/537.36",
    },
  })
  if (!res.ok) return null
  return res.text()
}

// Migu lyric. Resolves lyric URLs via the signed search (primary) then
// resourceinfo.do (fallback), preferring the plain lrcUrl and falling back to
// decrypting the encrypted mrcUrl (+ trcUrl for translation). Mirrors
// mg/lyric.js mrcTools.getLyric: lrcUrl wins, else decrypt+parse mrcUrl.
export async function getMgLyric(song: MusicInfo): Promise<LyricInfo | null> {
  try {
    let urls: MgLyricUrls | null = null
    try {
      urls = await mgSearchLyricUrls(song)
    } catch {
      urls = null
    }
    // Like lx-music's getMusicInfo (mrcUrl == null -> look it up by copyrightId),
    // fall back to resourceinfo.do when search gave us neither a plain nor an
    // encrypted lyric URL.
    if (!urls?.lrcUrl && !urls?.mrcUrl && song.meta.copyrightId) {
      try {
        urls = await mgResourceLyricUrls(song.meta.copyrightId)
      } catch {
        // keep whatever the search gave us (possibly null)
      }
    }
    if (!urls?.lrcUrl && !urls?.mrcUrl) return null // no lyric available

    // 优先取 mrcUrl（含逐字），失败或没有再退回纯 lrcUrl。
    let parsed: { lyric: string; lxlyric?: string | null } | null = null
    if (urls.mrcUrl) {
      const enc = await mgFetchText(urls.mrcUrl)
      if (enc) {
        const mrc = mgParseLyric(mgDecryptMrc(enc))
        if (mrc.lyric.trim()) parsed = mrc
      }
    }
    if (!parsed && urls.lrcUrl) {
      const text = await mgFetchText(urls.lrcUrl)
      if (text?.trim()) parsed = { lyric: text }
    }
    if (!parsed?.lyric?.trim()) return null

    let tlyric: string | null = null
    if (urls.trcUrl) {
      const trans = await mgFetchText(urls.trcUrl)
      tlyric = trans?.trim() ? trans : null
    }
    return {
      lyric: parsed.lyric,
      tlyric,
      lxlyric: parsed.lxlyric ?? null,
    }
  } catch {
    return null
  }
}
