<script setup lang="ts">
import { ref, computed, onMounted, watch, onActivated, onDeactivated, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { storeToRefs } from 'pinia'
import { musicSdk } from '@/services/musicSdk'
import { useSourceAccess } from '@/composables/useSourceAccess'

const router = useRouter()
const { t } = useI18n()
const localUserStore = LocalUserDetailStore()
const { userSource } = storeToRefs(localUserStore)
const { enabledSourceKeys } = useSourceAccess()

const playlists = ref<any[]>([])
const loading = ref(true)
const error = ref('')

const tags = ref<any[]>([])
const hotTag = ref<any[]>([])
const activeCategoryName = ref(t('music.playlistCategory.hot'))
const activeTagId = ref('')
const showMore = ref(false)
const activeGroupIndex = ref(0)
const activeGroupName = ref('')

const isSubsonic = computed(() => userSource.value.source === 'subsonic')

const page = ref(1)
const limit = ref(30)
const total = ref(0)
const loadingMore = ref(false)
const noMore = ref(false)

const categoryCache: Record<string, { list: any[]; page: number; total: number }> = {}

const scrollTop = ref(0)
const scrollRef = ref<HTMLDivElement>()
const hotTagsRef = ref<HTMLDivElement>()
const showRightFade = ref(true)

const fetchTags = async () => {
  try {
    const res = await musicSdk.getPlaylistTags()
    if (isSubsonic.value) {
      tags.value = []
      hotTag.value = res?.tags || []
    } else {
      tags.value = res?.tags || []
      hotTag.value = res?.hotTag || []
      activeGroupIndex.value = 0
      activeGroupName.value = tags.value[0]?.name || ''
    }
  } catch (e) {
    console.error('获取歌单标签失败:', e)
  }
}

const fetchCategoryPlaylists = async (reset = false) => {
  if (loadingMore.value) return
  if (reset) {
    page.value = 1
    noMore.value = false
    loading.value = true
    if (!categoryCache[(activeTagId.value || 'hot') + ':' + (userSource.value.source || 'kw')]) {
      playlists.value = []
    }
  }

  const cacheKey = (activeTagId.value || 'hot') + ':' + (userSource.value.source || 'kw')

  if (reset && categoryCache[cacheKey]) {
    const cache = categoryCache[cacheKey]
    playlists.value = cache.list
    page.value = cache.page
    total.value = cache.total
    loading.value = false
    return
  }

  loadingMore.value = true
  try {
    const sortId = isSubsonic.value ? (activeTagId.value || 'recent') : 'hot'
    const tagId = isSubsonic.value ? '' : activeTagId.value
    const res = await musicSdk.getCategoryPlaylists(sortId, tagId, page.value, limit.value)
    const list = Array.isArray(res?.list) ? res.list : []
    total.value = res?.total || 0

    const mapped = list.map((item: any) => ({
      id: item.id,
      title: item.name,
      description: item.desc || t('music.playlistCategory.featuredPlaylist'),
      cover: item.img,
      playCount: item.play_count || item.playCount,
      author: item.author,
      total: item.total,
      source: item.source
    }))

    playlists.value = reset ? mapped : [...playlists.value, ...mapped]
    noMore.value = playlists.value.length >= total.value
    if (!noMore.value) page.value += 1

    categoryCache[cacheKey] = {
      list: playlists.value.slice(),
      page: page.value,
      total: total.value
    }
    error.value = ''
  } catch (e) {
    console.error('获取分类歌单失败:', e)
    error.value = t('music.playlistCategory.fetchFailed')
  } finally {
    loading.value = false
    loadingMore.value = false
  }
}

const scrollToActiveTag = () => {
  requestAnimationFrame(() => {
    if (!hotTagsRef.value) return
    const active = hotTagsRef.value.querySelector('.tag-chip.active') as HTMLElement
    if (!active) return
    const container = hotTagsRef.value
    const scrollLeft = active.offsetLeft - container.offsetLeft - container.clientWidth / 2 + active.clientWidth / 2
    container.scrollTo({ left: Math.max(0, scrollLeft), behavior: 'smooth' })
  })
}

const updateRightFade = () => {
  if (!hotTagsRef.value) return
  const el = hotTagsRef.value
  showRightFade.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 8
}

const onSelectTag = async (tagId: string, name: string) => {
  activeTagId.value = tagId
  activeCategoryName.value = name
  showMore.value = false
  scrollToActiveTag()
  await fetchCategoryPlaylists(true)
}

const onSelectGroup = (groupName: string) => {
  activeGroupName.value = groupName
}

const openMoreCategories = () => {
  showMore.value = true
}

const closeMoreCategories = () => {
  showMore.value = false
}

const handleMoreKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') closeMoreCategories()
}

