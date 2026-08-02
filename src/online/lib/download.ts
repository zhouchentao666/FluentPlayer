import { resolveOnlineUrl, downloadQuality } from '../player'
import { cdnHeadersForUrl } from './cdnHeaders'
import type { MusicInfo } from '../types/music'
import type { AppSettings } from '../../composables/useConfig'
import { toast } from '../../composables/useToast'
import { SaveFile, DownloadFile, OpenMusicFolder, GetDefaultMusicFolder } from '../../bridge/app'

/** 影响下载行为的设置子集。 */
export type DownloadSettings = Pick<AppSettings, 'downloadWithoutDialog' | 'downloadFolder'>

/** 去除文件名中的非法字符，并限制长度。 */
function safeFileName(name: string): string {
  return name
    .replace(/[\\/:*?"<>|\r\n\t]+/g, '_')
    .replace(/\s{2,}/g, ' ')
    .trim()
    .slice(0, 120)
}

/** 把目录与文件名拼成完整路径（按所在平台选择分隔符）。 */
function joinPath(dir: string, file: string): string {
  const sep = dir.includes('\\') ? '\\' : '/'
  return dir.endsWith(sep) ? dir + file : dir + sep + file
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
 * 真正的离线文件下载（单首）：
 * 解析在线直链 → 写入本地文件。
 * 若 settings.downloadWithoutDialog 为真，直接下载到 downloadFolder（空则用系统音乐文件夹），
 * 不弹出“保存文件”对话框；否则弹出对话框让用户选择保存位置。
 * 返回是否下载成功。
 */
export async function downloadSong(m: MusicInfo, settings?: DownloadSettings): Promise<boolean> {
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

  let dest: string
  if (settings?.downloadWithoutDialog) {
    const folder = (settings.downloadFolder || '').trim() || (await GetDefaultMusicFolder())
    if (!folder) {
      toast('未设置下载文件夹，且无法获取系统音乐文件夹', 'error')
      return false
    }
    dest = joinPath(folder, name)
  } else {
    const picked = await SaveFile(name)
    if (!picked) return false // 用户取消
    dest = picked
  }

  toast(`开始下载：${m.name}`, 'info')
  try {
    const res = await DownloadFile(url, dest, cdnHeadersForUrl(url))
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
 * 若 settings.downloadWithoutDialog 为真，直接下载到 downloadFolder（空则用系统音乐文件夹）；
 * 否则弹出“选择文件夹”对话框让用户选择保存位置。之后逐首解析直链并下载（受限并发）。
 */
export async function downloadMany(musics: MusicInfo[], settings?: DownloadSettings): Promise<void> {
  if (!musics.length) return
  let folder: string
  if (settings?.downloadWithoutDialog) {
    const f = (settings.downloadFolder || '').trim() || (await GetDefaultMusicFolder())
    if (!f) {
      toast('未设置下载文件夹，且无法获取系统音乐文件夹', 'error')
      return
    }
    folder = f
  } else {
    const picked = await OpenMusicFolder()
    if (!picked) return // 用户取消
    folder = picked
  }

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
      const sep = folder.endsWith('/') || folder.endsWith('\\') ? '' : '\\'
      const dest = folder + sep + name
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
