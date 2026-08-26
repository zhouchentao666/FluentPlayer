/**
 * Format a template string with song info placeholders.
 * %t = title, %s = singer, %a = album, %u = source, %d = date, %q = quality
 */
export function formatMusicInfo(template: string, data: Record<string, any>): string {
  const patterns: Record<string, string[]> = {
    '%t': ['name'],
    '%s': ['singer'],
    '%a': ['albumName'],
    '%u': ['source', 'platform'],
    '%d': ['date'],
    '%q': ['quality']
  }

  let result = template || '%t - %s'
  const d = new Date()
  const defaultDate = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`

  result = result.replace(/%[tsaudq]/g, (match) => {
    const keys = patterns[match]
    if (!keys) return match
    for (const key of keys) {
      if (data[key] !== undefined && data[key] !== null) return String(data[key])
    }
    if (match === '%d') return defaultDate
    return match
  })

  return result
}
