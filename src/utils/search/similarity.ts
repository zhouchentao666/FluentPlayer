/**
 * Levenshtein edit distance between two strings.
 * Ported from lx-music-desktop's similar() function.
 */
export const levenshtein = (a: string, b: string): number => {
  if (!a || !b) return Math.max(a.length, b.length)
  if (a.length > b.length) {
    const t = b
    b = a
    a = t
  }
  const al = a.length
  const bl = b.length
  const mp: number[] = new Array(bl + 1)
  for (let i = 0; i <= bl; i++) mp[i] = i
  for (let i = 1; i <= al; i++) {
    const ai = a.charAt(i - 1)
    let lt = mp[0]
    mp[0] = lt + 1
    for (let j = 1; j <= bl; j++) {
      const tmp = Math.min(mp[j] + 1, mp[j - 1] + 1, lt + (ai === b.charAt(j - 1) ? 0 : 1))
      lt = mp[j]
      mp[j] = tmp
    }
  }
  return mp[bl]
}

/**
 * Similarity score in [0, 1]. 1 = identical, 0 = completely different.
 * Uses Levenshtein distance normalized by max length.
 */
export const similarity = (a: string, b: string): number => {
  if (!a && !b) return 1
  if (!a || !b) return 0
  const maxLen = Math.max(a.length, b.length)
  if (maxLen === 0) return 1
  return 1 - levenshtein(a, b) / maxLen
}

/**
 * Sort items by similarity to a keyword using binary insertion sort.
 * Equivalent to lx-music's handleSortList + sortInsert pattern.
 * Most similar items come first.
 */
export const sortBySimilarity = <T>(
  items: T[],
  keyword: string,
  getText: (item: T) => string
): T[] => {
  if (!keyword || items.length <= 1) return items

  const sorted: Array<{ score: number; data: T }> = []
  for (const item of items) {
    const text = getText(item)
    const score = similarity(keyword.toLowerCase(), text.toLowerCase())
    const entry = { score, data: item }

    let left = 0
    let right = sorted.length - 1
    while (left <= right) {
      const mid = Math.trunc((left + right) / 2)
      if (score === sorted[mid].score) {
        left = mid
        break
      } else if (score < sorted[mid].score) {
        right = mid - 1
      } else {
        left = mid + 1
      }
    }
    sorted.splice(left, 0, entry)
  }

  return sorted.map(entry => entry.data)
}
