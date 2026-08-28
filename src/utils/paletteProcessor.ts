import {
  angularDistance,
  clamp,
  lerp,
  normalizeHue,
  type BucketAccumulator,
  type HslColor,
  type PaletteCandidate,
} from './colorUtils'

export function createCandidate(bucket: BucketAccumulator): PaletteCandidate {
  const averageHue = normalizeHue((Math.atan2(bucket.hySum, bucket.hxSum) * 180) / Math.PI)
  const averageSaturation = bucket.sSum / bucket.count
  const averageLightness = bucket.lSum / bucket.count
  const midtoneAffinity = 1 - Math.min(1, Math.abs(averageLightness - 0.5) / 0.5) * 0.45
  const saturationWeight = 0.78 + averageSaturation * 1.4
  const neutralPenalty = averageSaturation < 0.12 ? 0.52 : 1
  const extremePenalty = averageLightness < 0.08 || averageLightness > 0.92 ? 0.3 : 1

  return {
    h: averageHue,
    s: averageSaturation,
    l: averageLightness,
    count: bucket.count,
    score: bucket.count * saturationWeight * midtoneAffinity * neutralPenalty * extremePenalty,
  }
}

export function polishColor(candidate: HslColor, role: number, colorBoost: number, depth: number): string {
  const hue = Math.round(normalizeHue(candidate.h))
  const saturation = candidate.s * 100
  const lightness = candidate.l * 100
  const boost = clamp(colorBoost / 100, 0, 1)
  const depthFactor = clamp(depth / 100, 0, 1)

  if (role === 0) {
    const refinedSaturation = saturation < 14
      ? lerp(18, 30, boost)
      : clamp(saturation * lerp(0.84, 1.02, boost) + lerp(8, 18, boost), 24, lerp(50, 68, boost))
    const refinedLightness = clamp(
      lightness * lerp(0.84, 0.58, depthFactor) + lerp(10, 4, depthFactor),
      lerp(32, 18, depthFactor),
      lerp(54, 38, depthFactor),
    )

    return `hsl(${hue}, ${Math.round(refinedSaturation)}%, ${Math.round(refinedLightness)}%)`
  }

  const refinedSaturation = saturation < 14
    ? lerp(24 + role * 6, 36 + role * 5, boost)
    : clamp(
      saturation * lerp(0.9, 1.08, boost) + lerp(12, 22, boost),
      lerp(34, 42, boost),
      lerp(66, 82, boost),
    )
  const refinedLightness = clamp(
    lightness * lerp(0.9, 0.7, depthFactor) + lerp(12 + role * 2, 8 + role, depthFactor),
    lerp(46, 34, depthFactor),
    lerp(72, 58, depthFactor),
  )

  return `hsl(${hue}, ${Math.round(refinedSaturation)}%, ${Math.round(refinedLightness)}%)`
}

export function createDerivedAccent(anchor: HslColor, role: number, colorBoost: number, depth: number): string {
  const hueShifts = [24, -28, 52, -58]
  const lightnessShifts = [10, 4, 14, 8]
  const shift = hueShifts[(role - 1) % hueShifts.length]
  const lightnessShift = lightnessShifts[(role - 1) % lightnessShifts.length]
  const saturationBase = anchor.s * 100
  const lightnessBase = anchor.l * 100
  const boost = clamp(colorBoost / 100, 0, 1)
  const depthFactor = clamp(depth / 100, 0, 1)

  const hue = Math.round(normalizeHue(anchor.h + shift))
  const saturation = saturationBase < 12
    ? lerp(30 + role * 4, 40 + role * 4, boost)
    : clamp(saturationBase * lerp(0.88, 1.02, boost) + lerp(16, 26, boost), 40, lerp(68, 84, boost))
  const lightness = clamp(
    lightnessBase * lerp(0.92, 0.74, depthFactor) + lerp(lightnessShift, lightnessShift - 6, depthFactor),
    lerp(48, 34, depthFactor),
    lerp(70, 56, depthFactor),
  )

  return `hsl(${hue}, ${Math.round(saturation)}%, ${Math.round(lightness)}%)`
}

export function selectPalette(candidates: PaletteCandidate[], count: number): HslColor[] {
  if (candidates.length === 0) return []

  const remaining = [...candidates].sort((a, b) => b.score - a.score)
  const selected: PaletteCandidate[] = [remaining.shift() as PaletteCandidate]

  while (selected.length < count && remaining.length > 0) {
    let bestIndex = 0
    let bestScore = -Infinity

    for (let index = 0; index < remaining.length; index += 1) {
      const candidate = remaining[index]
      const minGap = selected.reduce((closest, current) => {
        const hueGap = angularDistance(candidate.h, current.h) / 180
        const saturationGap = Math.abs(candidate.s - current.s)
        const lightnessGap = Math.abs(candidate.l - current.l)
        const distance = hueGap * 0.65 + saturationGap * 0.2 + lightnessGap * 0.15
        return Math.min(closest, distance)
      }, Number.POSITIVE_INFINITY)

      const diversifiedScore = candidate.score * (0.8 + minGap * 1.85)
      if (diversifiedScore > bestScore) {
        bestScore = diversifiedScore
        bestIndex = index
      }
    }

    selected.push(remaining.splice(bestIndex, 1)[0])
  }

  return selected.slice(0, count)
}
