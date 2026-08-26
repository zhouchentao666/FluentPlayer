import { ref, type Ref } from 'vue'
import { musicSdk, type MusicItem, type SearchResult } from '@/services/musicSdk'
import { dedupeAndSortSongs } from '@/utils/search/deduplicate'
import { useSourceAccess } from '@/composables/useSourceAccess'

const AGGREGATE_PAGE_SIZE = 20
const SINGLE_PAGE_SIZE = 30

interface SourcePageInfo {
  maxPage: number
  total: number
  fetchedPage: number
}

export function useSearchSongs() {
  const results = ref<MusicItem[]>([])
  const aggregateResults = ref<MusicItem[]>([])
  const loading = ref(false)
  const aggregateLoading = ref(false)
  const hasMore = ref(true)
  const aggregateHasMore = ref(true)
  const currentPage = ref(1)
  const totalItems = ref(0)
  const aggregateSearched = ref(false)

  let searchRequestId = 0
  let aggregateRequestId = 0

  const sourcePageInfos = new Map<string, SourcePageInfo>()

  const isCurrentSearch = (requestId: number, query: string, source: string | undefined) => (
    requestId === searchRequestId &&
    source !== undefined
  )

  const isCurrentAggregate = (requestId: number, query: string) => (
    requestId === aggregateRequestId
  )

  const fetchPage = async (keyword: string, source: string, reset = false) => {
    if (!keyword.trim()) return

    if (reset) {
      searchRequestId += 1
      currentPage.value = 1
      results.value = []
      totalItems.value = 0
      hasMore.value = true
      loading.value = false
    }
    if (loading.value || !hasMore.value) return

    const requestId = ++searchRequestId
    const page = currentPage.value
    loading.value = true

    try {
      const result = await musicSdk.search(keyword, page, SINGLE_PAGE_SIZE, source)
      if (requestId !== searchRequestId) return

      totalItems.value = result.total || 0
      const newSongs = (result.list || []).map((song, i) => ({
        ...song, id: song.songmid || `${page}-${i}`
      }))
      results.value = reset ? newSongs : [...results.value, ...newSongs]
      currentPage.value = page + 1
      hasMore.value = results.value.length < totalItems.value
    } catch (e) {
      if (requestId === searchRequestId) console.error('搜索失败:', e)
    } finally {
      if (requestId === searchRequestId) loading.value = false
    }
  }

  const fetchAggregate = async (keyword: string) => {
    const query = keyword.trim()
    if (!query) return

    const requestId = ++aggregateRequestId
    aggregateLoading.value = true
    aggregateSearched.value = false
    aggregateResults.value = []
    sourcePageInfos.clear()

    try {
      const results = await musicSdk.aggregateSearch(query, AGGREGATE_PAGE_SIZE)
      if (!isCurrentAggregate(requestId, query)) return

      // Track per-source pagination info
      for (const result of results) {
        if (result?.source) {
          sourcePageInfos.set(result.source, {
            maxPage: result.allPage || 1,
            total: result.total || 0,
            fetchedPage: 1,
          })
        }
      }

      const { filterByEnabledSources } = useSourceAccess()
      aggregateResults.value = filterByEnabledSources(dedupeAndSortSongs(results, query))
      aggregateHasMore.value = Array.from(sourcePageInfos.values()).some(info => info.fetchedPage < info.maxPage)
    } catch (e) {
      if (isCurrentAggregate(requestId, query)) console.error('聚合搜索失败:', e)
    } finally {
      if (requestId === aggregateRequestId) {
        aggregateLoading.value = false
        if (isCurrentAggregate(requestId, query)) aggregateSearched.value = true
      }
    }
  }

  /**
   * Fetch next aggregate page by querying each non-exhausted source individually.
   * lx-music pattern: skip sources where fetchedPage >= maxPage.
   */
  const fetchAggregateNextPage = async (keyword: string) => {
    const query = keyword.trim()
    if (!query || aggregateLoading.value || !aggregateHasMore.value) return

    const activeSources = Array.from(sourcePageInfos.entries())
      .filter(([, info]) => info.fetchedPage < info.maxPage)
      .map(([source, info]) => ({ source, nextPage: info.fetchedPage + 1 }))

    if (activeSources.length === 0) {
      aggregateHasMore.value = false
      return
    }

    const requestId = ++aggregateRequestId
    aggregateLoading.value = true

    try {
      const settled = await Promise.allSettled(
        activeSources.map(({ source, nextPage }) =>
          musicSdk.search(query, nextPage, AGGREGATE_PAGE_SIZE, source)
        )
      )

      if (!isCurrentAggregate(requestId, query)) return

      const newResults: SearchResult[] = []
      settled.forEach((result, index) => {
        const { source, nextPage } = activeSources[index]
        const info = sourcePageInfos.get(source)
        if (!info) return

        if (result.status === 'fulfilled' && Array.isArray(result.value?.list)) {
          info.fetchedPage = nextPage
          if (result.value.allPage !== undefined) info.maxPage = result.value.allPage
          if (result.value.list.length > 0) {
            newResults.push({ ...result.value, source })
          }
        } else {
          if (result.status === 'rejected') {
            console.warn(`[aggregatePage] source '${source}' page ${nextPage} failed:`, result.reason)
          }
          info.maxPage = nextPage - 1 // mark as exhausted
        }
      })

      if (newResults.length > 0) {
        // Merge with existing results and re-deduplicate
        const existingBySource = new Map<string, MusicItem[]>()
        for (const song of aggregateResults.value) {
          const list = existingBySource.get(song.source) || []
          list.push(song)
          existingBySource.set(song.source, list)
        }

        for (const result of newResults) {
          const list = existingBySource.get(result.source) || []
          list.push(...result.list)
          existingBySource.set(result.source, list)
        }

        const mergedResults = Array.from(existingBySource.entries()).map(([source, list]) => ({
          source,
          list,
        }))

        const { filterByEnabledSources } = useSourceAccess()
        aggregateResults.value = filterByEnabledSources(dedupeAndSortSongs(mergedResults, query))
      }

      aggregateHasMore.value = Array.from(sourcePageInfos.values()).some(info => info.fetchedPage < info.maxPage)
    } catch (e) {
      if (isCurrentAggregate(requestId, query)) console.error('聚合翻页失败:', e)
    } finally {
      if (requestId === aggregateRequestId) aggregateLoading.value = false
    }
  }

  const reset = () => {
    searchRequestId += 1
    aggregateRequestId += 1
    results.value = []
    aggregateResults.value = []
    currentPage.value = 1
    totalItems.value = 0
    hasMore.value = true
    aggregateHasMore.value = true
    loading.value = false
    aggregateLoading.value = false
    aggregateSearched.value = false
    sourcePageInfos.clear()
  }

  return {
    results,
    aggregateResults,
    loading,
    aggregateLoading,
    hasMore,
    aggregateHasMore,
    currentPage,
    totalItems,
    aggregateSearched,
    fetchPage,
    fetchAggregate,
    fetchAggregateNextPage,
    reset,
  }
}
