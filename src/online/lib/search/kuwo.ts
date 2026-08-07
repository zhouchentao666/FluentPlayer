import { httpFetch as tauriFetch } from "@online/lib/http"
import type { MusicInfo, MusicQuality, Quality, SearchResult } from "@online/types/music"
import { indexQualitySizes } from "@online/lib/quality"
import { formatDuration } from "@online/lib/utils"

interface KwSongRaw {
  MUSICRID: string
  SONGNAME: string
  ARTIST: string
  DURATION: string
  ALBUM: string
  ALBUMID: string
  N_MINFO?: string
  web_albumpic_short?: string
  pic?: string
}

interface KwSearchResponse {
  TOTAL?: string
  SHOW?: string
  abslist?: KwSongRaw[]
}

const bitrateToQuality: Record<string, Quality> = {
  "4000": "flac24bit",
  "2000": "flac",
  "320": "320k",
  "128": "128k",
}

const mInfoRx = /level:\w+,bitrate:(\d+),format:\w+,size:([\w.]+)/g

function parseNMinfo(nMinfo: string): MusicQuality[] {
  const result: MusicQuality[] = []
  mInfoRx.lastIndex = 0
  let match: RegExpExecArray | null
  while ((match = mInfoRx.exec(nMinfo)) !== null) {
    const q = bitrateToQuality[match[1]]
    if (q) result.push({ type: q, size: match[2] })
  }
  return result.reverse()
}

function decodeKwName(name: string): string {
  try {
    return decodeURIComponent(name.replace(/\+/g, " "))
  } catch {
    return name
  }
}

function normalizeKwSong(raw: KwSongRaw): MusicInfo {
  const songId = raw.MUSICRID.replace("MUSIC_", "")
  const qualitys = raw.N_MINFO ? parseNMinfo(raw.N_MINFO) : [{ type: "128k" as Quality, size: null }]
  const _qualitys = indexQualitySizes(qualitys)

  const duration = parseInt(raw.DURATION)
  const picFromSearch = raw.web_albumpic_short
    ? `https://img1.kuwo.cn/star/albumcover/${raw.web_albumpic_short}`
    : null
  // 搜索结果自带 pic 字段常为「//www.kuwo.cn/...」(无协议双斜杠) 或完整 http(s) URL。
  // 无协议时只补 https: 协议，不能拼成 https://www.kuwo.cn//www.kuwo.cn/... 这样的错误地址。
  const picFromRaw = raw.pic
    ? raw.pic.startsWith("http")
      ? raw.pic
      : raw.pic.startsWith("//")
        ? `https:${raw.pic}`
        : `https://www.kuwo.cn${raw.pic}`
    : null
  return {
    id: `kw_${songId}`,
    name: decodeKwName(raw.SONGNAME),
    singer: (raw.ARTIST || "").replace(/&/g, "、"),
    source: "kw",
    interval: isNaN(duration) ? "0:00" : formatDuration(duration),
    albumName: decodeKwName(raw.ALBUM || ""),
    meta: {
      songId,
      albumId: raw.ALBUMID || "",
      // 封面优先级：搜索结果自带 pic > albumcover 缩略图 > 稍后 searchKuwo 并行补真实封面。
      picUrl: picFromRaw ?? picFromSearch ?? "",
      qualitys,
      _qualitys,
    },
  }
}

// 通过 kuwo 的 artistpicserver 接口按歌曲 rid 获取专辑封面（参考 Mio-Music 实现）。
// 旧的 m.kuwo.cn musicInfo 接口已经失效、返回空，故改用 artistpicserver.pic.web。
async function getKwCoverUrl(songId: string): Promise<string> {
  try {
    const url =
      `http://artistpicserver.kuwo.cn/pic.web?corp=kuwo&type=rid_pic` +
      `&pictype=500&size=500&rid=${encodeURIComponent(songId)}`
    const res = await tauriFetch(url, {
      method: "GET",
      headers: {
        Referer: "https://www.kuwo.cn/",
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
      },
    })
    if (!res.ok) return ""
    // artistpicserver.kuwo.cn/pic.web 直接返回纯文本图片 URL（非 JSON），
    // 早期用 res.json() 取 data.url 永远为空，导致酷我封面缺失。改用 res.text()。
    const text = (await res.text()).trim()
    if (!text || !/^https?:\/\//i.test(text)) return ""
    return text
  } catch {
    return ""
  }
}

export async function searchKuwo(
  query: string,
  page = 1,
  limit = 30
): Promise<SearchResult> {
  const params = new URLSearchParams({
    client: "kt",
    all: query,
    pn: String(page - 1),
    rn: String(limit),
    uid: "794762570",
    ver: "kwplayer_ar_9.2.2.1",
    vipver: "1",
    show_copyright_off: "1",
    newver: "1",
    ft: "music",
    cluster: "0",
    strategy: "2012",
    encoding: "utf8",
    rformat: "json",
    mobi: "1",
  })

  const res = await tauriFetch(`http://search.kuwo.cn/r.s?${params}`, {
    method: "GET",
    headers: {
      Referer: "https://www.kuwo.cn/",
      "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    },
  })

  if (!res.ok) throw new Error(`KuWo search failed: ${res.status}`)

  const data = (await res.json()) as KwSearchResponse
  const total = parseInt(data.TOTAL || "0")
  const list = (data.abslist || []).map(normalizeKwSong)

  // 并行补充缺失封面（artistpicserver 旧接口已失效，统一走歌曲详情接口）。
  await Promise.all(
    list
      .filter((s) => !s.meta.picUrl)
      .map(async (s) => {
        const pic = await getKwCoverUrl(s.meta.songId)
        if (pic) s.meta.picUrl = pic
      }),
  )

  return {
    list,
    total,
    page,
    allPage: Math.ceil(total / limit),
    limit,
  }
}
