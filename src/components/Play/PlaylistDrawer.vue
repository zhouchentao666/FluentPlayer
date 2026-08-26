<script setup lang="ts">
import { ref, computed, nextTick, onUnmounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { MessagePlugin, DialogPlugin } from 'tdesign-vue-next'
import { DeleteIcon } from 'tdesign-icons-vue-next'
import { useVirtualList } from '@vueuse/core'
import type { SongList } from '@/types/audio'

const { t } = useI18n()

interface Props {
  show: boolean
  currentSongId: string | number | null | undefined
  fullScreenMode?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  fullScreenMode: false
})

const emit = defineEmits<{
  close: []
  playSong: [song: SongList]
}>()

const localUserStore = LocalUserDetailStore()
const { list } = storeToRefs(localUserStore)

// 虚拟滚动
const sourceList = ref([...list.value])

watch(list, (newVal) => { sourceList.value = [...newVal] })

const { list: visibleList, containerProps, wrapperProps } = useVirtualList(sourceList, {
  itemHeight: 66,
  overscan: 10
})

// 拖拽排序
const isDragSorting = ref(false)
const draggedIndex = ref(-1)
const dragOverIndex = ref(-1)
const longPressTimer = ref<number | null>(null)
const longPressDelay = 500
const dragStartY = ref(0)
const dragCurrentY = ref(0)
const dragThreshold = 10
const draggedSong = ref<any>(null)
const isDragStarted = ref(false)
const wasLongPressed = ref(false)
const autoScrollFrame = ref<number | null>(null)
const scrollSpeed = ref(0)
const originalList = ref<any[]>([])

// 悬停提示
const hoverTipVisible = ref(false)
const hoverTipIndex = ref(-1)
const hoverTimer = ref<number | null>(null)
const hoverDelay = 1500

const playlistSongsClass = computed(() => ({
  'playlist-songs': true,
  'drag-sorting': isDragSorting.value
}))

