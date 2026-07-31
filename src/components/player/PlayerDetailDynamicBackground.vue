<script lang="ts" setup>
import '@applemusic-like-lyrics/core/style.css'
import { BackgroundRender as CoreBackgroundRender, PixiRenderer } from '@applemusic-like-lyrics/core'
import { ref, watch, onBeforeUnmount, inject, type Ref } from 'vue'
import type { AppSettings } from '../../composables/useConfig'
import { toDisplayableCover } from '@online/lib/coverProxy'

const props = defineProps<{
  coverUrl: string | null
  active: boolean
  hasLyrics?: boolean
}>()

const settings = inject<Ref<AppSettings>>('settings')

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
  bgRef.value.setFlowSpeed(settings?.value.lyricFlowSpeed ?? 2)
  bgRef.value.setFPS(settings?.value.lyricFps ?? 30)
  bgRef.value.setHasLyric(props.hasLyrics ?? false)

  if (props.coverUrl) {
    await applyCover(props.coverUrl)
  }
}

// 在线封面多为跨域远程地址，Pixi 的 WebGL 纹理会因 CORS 而加载失败（背景变黑）。
// 这里先经 Tauri http 代理转成 data URL，再交给动态背景。
async function applyCover(url: string | null) {
  if (!bgRef.value || !url) return
  const display = await toDisplayableCover(url)
  if (display) await bgRef.value.setAlbum(display, false)
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

watch(() => props.coverUrl, applyCover)

watch(
  () => [settings?.value.lyricFlowSpeed, settings?.value.lyricFps],
  () => {
    if (!bgRef.value) return
    bgRef.value.setFlowSpeed(settings?.value.lyricFlowSpeed ?? 2)
    bgRef.value.setFPS(settings?.value.lyricFps ?? 30)
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
