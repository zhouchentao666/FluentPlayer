<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { CloseIcon, HeartIcon } from 'tdesign-icons-vue-next'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { withViewTransition } from '@/composables/useViewTransition'
import { storeToRefs } from 'pinia'

const props = withDefaults(
  defineProps<{
    show: boolean
    mainColor?: string
  }>(),
  {
    mainColor: 'var(--td-brand-color)'
  }
)

const emit = defineEmits(['close'])

const { t } = useI18n()

const globalPlayStatus = useGlobalPlayStatusStore()
const { player } = storeToRefs(globalPlayStatus)

const currentType = ref<'hot' | 'latest'>('hot')
const expandedReplies = ref<Record<string | number, boolean>>({})
const loadTrigger = ref<HTMLElement | null>(null)
const contentRef = ref<HTMLElement | null>(null)
let observer: IntersectionObserver | null = null

const scrollPositions = ref({ hot: 0, latest: 0 })

const list = computed(() => {
  return currentType.value === 'hot'
    ? player.value.comments.hotList
    : player.value.comments.latestList
})

const isLoading = computed(() => {
  return currentType.value === 'hot'
    ? player.value.comments.hotIsLoading
    : player.value.comments.latestIsLoading
})

const hasMore = computed(() => {
  if (currentType.value === 'hot') {
    if (player.value.comments.hotList.length >= player.value.comments.hotTotal) return false
    return player.value.comments.hotPage < player.value.comments.hotMaxPage
  }
  if (player.value.comments.latestList.length >= player.value.comments.latestTotal) return false
  return player.value.comments.latestPage < player.value.comments.latestMaxPage
})

const switchType = (type: 'hot' | 'latest') => {
  if (currentType.value === type) return

  if (contentRef.value) {
    scrollPositions.value[currentType.value] = contentRef.value.scrollTop
  }

  withViewTransition(() => {
    currentType.value = type
  })

  nextTick(() => {
    if (contentRef.value) {
      contentRef.value.scrollTop = scrollPositions.value[type]
    }
  })

  const targetList =
    type === 'hot' ? player.value.comments.hotList : player.value.comments.latestList
  if (targetList.length === 0) {
    globalPlayStatus.fetchComments(1, type)
  }
}

const loadMore = () => {
  if (isLoading.value || !hasMore.value) return
  const nextPage =
    currentType.value === 'hot'
      ? player.value.comments.hotPage + 1
      : player.value.comments.latestPage + 1
  globalPlayStatus.fetchComments(nextPage, currentType.value)
}

const toggleReply = (id: string | number) => {
  expandedReplies.value[id] = !expandedReplies.value[id]
}

const formatNumber = (num: number) => {
  if (num > 10000) return (num / 10000).toFixed(1) + 'w'
  return num
}

const initObserver = () => {
  if (observer) observer.disconnect()
  const rootEl = contentRef.value
  if (!rootEl) return

  observer = new IntersectionObserver(
    (entries) => {
      const entry = entries[0]
      if (entry.isIntersecting && !isLoading.value && hasMore.value) {
        loadMore()
      }
    },
    { threshold: 0.1, rootMargin: '200px', root: rootEl }
  )

  if (loadTrigger.value) observer.observe(loadTrigger.value)
}

watch(
  () => props.show,
  (val) => {
    if (val) {
      nextTick(() => {
        if (contentRef.value) {
          contentRef.value.scrollTop = scrollPositions.value[currentType.value]
        }
        initObserver()
      })
    } else {
      if (contentRef.value) {
        scrollPositions.value[currentType.value] = contentRef.value.scrollTop
      }
      if (observer) observer.disconnect()
    }
  }
)

watch(list, () => {
  nextTick(() => {
    if (props.show && loadTrigger.value) initObserver()
  })
})

watch(isLoading, (val) => {
  if (!val && hasMore.value && props.show) {
    nextTick(() => {
      if (observer && loadTrigger.value) {
        observer.unobserve(loadTrigger.value)
        observer.observe(loadTrigger.value)
      }
    })
  }
})

onMounted(() => {
  if (props.show) initObserver()
})

onUnmounted(() => {
  if (observer) observer.disconnect()
})

