import { httpFetch } from '@online/lib/http'
import type { Song } from '../../types'

// 听歌识曲：参考 CeruMusic 的识别流程（网易云 song_detect）。
// 1. 用 afp.js(Audio Fingerprint 库) 对 8kHz 单声道 PCM 生成指纹
// 2. 调网易云 api/music/audio/match 匹配
// 3. 用 api/song/music/detail/get 取详细信息

const TARGET_RATE = 8000

// afp.js 通过 <script> 注入后暴露全局 GenerateFP
declare global {
  interface Window {
    GenerateFP?: (float32: Float32Array<ArrayBufferLike>) => Promise<string>
    WASM_BINARY?: string
  }
}

let afpLoading: Promise<void> | null = null

/**
 * 在浏览器(Tauri)环境加载 afp 指纹库。
 * CeruMusic 在 Electron 下靠 Node require('./afp.wasm.js') 加载 wasm 二进制，
 * 但浏览器没有 require，这里用 Function 注入方式把 WASM_BINARY 提取到全局，
 * 并提供一个 require shim，使 afp.js 内部的 require('./afp.wasm.js') 能取到。
 */
export async function ensureAfpLoaded(): Promise<void> {
  if (window.GenerateFP) return
  if (afpLoading) return afpLoading

  afpLoading = (async () => {
    if (!window.WASM_BINARY) {
      try {
        const wasmText = await (await fetch('/afp.wasm.js')).text()
        const mod: { exports: { WASM_BINARY?: string } } = { exports: {} }
        // afp.wasm.js 是 'use strict'; const WASM_BINARY = "base64..."; module.exports = {...}
        // 用 Function 包裹执行并取出 WASM_BINARY
        // eslint-disable-next-line @typescript-eslint/no-implied-eval
        new Function('module', 'exports', wasmText)(mod, mod.exports)
        const bin = mod.exports.WASM_BINARY ?? (window as unknown as { WASM_BINARY?: string }).WASM_BINARY
        if (bin) window.WASM_BINARY = bin
      } catch {
        // 忽略，下次重试
      }
    }
    // 提供 require shim（Electron 环境靠 Node require 加载 wasm，浏览器用此 shim 替代）
    ;(window as unknown as { require: (id: string) => { WASM_BINARY?: string } }).require = () => ({
      WASM_BINARY: window.WASM_BINARY,
    })
    await loadScript('/afp.js')
  })()

  return afpLoading
}

function loadScript(src: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const existing = document.querySelector(`script[src="${src}"]`)
    if (existing) {
      resolve()
      return
    }
    const s = document.createElement('script')
    s.src = src
    s.onload = () => resolve()
    s.onerror = () => reject(new Error(`加载 ${src} 失败`))
    document.head.appendChild(s)
  })
}

/** 同步线性重采样到 8k（当 OfflineAudioContext 不可用时兜底） */
function linearResampleTo8k(mono: Float32Array, fromRate: number): Float32Array {
  const ratio = fromRate / TARGET_RATE
  const outLen = Math.max(1, Math.floor(mono.length / ratio))
  const out = new Float32Array(outLen)
  for (let i = 0; i < outLen; i++) {
    const idx = Math.floor(i * ratio)
    out[i] = mono[idx] ?? 0
  }
  return out
}

function mergeChannels(buffer: AudioBuffer): Float32Array {
  const len = buffer.length
  const out = new Float32Array(len)
  for (let ch = 0; ch < buffer.numberOfChannels; ch++) {
    const data = buffer.getChannelData(ch)
    for (let i = 0; i < len; i++) out[i] += data[i]
  }
  for (let i = 0; i < len; i++) out[i] /= buffer.numberOfChannels
  return out
}