const formatTime = (seconds: number) => {
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

const scrollToCurrentSong = () => {
  if (!props.currentSongId) return
  nextTick(() => {
    const index = list.value.findIndex((song) => song.songmid === props.currentSongId)
    const container = containerProps.ref.value
    if (index !== -1 && container) {
      const itemHeight = 66
      const containerHeight = container.clientHeight
      let targetScrollTop = index * itemHeight - containerHeight / 2 + itemHeight / 2
      targetScrollTop = Math.max(0, targetScrollTop)
      container.scrollTo({ top: targetScrollTop, behavior: 'smooth' })
    }
  })
}

const handleClose = () => { emit('close') }

const handleMouseEnter = (index: number) => {
  if (isDragSorting.value) return
  if (hoverTimer.value) clearTimeout(hoverTimer.value)
  hoverTimer.value = window.setTimeout(() => {
    hoverTipVisible.value = true
    hoverTipIndex.value = index
  }, hoverDelay)
}

const handleMouseLeave = () => {
  if (hoverTimer.value) { clearTimeout(hoverTimer.value); hoverTimer.value = null }
  hoverTipVisible.value = false
  hoverTipIndex.value = -1
}

const hideTip = () => {
  hoverTipVisible.value = false
  hoverTipIndex.value = -1
  if (hoverTimer.value) { clearTimeout(hoverTimer.value); hoverTimer.value = null }
}

const currentOperatingSong = ref<any>(null)

let activeDragMoveHandler: ((e: MouseEvent | TouchEvent) => void) | null = null
let activeDragEndHandler: ((e: MouseEvent | TouchEvent) => void) | null = null

const cleanupDragListeners = () => {
  if (activeDragMoveHandler) {
    document.removeEventListener('mousemove', activeDragMoveHandler as any)
    document.removeEventListener('touchmove', activeDragMoveHandler as any)
    activeDragMoveHandler = null
  }
  if (activeDragEndHandler) {
    document.removeEventListener('mouseup', activeDragEndHandler as any)
    document.removeEventListener('touchend', activeDragEndHandler as any)
    activeDragEndHandler = null
  }
}

const handleMouseDown = (event: MouseEvent, index: number, song: any) => {
  const target = event.target as HTMLElement
  if (target.closest('.song-remove')) return
  handlePointerStart(event, index, song, false)
}

const handleTouchStart = (event: TouchEvent, index: number, song: any) => {
  const target = event.target as HTMLElement
  if (target.closest('.song-remove')) return
  handlePointerStart(event, index, song, true)
}

const handlePointerStart = (event: MouseEvent | TouchEvent, index: number, song: any, isTouch: boolean) => {
  event.preventDefault()
  event.stopPropagation()
  cleanupDragListeners()
  isDragStarted.value = false
  wasLongPressed.value = false
  currentOperatingSong.value = song
  if (longPressTimer.value) clearTimeout(longPressTimer.value)

  const clientY = isTouch ? (event as TouchEvent).touches[0].clientY : (event as MouseEvent).clientY
  dragStartY.value = clientY
  dragCurrentY.value = clientY

  longPressTimer.value = window.setTimeout(() => {
    wasLongPressed.value = true
    startDragSort(index, song)
    isDragStarted.value = true
  }, longPressDelay)

  const handleMove = (e: MouseEvent | TouchEvent) => {
    const currentY = 'touches' in e ? (e as TouchEvent).touches[0].clientY : (e as MouseEvent).clientY
    const deltaY = Math.abs(currentY - dragStartY.value)
    if (deltaY > dragThreshold && longPressTimer.value) { clearTimeout(longPressTimer.value); longPressTimer.value = null }
    if (isDragSorting.value) { dragCurrentY.value = currentY; updateDragOverIndex(currentY) }
  }

  const handleEnd = () => {
    const hadLongPressTimer = !!longPressTimer.value
    const wasInDragMode = isDragSorting.value
    if (longPressTimer.value) { clearTimeout(longPressTimer.value); longPressTimer.value = null }
    if (isDragSorting.value) endDragSort()
    if (!wasLongPressed.value && !wasInDragMode && hadLongPressTimer && currentOperatingSong.value) {
      setTimeout(() => { emit('playSong', currentOperatingSong.value); wasLongPressed.value = false; isDragStarted.value = false; currentOperatingSong.value = null }, 10)
    } else {
      setTimeout(() => { wasLongPressed.value = false; isDragStarted.value = false; currentOperatingSong.value = null }, 200)
    }
    cleanupDragListeners()
  }

  activeDragMoveHandler = handleMove
  activeDragEndHandler = handleEnd
  document.addEventListener('mousemove', handleMove)
  document.addEventListener('mouseup', handleEnd)
  document.addEventListener('touchmove', handleMove, { passive: false })
  document.addEventListener('touchend', handleEnd)
}

const startDragSort = (index: number, song: any) => {
  hideTip()
  isDragSorting.value = true
  draggedIndex.value = index
  draggedSong.value = song
  dragOverIndex.value = index
  originalList.value = [...list.value]
  document.body.style.userSelect = 'none'
  if ('vibrate' in navigator) navigator.vibrate(50)
}

const updateDragOverIndex = (clientY: number) => {
  const container = containerProps.ref.value
  if (!container) return
  const containerRect = container.getBoundingClientRect()
  const scrollThreshold = 80
  const maxScrollSpeed = 15
  const distanceFromTop = clientY - containerRect.top
  const distanceFromBottom = containerRect.bottom - clientY
  const canScrollUp = container.scrollTop > 0
  const canScrollDown = container.scrollTop < container.scrollHeight - container.clientHeight

  if (distanceFromTop < scrollThreshold && distanceFromTop > 0 && canScrollUp) {
    scrollSpeed.value = -((scrollThreshold - distanceFromTop) / scrollThreshold) * maxScrollSpeed
    startAutoScroll()
  } else if (distanceFromBottom < scrollThreshold && distanceFromBottom > 0 && canScrollDown) {
    scrollSpeed.value = ((scrollThreshold - distanceFromBottom) / scrollThreshold) * maxScrollSpeed
    startAutoScroll()
  } else { stopAutoScroll() }

  const playlistSongs = document.querySelectorAll('.playlist-song')
  let newOverIndex = draggedIndex.value
  if (clientY >= containerRect.top && clientY <= containerRect.bottom) {
    for (let i = 0; i < playlistSongs.length; i++) {
      const songElement = playlistSongs[i] as HTMLElement
      const rect = songElement.getBoundingClientRect()
      const centerY = rect.top + rect.height / 2
      if (clientY < centerY) { if (visibleList.value[i]) newOverIndex = visibleList.value[i].index; break }
      else if (i === playlistSongs.length - 1) { if (visibleList.value[i]) newOverIndex = visibleList.value[i].index + 1 }
    }
  } else if (clientY < containerRect.top) { newOverIndex = 0 }
  else if (clientY > containerRect.bottom) { newOverIndex = playlistSongs.length }

  if (newOverIndex !== dragOverIndex.value && newOverIndex >= 0 && newOverIndex <= list.value.length) {
    dragOverIndex.value = newOverIndex
    updatePreviewList()
  }
}

const updatePreviewList = () => {
  if (draggedIndex.value === -1 || dragOverIndex.value === -1) return
  const newList = [...list.value]
  const draggedItem = newList.splice(draggedIndex.value, 1)[0]
  let insertIndex = dragOverIndex.value
  if (dragOverIndex.value > draggedIndex.value) insertIndex = dragOverIndex.value - 1
  newList.splice(insertIndex, 0, draggedItem)
  list.value = newList
  draggedIndex.value = insertIndex
}

const startAutoScroll = () => {
  if (autoScrollFrame.value !== null) return
  const tick = () => {
    const container = containerProps.ref.value
    if (container && scrollSpeed.value !== 0) {
      container.scrollTop += scrollSpeed.value
      if (isDragSorting.value) updateDragOverIndex(dragCurrentY.value)
      autoScrollFrame.value = requestAnimationFrame(tick)
      return
    }
    autoScrollFrame.value = null
  }
  autoScrollFrame.value = requestAnimationFrame(tick)
}

const stopAutoScroll = () => {
  if (autoScrollFrame.value !== null) {
    cancelAnimationFrame(autoScrollFrame.value)
    autoScrollFrame.value = null
  }
  scrollSpeed.value = 0
}

const endDragSort = () => {
  stopAutoScroll()
  isDragSorting.value = false
  draggedIndex.value = -1
  dragOverIndex.value = -1
  draggedSong.value = null
  isDragStarted.value = false
  wasLongPressed.value = false
  document.body.style.userSelect = ''
}

onUnmounted(() => {
  if (hoverTimer.value) clearTimeout(hoverTimer.value)
  if (longPressTimer.value) clearTimeout(longPressTimer.value)
  cleanupDragListeners()
  stopAutoScroll()
})

const handleClearPlaylist = () => {
  if (list.value.length === 0) { MessagePlugin.warning(t('play.playlistAlreadyEmpty')); return }
  localUserStore.clearList()
  MessagePlugin.success(t('play.playlistCleared'))
}

const handleClearConfirm = () => {
  if (list.value.length === 0) { MessagePlugin.warning(t('play.playlistAlreadyEmpty')); return }
  emit('close')
  nextTick(() => {
    const dialog = DialogPlugin.confirm({
      header: t('play.clearPlaylistConfirmTitle'),
      body: t('play.clearPlaylistConfirmBody'),
      confirmBtn: { content: t('play.confirmClear'), theme: 'danger' },
      cancelBtn: t('common.cancel'),
      onConfirm: () => {
        localUserStore.clearList()
        MessagePlugin.success(t('play.playlistCleared'))
        dialog.destroy()
      }
    })
  })
}

const handleLocateCurrentSong = () => {
  if (!props.currentSongId) { MessagePlugin.info(t('play.noSongPlaying')); return }
  const currentSongExists = list.value.some((song) => song.songmid === props.currentSongId)
  if (!currentSongExists) { MessagePlugin.warning(t('play.currentSongNotInPlaylist')); return }
  scrollToCurrentSong()
}

defineExpose({ scrollToCurrentSong })
</script>

<template>
  <transition name="playlist-mask">
    <div v-show="show" class="cover" @click="handleClose"></div>
  </transition>
  <transition name="playlist-drawer">
    <div
      v-show="show"
      class="playlist-container"
      :class="{ 'full-screen-mode': fullScreenMode }"
      @click.stop
    >
      <div class="playlist-header">
        <div class="playlist-title">{{ t('play.playlistTitle') }} ({{ list.length }})</div>
        <button class="playlist-close" @click.stop="handleClose">
          <span class="iconfont icon-guanbi"></span>
        </button>
      </div>

      <div class="playlist-content" v-bind="containerProps">
        <div v-if="list.length === 0" class="playlist-empty">
          <p>{{ t('play.playlistEmpty') }}</p>
          <p>{{ t('play.playlistEmptyTip') }}</p>
        </div>
        <div v-else :class="playlistSongsClass" :style="wrapperProps.style">
          <div
            v-for="item in visibleList"
            :key="`${item.data.songmid}-${item.index}`"
            class="playlist-song"
            :class="{
              active: item.data.songmid === currentSongId,
              dragging: isDragSorting && item.index === draggedIndex
            }"
            @mousedown="handleMouseDown($event, item.index, item.data)"
            @touchstart="handleTouchStart($event, item.index, item.data)"
            @mouseenter="handleMouseEnter(item.index)"
            @mouseleave="handleMouseLeave"
          >
            <div v-if="isDragSorting && item.index === draggedIndex" class="drag-handle">
              <span class="drag-dots">⋮⋮</span>
            </div>
            <div v-else class="song-index">{{ (item.index + 1).toString().padStart(2, '0') }}</div>

            <div class="song-info">
              <div class="song-name">{{ item.data.name }}</div>
              <div class="song-artist">{{ item.data.singer }}</div>
            </div>
            <div class="song-actions">
              <div class="song-duration">
                {{
                  (item.data.interval || '').includes(':')
                    ? item.data.interval
                    : formatTime(parseInt(item.data.interval || '0') / 1000)
                }}
              </div>
              <button class="song-remove" @click.stop="localUserStore.removeSong(item.data.songmid)">
                <span class="iconfont icon-xuanxiangshanchu"></span>
              </button>
            </div>

            <transition name="hover-tip">
              <div v-if="hoverTipVisible && hoverTipIndex === item.index" class="hover-tip" @click.stop>
                {{ t('play.longPressToSort') }}
              </div>
            </transition>
          </div>
        </div>
      </div>

      <div v-if="list.length > 0" class="playlist-footer">
        <button class="playlist-action-btn locate-btn" :disabled="!currentSongId" @click="handleLocateCurrentSong">
          <span class="iconfont icon-dingwei" style="font-size: 14px"></span>
          <span>{{ t('play.locateCurrentPlay') }}</span>
        </button>
        <button class="playlist-action-btn clear-btn" @click="handleClearConfirm">
          <DeleteIcon size="16" />
          <span>{{ t('play.clearPlaylist') }}</span>
        </button>
      </div>
    </div>
  </transition>
