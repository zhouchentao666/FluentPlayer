<script lang="ts" setup>
import { ref, computed, watch } from 'vue'
import type { MusicInfo } from '@online/types/music'
import {
  getComments,
  supportedSorts,
  COMMENT_SORTS,
  type CommentItem,
  type CommentPage,
  type CommentSort,
} from '@online/lib/comment'

const props = defineProps<{
  song: MusicInfo | null
}>()

const emit = defineEmits<{
  close: []
}>()

const page = ref(1)
const limit = 20
const loading = ref(false)
const errorMsg = ref('')
const data = ref<CommentPage | null>(null)
const sort = ref<CommentSort>('hot')
// 游标翻页平台（网易云最新/最旧、咪咕最新）只能顺序前进，
// 这里按页号缓存游标，从而也能正确后退。
const cursors = ref<Record<number, string | null>>({})

const comments = computed<CommentItem[]>(() => data.value?.comments ?? [])
const total = computed(() => data.value?.total ?? 0)
const maxPage = computed(() => data.value?.maxPage ?? 1)
const cursorPaging = computed(() => data.value?.cursorPaging === true)
// 游标模式下无法跳页，只有拿到 nextCursor 才有下一页。
const hasNext = computed(() =>
  cursorPaging.value ? !!data.value?.nextCursor : page.value < maxPage.value,
)
const hasPrev = computed(() => page.value > 1)
const showPager = computed(() => hasPrev.value || hasNext.value)

const SOURCE_LABELS: Record<string, string> = {
  wy: '网易云音乐',
  tx: 'QQ 音乐',
  kg: '酷狗音乐',
  kw: '酷我音乐',
  mg: '咪咕音乐',
}
const sourceLabel = computed(() =>
  props.song ? (SOURCE_LABELS[props.song.source] ?? props.song.source) : '',
)

// 当前平台可用的排序分类
const sortTabs = computed(() => {
  if (!props.song) return []
  const allowed = supportedSorts(props.song.source)
  return COMMENT_SORTS.filter((s) => allowed.includes(s.value))
})

async function load(pageNo = 1) {
  if (!props.song) return
  loading.value = true
  errorMsg.value = ''
  try {
    const res = await getComments(props.song, {
      page: pageNo,
      limit,
      sort: sort.value,
      cursor: cursors.value[pageNo] ?? null,
    })
    data.value = res
    page.value = pageNo
    // 记录下一页的游标
    if (res.cursorPaging) cursors.value[pageNo + 1] = res.nextCursor ?? null
  } catch (err) {
    errorMsg.value = (err as Error).message || '获取评论失败'
    data.value = null
  } finally {
    loading.value = false
  }
}

function goPage(next: number) {
  if (loading.value) return
  if (next < 1 || next === page.value) return
  if (next > page.value && !hasNext.value) return
  if (!cursorPaging.value && next > maxPage.value) return
  void load(next)
}

function changeSort(next: CommentSort) {
  if (sort.value === next || loading.value) return
  sort.value = next
  cursors.value = {}
  void load(1)
}

// 切换歌曲时重置分类与游标并重新加载；进入时立即加载
watch(
  () => props.song,
  () => {
    const allowed = props.song ? supportedSorts(props.song.source) : ['hot' as CommentSort]
    if (!allowed.includes(sort.value)) sort.value = allowed[0]
    cursors.value = {}
    void load(1)
  },
  { immediate: true },
)

function fmtCount(n: number): string {
  if (n >= 10000) return (n / 10000).toFixed(1).replace(/\.0$/, '') + '万'
  return String(n)
}
</script>

