export const MULTI_ARTIST_SEPARATOR = /\s*(?:[/／\\|、，,;；+＆&×]|\s+x\s+|\b(?:feat|featuring|ft|with)\.?\b)\s*/iu

export const DUPLICATE_DURATION_TOLERANCE_SECONDS = 3

export const normalizeSearchText = (value: unknown) => String(value ?? '')
  .toLowerCase()
  .normalize('NFKC')
  .replace(/&(?:amp|quot|apos|lt|gt);/g, '')
  .replace(/[\s\p{P}\p{S}]+/gu, '')

export const normalizeArtistText = (value: unknown) => String(value ?? '')
  .toLowerCase()
  .normalize('NFKC')
  .replace(/&(?:amp|quot|apos|lt|gt);/g, '')
  .split(MULTI_ARTIST_SEPARATOR)
  .map(part => normalizeSearchText(part))
  .filter(Boolean)
  .sort()
  .join('&')

export const parseDurationSeconds = (interval: unknown): number | null => {
  if (typeof interval === 'number') return Number.isFinite(interval) ? interval : null
  if (typeof interval !== 'string') return null

  const text = interval.trim()
  if (!text) return null
  if (/^\d+(?:\.\d+)?$/.test(text)) {
    const value = Number(text)
    return Number.isFinite(value) ? value : null
  }

  const parts = text.split(':').map(part => Number(part))
  if (parts.length < 2 || parts.length > 3 || parts.some(part => !Number.isFinite(part))) return null
  return parts.reduce((total, part) => total * 60 + part, 0)
}

export const unescapeHtml = (str: string) => str.replace(/&#(\d+);/g, (_, dec) => String.fromCharCode(dec))