</template>

<style lang="scss" scoped>
.playlist-container {
  position: fixed;
  border-radius: 16px 0 0 16px;
  top: 72px;
  right: 0;
  width: 380px;
  height: calc(100vh - var(--play-bottom-height) - 80px);
  transition: background-color 0.3s ease, color 0.3s ease;
  background: color-mix(in srgb, var(--td-bg-color-container) 60%, transparent);
  backdrop-filter: blur(8px);
  box-shadow: -3px 0 12px color-mix(in srgb, var(--td-text-color-primary) 15%, transparent);
  z-index: 9001;
  display: flex;
  flex-direction: column;
  color: var(--td-text-color-primary);
}

.cover {
  position: fixed;
  background-color: transparent;
  width: 100vw;
  height: 100vh;
  z-index: 9000;
  bottom: 0px;
  right: 0;
}

.playlist-container.full-screen-mode {
  background: color-mix(in srgb, var(--td-bg-color-container) 20%, transparent);
  color: var(--td-text-color-primary);
}
.playlist-container.full-screen-mode .song-artist,
.playlist-container.full-screen-mode .song-duration,
.playlist-container.full-screen-mode .playlist-close,
.playlist-container.full-screen-mode .song-remove { color: var(--td-text-color-secondary); }

.playlist-container .playlist-content {
  scrollbar-arrow-color: transparent;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--td-text-color-placeholder) 30%, transparent) transparent;
}
.playlist-container.full-screen-mode .playlist-content {
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--td-text-color-primary) 30%, transparent) transparent;
}
.playlist-container.full-screen-mode .playlist-content::-webkit-scrollbar { width: 8px; }
.playlist-container.full-screen-mode .playlist-content::-webkit-scrollbar-track { background: transparent; }
.playlist-container.full-screen-mode .playlist-content::-webkit-scrollbar-thumb { background: color-mix(in srgb, var(--td-text-color-primary) 30%, transparent); border-radius: 4px; }
.playlist-container.full-screen-mode .playlist-content::-webkit-scrollbar-thumb:hover { background: color-mix(in srgb, var(--td-text-color-primary) 50%, transparent); }
.playlist-container.full-screen-mode .playlist-song:hover { background-color: color-mix(in srgb, var(--td-text-color-primary) 10%, transparent); }
.playlist-container.full-screen-mode .playlist-song.active { border-left: color-mix(in srgb, var(--td-brand-color) 36%, transparent) 4px solid; background-color: color-mix(in srgb, var(--td-brand-color) 12%, transparent); }
.playlist-container .playlist-song.active { border-left: color-mix(in srgb, var(--td-brand-color) 58%, transparent) 4px solid; background-color: color-mix(in srgb, var(--td-brand-color) 12%, transparent); }
.playlist-container.full-screen-mode .playlist-header { border-bottom: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent); }
.playlist-container.full-screen-mode .playlist-empty { color: var(--td-text-color-secondary); }

