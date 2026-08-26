import { musicSdk, type MusicItem } from '@/services/musicSdk'
import { MessagePlugin } from 'tdesign-vue-next'
import i18n from '@/locales'

export async function downloadSong(song: MusicItem, quality?: string): Promise<void> {
  try {
    const url = await musicSdk.getMusicUrl(song, quality)
    if (!url) {
      MessagePlugin.error(i18n.global.t('download.getUrlFailed'))
      return
    }
    const dirRes = await (window as any).api?.directorySettings?.getDirectories?.()
    const downloadDir = dirRes?.downloadDir || ''
    const safeName = `${song.name} - ${song.singer}`.replace(/[\/\\:*?"<>|]/g, '_')
    const filePath = downloadDir ? `${downloadDir}/${safeName}.mp3` : `${safeName}.mp3`
    await (window as any).api?.download?.addTask(
      { name: song.name, singer: song.singer, songmid: song.songmid, source: song.source, img: song.img, albumName: song.albumName },
      url,
      filePath,
      song.source,
      quality,
      1
    )
    MessagePlugin.success(i18n.global.t('download.addedDownload', { name: song.name }))
  } catch (e) {
    console.error('下载失败:', e)
    MessagePlugin.error(i18n.global.t('download.downloadFailed'))
  }
}