const onScroll = (e: Event) => {
  const el = e.target as HTMLElement
  if (el.scrollHeight - el.scrollTop - el.clientHeight < 100 && !noMore.value && !loadingMore.value) {
    fetchCategoryPlaylists(false)
  }
}

watch(activeGroupName, (name) => {
  const idx = tags.value.findIndex((g: any) => g.name === name)
  activeGroupIndex.value = idx >= 0 ? idx : 0
})

const formatCount = (count: any) => {
  const n = typeof count === 'number' ? count : parseInt(count) || 0
  if (n >= 10000) return (n / 10000).toFixed(1) + t('common.unitTenThousand')
  return String(n)
}

const goToPlaylist = (item: any) => {
  router.push({
    name: 'list',
    params: { id: item.id },
    query: { title: item.title, source: item.source, cover: item.cover }
  })
}

const onDocClick = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  if (!target.closest('.category-bar') && showMore.value) showMore.value = false
}

let initialized = false
let tagsCacheSource = ''

const onSourceChange = (isInit: boolean) => {
  if (enabledSourceKeys.value.size === 0) {
    playlists.value = []
    tags.value = []
    hotTag.value = []
    loading.value = false
    return
  }
  const source = userSource.value.source || 'kw'
  if (!isInit && source === tagsCacheSource && tags.value.length + hotTag.value.length > 0) return
  loading.value = true
  error.value = ''
  if (!isInit) Object.keys(categoryCache).forEach(k => delete categoryCache[k])
  fetchTags().then(() => {
    tagsCacheSource = source
    if (!isInit) {
      activeTagId.value = ''
      activeCategoryName.value = t('music.playlistCategory.hot')
    }
    requestAnimationFrame(updateRightFade)
    fetchCategoryPlaylists(true)
  })
}

onMounted(() => {
  requestAnimationFrame(updateRightFade)
  if (!initialized) {
    initialized = true
    onSourceChange(true)
    watch(userSource, () => onSourceChange(false), { deep: true })
  }

  document.addEventListener('click', onDocClick)
  document.addEventListener('keydown', handleMoreKeydown)
})

onUnmounted(() => {
  document.removeEventListener('click', onDocClick)
  document.removeEventListener('keydown', handleMoreKeydown)
})

onActivated(() => {
  if (scrollRef.value) scrollRef.value.scrollTop = scrollTop.value
})

onDeactivated(() => {
  if (scrollRef.value) scrollTop.value = scrollRef.value.scrollTop
})
</script>