.playlist-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  flex-shrink: 0;
  border-bottom: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
}
.playlist-title { font-size: 16px; font-weight: 600; }
.playlist-close {
  background: transparent;
  border: none;
  color: var(--td-text-color-secondary);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.playlist-content { flex: 1; overflow-y: auto; margin: 10px 0; padding: 0 8px; }
.playlist-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100px;
  color: var(--td-text-color-secondary);
  font-size: 14px;
  text-align: center;
}
.playlist-songs { display: flex; flex-direction: column; position: relative; }
.playlist-songs.drag-sorting .playlist-song:not(.dragging) { transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1); }
.playlist-songs.drag-sorting .playlist-song.dragging { transition: none; z-index: 1000; opacity: 0.8; transform: scale(1.02); box-shadow: 0 4px 12px color-mix(in srgb, var(--td-text-color-primary) 15%, transparent); }

.playlist-song {
  display: flex;
  align-items: center;
  padding: 0 16px;
  cursor: pointer;
  border-radius: 10px;
  margin: 5px 0;
  height: 56px;
  box-sizing: border-box;
  transition: background-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  user-select: none;
  transform: translateY(0);
}
.playlist-song:hover { background-color: color-mix(in srgb, var(--td-text-color-primary) 8%, transparent); }
.playlist-song.active { background-color: color-mix(in srgb, var(--td-brand-color) 8%, transparent); }
.playlist-song.dragging { opacity: 0.8; transform: scale(1.02); box-shadow: 0 4px 12px color-mix(in srgb, var(--td-text-color-primary) 15%, transparent); z-index: 1000; background-color: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent) !important; }

