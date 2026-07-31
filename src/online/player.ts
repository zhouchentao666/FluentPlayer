import { ref } from "vue"
import { getBuiltinLyric } from "@online/lib/lyric"
import { bestQuality, QUALITY_LADDER, QUALITY_SHORT } from "@online/lib/quality"
import { sourceRunner } from "@online/lib/sourceRunner"
import type { LyricInfo, MusicInfo, Quality } from "@online/types/music"
import type { Song, SongMetadata } from "../types"

export { QUALITY_LADDER, QUALITY_SHORT }
export type { MusicInfo, Quality }

const PREF_KEY = "tideaudio-online:preferredQuality"
const DL_KEY = "tideaudio-online:downloadQuality"

/** 用户偏好音质（会自动向下降级）。 */
export const preferredQuality = ref<Quality>(loadQuality(PREF_KEY, "320k"))
/** 下载首选音质（同样会自动向下降级）。 */
export const downloadQuality = ref<Quality>(loadQuality(DL_KEY, "flac"))
/** 当前在线播放实际生效的音质（用于播放条角标显示）。 */
export const activeQuality = ref<Quality | null>(null)

function loadQuality(key: string, fallback: Quality): Quality {
  const v = localStorage.getItem(key) as Quality | null
  return v && QUALITY_LADDER.includes(v) ? v : fallback
}

export function setPreferredQuality(q: Quality): void {
  preferredQuality.value = q
  localStorage.setItem(PREF_KEY, q)
}

export function setDownloadQuality(q: Quality): void {
  downloadQuality.value = q
  localStorage.setItem(DL_KEY, q)
}

/** 把线上曲目 interval "mm:ss" 转成秒。 */
function intervalToSeconds(interval?: string): number {
  if (!interval) return 0
  const parts = interval.split(":").map((p) => parseInt(p, 10))
  if (parts.some((n) => isNaN(n))) return 0
  return parts.reduce((acc, n) => acc * 60 + n, 0)
}

/** 在线歌曲的 Song.path 前缀标识（不会真实存在于磁盘）。 */
export const ONLINE_PATH_PREFIX = "online://"

export function isOnlineSong(song: { path?: string; online?: MusicInfo | null } | null | undefined): boolean {
  return !!song?.online || !!song?.path?.startsWith(ONLINE_PATH_PREFIX)
}

/** MusicInfo → tideaudio Song，可加入播放队列 / 收藏进本地歌单。 */
export function musicInfoToSong(m: MusicInfo): Song {
  const duration = intervalToSeconds(m.interval)
  const metadata: SongMetadata = {
    title: m.name,
    artist: m.singer,
    album: m.albumName ?? "",
    genre: "",
    year: "",
    duration,
    bitrate: 0,
  }
  return {
    id: `online:${m.source}:${m.meta.songId}`,
    path: `${ONLINE_PATH_PREFIX}${m.source}/${m.meta.songId}`,
    title: m.name,
    cover: m.meta.picUrl ?? undefined,
    metadata,
    online: m,
  }
}

/** 解析在线播放直链（自定义音源 + 内置兜底，自动降级音质）。 */
export async function resolveOnlineUrl(
  m: MusicInfo,
  preferred?: Quality,
): Promise<{ url: string; quality: Quality }> {
  const start = preferred ?? preferredQuality.value
  const capped = capToSong(m, start)
  const result = await sourceRunner.getMusicUrlAdaptive(m, capped)
  return result
}

/** 首选音质不超过歌曲标称的最高音质（标称缺失时不设限）。 */
function capToSong(m: MusicInfo, preferred: Quality): Quality {
  const best = bestQuality(m)
  if (!best) return preferred
  const bi = QUALITY_LADDER.indexOf(best)
  const pi = QUALITY_LADDER.indexOf(preferred)
  return pi < bi ? best : preferred
}

/** 获取在线封面（元数据缺失时向音源脚本请求）。 */
export async function resolveOnlinePic(m: MusicInfo): Promise<string | null> {
  if (m.meta.picUrl) return m.meta.picUrl
  try {
    return await sourceRunner.getPic({ source: m.source, action: "pic", info: m })
  } catch {
    return null
  }
}

/** 获取在线歌词：平台内置接口优先，失败时走自定义音源脚本。 */
export async function resolveOnlineLyric(m: MusicInfo): Promise<LyricInfo | null> {
  let info: LyricInfo | null = null
  try {
    info = await getBuiltinLyric(m)
  } catch {
    info = null
  }
  if (!info?.lyric) {
    try {
      info = await sourceRunner.getLyric({ source: m.source, action: "lyric", info: m })
    } catch {
      info = null
    }
  }
  return info?.lyric ? info : null
}

/** 把翻译 LRC 按时间戳合并为 AMLL 可识别的逐行译文（简单时间匹配）。 */
export function mergeTranslation(lyric: string, tlyric?: string | null): string {
  if (!tlyric?.trim()) return lyric
  return `${lyric.trimEnd()}\n${tlyric.trim()}\n`
}