<template>
  <div ref="scrollRef" class="playlist-category" @scroll="onScroll">
    <div class="category-bar" :class="{ 'has-fade': showRightFade }">
      <div ref="hotTagsRef" class="hot-tags" @scroll.passive="updateRightFade">
        <button
          class="tag-chip"
          :class="{ active: activeTagId === '' }"
          @click="onSelectTag('', t('music.playlistCategory.hot'))"
        >
          {{ t('music.playlistCategory.hot') }}
        </button>
        <button
          v-for="t in hotTag"
          :key="t.id"
          class="tag-chip"
          :class="{ active: activeTagId === t.id }"
          @click="onSelectTag(t.id, t.name)"
        >
          {{ t.name }}
        </button>

        <div
          v-if="!isSubsonic"
          class="more-category-wrapper"
          @mouseenter="openMoreCategories"
          @mouseleave="closeMoreCategories"
        >
          <t-button
            class="tag-chip more"
            shape="round"
            variant="outline"
            :aria-expanded="showMore"
            aria-haspopup="dialog"
            @click.stop="openMoreCategories"
          >
            {{ t('music.playlistCategory.moreCategory') }}
            <template #suffix>
              <t-icon name="chevron-down" size="14px" :class="{ rotate: showMore }" />
            </template>
          </t-button>

          <transition name="dropdown">
            <div v-if="showMore" class="more-panel desktop-more-panel">
              <div class="panel-inner">
                <div class="panel-content">
                  <t-tabs v-model:value="activeGroupName" size="medium">
                    <t-tab-panel
                      v-for="group in tags"
                      :key="group.name"
                      :value="group.name"
                      :label="group.name"
                    />
                  </t-tabs>
                  <div v-if="tags[activeGroupIndex]" class="panel-tags">
                    <button
                      v-for="t in tags[activeGroupIndex].list"
                      :key="t.id"
                      class="tag-chip"
                      :class="{ active: activeTagId === t.id }"
                      @click="onSelectTag(t.id, t.name)"
                    >
                      {{ t.name }}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </transition>
        </div>
      </div>

      <teleport to="body">
        <transition name="sheet-fade">
          <div
            v-if="showMore"
            class="mobile-category-sheet-mask"
            role="presentation"
            @click.self="closeMoreCategories"
          >
            <section
              class="mobile-category-sheet"
              role="dialog"
              aria-modal="true"
              aria-labelledby="categorySheetTitle"
              @click.stop
            >
              <div class="sheet-handle" />
              <header class="sheet-header">
                <div>
                  <p class="sheet-eyebrow">{{ t('music.playlistCategory.selectCategory') }}</p>
                  <h3 id="categorySheetTitle">{{ t('music.playlistCategory.discoverMore') }}</h3>
                </div>
                <button class="sheet-close" :aria-label="t('music.playlistCategory.closeCategoryPanel')" @click="closeMoreCategories">
                  <t-icon name="close" size="18px" />
                </button>
              </header>

              <div class="sheet-body">
                <div class="sheet-groups" role="tablist" :aria-label="t('music.playlistCategory.categoryGroup')">
                  <button
                    v-for="group in tags"
                    :key="group.name"
                    class="sheet-group-tab"
                    :class="{ active: activeGroupName === group.name }"
                    role="tab"
                    :aria-selected="activeGroupName === group.name"
                    @click="onSelectGroup(group.name)"
                  >
                    {{ group.name }}
                  </button>
                </div>

                <div v-if="tags[activeGroupIndex]" class="sheet-tags">
                  <button
                    v-for="t in tags[activeGroupIndex].list"
                    :key="t.id"
                    class="sheet-tag-chip"
                    :class="{ active: activeTagId === t.id }"
                    @click="onSelectTag(t.id, t.name)"
                  >
                    {{ t.name }}
                  </button>
                </div>
              </div>
            </section>
          </div>
        </transition>
      </teleport>
    </div>

    <div class="section">
      <h3 class="section-title">{{ t('music.playlistCategory.hotPlaylists', { name: activeCategoryName }) }}</h3>

      <div v-if="loading && playlists.length === 0" class="loading-container">
        <div class="playlist-loading" role="status" aria-live="polite">
          <span class="playlist-loading-spinner" aria-hidden="true" />
          <span>{{ t('music.playlistCategory.loadingPlaylists') }}</span>
        </div>
      </div>

      <div v-else-if="error" class="error-container">
        <t-alert theme="error" :message="error" />
        <t-button theme="primary" style="margin-top: 1rem" @click="fetchCategoryPlaylists(true)">
          {{ t('music.playlistCategory.reload') }}
        </t-button>
      </div>

      <div v-else-if="playlists.length === 0" class="empty-container">
        <div class="empty-orb">
          <t-icon name="music" size="28px" />
        </div>
        <h4>{{ t('music.playlistCategory.noPlaylists') }}</h4>
        <p>{{ t('music.playlistCategory.tryOtherCategory') }}</p>
      </div>

      <div v-else class="playlist-grid">
        <div
          v-for="playlist in playlists"
          :key="playlist.id"
          class="playlist-card"
          @click="goToPlaylist(playlist)"
        >
          <div class="playlist-cover">
            <img :src="playlist.cover || '/default-cover.png'" :alt="playlist.title" loading="lazy" />
          </div>
          <div class="playlist-info">
            <h4 class="playlist-title">{{ playlist.title }}</h4>
            <p class="playlist-desc">{{ playlist.description }}</p>
            <div v-if="playlist.playCount || playlist.total" class="playlist-meta">
              <span v-if="playlist.playCount" class="play-count">
                <i class="iconfont icon-bofang"></i>
                {{ formatCount(playlist.playCount) }}
              </span>
              <span v-if="playlist.total" class="song-count">{{ t('music.playlistCategory.songUnit', { count: playlist.total }) }}</span>
            </div>
          </div>
        </div>
      </div>

      <div v-if="loadingMore && playlists.length > 0" class="load-status">
        <div class="playlist-loading small" role="status" aria-live="polite">
          <span class="playlist-loading-spinner" aria-hidden="true" />
          <span>{{ t('music.playlistCategory.loadingMore') }}</span>
        </div>
      </div>
      <div v-else-if="noMore && playlists.length > 0" class="load-status">
        <span class="no-more">{{ t('music.playlistCategory.noMore') }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.playlist-category {
  height: 100%;
  overflow-y: auto;
  padding: 0 2rem;
}

.category-bar {
  margin-bottom: 1rem;
  position: relative;
}

.hot-tags {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.tag-chip {
  padding: 4px 12px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--td-text-color-secondary);
  cursor: pointer;
  font-size: 13px;
  transition: background-color 0.2s ease, border-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease, transform 0.2s ease;
}

.tag-chip:hover {
  color: var(--td-text-color-primary);
  background: var(--td-bg-color-secondarycontainer);
}

.tag-chip.active {
  background: var(--td-brand-color-light);
  color: var(--td-brand-color);
  font-weight: 600;
}

.tag-chip.more {
  display: flex;
  align-items: center;
  gap: 4px;
  background: var(--td-bg-color-secondarycontainer);
}

.tag-chip.more:hover {
  background: var(--td-bg-color-component-hover);
}

.more-category-wrapper {
  position: static;
  display: inline-block;
}

.more-panel {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  z-index: 100;
  padding-top: 8px;
  margin-top: 0;
  width: 100%;
  min-width: unset;
  transform-origin: top center;
}

.panel-inner {
  background: var(--td-bg-color-container);
  border-radius: 12px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  border: 1px solid var(--td-border-level-1-color);
  padding: 8px 16px 16px;
}

.panel-tags {
  display: flex;
  flex-wrap: wrap;
  padding-top: 12px;
  gap: 8px;
  max-height: 300px;
  overflow-y: auto;
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: background-color 0.16s ease, border-color 0.16s ease, color 0.16s ease, box-shadow 0.16s ease, opacity 0.16s ease, transform 0.16s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.98);
}