.drag-handle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  margin-right: 8px;
  cursor: grab;
  color: var(--td-text-color-placeholder);
}
.song-index {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  margin-right: 15px;
  color: var(--td-text-color-placeholder);
  font-size: 13px;
  font-family: 'DIN Alternate', 'Roboto', sans-serif;
  font-weight: 500;
}
.playlist-container.full-screen-mode .song-index { color: var(--td-text-color-secondary); }
.drag-handle:active { cursor: grabbing; }
.drag-dots { font-size: 16px; line-height: 1; letter-spacing: -2px; transform: rotate(90deg); }

.playlist-songs.drag-sorting { pointer-events: none; }
.playlist-songs.drag-sorting .playlist-song { pointer-events: auto; }
.playlist-songs.drag-sorting .playlist-song:not(.dragging) { cursor: default; }

.playlist-song .song-info { flex: 1; min-width: 0; }
.playlist-song .song-name { font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.playlist-container.full-screen-mode .playlist-song .song-name { color: var(--td-text-color-primary); }
.playlist-song .song-artist { font-size: 12px; color: var(--td-text-color-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.playlist-song .song-duration {
  font-size: 12px;
  color: var(--td-text-color-placeholder);
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%) scale(1);
  transition: opacity 0.2s ease, transform 0.2s ease;
  opacity: 1;
}
.playlist-song:hover .song-duration { opacity: 0; transform: translate(-50%, -50%) scale(0.8); }
.playlist-song .song-remove {
  background: transparent;
  border: none;
  color: var(--td-text-color-placeholder);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%) scale(0.8);
  transition: opacity 0.2s ease, transform 0.2s ease;
  opacity: 0;
  pointer-events: none;
}
.playlist-song:hover .song-remove { opacity: 1; transform: translate(-50%, -50%) scale(1); pointer-events: auto; }
.playlist-song:hover .song-remove:hover { color: var(--td-error-color, #e5484d); border-radius: 4px; }
.song-actions { position: relative; width: 60px; height: 100%; flex-shrink: 0; }

.hover-tip {
  position: absolute;
  top: 50%;
  right: 70px;
  transform: translateY(-50%);
  background: color-mix(in srgb, var(--td-text-color-primary) 60%, transparent);
  color: var(--td-bg-color-container);
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  white-space: nowrap;
  pointer-events: none;
  z-index: 1001;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--td-text-color-primary) 20%, transparent);
}
.playlist-container.full-screen-mode .hover-tip { background: color-mix(in srgb, var(--td-bg-color-container) 60%, transparent); color: var(--td-text-color-primary); }
.hover-tip-enter-active, .hover-tip-leave-active { transition: opacity 0.2s ease, transform 0.2s ease; }
.hover-tip-enter-from, .hover-tip-leave-to { opacity: 0; transform: translateY(-50%) scale(0.9); }

.playlist-drawer-enter-active, .playlist-drawer-leave-active { transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1); }
.playlist-drawer-enter-from, .playlist-drawer-leave-to { transform: translateX(100%); }

