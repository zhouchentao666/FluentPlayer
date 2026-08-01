import { ref, watch } from 'vue'
import type { Quality } from '@online/player'

export type LyricFormat = 'lrc' | 'yrc' | 'ttml'

export interface DownloadConfig {
  /** 下载目录（绝对路径） */
  saveDir: string
  /** 内嵌标题/艺术家/专辑等元数据 */
  embedMetadata: boolean
  /** 内嵌歌词 */
  embedLyric: boolean
  /** 内嵌封面 */
  embedCover: boolean
  /** 内嵌歌词格式 */
  lyricFormat: LyricFormat
  /** 下载音质 */
  quality: Quality
}

const STORAGE_KEY = 'tide-download-config'

const DEFAULTS: DownloadConfig = {
  saveDir: '',
  embedMetadata: true,
  embedLyric: true,
  embedCover: true,
  lyricFormat: 'lrc',
  quality: '320k',
}

function load(): DownloadConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return { ...DEFAULTS }
    const parsed = JSON.parse(raw)
    return { ...DEFAULTS, ...parsed }
  } catch {
    return { ...DEFAULTS }
  }
}

const config = ref<DownloadConfig>(load())

watch(
  config,
  (v) => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(v))
  },
  { deep: true },
)

export function useDownloadConfig() {
  return config
}
