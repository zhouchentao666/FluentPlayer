interface Color { r: number; g: number; b: number }
interface AnalysisResult { dominantColor: Color; useBlackText: boolean }

self.onmessage = (e: MessageEvent<{ requestId: number; pixels: Uint8ClampedArray }>) => {
  const { requestId, pixels } = e.data
  const colors: Color[] = []
  let totalLuminance = 0
  let pixelCount = 0

  for (let i = 0; i < pixels.length; i += 4) {
    if (pixels[i + 3] < 128) continue
    const r = pixels[i], g = pixels[i + 1], b = pixels[i + 2]
    colors.push({ r, g, b })
    const rs = r / 255, gs = g / 255, bs = b / 255
    const R = rs <= 0.03928 ? rs / 12.92 : Math.pow((rs + 0.055) / 1.055, 2.4)
    const G = gs <= 0.03928 ? gs / 12.92 : Math.pow((gs + 0.055) / 1.055, 2.4)
    const B = bs <= 0.03928 ? bs / 12.92 : Math.pow((bs + 0.055) / 1.055, 2.4)
    totalLuminance += 0.2126 * R + 0.7152 * G + 0.0722 * B
    pixelCount++
  }

  const dominantColors = kMeansCluster(colors, 5)
  const filtered = dominantColors.filter(c => !(c.r < 30 && c.g < 30 && c.b < 30) && !(c.r > 225 && c.g > 225 && c.b > 225))
  let dominantColor: Color
  if (filtered.length > 0) dominantColor = getMostSaturatedColor(filtered)
  else dominantColor = dominantColors.length > 0 ? dominantColors[0] : { r: 76, g: 116, b: 206 }
  dominantColor = enhanceColor(dominantColor)
  const avgL = pixelCount > 0 ? totalLuminance / pixelCount : 0.5

  self.postMessage({ requestId, dominantColor, useBlackText: avgL >= 0.6 })
}

function kMeansCluster(colors: Color[], k: number): Color[] {
  if (colors.length <= k) return colors
  const centroids: Color[] = []; const used = new Set<number>()
  while (centroids.length < k) { const i = Math.floor(Math.random() * colors.length); if (!used.has(i)) { used.add(i); centroids.push({ ...colors[i] }) } }
  const clusters: Color[][] = Array(k).fill(0).map(() => [])
  for (let iter = 0; iter < 10; iter++) {
    clusters.forEach(c => c.length = 0)
    for (const color of colors) { let minD = Infinity, idx = 0; for (let i = 0; i < centroids.length; i++) { const d = Math.sqrt((color.r - centroids[i].r) ** 2 + (color.g - centroids[i].g) ** 2 + (color.b - centroids[i].b) ** 2); if (d < minD) { minD = d; idx = i } } clusters[idx].push(color) }
    let changed = false
    for (let i = 0; i < k; i++) { if (!clusters[i].length) continue; const nc = { r: Math.round(clusters[i].reduce((s, c) => s + c.r, 0) / clusters[i].length), g: Math.round(clusters[i].reduce((s, c) => s + c.g, 0) / clusters[i].length), b: Math.round(clusters[i].reduce((s, c) => s + c.b, 0) / clusters[i].length) }; if (nc.r !== centroids[i].r || nc.g !== centroids[i].g || nc.b !== centroids[i].b) { centroids[i] = nc; changed = true } }
    if (!changed) break
  }
  return centroids.map((c, i) => ({ centroid: c, size: clusters[i].length })).sort((a, b) => b.size - a.size).map(x => x.centroid)
}

function getMostSaturatedColor(colors: Color[]): Color {
  let maxS = -1, best = colors[0]
  for (const c of colors) { const { s } = rgbToHsl(c.r, c.g, c.b); if (s > maxS) { maxS = s; best = c } }
  return best
}

function rgbToHsl(r: number, g: number, b: number): { h: number; s: number; l: number } {
  r /= 255; g /= 255; b /= 255; const max = Math.max(r, g, b), min = Math.min(r, g, b); let h = 0, s = 0; const l = (max + min) / 2
  if (max !== min) { const d = max - min; s = l > 0.5 ? d / (2 - max - min) : d / (max + min); switch (max) { case r: h = (g - b) / d + (g < b ? 6 : 0); break; case g: h = (b - r) / d + 2; break; case b: h = (r - g) / d + 4; break } h /= 6 }
  return { h, s, l }
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  if (s === 0) return [Math.round(l * 255), Math.round(l * 255), Math.round(l * 255)]
  const hue2rgb = (p: number, q: number, t: number) => { if (t < 0) t += 1; if (t > 1) t -= 1; if (t < 1 / 6) return p + (q - p) * 6 * t; if (t < 1 / 2) return q; if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6; return p }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s, p = 2 * l - q
  return [Math.round(hue2rgb(p, q, h + 1 / 3) * 255), Math.round(hue2rgb(p, q, h) * 255), Math.round(hue2rgb(p, q, h - 1 / 3) * 255)]
}

function enhanceColor(color: Color): Color {
  const { h, s, l } = rgbToHsl(color.r, color.g, color.b)
  const es = Math.min(s * 1.2, 0.9), el = l < 0.3 ? Math.min(l * 1.5, 0.5) : l > 0.7 ? Math.max(l * 0.8, 0.5) : l
  const [r, g, b] = hslToRgb(h, es, el); return { r, g, b }
}
