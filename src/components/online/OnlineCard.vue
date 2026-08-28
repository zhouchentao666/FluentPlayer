<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Playlist } from '@online/lib/playlists'
import type { Album } from '@online/lib/albums'

const props = defineProps<{
  item: Playlist | Album
  kind: 'playlist' | 'album'
}>()
const emit = defineEmits<{ (e: 'open', item: Playlist | Album): void }>()

const coverFailed = ref(false)
const cover = computed(() => (coverFailed.value ? null : props.item.img))
const playCount = computed(() => (props.item as Playlist).playCount)
const initial = computed(() => (props.item.name || '?').charAt(0))

const sub = computed(() => {
  const it = props.item
  if (props.kind === 'album') {
    const parts = [it.author, it.publishTime].filter(Boolean)
    return parts.join(' · ')
  }
  const parts = [it.author]
  if (it.songCount) parts.push(`${it.songCount} 首`)
  return parts.filter(Boolean).join(' · ')
})
</script>

<template>
  <button class="online-card" @click="emit('open', item)">
    <div class="cover-wrap">
      <img v-if="cover" :src="cover" class="cover" alt="" loading="lazy"
           @error="coverFailed = true" />
      <div v-else class="cover placeholder">{{ initial }}</div>
      <span v-if="playCount && kind === 'playlist'" class="play-count">
        ▶ {{ playCount }}
      </span>
    </div>
    <div class="meta">
      <div class="name" :title="item.name">{{ item.name }}</div>
      <div v-if="sub" class="sub" :title="sub">{{ sub }}</div>
    </div>
  </button>
</template>

<style scoped>
.online-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px;
  border: 1px solid transparent;
  border-radius: 12px;
  background: transparent;
  cursor: pointer;
  text-align: left;
  transition: background 0.15s ease, border-color 0.15s ease;
}
.online-card:hover {
  background: var(--fluent-bg-hover);
  border-color: var(--fluent-border);
}
.cover-wrap {
  position: relative;
  aspect-ratio: 1 / 1;
  border-radius: 10px;
  overflow: hidden;
  background: var(--fluent-bg-card);
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.18);
}
.cover {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 40px;
  font-weight: 700;
  color: var(--fluent-text-secondary);
  background: linear-gradient(135deg, var(--fluent-bg-active), var(--fluent-bg-card));
}
.play-count {
  position: absolute;
  right: 6px;
  bottom: 6px;
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
}
.meta {
  min-width: 0;
}
.name {
  font-size: 13px;
  font-weight: 600;
  color: var(--fluent-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sub {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}
</style>
