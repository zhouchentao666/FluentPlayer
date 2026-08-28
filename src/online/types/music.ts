export type OnlineSource = "kw" | "kg" | "tx" | "wy" | "mg"
export type Source = OnlineSource | "local"
export type Quality = "128k" | "320k" | "flac" | "flac24bit"

export interface MusicQuality {
  type: Quality
  size: string | null
}

export interface MusicInfoMeta {
  songId: string
  albumId?: string
  picUrl?: string | null
  qualitys: MusicQuality[]
  _qualitys: Partial<Record<Quality, { size: string | null }>>
  // kg-specific
  hash?: string
  // tx-specific
  strMediaMid?: string
  // mg-specific
  copyrightId?: string
  /** Absolute path of a local library file (source === "local"). */
  filePath?: string
  /** AppData-relative cover extracted from tags, e.g. museek/localCovers/….jpg */
  localCoverRel?: string
  /** Timed LRC text extracted from tags at import (local only). */
  embeddedLyric?: string
}

export interface MusicInfo {
  id: string
  name: string
  singer: string
  source: Source
  interval: string
  albumName: string
  meta: MusicInfoMeta
}

/** Device-local library entry (not synced). */
export interface LocalTrack {
  id: string
  filePath: string
  addedAt: number
  /** One category at most; null / omitted = uncategorized. */
  categoryId?: string | null
  /** File missing/moved/unreadable — set after a failed play attempt. */
  unavailable?: boolean
  song: MusicInfo
}

/** User-defined local library category (one track → one category). */
export interface LocalCategory {
  id: string
  name: string
  createdAt: number
}

export interface SearchResult {
  list: MusicInfo[]
  total: number
  page: number
  allPage: number
  limit: number
}

export interface LyricInfo {
  lyric: string
  tlyric?: string | null
  rlyric?: string | null
  lxlyric?: string | null
  /**
   * lxlyric 的格式，由歌词抓取层显式声明，避免 useLyrics
   * 用脆弱正则（如要求同时含 [ti:][ar:][al:]）误判导致逐字丢失。
   *   'qrc'  — QQ 音乐 QRC 逐字（<dur,off,len> 元信息）
   *   'yrc'  — 网易云 YRC 逐字（[行起,行长](起,长,0)字…）
   *   'lrc'  — 普通逐行 LRC（无字级时间）
   */
  lyricType?: 'qrc' | 'yrc' | 'lrc'
  /** AMLL / TTML 逐字歌词原文（网易云 v1 接口返回）；优先于 lrc 使用，不转成 lrc。 */
  ttml?: string | null
}

export interface LyricLine {
  time: number
  text: string
  translation?: string
}