watch(
  () => player.value.songId,
  () => {
    expandedReplies.value = {}
    if (contentRef.value) {
      contentRef.value.scrollTop = 0
      scrollPositions.value = { hot: 0, latest: 0 }
    }
  }
)

const onEnter = (el: Element) => {
  const element = el as HTMLElement
  element.style.height = '0'
  element.style.opacity = '0'
  // Force reflow
  void element.offsetHeight
  element.style.transition = 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)'
  element.style.height = element.scrollHeight + 'px'
  element.style.opacity = '1'
}

const onAfterEnter = (el: Element) => {
  const element = el as HTMLElement
  element.style.height = 'auto'
  element.style.opacity = ''
  element.style.transition = ''
}

const onLeave = (el: Element) => {
  const element = el as HTMLElement
  element.style.transition = 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)'
  element.style.height = element.scrollHeight + 'px'
  element.style.opacity = '1'
  void element.offsetHeight
  element.style.height = '0'
  element.style.opacity = '0'
}
</script>

<template>
  <Teleport to="body">
    <Transition name="fade-overlay">
      <div v-show="show" class="comments-overlay" @click.self="$emit('close')">
        <div class="comments-card">
          <div class="header">
            <div class="tabs">
              <span
                class="tab-item"
                :class="{ active: currentType === 'hot' }"
                @click="switchType('hot')"
              >
                {{ t('play.comments.hot') }}
                <span
                  v-if="
                    player.comments.hotTotal &&
                    player.comments.hotTotal !== player.comments.latestTotal
                  "
                  class="count"
                  >{{ formatNumber(player.comments.hotTotal) }}</span
                >
              </span>
              <span
                class="tab-item"
                :class="{ active: currentType === 'latest' }"
                @click="switchType('latest')"
              >
                {{ t('play.comments.latest') }}
                <span v-if="player.comments.latestTotal" class="count">{{
                  formatNumber(player.comments.latestTotal)
                }}</span>
              </span>
            </div>
            <button class="close-btn" @click="$emit('close')">
              <CloseIcon size="24" />
            </button>
          </div>

          <div ref="contentRef" class="content custom-scrollbar">
            <div v-if="isLoading && list.length === 0" class="loading-state">
              <t-loading :text="t('common.loading')" size="small" />
            </div>
            <div v-else-if="list.length === 0" class="empty-state">{{ t('play.comments.empty') }}</div>
            <div v-else class="comment-list">
              <div v-for="item in list" :key="item.id" class="comment-item">
                <t-avatar :image="item.avatar" size="40px" shape="circle" class="avatar" />
                <div class="comment-body">
                  <div class="user-info">
                    <span class="name">{{ item.userName }}</span>
                    <span class="time">{{ item.timeStr }}</span>
                  </div>
                  <div class="text">{{ item.text }}</div>

                  <div v-if="item.images && item.images.length > 0" class="image-list">
                    <t-image-viewer :images="item.images">
                      <template #trigger="{ open }">
                        <div class="image-grid">
                          <t-image
                            v-for="(img, index) in item.images"
                            :key="img"
                            :src="img"
                            class="comment-image"
                            fit="cover"
                            @click="open(index)"
                          />
                        </div>
                      </template>
                    </t-image-viewer>
                  </div>

                  <div v-if="item.reply && item.reply.length > 0" class="reply-section">
                    <div
                      v-if="!expandedReplies[item.id]"
                      class="expand-reply"
                      @click="toggleReply(item.id)"
                    >
                      {{ t('play.comments.expandReplies', { count: item.reply.length }) }}
                    </div>
                    <Transition @enter="onEnter" @after-enter="onAfterEnter" @leave="onLeave">
                      <div v-if="expandedReplies[item.id]" class="replies-wrapper">
                        <div class="replies">
                          <div v-for="reply in item.reply" :key="reply.id" class="reply-item">
                            <t-avatar
                              :image="reply.avatar"
                              size="24px"
                              shape="circle"
                              class="reply-avatar"
                            />
                            <div class="reply-content">
                              <span class="reply-user">{{ reply.userName }}</span>
                              <span class="reply-text">: {{ reply.text }}</span>
                            </div>
                          </div>
                          <div class="collapse-reply" @click="toggleReply(item.id)">{{ t('play.comments.collapseReplies') }}</div>
                        </div>
                      </div>
                    </Transition>
                  </div>
                </div>
                <div class="like-info">
                  <HeartIcon size="16" />
                  <span class="count">{{ formatNumber(item.likedCount) }}</span>
                </div>
              </div>

              <div ref="loadTrigger" class="load-trigger">
                <t-loading v-if="isLoading" :text="t('common.loading')" size="small" />
                <span v-else-if="!hasMore && list.length > 0" class="no-more">{{ t('play.comments.noMore') }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style lang="scss" scoped>
.comments-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(8px);
}

