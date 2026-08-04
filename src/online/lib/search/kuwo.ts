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
      // 搜索接口常缺封面：picFromSearch 缺失时先给占位，稍后 searchKuwo 并行补真实封面。
      picUrl: picFromSearch ?? "",
      qualitys,
      _qualitys,
    },
  }
}

// 通过 kuwo 歌曲详情接口获取真实封面 URL（artistpicserver 旧接口已失效）。
async function getKwCoverUrl(songId: string): Promise<string> {
  try {
    const res = await tauriFetch(
      `https://m.kuwo.cn/api/www/music/musicInfo?mid=${encodeURIComponent(songId)}`,
      {
        method: "GET",
        headers: {
          Referer: "https://www.kuwo.cn/",
          "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        },
      },
    )
    if (!res.ok) return ""
    const data = (await res.json()) as any
    const pic = data?.data?.pic
    if (!pic) return ""
    // 返回的相对/绝对地址统一走 kuwo 域名。
    return pic.startsWith("http") ? pic : `https://www.kuwo.cn${pic}`
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