/** 生成音频指纹 */
export async function generateFingerprint(audioBuffer: AudioBuffer): Promise<string> {
  await ensureAfpLoaded()
  if (!window.GenerateFP) throw new Error('指纹库未就绪')
  const mono = audioBuffer.numberOfChannels > 1 ? mergeChannels(audioBuffer) : audioBuffer.getChannelData(0)
  const resampled = audioBuffer.sampleRate === TARGET_RATE
    ? mono
    : linearResampleTo8k(mono, audioBuffer.sampleRate)
  return window.GenerateFP(resampled as unknown as Float32Array)
}

export interface RecognizeCandidate {
  song: Song
  score: number
}

/**
 * 调用网易云听歌识曲接口。
 * @param fingerprint afp 生成的指纹 base64
 * @param durationSec 音频时长(秒)
 */
export async function recognizeByFingerprint(
  fingerprint: string,
  durationSec: number,
): Promise<RecognizeCandidate[]> {
  const sessionId = String(Math.floor(Math.random() * 1e15))
  const params = new URLSearchParams({
    sessionId,
    algorithmCode: 'shazam_v2',
    duration: String(Math.round(durationSec)),
    rawdata: fingerprint,
    times: '1',
    decrypt: '1',
  })
  const url = `https://interface.music.163.com/api/music/audio/match?${params.toString()}`
  const res = await httpFetch(url, {
    method: 'POST',
    headers: {
      'User-Agent':
        'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36',
      origin: 'https://music.163.com',
      referer: 'https://music.163.com/',
    },
  })
  if (!res.ok) throw new Error(`识别请求失败: ${res.status}`)
  const body = (await res.json()) as { data?: { result?: Array<{ score?: number; song?: { id: number | string } }> } }
  const items = body.data?.result ?? []
  if (items.length === 0) return []

  const candidates: RecognizeCandidate[] = []
  for (const item of items) {
    const songId = item.song?.id
    if (!songId) continue
    try {
      const detail = await fetchSongDetail(String(songId))
      candidates.push({ song: detail, score: item.score ?? 0 })
    } catch {
      // 跳过单条失败
    }
  }
  return candidates
}

/** 取歌曲详情 */
async function fetchSongDetail(songId: string): Promise<Song> {
  const url = `https://music.163.com/api/song/music/detail/get?songId=${encodeURIComponent(songId)}`
  const res = await httpFetch(url, {
    method: 'GET',
    headers: {
      'User-Agent':
        'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36',
      origin: 'https://music.163.com',
      referer: 'https://music.163.com/',
    },
  })
  if (!res.ok) throw new Error(`详情请求失败: ${res.status}`)
  const body = (await res.json()) as {
    data?: {
      song?: {
        id?: number | string
        name?: string
        alias?: string[]
        artists?: Array<{ name?: string }>
        album?: { name?: string; picUrl?: string }
        duration?: number
        fee?: number
      }
    }
  }
  const s = body.data?.song
  if (!s) throw new Error('无歌曲详情')
  // 复用 musicInfoToSong 需要一个 MusicInfo；这里直接构造最小 Song
  const song: Song = {
    id: `wy_${s.id}`,
    title: s.name ?? '未知',
    path: '',
    cover: s.album?.picUrl ?? undefined,
    metadata: {
      title: s.name ?? '',
      artist: (s.artists ?? []).map((a) => a.name ?? '').filter(Boolean).join('、'),
      album: s.album?.name ?? '',
      duration: (s.duration ?? 0) / 1000,
      bitrate: 0,
      year: '',
      genre: '',
    },
    online: {
      source: 'wy',
      id: String(s.id),
      name: s.name ?? '',
      singer: (s.artists ?? []).map((a) => a.name ?? '').filter(Boolean).join('、'),
      albumName: s.album?.name ?? '',
      interval: String((s.duration ?? 0) / 1000),
      meta: {
        songId: String(s.id),
        albumId: '',
        picUrl: s.album?.picUrl ?? null,
        qualitys: [],
        _qualitys: {},
      },
    } as Song['online'],
  }
  return song
}