<template>
  <div class="comment-view">
    <header class="cv-header">
      <button class="cv-back" title="返回" @click="emit('close')">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M15 18l-6-6 6-6" />
        </svg>
      </button>
      <div class="cv-title-wrap">
        <div class="cv-title">评论</div>
        <div class="cv-sub" v-if="song">
          {{ song.name }} - {{ song.singer }}
          <span class="cv-source">· {{ sourceLabel }}</span>
          <span class="cv-total" v-if="total">· 共 {{ fmtCount(total) }} 条</span>
        </div>
        <div class="cv-sub" v-else>未播放在线歌曲</div>
      </div>
    </header>

    <nav v-if="song && sortTabs.length > 1" class="cv-tabs">
      <button
        v-for="t in sortTabs"
        :key="t.value"
        class="cv-tab"
        :class="{ active: sort === t.value }"
        :disabled="loading"
        @click="changeSort(t.value)"
      >
        {{ t.label }}
      </button>
    </nav>

    <div class="cv-body">
      <div v-if="!song" class="cv-empty">当前没有可显示评论的在线歌曲，请先在在线页面播放一首歌。</div>

      <div v-else-if="loading" class="cv-loading">
        <div class="cv-spinner"></div>
        <span>加载评论中…</span>
      </div>

      <div v-else-if="errorMsg" class="cv-error">
        <p>{{ errorMsg }}</p>
        <button class="cv-retry" @click="load(page)">重试</button>
      </div>

      <div v-else-if="comments.length === 0" class="cv-empty">暂时还没有评论。</div>

      <ul v-else class="cv-list">
        <li v-for="c in comments" :key="c.id" class="cv-item">
          <img v-if="c.avatar" class="cv-avatar" :src="c.avatar" alt="" referrerpolicy="no-referrer" />
          <div v-else class="cv-avatar cv-avatar-default">{{ (c.userName || '?').slice(0, 1) }}</div>
          <div class="cv-main">
            <div class="cv-user">
              <span class="cv-name">{{ c.userName }}</span>
              <span v-if="c.location" class="cv-loc">{{ c.location }}</span>
            </div>
            <div class="cv-text">{{ c.text }}</div>
            <div v-if="c.reply && c.reply.length" class="cv-reply">
              <div v-for="r in c.reply" :key="r.id" class="cv-reply-item">
                <span class="cv-reply-name">{{ r.userName }}：</span><span>{{ r.text }}</span>
              </div>
            </div>
            <div class="cv-meta">
              <span class="cv-time">{{ c.timeStr }}</span>
              <span class="cv-like" v-if="c.likedCount">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3zM7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3" />
                </svg>
                {{ fmtCount(c.likedCount) }}
              </span>
            </div>
          </div>
        </li>
      </ul>
    </div>

    <footer v-if="song && showPager" class="cv-pager">
      <button class="cv-page-btn" :disabled="!hasPrev || loading" @click="goPage(page - 1)">上一页</button>
      <span class="cv-page-info">
        {{ page }}<template v-if="!cursorPaging"> / {{ maxPage }}</template>
      </span>
      <button class="cv-page-btn" :disabled="!hasNext || loading" @click="goPage(page + 1)">下一页</button>
    </footer>
  </div>
</template>