.playlist-mask-enter-active, .playlist-mask-leave-active { transition: opacity 0.25s ease; }
.playlist-mask-enter-from, .playlist-mask-leave-to { opacity: 0; }

.playlist-footer {
  display: flex;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  flex-shrink: 0;
}
.playlist-container.full-screen-mode .playlist-footer { border-top: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent); }

.playlist-action-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 12px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s ease, border-color 0.2s ease, transform 0.2s ease;
  backdrop-filter: blur(10px);
}
.locate-btn { background: color-mix(in srgb, var(--td-brand-color) 10%, transparent); color: var(--td-brand-color); border: 1px solid color-mix(in srgb, var(--td-brand-color) 20%, transparent); }
.locate-btn:hover:not(:disabled) { background: color-mix(in srgb, var(--td-brand-color) 15%, transparent); border-color: color-mix(in srgb, var(--td-brand-color) 30%, transparent); transform: translateY(-1px); }
.locate-btn:disabled { opacity: 0.5; cursor: not-allowed; color: var(--td-text-color-placeholder); background: color-mix(in srgb, var(--td-text-color-primary) 5%, transparent); border-color: color-mix(in srgb, var(--td-text-color-primary) 10%, transparent); }
.clear-btn { background: color-mix(in srgb, var(--td-error-color, #e5484d) 10%, transparent); color: var(--td-error-color, #e5484d); border: 1px solid color-mix(in srgb, var(--td-error-color, #e5484d) 20%, transparent); }
.clear-btn:hover { background: color-mix(in srgb, var(--td-error-color, #e5484d) 15%, transparent); border-color: color-mix(in srgb, var(--td-error-color, #e5484d) 30%, transparent); transform: translateY(-1px); }

@media (max-width: 768px) {
  .cover {
    height: 100dvh;
    background: var(--mobile-scrim);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    z-index: calc(var(--mobile-overlay-layer-z) - 1);
  }

  .playlist-container {
    top: auto;
    right: 0;
    bottom: 0;
    width: 100%;
    height: min(76dvh, 620px);
    max-height: calc(100dvh - var(--mobile-safe-top) - 12px);
    border-radius: var(--mobile-card-radius) var(--mobile-card-radius) 0 0;
    background: var(--mobile-glass-bg-strong);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    border: 0.5px solid var(--mobile-glass-border);
    border-bottom: none;
    box-shadow: var(--mobile-surface-shadow);
    z-index: var(--mobile-overlay-layer-z);
    overflow: hidden;
  }

  .playlist-container::before {
    content: '';
    width: 38px;
    height: 4px;
    border-radius: 999px;
    background: rgba(120, 120, 128, 0.36);
    align-self: center;
    margin-top: 8px;
    flex-shrink: 0;
  }

  .playlist-header {
    padding: 12px var(--mobile-page-gutter);
  }

  .playlist-title {
    font-size: 17px;
  }

  .playlist-close {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    background: rgba(120, 120, 128, 0.12);
    touch-action: manipulation;
  }

  .playlist-content {
    margin: 6px 0;
    padding: 0 10px;
    -webkit-overflow-scrolling: touch;
  }

  .playlist-empty {
    height: 180px;
    padding: 0 var(--mobile-page-gutter);
    line-height: 1.5;
  }

  .playlist-song {
    height: 60px;
    min-height: 60px;
    margin: 4px 0;
    padding: 0 12px;
    border-radius: var(--mobile-card-radius-small);
    touch-action: manipulation;
  }

  .song-index,
  .drag-handle {
    margin-right: 12px;
  }

  .song-actions {
    width: 88px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
  }

  .playlist-song .song-duration,
  .playlist-song:hover .song-duration {
    position: static;
    transform: none;
    opacity: 0.72;
    flex-shrink: 0;
  }

  .playlist-song .song-remove,
  .playlist-song:hover .song-remove {
    position: static;
    min-width: 36px;
    min-height: 36px;
    border-radius: var(--mobile-control-radius);
    background: color-mix(in srgb, var(--td-error-color, #e5484d) 10%, transparent);
    color: var(--td-error-color, #e5484d);
    transform: none;
    opacity: 1;
    pointer-events: auto;
    flex-shrink: 0;
    touch-action: manipulation;
  }

  .hover-tip {
    display: none;
  }

  .playlist-footer {
    padding: 10px var(--mobile-page-gutter) calc(var(--mobile-safe-bottom) + 12px);
    gap: 8px;
  }

  .playlist-action-btn {
    min-height: var(--mobile-touch-target);
    padding: 0 12px;
    border-radius: var(--mobile-control-radius);
    font-size: 13px;
    touch-action: manipulation;
  }

  .playlist-drawer-enter-from,
  .playlist-drawer-leave-to {
    transform: translateY(100%);
  }
}

@media (prefers-reduced-motion: reduce) {
  .playlist-drawer-enter-active,
  .playlist-drawer-leave-active,
  .playlist-mask-enter-active,
  .playlist-mask-leave-active,
  .playlist-songs.drag-sorting .playlist-song:not(.dragging),
  .playlist-song {
    transition: none !important;
  }
}
</style>
