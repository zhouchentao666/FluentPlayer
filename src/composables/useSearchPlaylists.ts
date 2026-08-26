import { ref, type Ref } from 'vue'
import { musicSdk } from '@/services/musicSdk'
import {
  dedupeAndSortPlaylists,
  mapPlaylistItem,
  type PlaylistCardItem,
} from '@/utils/search/deduplicate'
import { useSourceAccess } from '@/composables/useSourceAccess'

const PLAYLIST_PAGE_SIZE = 30
const AGGREGATE_PLAYLIST_PAGE_SIZE = 20

export type { PlaylistCardItem }

export function useSearchPlaylists() {
  const results = ref<PlaylistCardItem[]>([])
  const loading = ref(false)
  const searched = ref(false)
  const currentPage = ref(1)
  const total = ref(0)

  let requestId = 0

  const fetch = async (
    keyword: string,
    source: string,
    sourceKeys: string[],
    sourceOrderMap?: Map<string, number>,
    reset = false
  ) => {
    const query = keyword.trim()
    if (!query) return

    if (reset) {
      requestId += 1
      currentPage.value = 1
      results.value = []
      total.value = 0
      loading.value = false
      searched.value = false
    }
    if (loading.value) return

    const currentRequestId = ++requestId
    const page = currentPage.value
    loading.value = true

    try {
      if (source === 'all') {
        const { enabledSourceKeys } = useSourceAccess()
        const filteredSourceKeys = sourceKeys.filter(k => enabledSourceKeys.value.has(k))
        if (filteredSourceKeys.length === 0) {
          if (currentRequestId === requestId) {
            results.value = []
            total.value = 0
            searched.value = true
          }
          return
        }

        const rawResults = await musicSdk.aggregateSearchPlaylists(query, AGGREGATE_PLAYLIST_PAGE_SIZE, filteredSourceKeys)
        if (currentRequestId !== requestId) return

        const list = dedupeAndSortPlaylists(rawResults, query, sourceOrderMap)
        results.value = list
        total.value = list.length
        searched.value = true
        return
      }

      const res = await musicSdk.searchPlaylist(query, page, PLAYLIST_PAGE_SIZE, source)
      if (currentRequestId !== requestId) return

      total.value = res?.total || 0
      const list = Array.isArray(res?.list) ? res.list : []
      const mapped = list.map(item => mapPlaylistItem(item, res?.source || source))
      results.value = reset ? mapped : [...results.value, ...mapped]
      currentPage.value = page + 1
      searched.value = true
    } catch (e) {
      if (currentRequestId === requestId) console.error('歌单搜索失败:', e)
    } finally {
      if (currentRequestId === requestId) {
        loading.value = false
        if (currentRequestId === requestId) searched.value = true
      }
    }
  }

  const resetState = () => {
    requestId += 1
    results.value = []
    currentPage.value = 1
    total.value = 0
    loading.value = false
    searched.value = false
  }

  return {
    results,
    loading,
    searched,
    currentPage,
    total,
    fetch,
    reset: resetState,
  }
}