.section {
  margin-bottom: 3rem;
}

.section-title {
  color: var(--td-text-color-primary);
  font-size: 1.25rem;
  font-weight: 600;
  margin-bottom: 1.5rem;
}

.loading-container {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 4rem 0;
}

.playlist-loading {
  display: inline-flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--td-text-color-secondary);
  font-size: 14px;
}

.playlist-loading.small {
  flex-direction: row;
  gap: 8px;
  font-size: 12px;
}

.playlist-loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--td-bg-color-component);
  border-top-color: var(--td-brand-color);
  border-radius: 50%;
  will-change: transform;
  animation: playlist-loading-spin 1s linear infinite;
}

.playlist-loading.small .playlist-loading-spinner {
  width: 16px;
  height: 16px;
  border-width: 2px;
}

.error-container {
  text-align: center;
  padding: 2rem;
}

.load-status {
  display: flex;
  justify-content: center;
  padding: 12px 0;
}

.no-more {
  font-size: 12px;
  color: var(--td-text-color-placeholder);
}

/* 歌单网格 */
.playlist-grid {
  display: grid;
  gap: 1.25rem;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
}

.sheet-fade-enter-active,
.sheet-fade-leave-active {
  transition: opacity 0.22s ease;
}

.sheet-fade-enter-active .mobile-category-sheet,
.sheet-fade-leave-active .mobile-category-sheet {
  transition: transform 0.26s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.22s ease;
}

.sheet-fade-enter-from,
.sheet-fade-leave-to {
  opacity: 0;
}

.sheet-fade-enter-from .mobile-category-sheet,
.sheet-fade-leave-to .mobile-category-sheet {
  opacity: 0;
  transform: translateY(24px) scale(0.98);
}

.mobile-category-sheet-mask {
  display: none;
}

.empty-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  min-height: 260px;
  padding: 2rem;
  text-align: center;
  color: var(--td-text-color-secondary);
}

.empty-orb {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 64px;
  height: 64px;
  border-radius: 22px;
  color: var(--td-brand-color);
  background: var(--td-brand-color-light);
  box-shadow: 0 16px 36px rgba(0, 0, 0, 0.1);
}

