/**
 * 听歌识曲（music recognition）
 *
 * 参考 Mio-Music-main 的 AudioMatch 实现：用浏览器麦克风录音，重采样到 8kHz，
 * 送入网易云音频指纹 WASM（public/afp.js + public/afp.wasm.js）生成指纹，
 * 再 POST 到网易云 audio/match 接口识曲。
 *
 * 与原版差异：
 * - 原版（Mio）用 Rust 后端捕获麦克风 PCM；本版（FluentPlayer）直接在前端用
 *   Web Audio getUserMedia 采集并重采样，逻辑全部在浏览器侧完成。
 * - 网络请求用 Tauri 的 @tauri-apps/plugin-http fetch（见 @online/lib/http），
 *   不受浏览器同源限制，可直接打到 interface.music.163.com。
 */

import { httpFetch as tauriFetch } from "@online/lib/http"
import type { MusicInfo, MusicQuality, Quality } from "@online/types/music"
import { formatDuration } from "@online/lib/utils"
import { indexQualitySizes } from "@online/lib/quality"

const TARGET_RATE = 8000
// 一次录音长度（秒）。太短指纹不足，太长接口会拒绝，网易云约 3s 最佳。
const CLIP_SECONDS = 3

declare global {
  interface Window {
    // afp.js 的 GenerateFP 是异步的：返回 Promise<base64string>
    GenerateFP?: (pcm: Float32Array) => Promise<string | null>
  }
}

// ---------------------------------------------------------------------------
// 指纹 WASM 动态加载
// ---------------------------------------------------------------------------

let afpPromise: Promise<void> | null = null

function loadScript(src: string, async = true): Promise<void> {
  return new Promise((resolve, reject) => {
    const existing = document.querySelector(`script[src="${src}"]`)
    if (existing) {
      if ((existing as HTMLScriptElement).dataset.loaded === "1") resolve()
      else (existing as HTMLScriptElement).addEventListener("load", () => resolve())
      return
    }
    const script = document.createElement("script")
    script.src = src
    script.async = async
    script.onload = () => {
      script.dataset.loaded = "1"
      resolve()
    }
    script.onerror = () => reject(new Error(`音频指纹脚本加载失败：${src}`))
    document.head.appendChild(script)
  })
}

/**
 * 动态加载指纹模块。注意顺序：必须先加载 afp.wasm.js（它把 WASM 二进制挂到
 * globalThis.WASM_BINARY），再加载 afp.js（其内部 const Module = ... 会读取
 * 全局 WASM_BINARY 来实例化，避免浏览器下走 require 分支报错）。
 * 两个脚本都用同步（非 async）加载，保证 wasm 先于 afp 执行完成。
 * afp.js 末尾会把 GenerateFP 挂到 window。
 */
export function ensureAFP(): Promise<void> {
  if (typeof window !== "undefined" && window.GenerateFP) return Promise.resolve()
  if (afpPromise) return afpPromise
  afpPromise = (async () => {
    await loadScript("/afp.wasm.js", false)
    await loadScript("/afp.js", false)
    if (!window.GenerateFP) throw new Error("音频指纹模块加载失败（GenerateFP 未就绪）")
  })().catch((err) => {
    afpPromise = null
    throw err
  })
  return afpPromise
}

// ---------------------------------------------------------------------------
// 麦克风录音 + 重采样到 8kHz
// ---------------------------------------------------------------------------

export interface RecordHandle {
  /** 停止录音（若仍在录）并释放麦克风。 */
  stop: () => void
}

/**
 * 录制一段 CLIP_SECONDS 秒的音频，返回重采样到 8kHz 的 Float32 PCM（单声道）。
 * onStop 在自动/手动停止后回调。返回的 Promise 在录音结束时 resolve 出 PCM。
 */
export function recordClip(
  seconds = CLIP_SECONDS,
  onStop?: () => void,
): { pcm: Promise<Float32Array>; handle: RecordHandle } {
  let stream: MediaStream | null = null
  let source: AudioNode | null = null
  let processor: ScriptProcessorNode | null = null
  let audioCtx: AudioContext | null = null
  let stopped = false

  const cleanup = () => {
    if (processor) {
      try {
        processor.disconnect()
      } catch {
        /* ignore */
      }
    }
    if (source) {
      try {
        source.disconnect()
      } catch {
        /* ignore */
      }
    }
    if (stream) stream.getTracks().forEach((t) => t.stop())
    if (audioCtx && audioCtx.state !== "closed") {
      audioCtx.close().catch(() => {})
    }
  }

  const pcm = new Promise<Float32Array>((resolve, reject) => {
    if (typeof navigator === "undefined" || !navigator.mediaDevices?.getUserMedia) {
      reject(new Error("当前环境不支持麦克风采集"))
      return
    }
    navigator.mediaDevices
      .getUserMedia({ audio: true })
      .then((s) => {
        stream = s
        const Ctx = window.AudioContext || (window as any).webkitAudioContext
        audioCtx = new Ctx()
        const ctxRate = audioCtx.sampleRate
        source = audioCtx.createMediaStreamSource(s)

        // 用 ScriptProcessor 分段采集（兼容性好，无需 AudioWorklet 线程文件）
        const bufferSize = 4096
        processor = audioCtx.createScriptProcessor(bufferSize, 1, 1)

        const totalSamples = Math.floor(TARGET_RATE * seconds)
        const out = new Float32Array(totalSamples)
        let written = 0
        let started = false
        let startStamp = 0
        const ratio = TARGET_RATE / ctxRate

        const finish = () => {
          if (stopped) return
          stopped = true
          cleanup()
          onStop?.()
          resolve(out.subarray(0, Math.max(written, 1)))
        }

        const stopInner = () => finish()

        processor.onaudioprocess = (e: AudioProcessingEvent) => {
          if (stopped) return
          if (!started) {
            started = true
            startStamp = e.playbackTime
          }
          const input = e.inputBuffer.getChannelData(0)
          for (let i = 0; i < input.length && written < totalSamples; i++) {
            // 简单整数下标重采样（8kHz 目标，源通常 44.1/48kHz）
            const srcIdx = Math.floor(i * ratio)
            if (srcIdx < input.length) out[written++] = input[srcIdx]
          }
          if (written >= totalSamples) stopInner()
        }

        ;(processor as any)._stop = stopInner
        source.connect(processor)
        processor.connect(audioCtx.destination)
      })
      .catch((err) => {
        cleanup()
        reject(new Error("麦克风打开失败：" + (err?.message || err)))
      })
  })

  const handle: RecordHandle = {
    stop: () => {
      if (stopped) return
      stopped = true
      cleanup()
      onStop?.()
    },
  }

  return { pcm, handle }
}

