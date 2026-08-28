<script lang="ts" setup>
import { ref, nextTick } from 'vue'
import type { SortMode, SortOrder } from '../composables/usePlaylistView'

const props = defineProps<{
  searchQuery: string
  sortMode: SortMode
  sortOrder: SortOrder
  batchMode: boolean
  sortLabels: Record<SortMode, string>
}>()

const emit = defineEmits<{
  'update:searchQuery': [value: string]
  'update:sortMode': [value: SortMode]
  'update:sortOrder': [value: SortOrder]
  'toggle-batch': []
}>()

const showSearch = ref(false)
const showSortMenu = ref(false)
const searchInputRef = ref<HTMLInputElement | null>(null)

function toggleSearch() {
  showSearch.value = !showSearch.value
  if (showSearch.value) {
    nextTick(() => searchInputRef.value?.focus())
  } else {
    emit('update:searchQuery', '')
  }
}

function updateSearch(e: Event) {
  emit('update:searchQuery', (e.target as HTMLInputElement).value)
}

function selectSort(mode: SortMode) {
  emit('update:sortMode', mode)
  if (mode === 'custom') {
    showSortMenu.value = false
  }
}

function selectOrder(order: SortOrder) {
  emit('update:sortOrder', order)
  showSortMenu.value = false
}
</script>

<template>
  <div class="toolbar">
    <div v-if="showSearch" class="search-box">
      <input
        ref="searchInputRef"
        type="text"
        class="search-input"
        placeholder="搜索歌曲、艺术家、专辑"
        :value="props.searchQuery"
        @input="updateSearch"
      />
    </div>

    <div class="icon-group">
      <button
        class="icon-btn"
        :class="{ active: props.batchMode }"
        title="批量选择"
        @click="emit('toggle-batch')"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="20 6 9 17 4 12" />
        </svg>
      </button>

      <div class="sort-menu-wrapper">
        <button
          class="icon-btn"
          :class="{ active: showSortMenu }"
          title="排序"
          @click="showSortMenu = !showSortMenu"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m3 8 4-4 4 4" />
            <path d="M7 4v16" />
            <path d="m11 16 4 4 4-4" />
            <path d="M15 20V4" />
          </svg>
        </button>

        <div v-if="showSortMenu" class="sort-dropdown">
          <div
            v-for="(label, mode) in props.sortLabels"
            :key="mode"
            class="sort-option"
            :class="{ active: props.sortMode === mode }"
            @click="selectSort(mode)"
          >
            <span class="check">
              <svg v-if="props.sortMode === mode" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12" />
              </svg>
            </span>
            <span>{{ label }}</span>
          </div>
          <div class="divider"></div>
          <div
            class="sort-option"
            :class="{ active: props.sortOrder === 'asc' }"
            @click="selectOrder('asc')"
          >
            <span class="check">
              <svg v-if="props.sortOrder === 'asc'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12" />
              </svg>
            </span>
            <span>升序</span>
          </div>
          <div
            class="sort-option"
            :class="{ active: props.sortOrder === 'desc' }"
            @click="selectOrder('desc')"
          >
            <span class="check">
              <svg v-if="props.sortOrder === 'desc'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12" />
              </svg>
            </span>
            <span>降序</span>
          </div>
        </div>
      </div>

      <button
        class="icon-btn"
        :class="{ active: showSearch }"
        title="搜索"
        @click="toggleSearch"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.3-4.3" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
}

.search-box {
  position: relative;
}

.search-input {
  width: 180px;
  padding: 7px 12px;
  border: 1px solid var(--fluent-border);
  border-radius: 8px;
  background: var(--fluent-bg-glass);
  color: var(--fluent-text);
  font-size: 13px;
  outline: none;
  transition: border-color 0.18s ease, background 0.18s ease;
}

.search-input::placeholder {
  color: var(--fluent-text-secondary);
}

.search-input:focus {
  border-color: var(--fluent-accent);
  background: var(--fluent-bg-card);
}

.icon-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.icon-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--fluent-border);
  border-radius: 8px;
  background: var(--fluent-bg-glass);
  color: var(--fluent-text-secondary);
  cursor: pointer;
  transition: background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
}

.icon-btn:hover {
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
}

.icon-btn.active {
  border-color: var(--fluent-accent);
  background: var(--fluent-accent);
  color: #fff;
}

.sort-menu-wrapper {
  position: relative;
}

.sort-dropdown {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 140px;
  padding: 6px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: var(--fluent-bg-card);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  z-index: 20;
}

.sort-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border-radius: 6px;
  font-size: 13px;
  color: var(--fluent-text);
  cursor: pointer;
  transition: background 0.15s ease;
}

.sort-option:hover {
  background: var(--fluent-bg-hover);
}

.sort-option.active {
  color: var(--fluent-accent);
}

.check {
  width: 16px;
  height: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.divider {
  height: 1px;
  margin: 6px 0;
  background: var(--fluent-border);
}
</style>
