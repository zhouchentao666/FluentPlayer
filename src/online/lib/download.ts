import { resolveOnlineUrl, downloadQuality } from '../player'
import { cdnHeadersForUrl } from './cdnHeaders'
import { getBuiltinLyric } from './lyric'
import type { MusicInfo } from '../types/music'
import { toast } from '../../composables/useToast'
import { SaveFile, DownloadFile, OpenMusicFolder, LoadConfig, DefaultMusicFolder } from '../../bridge/app'

/** 去除文件名中的非法字符，并限制长度。 */
function safeFileName(name: string): string {
  return name
    .replace(/[\\/:*?"<>|\r\n\t]+/g, '_')
    .replace(/\s{2,}/g, ' ')
    .trim()
    .slice(0, 120)
}

/** 根据直链路径推断音频扩展名；无法判断时回退为 mp3。 */
function inferExt(url: string): string {
  const path = url.split('?')[0].split('#')[0].toLowerCase()
  if (path.endsWith('.flac')) return 'flac'
  if (path.endsWith('.m4a')) return 'm4a'
  if (path.endsWith('.ogg') || path.endsWith('.oga')) return 'ogg'
  if (path.endsWith('.wav')) return 'wav'
  if (path.endsWith('.ape')) return 'ape'
  if (path.endsWith('.mp3')) return 'mp3'
  return 'mp3'
}

/**
 * 解析下载目标目录：
 * 优先使用设置中的 downloadFolder；为空时回退到系统音乐文件夹。
 * 若两者都不可用则返回空字符串（调用方需回退到弹窗选择）。
 */
async function resolveFolder(): Promise<string> {
  try {
    const config = await LoadConfig()
    const saved = (config.settings as unknown as Record<string, unknown>)?.downloadFolder
    if (typeof saved === 'string' && saved.trim()) return saved.trim()
  } catch {
    // 忽略，回退到默认音乐文件夹
  }
  try {
    const def = await DefaultMusicFolder()
    if (def && def.trim() && def !== '.') return def.trim()
  } catch {
    // 忽略
  }
  return ''
}

/**
 * 真正的离线文件下载（单首）：
 * 解析在线直链 → 按设置目录（默认音乐文件夹）直接保存，不弹窗；
 * 仅当无法解析到目标目录时回退到“保存文件”对话框。
 * 返回是否下载成功。
 */
export async function downloadSong(m: MusicInfo, folder?: string): Promise<boolean> {
  let url: string
  try {
    const r = await resolveOnlineUrl(m, downloadQuality.value)
    url = r.url
  } catch (e) {
    toast(`获取下载链接失败：${(e as Error)?.message || e}`, 'error')
    return false
  }
  if (!url) {
    toast('无法获取下载链接（该音源可能不支持下载）', 'error')
    return false
  }

  const base = `${m.singer ? m.singer + ' - ' : ''}${m.name}`
  const name = safeFileName(base) + '.' + inferExt(url)
  const dir = folder !== undefined ? folder : await resolveFolder()
  let dest: string
  if (dir) {
    const sep = dir.endsWith('/') || dir.endsWith('\\') ? '' : '\\'
    dest = dir + sep + name
  } else {
    // 无可用目录时回退到保存文件对话框
    const picked = await SaveFile(name)
    if (!picked) return false
    dest = picked
  }

  toast(`开始下载：${m.name}`, 'info')
  try {
    // 读取内嵌歌词 / 封面设置
    let embedLyrics = false
    let embedCover = false
    try {
      const cfg = await LoadConfig()
      const s = (cfg.settings as unknown as Record<string, unknown>) ?? {}
      embedLyrics = s.embedLyrics !== false
      embedCover = s.embedCover !== false
    } catch {
      // 读取失败则默认不内嵌
    }
    // 内嵌歌词：提前获取歌词文本
    let lyricText: string | null = null
    if (embedLyrics) {
      try {
        const li = await getBuiltinLyric(m)
        lyricText = li?.lyric?.trim() || null
      } catch {
        lyricText = null
      }
    }
    const coverUrl = embedCover ? (m.cover || null) : null

    const res = await DownloadFile(url, dest, cdnHeadersForUrl(url), {
      embedLyrics,
      embedCover,
      lyric: lyricText,
      coverUrl,
    })
    const mb = (res.size / 1024 / 1024).toFixed(1)
    toast(`已下载：${m.name}（${mb} MB）`, 'success', 5000)
    return true
  } catch (e) {
    toast(`下载失败：${(e as Error)?.message || e}`, 'error', 6000)
    return false
  }
}

/**
 * 真正的离线文件下载（批量）：
 * 优先使用设置目录（默认音乐文件夹）直接下载，不弹窗；
 * 仅当无法解析到目标目录时回退到文件夹选择对话框。
 * 逐首解析直链并下载（受限并发）。
 */
export async function downloadMany(musics: MusicInfo[], folder?: string): Promise<void> {
  if (!musics.length) return
  const dir = folder !== undefined ? folder : await resolveFolder()
  const target = dir || (await OpenMusicFolder())
  if (!target) return // 用户取消

  let done = 0
  let fail = 0
  const total = musics.length
  toast(`开始批量下载 ${total} 首…`, 'info', 4000)

  const CONCURRENCY = 3
  let index = 0
  async function worker() {
    while (index < musics.length) {
      const i = index++
      const m = musics[i]
      let url = ''
      try {
        const r = await resolveOnlineUrl(m, downloadQuality.value)
        url = r.url
      } catch {
        fail++
        continue
      }
      if (!url) {
        fail++
        continue
      }
      const base = `${String(i + 1).padStart(2, '0')}. ${m.singer ? m.singer + ' - ' : ''}${m.name}`
      const name = safeFileName(base) + '.' + inferExt(url)
      const sep = target.endsWith('/') || target.endsWith('\\') ? '' : '\\'
      const dest = target + sep + name
      try {
        await DownloadFile(url, dest, cdnHeadersForUrl(url))
        done++
      } catch {
        fail++
      }
    }
  }

  const workers = Array.from({ length: Math.min(CONCURRENCY, total) }, () => worker())
  await Promise.all(workers)

  toast(
    `批量下载完成：成功 ${done} 首，失败 ${fail} 首`,
    fail > 0 && fail === total ? 'error' : 'success',
    6000,
  )
}