.empty-container h4 {
  margin: 0.5rem 0 0;
  color: var(--td-text-color-primary);
  font-size: 1rem;
}

.empty-container p {
  margin: 0;
  font-size: 0.875rem;
}



@media (min-width: 481px) and (max-width: 768px) {
  .playlist-grid {
    gap: 1rem;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  }
}

@media (min-width: 769px) and (max-width: 1024px) {
  .playlist-grid {
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
  }
}

@media (min-width: 1200px) {
  .playlist-grid {
    gap: 1.5rem;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
  }
}

/* 歌单卡片 — 对齐 CeruMusic-main 风格 */
.playlist-card {
  background: var(--td-bg-color-container);
  border-radius: 1rem;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  cursor: pointer;
  position: relative;
  content-visibility: auto;
  contain-intrinsic-size: 0 320px;
}

.playlist-card:hover {
  transform: translateY(-4px) scale(1.02);
  box-shadow: 0 6px 12px rgba(0, 0, 0, 0.15);
}

.playlist-card:hover .playlist-cover::after {
  opacity: 1;
}

.playlist-card:active {
  transform: translateY(-2px) scale(1.01);
}

.playlist-cover {
  position: relative;
  aspect-ratio: 1;
  overflow: hidden;
}

/* 悬浮遮罩层 */
.playlist-cover::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, rgba(0, 0, 0, 0.1) 0%, rgba(0, 0, 0, 0.3) 100%);
  opacity: 0;
  transition: opacity 0.3s ease;
  pointer-events: none;
}

.playlist-cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  user-select: none;
  -webkit-user-drag: none;
  transition: transform 0.3s ease;
}

.playlist-card:hover .playlist-cover img {
  transform: scale(1.05);
}

.playlist-info {
  padding: 1.25rem 1rem;
  position: relative;
  background: var(--td-bg-color-container);
  transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease, box-shadow 0.3s ease, opacity 0.3s ease, transform 0.3s ease;
}

.playlist-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--td-text-color-primary);
  margin: 0 0 0.5rem;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: 2.8rem;
}

.playlist-desc {
  font-size: 0.875rem;
  color: var(--td-text-color-secondary);
  margin: 0 0 0.75rem;
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: 2.625rem;
  transition: color 0.3s ease;
}

.playlist-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-top: auto;
  padding-top: 0.5rem;
  border-top: 1px solid var(--td-border-level-1-color);
  transition: color 0.3s ease;
}

.play-count {
  font-size: 0.75rem;
  color: var(--td-text-color-placeholder);
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-weight: 500;
  transition: color 0.3s ease;
}

.play-count .iconfont {
  font-size: 0.875rem;
  opacity: 0.8;
}

.song-count {
  font-size: 0.75rem;
  color: var(--td-text-color-placeholder);
  font-weight: 500;
  background: var(--td-bg-color-secondarycontainer);
  padding: 0.125rem 0.5rem;
  border-radius: 0.375rem;
  transition: color 0.3s ease;
}

