export interface HslColor {
  h: number
  s: number
  l: number
}

export interface BucketAccumulator {
  count: number
  rSum: number
  gSum: number
  bSum: number
  sSum: number
  lSum: number
  hxSum: number
  hySum: number
}

export interface PaletteCandidate extends HslColor {
  count: number
  score: number
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

export function lerp(start: number, end: number, amount: number): number {
  return start + (end - start) * amount
}

export function normalizeHue(hue: number): number {
  const normalized = hue % 360
  return normalized < 0 ? normalized + 360 : normalized
}

export function angularDistance(a: number, b: number): number {
  const diff = Math.abs(normalizeHue(a) - normalizeHue(b))
  return Math.min(diff, 360 - diff)
}

export function rgbToHsl(r: number, g: number, b: number): HslColor {
  const rNorm = r / 255
  const gNorm = g / 255
  const bNorm = b / 255
  const max = Math.max(rNorm, gNorm, bNorm)
  const min = Math.min(rNorm, gNorm, bNorm)
  const lightness = (max + min) / 2

  if (max === min) {
    return { h: 0, s: 0, l: lightness }
  }

  const delta = max - min
  const saturation = lightness > 0.5
    ? delta / (2 - max - min)
    : delta / (max + min)

  let hue = 0
  if (max === rNorm) {
    hue = (gNorm - bNorm) / delta + (gNorm < bNorm ? 6 : 0)
  } else if (max === gNorm) {
    hue = (bNorm - rNorm) / delta + 2
  } else {
    hue = (rNorm - gNorm) / delta + 4
  }

  return {
    h: normalizeHue((hue / 6) * 360),
    s: saturation,
    l: lightness,
  }
}

export function getBucketKey(color: HslColor): string {
  const lightBucket = Math.round(color.l * 4)

  if (color.s < 0.12) {
    return `neutral-${lightBucket}`
  }

  const hueBucket = Math.round(normalizeHue(color.h) / 18)
  const satBucket = Math.round(color.s * 5)
  return `${hueBucket}-${satBucket}-${lightBucket}`
}