.comments-card {
  width: 80vw;
  height: 80vh;
  background: rgba(255, 255, 255, 0.05);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border-radius: 24px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 6px 12px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: #fff;
}

.header {
  height: 60px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 24px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.1);
}

.tabs {
  display: flex;
  gap: 20px;
}

.tab-item {
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  opacity: 0.6;
  transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease, box-shadow 0.3s ease, opacity 0.3s ease, transform 0.3s ease;
  position: relative;
  text-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  display: flex;
  align-items: center;
  gap: 6px;

  .count {
    font-size: 12px;
    opacity: 0.8;
    font-weight: normal;
  }

  &:hover {
    opacity: 0.9;
  }

  &.active {
    opacity: 1;
    font-size: 18px;

    &::after {
      content: '';
      position: absolute;
      bottom: -6px;
      left: 0;
      width: 100%;
      height: 3px;
      background: v-bind(mainColor);
      border-radius: 2px;
      box-shadow: 0 0 8px v-bind(mainColor);
    }
  }
}

.close-btn {
  background: transparent;
  border: none;
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  padding: 8px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.3s;

  &:hover {
    background: rgba(255, 255, 255, 0.1);
  }
}

.content {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.loading-state,
.empty-state {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  color: rgba(255, 255, 255, 0.6);
}

.comment-list {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.comment-item {
  display: flex;
  gap: 16px;
  position: relative;
}

.avatar {
  flex-shrink: 0;
  border: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
}

.comment-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 10px;

  .name {
    font-size: 14px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.95);
  }

  .time {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.5);
  }
}

.text {
  font-size: 14px;
  line-height: 1.6;
  color: rgba(255, 255, 255, 0.9);
  white-space: pre-wrap;
  font-weight: 400;
  user-select: text;
}

.like-info {
  position: absolute;
  top: 0;
  right: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);

  .count {
    min-width: 20px;
  }
}

.image-list {
  margin-top: 8px;
}

.image-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.comment-image {
  width: 100px;
  height: 100px;
  border-radius: 8px;
  cursor: zoom-in;
  border: 1px solid rgba(255, 255, 255, 0.1);
  transition: transform 0.2s;

  &:hover {
    transform: scale(1.02);
    border-color: rgba(255, 255, 255, 0.3);
  }
}

.reply-section {
  margin-top: 12px;
  background: rgba(0, 0, 0, 0.15);
  border-radius: 12px;
  padding: 12px;
  font-size: 13px;
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.expand-reply,
.collapse-reply {
  color: v-bind(mainColor);
  cursor: pointer;
  font-size: 12px;
  opacity: 0.9;

  &:hover {
    text-decoration: underline;
    opacity: 1;
  }
}

.replies-wrapper {
  overflow: hidden;
}

.replies {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-top: 8px;
}

.reply-item {
  display: flex;
  gap: 8px;
  align-items: flex-start;

  .reply-avatar {
    flex-shrink: 0;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .reply-content {
    line-height: 1.5;
    flex: 1;
  }

  .reply-user {
    color: rgba(255, 255, 255, 0.95);
    font-weight: 600;
    margin-right: 4px;
  }

  .reply-text {
    color: rgba(255, 255, 255, 0.85);
    user-select: text;
  }
}

.load-trigger {
  display: flex;
  justify-content: center;
  padding: 20px 0;
  min-height: 40px;
}

.no-more {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.4);
}

.custom-scrollbar {
  scrollbar-arrow-color: transparent;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.3) transparent;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 8px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.3);
  border-radius: 4px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.5);
}

.fade-overlay-enter-active,
.fade-overlay-leave-active {
  transition: opacity 0.3s ease;
}

.fade-overlay-enter-from,
.fade-overlay-leave-to {
  opacity: 0;
}
</style>
