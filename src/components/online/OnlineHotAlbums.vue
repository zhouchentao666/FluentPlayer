<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import type { Album, AlbumTag } from '@online/lib/albums'
import { getHotAlbums, getAlbumTags } from '@online/lib/albums'
import OnlinePlatformTabs from './OnlinePlatformTabs.vue'
import OnlineCard from './OnlineCard.vue'

const emit = defineEmits<{ (e: 'open', item: Album): void }>()

const platform = ref<'wy' | 'kw' | 'kg' | 'tx' | 'mg'>('wy')
const tagId = ref<string | null>(null)
const page = ref(1)
const items = ref<Album[]>([])
const tags = ref<AlbumTag[]>([])
const loading = ref(false)
const error = ref('')

async function loadTags() {
  try {
    const t = await getAlbumTags(platform.value)
    tags.value = [{ id: '', name: '全部' }, ...t]
  } catch {
    tags.value = [{ id: '', name: '全部' }]
  }
}

async function reload(p = 1) {
  loading.value = true
  error.value = ''
  page.value = p
  try {
    items.value = await getHotAlbums(platform.value, p, tagId.value || null)
  } catch (e: unknown) {
    error.value = (e as Error).message || '加载失败'
    items.value = []
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadTags()
  reload(1)
})

watch(platform, () => {
  tagId.value = null
  loadTags()
  reload(1)
})
watch(tagId, () => reload(1))
</script>

<template>
  <div class="hot-view">
    <div class="head">
      <h2 class="title">新碟上架</h2>
      <OnlinePlatformTabs v-model="platform" :options="[
        { id: 'wy', name: '网易云' },
        { id: 'tx', name: 'QQ音乐' },
        { id: 'kg', name: '酷狗' },
        { id: 'kw', name: '酷我' },
        { id: 'mg', name: '咪咕' },
      ]" />
    </div>

    <div class="tags">
      <button
        v-for="t in tags"
        :key="t.id"
        class="tag"
        :class="{ active: tagId === t.id }"
        @click="tagId = t.id"
      >
        {{ t.name }}
      </button>
    </div>

    <div v-if="loading" class="state">加载中…</div>
    <div v-else-if="error" class="state error">{{ error }}</div>
    <div v-else-if="items.length === 0" class="state">暂无专辑</div>

    <div v-else class="card-grid">
      <OnlineCard
        v-for="it in items"
        :key="it.id"
        :item="it"
        kind="album"
        @open="emit('open', $event as Album)"
      />
    </div>

    <div v-if="!loading && items.length" class="pager">
      <button :disabled="page <= 1" @click="reload(page - 1)">上一页</button>
      <span>第 {{ page }} 页</span>
      <button :disabled="items.length < 30" @click="reload(page + 1)">下一页</button>
    </div>
  </div>
</template>

<style scoped>
.hot-view {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 18px 28px 28px;
  height: 100%;
  overflow-y: auto;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fluent-text);
  margin: 0;
}
.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.tag {
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text-secondary);
  padding: 5px 12px;
  border-radius: 16px;
  font-size: 12px;
  cursor: pointer;
}
.tag.active {
  background: var(--fluent-accent);
  color: #fff;
  border-color: transparent;
}
.state {
  color: var(--fluent-text-secondary);
  text-align: center;
  padding: 40px 0;
}
.state.error {
  color: #f87171;
}
.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 14px;
}
.pager {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  margin-top: 8px;
  color: var(--fluent-text-secondary);
  font-size: 13px;
}
.pager button {
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  padding: 6px 14px;
  border-radius: 8px;
  cursor: pointer;
}
.pager button:disabled {
  opacity: 0.4;
  cursor: default;
}
</style>