<style scoped>
.comment-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  background: var(--fluent-bg-glass, #1c1c1c);
  color: var(--fluent-text, #fff);
}
.cv-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 20px;
  border-bottom: 1px solid var(--fluent-border, rgba(255, 255, 255, 0.08));
  flex-shrink: 0;
}
.cv-back {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  border: none;
  background: var(--fluent-bg-hover, rgba(255, 255, 255, 0.06));
  color: inherit;
  cursor: pointer;
  transition: background 0.15s;
}
.cv-back:hover {
  background: var(--fluent-bg-active, rgba(255, 255, 255, 0.12));
}
.cv-title-wrap {
  min-width: 0;
}
.cv-title {
  font-size: 17px;
  font-weight: 600;
}
.cv-sub {
  font-size: 12px;
  color: var(--fluent-text-secondary, rgba(255, 255, 255, 0.6));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.cv-source {
  color: var(--fluent-accent, #4a90d9);
}
.cv-tabs {
  display: flex;
  gap: 8px;
  padding: 10px 20px 0;
  flex-shrink: 0;
}
.cv-tab {
  padding: 5px 16px;
  border-radius: 999px;
  border: 1px solid transparent;
  background: var(--fluent-bg-hover, rgba(255, 255, 255, 0.06));
  color: var(--fluent-text-secondary, rgba(255, 255, 255, 0.65));
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.cv-tab:hover:not(:disabled):not(.active) {
  background: var(--fluent-bg-active, rgba(255, 255, 255, 0.12));
  color: var(--fluent-text, #fff);
}
.cv-tab.active {
  background: var(--fluent-accent, #4a90d9);
  color: #fff;
}
.cv-tab:disabled {
  opacity: 0.5;
  cursor: default;
}
.cv-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 20px 16px;
}
.cv-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.cv-item {
  display: flex;
  gap: 12px;
  padding: 14px 0;
  border-bottom: 1px solid var(--fluent-border, rgba(255, 255, 255, 0.06));
}
.cv-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  object-fit: cover;
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.08);
}
.cv-avatar-default {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  color: var(--fluent-text-secondary, rgba(255, 255, 255, 0.7));
}
.cv-main {
  min-width: 0;
  flex: 1;
}
.cv-user {
  display: flex;
  align-items: center;
  gap: 8px;
}
.cv-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--fluent-text-secondary, rgba(255, 255, 255, 0.85));
}
.cv-loc {
  font-size: 11px;
  color: var(--fluent-text-secondary, rgba(255, 255, 255, 0.5));
}
.cv-text {
  margin-top: 4px;
  font-size: 14px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}
.cv-reply {
  margin-top: 8px;
  padding: 8px 12px;
  border-radius: 8px;
  background: var(--fluent-bg-hover, rgba(255, 255, 255, 0.05));
  font-size: 13px;
  line-height: 1.6;
}
.cv-reply-name {
  color: var(--fluent-accent, #4a90d9);
}
.cv-meta {
  margin-top: 6px;
  display: flex;
  align-items: center;
  gap: 14px;
  font-size: 12px;
  color: var(--fluent-text-secondary, rgba(255, 255, 255, 0.5));
}
.cv-like {
  display: inline-flex;
  align-items: center;
  gap: 3px;
}
.cv-empty,
.cv-loading,
.cv-error {
  text-align: center;
  padding: 48px 16px;
  color: var(--fluent-text-secondary, rgba(255, 255, 255, 0.6));
}
.cv-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}
.cv-spinner {
  width: 28px;
  height: 28px;
  border: 3px solid var(--fluent-border, rgba(255, 255, 255, 0.15));
  border-top-color: var(--fluent-accent, #4a90d9);
  border-radius: 50%;
  animation: cv-spin 0.8s linear infinite;
}
@keyframes cv-spin {
  to {
    transform: rotate(360deg);
  }
}
.cv-retry {
  margin-top: 12px;
  padding: 6px 18px;
  border-radius: 8px;
  border: none;
  background: var(--fluent-accent, #4a90d9);
  color: #fff;
  cursor: pointer;
}
.cv-pager {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 12px;
  border-top: 1px solid var(--fluent-border, rgba(255, 255, 255, 0.08));
  flex-shrink: 0;
}
.cv-page-btn {
  padding: 6px 16px;
  border-radius: 8px;
  border: none;
  background: var(--fluent-bg-hover, rgba(255, 255, 255, 0.08));
  color: inherit;
  cursor: pointer;
  transition: background 0.15s;
}
.cv-page-btn:hover:not(:disabled) {
  background: var(--fluent-bg-active, rgba(255, 255, 255, 0.14));
}
.cv-page-btn:disabled {
  opacity: 0.4;
  cursor: default;
}
.cv-page-info {
  font-size: 13px;
  color: var(--fluent-text-secondary, rgba(255, 255, 255, 0.7));
}
</style>
