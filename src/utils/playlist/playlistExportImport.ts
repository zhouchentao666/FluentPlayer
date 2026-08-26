import type { SongList } from '@/types/audio'
import CryptoJS from 'crypto-js'
import i18n from '@/locales'

const SECRET_KEY = 'CeruMusic-PlaylistSecretKey'

export function encryptPlaylist(data: SongList[]): string {
  try {
    const jsonString = JSON.stringify(data)
    const encrypted = CryptoJS.AES.encrypt(jsonString, SECRET_KEY).toString()
    return encrypted
  } catch (error) {
    console.error('加密播放列表失败:', error)
    throw new Error(i18n.global.t('error.playlist.encrypt'))
  }
}

export function decryptPlaylist(encryptedData: string): SongList[] {
  try {
    const decrypted = CryptoJS.AES.decrypt(encryptedData, SECRET_KEY).toString(CryptoJS.enc.Utf8)
    return JSON.parse(decrypted) as SongList[]
  } catch (error) {
    console.error('解密播放列表失败:', error)
    throw new Error(i18n.global.t('error.playlist.decrypt'))
  }
}

async function gzipString(str: string): Promise<Blob> {
  const cs = new CompressionStream('gzip')
  const stream = new Blob([str]).stream().pipeThrough(cs)
  return await new Response(stream).blob()
}

async function gunzipToString(input: ArrayBuffer | Blob): Promise<string> {
  const ds = new DecompressionStream('gzip')
  const inStream = input instanceof Blob ? input.stream() : new Blob([input]).stream()
  const stream = inStream.pipeThrough(ds)
  return await new Response(stream).text()
}

function getExtFromPath(p: string): string {
  const m = p.match(/\.([^.]+)$/)
  return m ? m[1].toLowerCase() : ''
}

export async function importPlaylistFromPath(path: string): Promise<SongList[]> {
  const buf = await (window as any).api.file.readFile(path)
  const ext = getExtFromPath(path)
  if (ext === 'cpl') {
    const text = new TextDecoder().decode(buf)
    return decryptPlaylist(text)
  }
  if (ext === 'cmpl') {
    const text = await gunzipToString(buf)
    return decryptPlaylist(text)
  }
  throw new Error(i18n.global.t('error.playlist.unsupportedType'))
}

export async function exportPlaylistToFile(
  playlist: SongList[],
  customFileName?: string
): Promise<string> {
  try {
    if (!playlist || playlist.length === 0) {
      throw new Error(i18n.global.t('error.playlist.empty'))
    }

    const encryptedData = encryptPlaylist(playlist)
    const fileName =
      customFileName || `cerumusic-playlist-${new Date().toISOString().slice(0, 10)}.cmpl`

    const blob = await gzipString(encryptedData)

    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = fileName

    document.body.appendChild(link)
    link.click()

    setTimeout(() => {
      document.body.removeChild(link)
      URL.revokeObjectURL(url)
    }, 100)

    return fileName
  } catch (error) {
    console.error('导出播放列表失败:', error)
    throw error
  }
}

export async function copyPlaylistToClipboard(playlist: SongList[]): Promise<void> {
  try {
    if (!playlist || playlist.length === 0) {
      throw new Error(i18n.global.t('error.playlist.empty'))
    }

    const encryptedData = encryptPlaylist(playlist)
    await navigator.clipboard.writeText(encryptedData)
  } catch (error) {
    console.error('复制播放列表到剪贴板失败:', error)
    throw error
  }
}

export function importPlaylistFromFile(file: File): Promise<SongList[]> {
  return new Promise((resolve, reject) => {
    if (!file) {
      reject(new Error(i18n.global.t('error.playlist.noFile')))
      return
    }

    const isCompressed = file.name.endsWith('.cmpl')
    const isLegacy = file.name.endsWith('.cpl')
    if (!isCompressed && !isLegacy) {
      reject(new Error(i18n.global.t('error.playlist.wrongFormat')))
      return
    }

    const reader = new FileReader()

    reader.onload = async (event) => {
      try {
        if (!event.target || event.target.result == null) {
          throw new Error(i18n.global.t('error.playlist.readFailed'))
        }

        if (isLegacy) {
          if (typeof event.target.result !== 'string') throw new Error(i18n.global.t('error.playlist.readFailed'))
          const encryptedData = event.target.result
          const playlist = decryptPlaylist(encryptedData)
          resolve(playlist)
          return
        }

        const buf = event.target.result as ArrayBuffer
        const encryptedText = await gunzipToString(buf)
        const playlist = decryptPlaylist(encryptedText)
        resolve(playlist)
      } catch (error) {
        reject(error)
      }
    }

    reader.onerror = () => {
      reject(new Error(i18n.global.t('error.playlist.readFailed')))
    }

    if (isLegacy) {
      reader.readAsText(file)
    } else {
      reader.readAsArrayBuffer(file)
    }
  })
}

export async function importPlaylistFromClipboard(): Promise<SongList[]> {
  try {
    const clipboardText = await navigator.clipboard.readText()

    if (!clipboardText) {
      throw new Error(i18n.global.t('error.playlist.clipboardEmpty'))
    }

    return decryptPlaylist(clipboardText)
  } catch (error) {
    console.error('从剪贴板导入播放列表失败:', error)
    throw error
  }
}

export function validateImportedPlaylist(playlist: any[]): boolean {
  if (!Array.isArray(playlist) || playlist.length === 0) {
    return false
  }

  return playlist.every(
    (song) =>
      song.songmid &&
      song.name &&
      typeof song.img === 'string' &&
      typeof song.singer === 'string' &&
      typeof song.interval === 'string' &&
      typeof song.source === 'string'
  )
}
