<script lang="ts" setup>
import '@applemusic-like-lyrics/core/style.css'
import { BackgroundRender as CoreBackgroundRender, PixiRenderer } from '@applemusic-like-lyrics/core'
import { ref, watch, onBeforeUnmount } from 'vue'

const props = defineProps<{
  coverUrl: string | null
  active: boolean
  hasLyrics?: boolean
}>()

const containerRef = ref<HTMLDivElement | null>(null)
const bgRef = ref<CoreBackgroundRender<PixiRenderer> | undefined>(undefined)

async function init() {
  if (!containerRef.value || bgRef.value) return

  bgRef.value = CoreBackgroundRender.new(PixiRenderer)
  const canvas = bgRef.value.getElement()
  canvas.style.position = 'absolute'
  canvas.style.top = '0'
  canvas.style.left = '0'
  canvas.style.width = '100%'
  canvas.style.height = '100%'
  containerRef.value.appendChild(canvas)

  bgRef.value.setRenderScale(0.5)
  bgRef.value.setFlowSpeed(1)
  bgRef.value.setFPS(30)
  bgRef.value.setHasLyric(props.hasLyrics ?? false)

  if (props.coverUrl) {
    await bgRef.value.setAlbum(props.coverUrl, false)
  }
}

function dispose() {
  if (!bgRef.value) return
  const canvas = bgRef.value.getElement()
  canvas?.parentNode?.removeChild(canvas)
  bgRef.value.dispose()
  bgRef.value = undefined
}

watch(
  () => props.active,
  async (active) => {
    if (active) {
      await init()
      bgRef.value?.resume()
    } else {
      bgRef.value?.pause()
    }
  },
  { immediate: true },
)

watch(
  () => props.coverUrl,
  async (url) => {
    if (!url || !bgRef.value) return
    await bgRef.value.setAlbum(url, false)
  },
)

onBeforeUnmount(dispose)
</script>

<template>
  <div ref="containerRef" class="dynamic-background"></div>
</template>

<style scoped>
.dynamic-background {
  position: absolute;
  inset: 0;
  z-index: 0;
  pointer-events: none;
}

.dynamic-background canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}
</style>