/** 生成音频指纹（base64）。需先 ensureAFP()。GenerateFP 本身是异步的。 */
export async function generateFingerprint(pcm: Float32Array): Promise<string> {
  if (!window.GenerateFP) throw new Error("音频指纹模块未加载")
  const fp = await window.GenerateFP(pcm)
  return fp || ""
}

// ---------------------------------------------------------------------------
// 网易云 audio/match 识曲
// ---------------------------------------------------------------------------

interface WyMatchSongRaw {
  id?: number | string
  name?: string
  dt?: number
  ar?: { name?: string }[]
  al?: { id?: number | string; name?: string; picUrl?: string }
  privilege?: { maxBrLevel?: string; maxbr?: number }
  hr?: { size?: number }
  sq?: { size?: number }
  h?: { size?: number }
  l?: { size?: number }
}

interface WyMatchResponse {
  code?: number
  data?: { result?: WyMatchSongRaw[] }
}

const QUALITY_ORDER: Quality[] = ["flac24bit", "flac", "320k", "128k"]

function sizeFormate(size: number): string {
  if (!size) return "0 B"
  const units = ["B", "KB", "MB", "GB", "TB"]
  const number = Math.floor(Math.log(size) / Math.log(1024))
  return `${(size / Math.pow(1024, Math.floor(number))).toFixed(2)} ${units[number]}`
}

function normalizeMatchSong(item: WyMatchSongRaw): MusicInfo {
  const qualitys: MusicQuality[] = []
  const privilege = item.privilege ?? {}
  const maxbr = privilege.maxbr ?? 0
  if (privilege.maxBrLevel === "hires") {
    qualitys.push({ type: "flac24bit", size: item.hr ? sizeFormate(item.hr.size ?? 0) : null })
  }
  if (maxbr >= 999000) qualitys.push({ type: "flac", size: item.sq ? sizeFormate(item.sq.size ?? 0) : null })
  if (maxbr >= 320000) qualitys.push({ type: "320k", size: item.h ? sizeFormate(item.h.size ?? 0) : null })
  if (maxbr >= 128000) qualitys.push({ type: "128k", size: item.l ? sizeFormate(item.l.size ?? 0) : null })
  if (qualitys.length === 0) qualitys.push({ type: "128k", size: null })

  const byType = new Map<Quality, MusicQuality>()
  for (const q of qualitys) if (!byType.has(q.type)) byType.set(q.type, q)
  const ordered = QUALITY_ORDER.filter((t) => byType.has(t))
    .map((t) => byType.get(t)!)
    .reverse()

  const songId = String(item.id)
  const singers = Array.isArray(item.ar)
    ? item.ar.map((s) => s.name).filter(Boolean).join("、")
    : ""

  return {
    id: `wy_${songId}`,
    name: item.name ?? "",
    singer: singers,
    source: "wy",
    interval: formatDuration((item.dt ?? 0) / 1000),
    albumName: item.al?.name ?? "",
    meta: {
      songId,
      albumId: item.al?.id != null ? String(item.al.id) : "",
      picUrl: item.al?.picUrl ?? null,
      qualitys: ordered,
      _qualitys: indexQualitySizes(ordered),
    },
  }
}

/**
 * 完整识曲流程：确保指纹模块 → 录音 → 生成指纹 → 请求 audio/match。
 * 返回识别到的歌曲列表（网易云）。空数组表示未识别。
 */
export async function recognizeSong(
  onRecordingStop?: () => void,
): Promise<{ songs: MusicInfo[]; fingerprint?: string }> {
  await ensureAFP()
  const { pcm, handle } = recordClip(CLIP_SECONDS, onRecordingStop)
  try {
    const samples = await pcm
    const fp = await generateFingerprint(samples)
    if (!fp) throw new Error("音频指纹生成失败")

    const params = new URLSearchParams({
      sessionId: String(Date.now()),
      algorithmCode: "shazam_v2",
      duration: String(CLIP_SECONDS),
      rawdata: fp,
      times: "1",
      decrypt: "1",
    })
    const url = `https://interface.music.163.com/api/music/audio/match?${params.toString()}`

    const res = await tauriFetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        "User-Agent":
          "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36",
        origin: "https://music.163.com",
      },
      body: "",
    })

    if (!res.ok) throw new Error(`识曲请求失败：${res.status}`)
    const data = (await res.json()) as WyMatchResponse
    if (!data || data.code !== 200) throw new Error("识曲失败：接口返回异常")

    const list = (data.data?.result ?? []).map(normalizeMatchSong)
    return { songs: list, fingerprint: fp }
  } finally {
    // 超时/异常兜底停止
    handle.stop()
  }
}
