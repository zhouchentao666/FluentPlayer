import { createCandidate, createDerivedAccent, polishColor, selectPalette } from './paletteProcessor'
import { rgbToHsl, getBucketKey, type BucketAccumulator } from './colorUtils'

export const FALLBACK_PALETTE = [
  'hsl(220, 28%, 34%)',
  'hsl(196, 58%, 56%)',
  'hsl(340, 52%, 58%)',
  'hsl(42, 72%, 60%)',
]

const CANVAS_SIZE = 56
const SAMPLE_STEP = 2

export function createFallbackPalette(count: number): string[] {
  return FALLBACK_PALETTE.slice(0, count)
}

export async function extractDominantColors(
  imageUrl: string,
  count: number = 4,
): Promise<string[]> {
  return new Promise((resolve) => {
    const colorBoost = 56
    const depth = 58
    const image = new Image()
    let canvas: HTMLCanvasElement | null = null
    let settled = false

    const cleanup = () => {
      image.onload = null
      image.onerror = null
      image.src = ''
      if (canvas) {
        canvas.width = 0
        canvas.height = 0
        canvas = null
      }
    }

    const finish = (palette: string[]) => {
      if (settled) return
      settled = true
      cleanup()
      resolve(palette)
    }

    image.onload = () => {
      try {
        canvas = document.createElement('canvas')
        const context = canvas.getContext('2d', { willReadFrequently: true })
        if (!context) {
          finish(createFallbackPalette(count))
          return
        }

        canvas.width = CANVAS_SIZE
        canvas.height = CANVAS_SIZE
        context.drawImage(image, 0, 0, CANVAS_SIZE, CANVAS_SIZE)

        const imageData = context.getImageData(0, 0, CANVAS_SIZE, CANVAS_SIZE).data
        const buckets = new Map<string, BucketAccumulator>()

        for (let y = 0; y < CANVAS_SIZE; y += SAMPLE_STEP) {
          for (let x = 0; x < CANVAS_SIZE; x += SAMPLE_STEP) {
            const offset = (y * CANVAS_SIZE + x) * 4
            const alpha = imageData[offset + 3]
            if (alpha < 160) continue

            const red = imageData[offset]
            const green = imageData[offset + 1]
            const blue = imageData[offset + 2]
            const hsl = rgbToHsl(red, green, blue)

            if (hsl.l < 0.02 || hsl.l > 0.98) continue

            const key = getBucketKey(hsl)
            const bucket = buckets.get(key) ?? {
              count: 0,
              rSum: 0,
              gSum: 0,
              bSum: 0,
              sSum: 0,
              lSum: 0,
              hxSum: 0,
              hySum: 0,
            }

            bucket.count += 1
            bucket.rSum += red
            bucket.gSum += green
            bucket.bSum += blue
            bucket.sSum += hsl.s
            bucket.lSum += hsl.l
            bucket.hxSum += Math.cos((hsl.h * Math.PI) / 180)
            bucket.hySum += Math.sin((hsl.h * Math.PI) / 180)

            buckets.set(key, bucket)
          }
        }

        const candidates = [...buckets.values()]
          .map(createCandidate)
          .filter(candidate => candidate.count > 3)
          .sort((a, b) => b.score - a.score)

        if (candidates.length === 0) {
          finish(createFallbackPalette(count))
          return
        }

        const selected = selectPalette(candidates, count)
        const polished = selected.map((candidate, index) => polishColor(candidate, index, colorBoost, depth))
        const anchor = selected[0] ?? { h: 220, s: 0.35, l: 0.38 }

        while (polished.length < count) {
          polished.push(createDerivedAccent(anchor, polished.length, colorBoost, depth))
        }

        finish(polished.slice(0, count))
      } catch {
        finish(createFallbackPalette(count))
      }
    }

    image.onerror = () => {
      finish(createFallbackPalette(count))
    }

    image.crossOrigin = 'Anonymous'
    image.src = imageUrl
  })
}
