export function formatDuration(seconds?: number | null): string {
  if (seconds == null || !isFinite(seconds) || seconds <= 0) return ""
  const s = Math.round(seconds)
  const m = Math.floor(s / 60)
  const r = s % 60
  return `${String(m).padStart(2, "0")}:${String(r).padStart(2, "0")}`
}

export function formatFileSize(bytes?: number | null): string {
  if (bytes == null || !isFinite(bytes) || bytes <= 0) return ""
  const units = ["B", "KB", "MB", "GB"]
  let v = bytes
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  return `${v.toFixed(i === 0 ? 0 : 1)} ${units[i]}`
}