@media (max-width: 768px) {
  .playlist-category {
    width: 100%;
    max-width: 100%;
    padding: 0 var(--mobile-page-gutter) 1.5rem;
    overflow-x: hidden;
    -webkit-overflow-scrolling: touch;
  }

  .category-bar,
  .section,
  .playlist-grid {
    width: 100%;
    max-width: 100%;
    min-width: 0;
  }

  .playlist-card {
    max-width: 100%;
    min-width: 0;
  }

  .category-bar {
    position: sticky;
    top: 0;
    z-index: 10;
    margin: 0 0 0.75rem;
    padding: 0.5rem 0 0.375rem;
    max-width: 100%;
    min-width: 0;
    overflow: hidden;
    transition: box-shadow 0.2s ease;
  }

  .category-bar.has-fade::after {
    content: '';
    position: absolute;
    top: 0.5rem;
    right: 0;
    bottom: 0.375rem;
    width: 32px;
    pointer-events: none;
    background: linear-gradient(90deg, transparent, var(--td-bg-color-page, var(--td-bg-color-container)) 85%);
  }

  .hot-tags {
    width: calc(100vw - var(--mobile-page-gutter) * 2);
    max-width: calc(100vw - var(--mobile-page-gutter) * 2);
    min-width: 0;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: max-content;
    justify-content: start;
    gap: 0.5rem;
    overflow-x: scroll;
    overflow-y: hidden;
    overscroll-behavior-x: contain;
    padding: 0.25rem 2rem 0.5rem 0;
    scroll-padding-inline: 0;
    scroll-snap-type: x mandatory;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
    touch-action: pan-x;
  }

  .hot-tags::-webkit-scrollbar {
    display: none;
  }

  .more-category-wrapper {
    min-width: 0;
    display: block;
  }

  .tag-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    vertical-align: top;
    scroll-snap-align: start;
    flex: 0 0 auto;
    min-height: var(--mobile-touch-target);
    padding: 0 0.875rem;
    border: 1px solid var(--td-border-level-1-color);
    border-radius: 999px;
    color: var(--td-text-color-secondary);
    background: var(--td-bg-color-container);
    box-shadow: none;
    font-size: 0.875rem;
    font-weight: 500;
    line-height: 1;
    touch-action: manipulation;
    position: relative;
    overflow: hidden;
  }

  .tag-chip::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    opacity: 0;
    background: var(--td-brand-color);
    transition: opacity 0.22s ease;
    pointer-events: none;
  }

  .tag-chip:hover {
    color: var(--td-text-color-secondary);
    background: var(--td-bg-color-container);
    transform: none;
  }

  .tag-chip:active,
  .tag-chip.more:active {
    transform: scale(0.96);
    background: var(--td-bg-color-secondarycontainer);
  }

  .tag-chip.active {
    color: var(--td-brand-color);
    background: var(--td-brand-color-light);
    border-color: color-mix(in srgb, var(--td-brand-color) 40%, transparent);
    font-weight: 600;
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
  }

  .tag-chip.more {
    position: static;
    z-index: auto;
    gap: 0.25rem;
    color: var(--td-text-color-primary);
    background: var(--td-bg-color-secondarycontainer);
    border-color: var(--td-border-level-2-color);
  }

  .desktop-more-panel {
    display: none;
  }

  .mobile-category-sheet-mask {
    position: fixed;
    inset: 0;
    z-index: var(--mobile-overlay-layer-z);
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding: 0 0 var(--mobile-safe-bottom);
    background: rgba(0, 0, 0, 0.42);
  }

  .mobile-category-sheet {
    width: 100%;
    max-width: 430px;
    max-height: min(78dvh, 640px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 20px 20px 0 0;
    background: var(--td-bg-color-container);
    box-shadow: 0 -8px 28px rgba(0, 0, 0, 0.18);
  }

  .sheet-handle {
    width: 36px;
    height: 4px;
    flex: 0 0 auto;
    margin: 10px auto 0;
    border-radius: 999px;
    background: var(--td-border-level-2-color);
  }

  .sheet-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.875rem var(--mobile-page-gutter) 0.75rem;
    border-bottom: 1px solid var(--td-border-level-1-color);
  }

  .sheet-eyebrow {
    display: none;
  }

  .sheet-header h3 {
    margin: 0;
    color: var(--td-text-color-primary);
    font-size: 1rem;
    font-weight: 600;
    line-height: 1.4;
  }

  .sheet-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border: 0;
    border-radius: 12px;
    color: var(--td-text-color-secondary);
    background: transparent;
    cursor: pointer;
    touch-action: manipulation;
  }

  .sheet-close:active {
    background: var(--td-bg-color-secondarycontainer);
    transform: scale(0.97);
  }

  .sheet-body {
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 0.875rem;
    padding: 0.875rem var(--mobile-page-gutter) 1rem;
    overflow-y: auto;
    overflow-x: hidden;
    -webkit-overflow-scrolling: touch;
  }

  .sheet-groups {
    display: flex;
    gap: 0.5rem;
    overflow-x: auto;
    padding-bottom: 0.125rem;
    scrollbar-width: none;
  }

  .sheet-groups::-webkit-scrollbar {
    display: none;
  }

  .sheet-group-tab,
  .sheet-tag-chip {
    border: 1px solid var(--td-border-level-1-color);
    cursor: pointer;
    touch-action: manipulation;
  }

  .sheet-group-tab {
    flex: 0 0 auto;
    min-height: 38px;
    padding: 0 0.875rem;
    border-radius: 999px;
    color: var(--td-text-color-secondary);
    background: var(--td-bg-color-container);
    font-size: 0.875rem;
    font-weight: 500;
  }

  .sheet-group-tab.active {
    color: var(--td-brand-color);
    border-color: var(--td-brand-color);
    background: var(--td-brand-color-light);
    font-weight: 600;
  }

  .sheet-tags {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.5rem;
  }

  .sheet-tag-chip {
    min-height: var(--mobile-touch-target);
    padding: 0 0.375rem;
    border-radius: 12px;
    color: var(--td-text-color-primary);
    background: var(--td-bg-color-secondarycontainer);
    font-size: 0.875rem;
    font-weight: 500;
  }

  .sheet-tag-chip.active {
    color: var(--td-brand-color);
    border-color: var(--td-brand-color);
    background: var(--td-brand-color-light);
    font-weight: 600;
  }

  .section {
    margin-bottom: 1.5rem;
  }

  .section-title {
    margin: 0 0 0.75rem;
    font-size: 1.0625rem;
    line-height: 1.4;
  }

  .loading-container,
  .error-container,
  .empty-container {
    min-height: 220px;
    margin-top: 0.5rem;
    padding: 1.5rem 1rem;
    border: 1px solid var(--td-border-level-1-color);
    border-radius: 16px;
    background: var(--td-bg-color-container);
    box-shadow: none;
  }

  .empty-orb {
    width: 56px;
    height: 56px;
    border-radius: 18px;
    box-shadow: none;
  }

  .error-container :deep(.t-alert) {
    text-align: left;
    border-radius: 12px;
  }

  .error-container :deep(.t-button) {
    min-height: var(--mobile-touch-target);
    border-radius: 999px;
  }

  .playlist-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0.75rem;
  }

  .playlist-card {
    min-height: 104px;
    display: flex;
    align-items: stretch;
    gap: 0.75rem;
    padding: 0.625rem;
    border: 1px solid var(--td-border-level-1-color);
    border-radius: 16px;
    background: var(--td-bg-color-container);
    box-shadow: none;
    content-visibility: auto;
    contain-intrinsic-size: 0 116px;
    touch-action: manipulation;
  }

  .playlist-card:hover {
    transform: none;
    box-shadow: none;
  }

  .playlist-card:active {
    transform: scale(0.99);
    background: var(--td-bg-color-secondarycontainer);
  }

  .playlist-cover {
    width: 84px;
    height: 84px;
    flex: 0 0 84px;
    border-radius: 12px;
    overflow: hidden;
    aspect-ratio: auto;
  }

  .playlist-cover::after {
    display: none;
  }

  .playlist-cover img,
  .playlist-card:hover .playlist-cover img {
    transform: none;
  }

  .playlist-info {
    min-width: 0;
    max-width: 100%;
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 0.125rem 0;
    background: transparent;
  }

  .playlist-title {
    min-height: 0;
    margin: 0 0 0.375rem;
    font-size: 0.9375rem;
    font-weight: 600;
    line-height: 1.4;
    -webkit-line-clamp: 2;
  }

  .playlist-desc {
    min-height: 0;
    margin: 0 0 0.5rem;
    color: var(--td-text-color-secondary);
    font-size: 0.8125rem;
    line-height: 1.4;
    -webkit-line-clamp: 1;
  }

  .playlist-meta {
    justify-content: flex-start;
    gap: 0.5rem;
    margin-top: 0;
    padding-top: 0;
    border-top: 0;
  }

  .play-count,
  .song-count {
    color: var(--td-text-color-placeholder);
    font-size: 0.75rem;
  }

  .song-count {
    background: var(--td-bg-color-secondarycontainer);
  }

  .load-status {
    padding: 1rem 0 0;
  }

  .no-more {
    display: inline-flex;
    align-items: center;
    min-height: 32px;
    padding: 0 0.875rem;
    border-radius: 999px;
    background: var(--td-bg-color-secondarycontainer);
  }
}

@media (prefers-reduced-motion: reduce) {
  .sheet-fade-enter-active,
  .sheet-fade-leave-active,
  .sheet-fade-enter-active .mobile-category-sheet,
  .sheet-fade-leave-active .mobile-category-sheet,
  .playlist-card,
  .playlist-cover img,
  .tag-chip {
    transition: none;
  }
}

@keyframes playlist-loading-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
