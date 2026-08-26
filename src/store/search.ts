import { defineStore } from 'pinia'

export interface SearchResult {
  songs: any[]
  total: number
  page: number
  limit: number
  source: string
}

export const searchValue = defineStore('search', {
  state: () => ({
    value: '',
    focus: false,
    results: null as SearchResult | null,
    loading: false,
    history: JSON.parse(localStorage.getItem('searchHistory') || '[]') as string[],
    page: 1
  }),
  getters: {
    getValue: (state) => state.value,
    getFocus: (state) => state.focus
  },
  actions: {
    setValue(value: string) {
      this.value = value
    },
    setFocus(focus: boolean) {
      this.focus = focus
    },
    async search(keyword?: string, page = 1) {
      const query = keyword ?? this.value
      if (!query?.trim()) return
      this.loading = true
      this.page = page
      try {
        const res = await (window as any).api?.music?.requestSdk?.('search', {
          keyword: query,
          page,
          limit: 30
        })
        if (res) {
          this.results = {
            songs: res.songs || res.list || [],
            total: res.total || res.songCount || 0,
            page,
            limit: res.limit || 30,
            source: res.source || ''
          }
        }
      } catch (e) {
        console.warn('[Search] search failed:', e)
      } finally {
        this.loading = false
      }
    },
    async tipSearch(keyword: string) {
      if (!keyword?.trim()) return []
      try {
        const res = await (window as any).api?.music?.requestSdk?.('tipSearch', { keyword })
        return res?.list || res || []
      } catch {
        return []
      }
    },
    clearResults() {
      this.results = null
      this.page = 1
    },
    addToHistory(keyword: string) {
      if (!keyword?.trim()) return
      const idx = this.history.indexOf(keyword)
      if (idx !== -1) this.history.splice(idx, 1)
      this.history.unshift(keyword)
      if (this.history.length > 20) this.history = this.history.slice(0, 20)
      localStorage.setItem('searchHistory', JSON.stringify(this.history))
    },
    clearHistory() {
      this.history = []
      localStorage.removeItem('searchHistory')
    }
  },
  persist: false
})
