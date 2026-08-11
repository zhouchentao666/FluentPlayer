// 移动端文件选择 / 导入（仅移动端生效）
//
// 移动端沙盒无法访问任意磁盘路径，只能拿到系统 picker 授予的 URI
// （Android content:// / iOS file://）。这些 URI 可直接喂给 <audio> 播放。
//
// 元数据降级策略（详见交付说明）：
// - iOS 的 file:// 是真实路径，Rust read_metadata_mobile 走 lofty 返回完整标签。
// - Android 的 content:// Rust 无法直读，read_metadata_mobile 仅返回文件名推断的标题，
//   时长由 useAudioPlayer 在 <audio> loadedmetadata 时回填到 Song.metadata.duration。
import { ReadMetadataMobile, PickMusicFilesMobile, PickMusicFolderMobile } from '@bridge/app'
import type { Song } from '../types'

function uriToTitle(uri: string): string {
  const decoded = decodeURIComponent(uri.split('?')[0])
  const seg = decoded.replace(/\\/g, '/').replace(/\/$/, '').split('/').pop() || uri
  return seg.replace(/\.[^.]+$/, '') || uri
}

/** 由移动端 picker 返回的 URI 构造 Song（元数据用降级版）。 */
export async function createSongFromMobileUri(uri: string): Promise<Song> {
  let title = uriToTitle(uri)
  let duration = 0
  let meta: Song['metadata'] = undefined
  try {
    const m = await ReadMetadataMobile(uri)
    if (m.title) title = m.title
    duration = m.duration
    meta = {
      title: m.title || title,
      artist: m.artist,
      album: m.album,
      genre: m.genre,
      year: m.year,
      duration,
      bitrate: m.bitrate,
      sample_rate: m.sample_rate,
    } as Song['metadata']
  } catch {
    // 降级：仅文件名
  }
  return {
    id: crypto.randomUUID(),
    path: uri,
    title,
    metadata: meta,
  }
}

/** 移动端：弹出文件选择器导入音频。返回新增的 Song 列表。 */
export async function pickMobileMusicFiles(): Promise<Song[]> {
  const uris = await PickMusicFilesMobile()
  const out: Song[] = []
  for (const uri of uris) {
    if (/\.(mp3|flac|wav|ogg|m4a|aac|opus|wma|ape|tta|ac3|dts|mp2|mid|midi)$/i.test(uri)) {
      out.push(await createSongFromMobileUri(uri))
    }
  }
  return out
}

/** 移动端：弹出文件夹/文档选择器（实际复用多选文件）导入音频。 */
export async function pickMobileMusicFolder(): Promise<Song[]> {
  const uris = await PickMusicFolderMobile()
  const out: Song[] = []
  for (const uri of uris) {
    if (/\.(mp3|flac|wav|ogg|m4a|aac|opus|wma|ape|tta|ac3|dts|mp2|mid|midi)$/i.test(uri)) {
      out.push(await createSongFromMobileUri(uri))
    }
  }
  return out
}
