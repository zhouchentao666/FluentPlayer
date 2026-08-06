// 音频可视化频谱的后处理数学逻辑。
// 改编自 LyciaMusic：对原始频域样本做平滑、轻微随机化，并在暂停时产生柔和的空闲动画。
// 纯函数，无外部依赖，便于测试与复用。

// 对相邻样本做轻度平滑，避免频谱剧烈抖动。
export function getAudioVisualizerSamples(samples: number[]): number[] {
  if (samples.length === 0) return []

  // 平滑处理：每个点与其相邻点的加权平均值。
  const result = new Array(samples.length)
  for (let i = 0; i < samples.length; i++) {
    const prev = samples[Math.max(0, i - 1)]
    const cur = samples[i]
    const next = samples[Math.min(samples.length - 1, i + 1)]
    // 当前点权重最高，前后各 0.15，整体仍保持波形趋势。
    result[i] = cur * 0.7 + prev * 0.15 + next * 0.15
  }

  // 对静止区域（值接近 0）注入轻微随机扰动，让空闲不是死寂的直线。
  for (let i = 0; i < result.length; i++) {
    if (result[i] < 0.02) {
      result[i] = 0.02 + Math.random() * 0.03
    }
  }

  return result
}

// 当没有真实音频样本（未播放、或浏览器因 CORS 无法分析在线音频）时，
// 基于时间产生一个柔和起伏的伪频谱，保证界面始终有动感。
export function getIdleSamples(count: number, time: number): number[] {
  const result = new Array(count)
  for (let i = 0; i < count; i++) {
    const t = time / 1000
    // 多个正弦叠加，模拟呼吸般的低频律动。
    const wave =
      Math.sin(t * 1.6 + i * 0.35) * 0.5 +
      Math.sin(t * 0.9 - i * 0.2) * 0.3 +
      Math.sin(t * 2.3 + i * 0.1) * 0.2
    const base = 0.06 + (1 - Math.abs(i / count - 0.5) * 2) * 0.06
    result[i] = Math.max(0.02, base + (wave * 0.5 + 0.5) * 0.08)
  }
  return result
}

// 将 0..1 的强度映射为圆角条形的高度占比（已含最小可见高度）。
export function sampleToHeight(value: number): number {
  return Math.max(0.02, Math.min(1, value))
}
