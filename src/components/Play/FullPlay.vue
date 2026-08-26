<script lang="ts" setup>
import '@applemusic-like-lyrics/core/style.css'
import {
  BackgroundRender as CoreBackgroundRender,
  PixiRenderer
} from '@applemusic-like-lyrics/core'
import { LyricPlayer, type LyricPlayerRef } from '@applemusic-like-lyrics/vue'
import type { SongList } from '@/types/audio'
import { ref, computed, onMounted, watch, reactive, onBeforeUnmount, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { PerformanceDegrader } from '@/utils/performanceMonitor'
import { ControlAudioStore } from '@/store/ControlAudio'
import {
  Fullscreen1Icon,
  FullscreenExit1Icon,
  ChevronDownIcon,
  PenBallIcon
} from 'tdesign-icons-vue-next'
import debounce from 'lodash/debounce'
import { storeToRefs } from 'pinia'
import { useSettingsStore } from '@/store/Settings'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import { useDlnaStore } from '@/store/dlna'
import { invoke } from '@tauri-apps/api/core'
import { MessagePlugin } from 'tdesign-vue-next'
import { usePlaySettingStore } from '@/store'
import PlaySettings from './PlaySettings.vue'
import TitleBarControls from '@/components/TitleBarControls.vue'
import CommentsOverlay from './CommentsOverlay.vue'

const { t } = useI18n()

const playSetting = usePlaySettingStore()
const settingsStore = useSettingsStore()
const dlnaStore = useDlnaStore()
const globalPlayStatus = useGlobalPlayStatusStore()
const { player } = storeToRefs(globalPlayStatus)
const showSettings = ref(false)
const showMobileLyrics = ref(false)
const router = useRouter()

const toggleMobileLyrics = (event: MouseEvent) => {
  if (!window.matchMedia('(max-width: 768px)').matches) return
  const target = event.target as HTMLElement
  if (target.closest('button, a, input, textarea, select, [role="button"], .artist.singer-link')) return
  showMobileLyrics.value = !showMobileLyrics.value
}

function goToSinger() {
  const song = player.value.songInfo as any
  if (song?.singerId && song?.source && song.source !== 'local') {
    router.push({ name: 'singer', params: { id: song.singerId }, query: { source: song.source } })
  }
}

const lyricFontSize = computed(() => {
  const rate = settingsStore.settings.FullPlayLyricFontRate || 1.0
  return `calc(min(clamp(30px, 2.5vw, 50px), 5vh) * ${rate})`
})
const lyricFontWeight = computed(() => settingsStore.settings.lyricFontWeight || 600)
const lyricFontFamily = computed(
  () => settingsStore.settings.lyricFontFamily || 'lyricfont'
)

const safeLyricLines = computed(() => player.value.lyrics.lines || [])

const showLeftPanel = computed({
  get: () => playSetting.getShowLeftPanel,
  set: (val) => playSetting.setShowLeftPanel(val)
})

interface Props {
  show?: boolean
  showComments?: boolean
  coverImage?: string
  songId?: string | null
  songInfo: SongList | { songmid: number | null | string; lrc: string | null }
  mainColor: string
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  showComments: false,
  coverImage: '/src/assets/images/Default.jpg',
  songId: '',
  mainColor: '#rgb(0,0,0)'
})

const emit = defineEmits(['toggle-fullscreen', 'idle-change', 'update:showComments'])

// 全屏状态
const isFullscreen = ref(false)
const isAnimating = ref(false)
let animatingTimer: any = null

// 自动隐藏
const isIdle = ref(false)
const isHide = ref(false)
let idleTimer: any = null
const isMobileViewport = () => window.matchMedia('(max-width: 768px)').matches

const resetIdleTimer = () => {
  if (isMobileViewport()) {
    if (idleTimer) clearTimeout(idleTimer)
    isIdle.value = false
    emit('idle-change', false)
    return
  }
  if (isHide.value) return
  if (!playSetting.getAutoHideBottom) {
    isIdle.value = false
    emit('idle-change', false)
    return
  }
  if (isIdle.value) {
    isIdle.value = false
    emit('idle-change', false)
  }
  if (idleTimer) clearTimeout(idleTimer)
  if (props.show) {
    idleTimer = setTimeout(() => {
      if (props.show && !isMobileViewport() && playSetting.getAutoHideBottom && !showSettings.value) {
        isIdle.value = true
        emit('idle-change', true)
      }
    }, 3000)
  }
}

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'F1') {
    e.preventDefault()
    isHide.value = !isHide.value
    isIdle.value = isHide.value
    emit('idle-change', isHide.value)
  }
}

watch(
  () => props.show,
  (val) => {
    isAnimating.value = true
    if (animatingTimer) clearTimeout(animatingTimer)
    animatingTimer = setTimeout(() => { isAnimating.value = false }, 300)
    if (val) {
      showMobileLyrics.value = false
      resetIdleTimer()
      window.addEventListener('mousemove', resetIdleTimer)
    } else {
      window.removeEventListener('mousemove', resetIdleTimer)
      if (idleTimer) clearTimeout(idleTimer)
      isIdle.value = false
      emit('idle-change', false)
    }
  },
  { immediate: true }
)

watch(() => playSetting.getAutoHideBottom, (val) => {
  if (!val) {
    if (idleTimer) clearTimeout(idleTimer)
    isIdle.value = false
    emit('idle-change', false)
  } else { resetIdleTimer() }
})

watch(() => showSettings.value, (val) => {
  if (val) {
    if (idleTimer) clearTimeout(idleTimer)
    isIdle.value = false
    emit('idle-change', false)
  } else { resetIdleTimer() }
})

// 全屏切换
const toggleFullscreen = () => {
  isFullscreen.value = !isFullscreen.value
  window.api.toggleFullscreen()
}

onMounted(() => {
  document.addEventListener('fullscreenchange', handleFullscreenChange)
})

const handleFullscreenChange = () => {
  isFullscreen.value = !!document.fullscreenElement
}

// 音频控制
const controlAudio = ControlAudioStore()
const { Audio } = storeToRefs(controlAudio)
const isAudioPlaying = ref(false)

const updatePlayState = () => {
  isAudioPlaying.value = Audio.value.isPlay
}

watch(
  () => Audio.value.isPlay,
  (playing) => {
    isAudioPlaying.value = playing
  },
  { immediate: true }
)

// 内部状态
const state = reactive({
  audioUrl: Audio.value.url,
  albumUrl: props.coverImage,
  currentTime: 0,
})

const bgRef = ref<CoreBackgroundRender<PixiRenderer> | undefined>(undefined)
const lyricPlayerRef = ref<LyricPlayerRef | undefined>(undefined)
const backgroundContainer = ref<HTMLDivElement | null>(null)

// 音频响应相关
let spectrumUnlisten: UnlistenFn | null = null
const backgroundConfig = computed(() => settingsStore.settings.backgroundRender?.fullPlay)
const audioResponseEnabled = computed(() => backgroundConfig.value?.audioResponse ?? true)

// 性能监控和自动降级
const performanceDegrader = new PerformanceDegrader()
const showPerformanceWarning = ref(false)
const hasAutoDegraded = ref(false)

const actualCoverImage = computed(() => {
  return player.value.cover || props.coverImage || '/src/assets/images/Default.jpg'
})

const jumpTime = (e: any) => {
  if (dlnaStore.currentDevice) {
    MessagePlugin.warning(t('play.screenCastingNoSeekFull'))
    return
  }
  const startTime = e?.line?.getLine?.()?.startTime ?? 0
  if (Audio.value.isPlay) invoke('player__seek', { position: startTime / 1000 })
}

// 封面变化 → 更新背景
watch(
  () => actualCoverImage.value,
  async (newImage) => {
    if (bgRef.value) {
      const renderer = bgRef.value as any
      const oldTexture = renderer.curContainer?.children?.[0]?.texture
      await bgRef.value.setAlbum(newImage, false)
      if (oldTexture) {
        setTimeout(() => {
          if (oldTexture.baseTexture && !oldTexture.baseTexture.destroyed) {
            try { oldTexture.destroy(true) } catch (e) { console.warn('Failed to clean up old album texture:', e) }
          }
        }, 2000)
      }
    }
  },
  { immediate: true }
)

// 电源阻止休眠
const blockerActive = ref(false)
watch(
  () => props.show,
  async (visible) => {
    try {
      if (visible && !blockerActive.value) {
        await (window as any)?.api?.powerSaveBlocker?.start?.()
        blockerActive.value = true
      } else if (!visible && blockerActive.value) {
        await (window as any)?.api?.powerSaveBlocker?.stop?.()
        blockerActive.value = false
      }
    } catch (e) {
      console.error('powerSaveBlocker 切换失败:', e)
    }
  },
  { immediate: true }
)

// 初始化背景渲染器
const isDocumentHidden = ref(document.hidden)
const isFullPlayActive = computed(() => props.show && !isDocumentHidden.value)
const shouldRunBackground = computed(() => isFullPlayActive.value && !!backgroundConfig.value?.enabled)

let performanceWarningTimer: number | null = null
const startPerformanceDegrader = () => {
  if (!settingsStore.settings.backgroundRender?.fullPlay?.enabled) return
  performanceDegrader.stop()
  performanceDegrader.start({
    onTick: () => {},
    onDegrade: (degradedConfig) => {
      console.warn('[FullPlay] 检测到性能问题，自动降低背景效果')
      hasAutoDegraded.value = true
      showPerformanceWarning.value = true

      // 应用降级配置
      settingsStore.updateSettings({
        backgroundRender: {
          ...settingsStore.settings.backgroundRender!,
          fullPlay: {
            ...settingsStore.settings.backgroundRender!.fullPlay,
            ...degradedConfig
          }
        }
      })

      // 3秒后自动隐藏提示
      if (performanceWarningTimer) clearTimeout(performanceWarningTimer)
      performanceWarningTimer = window.setTimeout(() => {
        showPerformanceWarning.value = false
        performanceWarningTimer = null
      }, 5000)
    },
    enabled: true
  })
}

const pauseBackgroundRender = () => {
  bgRef.value?.pause()
  stopAudioResponse()
  performanceDegrader.stop()
}

const disposeBackgroundRender = () => {
  stopAudioResponse()
  performanceDegrader.stop()
  if (!bgRef.value) return
  const canvas = bgRef.value.getElement()
  canvas?.parentNode?.removeChild(canvas)
  bgRef.value.dispose()
  bgRef.value = undefined
}

const resumeBackgroundRender = async () => {
  if (!shouldRunBackground.value) {
    pauseBackgroundRender()
    return
  }

  if (!bgRef.value) await initBackgroundRender()
  else {
    applyBackgroundConfig()
    bgRef.value.resume()
    if (audioResponseEnabled.value && Audio.value.isPlay) startAudioResponse()
  }

  if (bgRef.value) startPerformanceDegrader()
}

const initBackgroundRender = async () => {
  if (!shouldRunBackground.value) return
  if (!backgroundContainer.value) {
    console.warn('[FullPlay] backgroundContainer 不存在，跳过背景初始化')
    return
  }

  disposeBackgroundRender()
  bgRef.value = CoreBackgroundRender.new(PixiRenderer)
  const canvas = bgRef.value.getElement()
  canvas.style.position = 'absolute'
  canvas.style.top = '0'
  canvas.style.left = '0'
  canvas.style.width = '100%'
  canvas.style.height = '100%'
  canvas.style.zIndex = '-1'
  backgroundContainer.value.appendChild(canvas)

  // 应用配置
  applyBackgroundConfig()

  bgRef.value.setHasLyric(player.value.lyrics.lines.length > 10)
  await bgRef.value.setAlbum(actualCoverImage.value, false)
  bgRef.value.resume()

  if (audioResponseEnabled.value && Audio.value.isPlay) {
    startAudioResponse()
  }
}

// 应用背景配置到渲染器
const applyBackgroundConfig = () => {
  if (!bgRef.value || !backgroundConfig.value) return

  bgRef.value.setRenderScale(backgroundConfig.value.renderScale)
  bgRef.value.setFlowSpeed(backgroundConfig.value.flowSpeed)
  bgRef.value.setFPS(backgroundConfig.value.fps)
  bgRef.value.setStaticMode(backgroundConfig.value.staticMode)
}

// 启动音频响应效果
const startAudioResponse = async () => {
  if (!audioResponseEnabled.value || spectrumUnlisten || !shouldRunBackground.value) return

  try {
    spectrumUnlisten = await listen('player:spectrum', (event: any) => {
      if (!bgRef.value || !Audio.value.isPlay || !shouldRunBackground.value) return

      const { bands } = event.payload
      if (bands && Array.isArray(bands) && bands.length > 0) {
        // 提取低频能量（前 10 个频段）
        const lowFreqBands = bands.slice(0, 10)
        const avgLowFreq = lowFreqBands.reduce((sum: number, val: number) => sum + val, 0) / lowFreqBands.length

        // 转换为 0-1 范围（-80dB 到 0dB）
        const normalizedVolume = Math.max(0, Math.min(1, (avgLowFreq + 80) / 80))

        // 应用到背景渲染器
        bgRef.value.setLowFreqVolume(normalizedVolume)
      }
    })
  } catch (error) {
    console.error('[FullPlay] 启动音频响应失败:', error)
  }
}

// 停止音频响应效果
const stopAudioResponse = () => {
  if (spectrumUnlisten) {
    spectrumUnlisten()
    spectrumUnlisten = null
  }
  // 重置低频音量
  bgRef.value?.setLowFreqVolume(0)
}


onBeforeUnmount(async () => {
  document.removeEventListener('fullscreenchange', handleFullscreenChange)
  window.removeEventListener('mousemove', resetIdleTimer)
  window.removeEventListener('resize', debouncedCheckOverflow)
  document.removeEventListener('click', handleClickOutside)
  if (idleTimer) clearTimeout(idleTimer)
  if (animatingTimer) clearTimeout(animatingTimer)
  if (blockerActive.value) {
    try { await (window as any)?.api?.powerSaveBlocker?.stop?.() } catch {}
    blockerActive.value = false
  }
  if (Audio.value.isPlay) {
    // Rust 后端管理播放状态
  }
  // 停止音频响应
  if (performanceWarningTimer) { clearTimeout(performanceWarningTimer); performanceWarningTimer = null }
  disposeBackgroundRender()
  lyricPlayerRef.value?.lyricPlayer?.dispose()
})

watch(() => Audio.value.url, (newUrl) => { state.audioUrl = newUrl })
watch(() => Audio.value.currentTime, (newTime) => { state.currentTime = Math.round(newTime * 1000) })

// 监听播放状态，自动启停音频响应
watch(
  () => Audio.value.isPlay,
  (isPlaying) => {
    if (audioResponseEnabled.value && shouldRunBackground.value) {
      if (isPlaying) {
        startAudioResponse()
      } else {
        stopAudioResponse()
      }
    } else {
      stopAudioResponse()
    }
  }
)

// 监听音频响应配置变化
watch(
  () => audioResponseEnabled.value,
  (enabled) => {
    if (enabled && shouldRunBackground.value) {
      if (Audio.value.isPlay) {
        startAudioResponse()
      }
    } else {
      stopAudioResponse()
    }
  }
)

// 监听全屏播放可见性，避免隐藏状态持续渲染
watch(
  () => shouldRunBackground.value,
  (shouldRun) => {
    if (shouldRun) {
      resumeBackgroundRender()
    } else {
      pauseBackgroundRender()
    }
  },
  { immediate: true }
)
// 监听歌词变化，更新 hasLyric 状态
watch(
  () => player.value.lyrics.lines,
  (lines) => {
    if (bgRef.value) {
      bgRef.value.setHasLyric(lines.length > 10)
    }
  }
)

// 监听背景配置变化，动态更新渲染器
watch(
  [
    () => backgroundConfig.value?.enabled,
    () => backgroundConfig.value?.renderScale,
    () => backgroundConfig.value?.flowSpeed,
    () => backgroundConfig.value?.fps,
    () => backgroundConfig.value?.staticMode,
    () => backgroundConfig.value?.audioResponse,
  ],
  () => {
    const newConfig = backgroundConfig.value
    if (!newConfig) return

    if (!newConfig.enabled) {
      disposeBackgroundRender()
      return
    }

    if (!shouldRunBackground.value) {
      pauseBackgroundRender()
      return
    }

    if (!bgRef.value) {
      initBackgroundRender()
    } else {
      applyBackgroundConfig()

      if (newConfig.audioResponse && Audio.value.isPlay && !spectrumUnlisten) {
        startAudioResponse()
      } else if (!newConfig.audioResponse && spectrumUnlisten) {
        stopAudioResponse()
      }
    }
  }
)

const lightMainColor = computed(() => player.value.coverDetail.lightMainColor || 'rgba(255, 255, 255, 0.9)')
const useBlackText = computed(() => player.value.coverDetail.useBlackText)
const lyricViewColor = computed(() => playSetting.getIsImmersiveLyricColor ? lightMainColor.value : 'rgba(255, 255, 255, 1)')

// 滚动标题
const titleRef = ref<HTMLElement | null>(null)
const shouldScrollTitle = ref(false)
const titleContentRef = ref<HTMLElement | null>(null)

const songName = computed(() => {
  const info = player.value.songInfo
  if (info && 'name' in info && typeof info.name === 'string') return info.name
  return t('common.unknownSong')
})

const checkOverflow = async () => {
  await nextTick()
  if (titleRef.value && titleContentRef.value) {
    const containerWidth = titleRef.value.clientWidth
    const contentWidth = titleContentRef.value.offsetWidth
    shouldScrollTitle.value = contentWidth > containerWidth
  }
}

watch(() => [props.songInfo, props.show], checkOverflow, { immediate: true })

// 点击外部关闭设置面板
const floatActionRef = ref<HTMLElement | null>(null)
const handleClickOutside = (event: MouseEvent) => {
  if (showSettings.value && floatActionRef.value && !floatActionRef.value.contains(event.target as Node)) {
    showSettings.value = false
  }
}

// 后台暂停动画
const handleVisibilityChange = () => {
  isDocumentHidden.value = document.hidden
  if (shouldRunBackground.value) resumeBackgroundRender()
  else pauseBackgroundRender()
}

const handleWindowFocus = () => {
  if (shouldRunBackground.value) resumeBackgroundRender()
}

const debouncedCheckOverflow = debounce(checkOverflow, 200)

onMounted(() => {
  window.addEventListener('resize', debouncedCheckOverflow)
  document.addEventListener('click', handleClickOutside)
  window.addEventListener('keydown', handleKeyDown)
  document.addEventListener('visibilitychange', handleVisibilityChange)
  window.addEventListener('focus', handleWindowFocus)
  setTimeout(checkOverflow, 500)
  resumeBackgroundRender()
})

onUnmounted(() => {
  window.removeEventListener('resize', debouncedCheckOverflow)
  document.removeEventListener('click', handleClickOutside)
  window.removeEventListener('keydown', handleKeyDown)
  document.removeEventListener('visibilitychange', handleVisibilityChange)
  window.removeEventListener('focus', handleWindowFocus)
})
</script>

<template>
  <div
    class="full-play"
    :class="{
      active: props.show,
      'use-black-text': useBlackText,
      idle: isIdle,
      animating: isAnimating
    }"
  >
    <div
      ref="backgroundContainer"
      style="position: absolute; top: 0; left: 0; width: 100%; height: 100%; z-index: -1"
    ></div>

    <!-- 性能警告提示 -->
    <Transition name="fade">
      <div v-if="showPerformanceWarning" class="performance-warning-toast">
        <t-icon name="info-circle" size="1.2rem" />
        <span>{{ t('play.performanceWarning') }}</span>
        <button class="close-btn" @click="showPerformanceWarning = false">
          <t-icon name="close" size="1rem" />
        </button>
      </div>
    </Transition>
    <!-- 全屏按钮 -->
    <button
      class="fullscreen-btn"
      :class="{ 'black-text': useBlackText }"
      @click="toggleFullscreen"
    >
      <Fullscreen1Icon v-if="!isFullscreen" class="icon" />
      <FullscreenExit1Icon v-else class="icon" />
    </button>
    <button
      class="putawayscreen-btn"
      :class="{ 'black-text': useBlackText }"
      @click="emit('toggle-fullscreen')"
    >
      <ChevronDownIcon class="icon" />
    </button>
    <Transition name="fade-nav">
      <TitleBarControls
        v-if="props.show"
        class="top"
        data-tauri-drag-region
        :color="useBlackText ? 'black' : 'white'"
        :show-account="false"
      />
    </Transition>
    <div
      class="playbox"
      :style="{
        padding:
          playSetting.getLayoutMode === 'cover' || !playSetting.getShowLeftPanel
            ? '0 min(4.5vw, 100px)'
            : '0 10vw'
      }"
      :class="{
        'mode-cover': playSetting.getLayoutMode === 'cover',
        'single-column': !showLeftPanel,
        'mobile-show-lyrics': showMobileLyrics
      }"
      @click="toggleMobileLyrics"
    >
      <div
        class="left"
      >
        <!-- 黑胶模式 -->
        <template v-if="playSetting.getLayoutMode === 'cd'">
          <div
            class="cd-container"
            :class="{ playing: isAudioPlaying }"
            :style="
              !isAudioPlaying
                ? 'animation-play-state: paused;'
                : ''
            "
          >
            <img
              class="pointer"
              :class="{ playing: isAudioPlaying }"
              src="@/assets/pointer.png"
              alt="pointer"
            />
            <div
              class="vinyl-disc"
              :class="{ playing: isAudioPlaying }"
              :style="
                !isAudioPlaying
                  ? 'animation-play-state: paused;'
                  : ''
              "
            >
              <div class="vinyl-record"></div>
              <div class="vinyl-label">
                <img
                  v-if="playSetting.getLayoutMode === 'cd'"
                  :src="coverImage"
                  alt="cover"
                  class="cover"
                  style="view-transition-name: player-cover"
                />
                <div class="label-shine"></div>
              </div>
              <div class="center-hole"></div>
            </div>
          </div>
        </template>

        <!-- 封面模式 -->
        <template v-else-if="playSetting.getLayoutMode === 'cover'">
          <div class="cover-layout-container">
            <div class="cover-wrapper-square" :class="{ playing: controlAudio.Audio.isPlay }">
              <img
                v-if="playSetting.getLayoutMode === 'cover'"
                :src="actualCoverImage"
                class="cover-img-square"
                alt="cover"
                style="view-transition-name: player-cover"
              />
            </div>
            <div class="song-info-area">
              <div ref="titleRef" class="song-title-large text-scroll-container">
                <div class="text-scroll-wrapper" :class="{ 'animate-scroll': shouldScrollTitle }">
                  <div ref="titleContentRef" class="text-scroll-item">
                    {{ songName }}
                  </div>
                  <div v-if="shouldScrollTitle" class="text-scroll-item">
                    {{ songName }}
                  </div>
                </div>
              </div>
              <div class="song-meta-large">
                <span
                  class="artist"
                  :class="{ 'singer-link': (player.songInfo as any)?.singerId && (player.songInfo as any)?.source !== 'local' }"
                  @click="goToSinger()"
                >{{ (player.songInfo as any)?.singer }}</span>
                <span
                  v-if="(player.songInfo as any)?.singer && (player.songInfo as any)?.albumName"
                  class="divider"
                >
                  /
                </span>
                <span class="album">{{ (player.songInfo as any)?.albumName }}</span>
              </div>
            </div>
          </div>
        </template>
        <div v-if="playSetting.getLayoutMode === 'cd'" class="mobile-song-info-area">
          <div ref="titleRef" class="song-title-large text-scroll-container">
            <div class="text-scroll-wrapper" :class="{ 'animate-scroll': shouldScrollTitle }">
              <div ref="titleContentRef" class="text-scroll-item">
                {{ songName }}
              </div>
              <div v-if="shouldScrollTitle" class="text-scroll-item">
                {{ songName }}
              </div>
            </div>
          </div>
          <div class="song-meta-large">
            <span
              class="artist"
              :class="{ 'singer-link': (player.songInfo as any)?.singerId && (player.songInfo as any)?.source !== 'local' }"
              @click="goToSinger()"
            >{{ (player.songInfo as any)?.singer }}</span>
            <span
              v-if="(player.songInfo as any)?.singer && (player.songInfo as any)?.albumName"
              class="divider"
            >/</span>
            <span class="album">{{ (player.songInfo as any)?.albumName }}</span>
          </div>
        </div>
      </div>
      <div class="right">
        <div v-if="player.lyrics.lines.length <= 0 && !player.isLoading" class="lyric-empty">
          <span>{{ t('play.noLyrics') }}</span>
        </div>
        <LyricPlayer
          v-if="player.lyrics.lines.length > 0"
          ref="lyricPlayerRef"
          :lyric-lines="safeLyricLines"
          :current-time="state.currentTime"
          :word-fade-width="0.5"
          :playing="isAudioPlaying"
          class="lyric-player"
          :align-position="
            playSetting.getLayoutMode === 'cd' && playSetting.getShowLeftPanel ? 0.5 : 0.34
          "
          :enable-blur="playSetting.getIsBlurLyric"
          :enable-spring="playSetting.getisJumpLyric"
          :enable-scale="playSetting.getisJumpLyric"
          :text-align="!playSetting.getShowLeftPanel ? 'center' : 'left'"
          :style="playSetting.getShowLeftPanel ? '' : 'text-align: center;'"
          @line-click="jumpTime"
        />
      </div>
    </div>
    <!-- 音频可视化 -->
    <div
      v-if="props.show && coverImage && playSetting.getIsAudioVisualizer"
      class="audio-visualizer-container"
      :class="{ idle: isIdle }"
    >
      <AudioVisualizer
        :show="Audio.isPlay"
        :height="70"
        :bar-count="80"
        :color="mainColor"
      />
    </div>
    <!-- 播放设置浮动按钮 -->
    <div ref="floatActionRef" class="float-action" :class="{ idle: isIdle }">
      <button class="skin-btn" :data-tooltip="t('play.playerTheme')" @click="showSettings = !showSettings">
        <pen-ball-icon
          :fill-color="'transparent'"
          :stroke-color="'currentColor'"
          :stroke-width="2"
          :style="{ fontSize: '20px' }"
        />
      </button>
      <Transition name="fade-up">
        <div v-if="showSettings" class="settings-panel">
          <PlaySettings />
        </div>
      </Transition>
    </div>
    <!-- 评论弹窗 -->
    <CommentsOverlay
      :show="props.showComments"
      :main-color="lightMainColor"
      @close="emit('update:showComments', false)"
    />
  </div>
</template>

<style lang="scss" scoped>
.performance-warning-toast {
  position: absolute;
  top: 80px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.875rem 1.25rem;
  background: rgba(255, 152, 0, 0.95);
  border-radius: 0.75rem;
  color: #fff;
  font-size: 0.9rem;
  font-weight: 500;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  z-index: 1000;
  pointer-events: auto;
  max-width: 600px;

  .close-btn {
    margin-left: auto;
    padding: 0.25rem;
    background: rgba(255, 255, 255, 0.2);
    border: none;
    border-radius: 0.25rem;
    color: #fff;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color var(--motion-duration-quick) var(--motion-ease-standard);

    &:hover {
      background: rgba(255, 255, 255, 0.3);
    }
  }
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity var(--motion-duration-standard) var(--motion-ease-standard);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.fade-nav-enter-active,
.fade-nav-leave-active {
  transition: background-color var(--motion-duration-slow) var(--motion-ease-emphasized), border-color var(--motion-duration-slow) var(--motion-ease-emphasized), color var(--motion-duration-slow) var(--motion-ease-emphasized), box-shadow var(--motion-duration-slow) var(--motion-ease-emphasized), opacity var(--motion-duration-slow) var(--motion-ease-emphasized), transform var(--motion-duration-slow) var(--motion-ease-emphasized);
}
.fade-nav-enter-from,
.fade-nav-leave-to {
  opacity: 0;
}

.fullscreen-btn,
.putawayscreen-btn {
  position: absolute;
  top: 25px;
  left: 30px;
  padding: 10px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(var(--glass-blur-control));
  -webkit-backdrop-filter: blur(var(--glass-blur-control));
  border: none;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  z-index: 10;
  color: white;
  font-size: 18px;
  transition: background-color var(--motion-duration-standard) var(--motion-ease-standard), transform var(--motion-duration-standard) var(--motion-ease-standard);

  &:hover {
    background: rgba(255, 255, 255, 0.3);
    transform: scale(1.05);
  }
  &:active {
    transform: scale(0.95);
  }
  .icon {
    color: rgba(255, 255, 255, 0.6);
    width: 25px;
    height: 25px;
  }
  &.black-text {
    background: rgba(0, 0, 0, 0.1);
    .icon { color: rgba(0, 0, 0, 0.6); }
    &:hover { background: rgba(0, 0, 0, 0.2); }
  }
}

.putawayscreen-btn { left: 90px; }

.full-play {
  --height: calc(100vh - var(--play-bottom-height));
  --text-color: rgba(255, 255, 255, 0.9);
  z-index: 120;
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  color: var(--text-color);
  transform: translateY(var(--height));
  will-change: transform;

  &.animating {
    transition: transform var(--motion-duration-standard) var(--motion-ease-emphasized);
  }
  &.use-black-text {
    --text-color: rgba(255, 255, 255, 0.9);
  }
  &.active { transform: translateY(0); }

  &.idle {
    .playbox {
      cursor: none;
      .left, .right { margin-bottom: 0; }
      .right {
        :deep(.lyric-player) { height: 100vh; }
      }
    }
    .fullscreen-btn, .putawayscreen-btn, .top {
      opacity: 0;
      pointer-events: none;
      transform: translateY(-100%);
    }
  }

  .top {
    position: absolute;
    width: calc(100% - 200px);
    z-index: 1;
    right: 0;
    padding: 30px 30px;
    padding-bottom: 10px;
    transition: opacity var(--motion-duration-expressive) var(--motion-ease-spring), transform var(--motion-duration-expressive) var(--motion-ease-spring);
  }

  .playbox {
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.256);
    -webkit-drop-filter: blur(8px);
    padding: 0 10vw;
    overflow: hidden;
    display: flex;
    position: relative;
    --cd-width-auto: max(200px, min(30vw, 700px, calc(100vh - var(--play-bottom-height) - 250px)));

    .left {
      width: 40%;
      transition: opacity var(--motion-duration-expressive) var(--motion-ease-spring), transform var(--motion-duration-expressive) var(--motion-ease-spring);
      opacity: 1;
      transform: translateX(0);
      display: flex;
      flex-direction: column;
      justify-content: center;
      align-items: center;
      margin: 0 0 var(--play-bottom-height) 0;
      perspective: 1000px;

      .cd-container {
        width: var(--cd-width-auto);
        height: var(--cd-width-auto);
        position: relative;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: filter var(--motion-duration-standard) var(--motion-ease-standard);
        filter: drop-shadow(0 10px 24px rgba(0, 0, 0, 0.45));
        &:not(.playing) {
          .vinyl-record::after,
          .label-shine { animation-play-state: paused; }
        }
        &:hover { filter: drop-shadow(0 12px 28px rgba(0, 0, 0, 0.52)); }

        .pointer {
          user-select: none;
          -webkit-user-drag: none;
          position: absolute;
          width: calc(var(--cd-width-auto) * 0.3);
          left: 52%;
          top: -24%;
          z-index: 20;
          transform: rotate(-20deg);
          transform-origin: 16% 16%;
          transition: transform var(--motion-duration-standard) var(--motion-ease-standard);
          pointer-events: none;
          &.playing { transform: rotate(0deg); }
        }

        .vinyl-disc {
          position: absolute;
          inset: 0;
          display: flex;
          align-items: center;
          justify-content: center;
          will-change: transform;
          animation: rotateRecord 33s linear infinite;
        }

        .vinyl-record {
          aspect-ratio: 1/1;
          width: 100%;
          height: 100%;
          position: relative;
          border-radius: 50%;
          background: radial-gradient(circle at 50% 50%, #1a1a1a 0%, #0d0d0d 100%);
          display: flex;
          align-items: center;
          justify-content: center;
          overflow: hidden;

          &::before {
            content: '';
            position: absolute;
            top: 0; left: 0;
            width: 100%; height: 100%;
            border-radius: 50%;
            background:
              repeating-conic-gradient(from 0deg, transparent 0deg, rgba(255,255,255,0.02) 0.5deg, transparent 1deg, rgba(255,255,255,0.01) 1.5deg, transparent 2deg),
              repeating-radial-gradient(circle at 50% 50%, transparent 0px, rgba(255,255,255,0.03) 1px, transparent 2px, transparent 8px);
            z-index: 1;
          }
          &::after {
            content: '';
            position: absolute;
            top: 0; left: 0;
            width: 100%; height: 100%;
            background: radial-gradient(ellipse at 30% 30%, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.04) 25%, rgba(255,255,255,0.02) 50%, rgba(255,255,255,0.01) 75%, transparent 100%);
            border-radius: 50%;
            z-index: 2;
            will-change: opacity; animation: vinylShine 6s ease-in-out infinite;
          }
        }

        .vinyl-label {
          position: absolute;
          width: 60%;
          height: 60%;
          background: radial-gradient(circle at 50% 50%, rgba(40,40,40,0.95) 0%, rgba(25,25,25,0.98) 70%, rgba(15,15,15,1) 100%);
          border-radius: 50%;
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 3;
          box-shadow: inset 0 0 20px rgba(0,0,0,0.8), inset 0 0 0 1px rgba(255,255,255,0.05), 0 0 10px rgba(0,0,0,0.5);

          .cover {
            position: relative;
            z-index: 4;
            border-radius: 50%;
            overflow: hidden;
            box-shadow: 0 0 20px rgba(0,0,0,0.4), inset 0 0 0 2px rgba(255,255,255,0.1);
            width: 95% !important;
            height: 95% !important;
            aspect-ratio: 1 / 1;
            object-fit: cover;
            filter: brightness(0.85) contrast(1.15) saturate(1.1);
          }

          .label-shine {
            position: absolute;
            top: 0; left: 0;
            width: 100%; height: 100%;
            background: radial-gradient(ellipse at 25% 25%, rgba(255,255,255,0.1) 0%, transparent 50%);
            border-radius: 50%;
            z-index: 5;
            pointer-events: none;
            will-change: opacity; animation: labelShine 8s ease-in-out infinite;
          }
        }

        .center-hole {
          position: absolute;
          width: 8%;
          height: 8%;
          background: radial-gradient(circle, #000 0%, #111 30%, #222 70%, #333 100%);
          border-radius: 50%;
          z-index: 10;
          box-shadow: inset 0 0 8px rgba(0,0,0,0.9), 0 0 3px rgba(0,0,0,0.8);
        }
      }

      .mobile-song-info-area {
        width: min(100%, 520px);
        min-width: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 12px;
        margin-top: 36px;
        text-align: center;

        .song-title-large {
          width: 100%;
          max-width: 100%;
          font-size: min(3vw, 42px);
          font-weight: 800;
          color: rgba(255,255,255,0.95);
          line-height: 1.2;
          letter-spacing: -0.5px;
          text-shadow: 0 2px 10px rgba(0,0,0,0.3);

          &.text-scroll-container {
            overflow: hidden;
            white-space: nowrap;
            position: relative;
            width: 100%;
          }

          .text-scroll-wrapper {
            display: inline-flex;
            &.animate-scroll { will-change: transform; animation: scroll 15s linear infinite; }
          }

          .text-scroll-item {
            font-weight: 800;
            flex-shrink: 0;
            padding-right: 2rem;
            display: inline-block;
          }
        }

        .song-meta-large {
          justify-content: center;
          font-size: min(1.5vw, 20px);
          color: rgba(255,255,255,0.6);
          font-weight: 600;
          display: flex;
          align-items: center;
          gap: 8px;
          flex-wrap: wrap;
          opacity: 0.55;

          .artist { color: v-bind(lightMainColor); filter: brightness(1.2); }
          .artist.singer-link { cursor: pointer; &:hover { filter: brightness(1.5); } }
          .divider { opacity: 0.4; }
        }
      }
    }

    .right {
      width: 60%;
      transition: opacity var(--motion-duration-expressive) var(--motion-ease-spring);
      mask: linear-gradient(rgba(255,255,255,0) 0px, rgba(255,255,255,0.6) 5%, rgb(255,255,255) 15%, rgb(255,255,255) 75%, rgba(255,255,255,0.6) 85%, rgba(255,255,255,0));

      .lyric-empty {
        height: calc(100vh - var(--play-bottom-height));
        display: flex;
        align-items: center;
        justify-content: center;
        transform: translateY(-80px);
        span {
          font-size: 16px;
          opacity: 0.4;
          font-weight: 400;
        }
      }

      :deep(.lyric-player) {
        height: calc(100vh - var(--play-bottom-height));
        transform: translateY(-80px);
        --amll-lyric-view-color: v-bind(lyricViewColor);
        --amll-lp-color: v-bind(lyricViewColor);
        --amll-lyric-player-font-size: v-bind(lyricFontSize);
        --amll-lp-font-size: v-bind(lyricFontSize);
        --amll-lyric-player-font-weight: v-bind(lyricFontWeight);
        --amll-lp-bg-line-scale: 0.7;
        font-family: v-bind(lyricFontFamily);

        [class*='romanWord'] {
          font-size: calc(var(--amll-lp-font-size) * 0.5);
          font-family: v-bind(lyricFontFamily) !important;
          opacity: 0.8;
        }
        [class*='lyricSubLine'] {
          font-weight: v-bind(lyricFontWeight) !important;
        }
        [class^='_interludeDots'] {
          padding: auto;
          height: calc(var(--amll-lyric-player-font-size) + 1em);
          justify-content: center;
          align-items: center;
        }
      }

      padding: 0 20px;
      margin: 80px 0 calc(var(--play-bottom-height)) 0;
      overflow: hidden;
    }

    &.mode-cover {
      .left {
        width: 35%;
        padding: 0 3vw;
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: flex-start;
      }
      .right { padding-left: 3vw; width: 65%; }
    }

    &.single-column {
      .left {
        width: 0 !important;
        padding: 0 !important;
        margin: 0 !important;
        opacity: 0;
        transform: translateX(-100px);
        pointer-events: none;
      }
      .right {
        width: 100%;
        padding: 0 10vw;
        display: flex;
        justify-content: center;
        :deep(.lyric-player) {
          width: 100%;
          max-width: 1000px;
          margin: 0 auto;
        }
      }
    }

    .cover-layout-container {
      width: 100%;
      display: flex;
      flex-direction: column;
      gap: 40px;
      margin-top: calc(var(--play-bottom-height) / 2);
      max-height: calc(100vh - 200px);

      .cover-wrapper-square {
        width: 100%;
        max-width: min(480px, 45vh);
        aspect-ratio: 1/1;
        border-radius: 24px;
        overflow: hidden;
        box-shadow: 0 12px 16px -12px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.1);
        transition: transform var(--motion-duration-expressive) var(--motion-ease-spring);
        margin: 0 auto;
        transform: scale(0.8);

        &.playing {
          transform: scale(1);
          &:hover { transition: transform var(--motion-duration-quick) var(--motion-ease-standard); transform: scale(1.02); }
        }
        &:hover { transform: scale(0.82); }

        .cover-img-square {
          width: 100%;
          height: 100%;
          object-fit: cover;
          user-select: none;
        }
      }

      .song-info-area {
        width: 100%;
        display: flex;
        flex-direction: column;
        gap: 12px;

        .song-title-large {
          font-size: min(3vw, 42px);
          font-weight: 800;
          color: rgba(255,255,255,0.95);
          line-height: 1.2;
          letter-spacing: -0.5px;
          text-shadow: 0 2px 10px rgba(0,0,0,0.3);

          &.text-scroll-container {
            overflow: hidden;
            white-space: nowrap;
            position: relative;
            width: 100%;
          }
          .text-scroll-wrapper {
            display: inline-flex;
            &.animate-scroll { will-change: transform; animation: scroll 15s linear infinite; }
          }
          .text-scroll-item {
            font-weight: 800;
            flex-shrink: 0;
            padding-right: 2rem;
            display: inline-block;
          }
        }

        .song-meta-large {
          font-size: min(1.5vw, 20px);
          color: rgba(255,255,255,0.6);
          font-weight: 600;
          display: flex;
          align-items: center;
          gap: 8px;
          flex-wrap: wrap;
          opacity: 0.55;
          .artist { color: v-bind(lightMainColor); filter: brightness(1.2); }
          .artist.singer-link { cursor: pointer; &:hover { filter: brightness(1.5); } }
          .divider { opacity: 0.4; }
        }
      }
    }
  }

  > .mobile-song-info-area {
    display: none;
  }

  @keyframes scroll {
    0% { transform: translateX(0); }
    25% { transform: translateX(0); }
    100% { transform: translateX(-50%); }
  }

  .audio-visualizer-container {
    position: absolute;
    bottom: calc(var(--play-bottom-height) - 10px);
    z-index: 5;
    left: 0;
    right: 0;
    height: 60px;
    filter: blur(4px);
    display: flex;
    align-items: center;
    pointer-events: none;
    transform: translateY(0);
    transition: transform var(--motion-duration-expressive) var(--motion-ease-spring);
    will-change: transform;
    &.idle { transform: translateY(var(--play-bottom-height)); }
  }
}

.float-action {
  position: absolute;
  z-index: 5;
  transition: opacity var(--motion-duration-expressive) var(--motion-ease-standard), transform var(--motion-duration-expressive) var(--motion-ease-standard);
  &.idle {
    opacity: 0;
    transform: translateY(20px);
    pointer-events: none;
  }
  --bottom-height: 60px;
  right: 20px;
  bottom: calc(var(--bottom-height) + var(--play-bottom-height));

  .skin-btn {
    position: relative;
    backdrop-filter: blur(var(--glass-blur-control));
    background: rgba(255,255,255,0.15);
    border: 1px solid rgba(255,255,255,0.628);
    height: 50px;
    width: 50px;
    border-radius: 50%;
    &[data-tooltip]::after {
      content: attr(data-tooltip);
      position: absolute;
      bottom: -30px;
      left: 50%;
      transform: translateX(-50%);
      background: rgba(0,0,0,0.75);
      color: #fff;
      font-size: 12px;
      padding: 4px 8px;
      border-radius: 4px;
      white-space: nowrap;
      pointer-events: none;
      opacity: 0;
      transition: opacity var(--motion-duration-quick) var(--motion-ease-standard);
    }
    &:hover[data-tooltip]::after {
      opacity: 1;
    }
    cursor: pointer;
    transition: background-color var(--motion-duration-standard) var(--motion-ease-spring), border-color var(--motion-duration-standard) var(--motion-ease-spring), color var(--motion-duration-standard) var(--motion-ease-spring), box-shadow var(--motion-duration-standard) var(--motion-ease-spring), opacity var(--motion-duration-standard) var(--motion-ease-spring), transform var(--motion-duration-standard) var(--motion-ease-spring);
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(255,255,255,0.9);
    font-size: 14px;
    font-weight: 500;
    letter-spacing: 1px;

    &:hover {
      background-color: rgba(255,255,255,0.25);
      box-shadow: 0 6px 16px 0 rgba(0,0,0,0.15), 0 0 16px v-bind(lightMainColor), inset 0 0 0 1px rgba(255,255,255,0.4);
    }
    &:active {
      transform: translateY(1px) scale(0.92);
      box-shadow: 0 4px 10px 0 rgba(0,0,0,0.1), 0 0 10px v-bind(lightMainColor), inset 0 0 0 1px rgba(255,255,255,0.1);
      transition: background-color var(--motion-duration-instant) var(--motion-ease-standard), border-color var(--motion-duration-instant) var(--motion-ease-standard), color var(--motion-duration-instant) var(--motion-ease-standard), box-shadow var(--motion-duration-instant) var(--motion-ease-standard), opacity var(--motion-duration-instant) var(--motion-ease-standard), transform var(--motion-duration-instant) var(--motion-ease-standard);
    }
  }

  .settings-panel {
    max-height: calc(100vh - 40px - 2.25rem - 70px - calc(var(--bottom-height) + var(--play-bottom-height)));
    display: flex;
    flex-direction: column;
    position: absolute;
    bottom: 70px;
    right: 0;
    width: 340px;
    background: rgb(30 30 30 / 29%);
    backdrop-filter: blur(var(--glass-blur-control));
    -webkit-backdrop-filter: blur(var(--glass-blur-control));
    border-radius: 24px;
    padding: 20px;
    box-shadow: 0 6px 12px rgba(0,0,0,0.4), 0 0 0 1px rgba(255,255,255,0.1);
    transform-origin: bottom right;
    z-index: 100;
  }
}

.fade-up-enter-active, .fade-up-leave-active {
  transition: opacity var(--motion-duration-standard) var(--motion-ease-spring), transform var(--motion-duration-standard) var(--motion-ease-spring);
}
.fade-up-enter-from, .fade-up-leave-to {
  opacity: 0;
  transform: translateY(20px) scale(0.95);
}

@keyframes rotateRecord {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

@keyframes vinylShine {
  0% { opacity: 0.1; transform: rotate(0deg) scale(1); }
  50% { opacity: 0.2; transform: rotate(180deg) scale(1.1); }
  100% { opacity: 0.1; transform: rotate(360deg) scale(1); }
}

@keyframes labelShine {
  0% { opacity: 0.05; transform: rotate(0deg); }
  25% { opacity: 0.15; }
  50% { opacity: 0.1; transform: rotate(180deg); }
  75% { opacity: 0.15; }
  100% { opacity: 0.05; transform: rotate(360deg); }
}

@media (max-width: 768px) {
  .performance-warning-toast {
    top: calc(var(--mobile-safe-top) + var(--mobile-page-top-gutter));
    width: calc(100vw - var(--mobile-page-gutter) * 2);
    max-width: none;
    padding: 0.75rem 1rem;
    font-size: 0.8rem;
  }

  .fullscreen-btn,
  .putawayscreen-btn {
    top: calc(var(--mobile-safe-top) + var(--mobile-page-top-gutter));
    left: var(--mobile-page-gutter);
    width: var(--mobile-touch-target);
    height: var(--mobile-touch-target);
    padding: 0;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.2);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
    box-shadow: 0 10px 26px rgba(0, 0, 0, 0.18);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    touch-action: manipulation;

    .icon {
      width: 22px;
      height: 22px;
      color: rgba(255, 255, 255, 0.9);
    }

    &:hover {
      transform: none;
      background: rgba(0, 0, 0, 0.2);
    }

    &:active {
      transform: scale(0.94);
    }
  }

  .fullscreen-btn {
    display: none;
  }

  .putawayscreen-btn {
    left: var(--mobile-page-gutter);
  }

  .top {
    display: none;
  }

  .full-play {
    --height: 100dvh;
    height: 100dvh;
    max-width: 100vw;
    overflow: hidden;
  }

  .full-play.idle {
    .playbox {
      cursor: auto;

      .left,
      .right {
        margin-bottom: 0;
      }

      .right {
        :deep(.lyric-player) {
          height: 100%;
        }
      }
    }

    .fullscreen-btn,
    .putawayscreen-btn,
    .float-action {
      opacity: 1;
      pointer-events: auto;
      transform: none;
    }
  }

  .playbox {
    --mobile-art-size: min(74vw, 330px, 40dvh);
    width: 100%;
    height: 100dvh;
    max-width: 100vw;
    min-width: 0;
    padding: calc(var(--mobile-safe-top) + var(--mobile-page-top-gutter) + var(--mobile-touch-target) + 12px) var(--mobile-page-gutter) calc(var(--play-bottom-height) + var(--mobile-safe-bottom) + 18px) !important;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: clamp(1.1rem, 3dvh, 1.8rem);
    overflow: hidden;
    background-color: rgba(0, 0, 0, 0.34);
    box-sizing: border-box;
    --cd-width-auto: var(--mobile-art-size);

    .left {
      width: 100dvw !important;
      max-width: 100dvw;
      min-width: 0;
      min-height: 0;
      margin: 0;
      padding: 0;
      flex: 0 0 auto;
      flex-direction: column;
      justify-content: center;
      align-items: center;
      gap: clamp(1rem, 2.6dvh, 1.5rem);
      opacity: 1;
      transform: none;
      pointer-events: auto;
      position: relative;
      overflow: visible;
      transition: opacity var(--motion-duration-standard) var(--motion-ease-standard), transform var(--motion-duration-standard) var(--motion-ease-standard);

      .cd-container {
        width: var(--mobile-art-size);
        height: var(--mobile-art-size);
        flex: 0 0 auto;
        margin-top: calc(var(--mobile-art-size) * 0.08);
        filter: drop-shadow(0 24px 42px rgba(0, 0, 0, 0.38));

        .pointer {
          display: block;
          width: calc(var(--mobile-art-size) * 0.3);
          left: 52%;
          top: -20%;
          z-index: 20;
          transform: rotate(-22deg);
          transform-origin: 16% 16%;
          filter: drop-shadow(0 10px 18px rgba(0, 0, 0, 0.34));

          &.playing {
            transform: rotate(-4deg);
          }
        }

        &:hover {
          filter: drop-shadow(0 24px 42px rgba(0, 0, 0, 0.38));
        }
      }
    }

    &.single-column {
      .left {
        width: 100dvw !important;
        max-width: 100dvw;
        padding: 0 !important;
        margin: 0 !important;
        opacity: 1;
        transform: none;
        pointer-events: auto;
      }

      .right {
        width: 100vw;
        padding: 0;
      }
    }

    &.mode-cover {
      .left {
        width: 100dvw !important;
        max-width: 100dvw;
      }

      .right {
        width: 100vw;
        padding: 0;
      }
    }

    .cover-layout-container {
      width: 100%;
      min-width: 0;
      gap: clamp(0.7rem, 1.6dvh, 0.85rem);
      margin-top: 0;
      max-height: none;
      align-items: center;

      .cover-wrapper-square {
        width: var(--mobile-art-size);
        max-width: none;
        border-radius: clamp(20px, 6vw, 28px);
        transform: none;
        box-shadow: 0 24px 52px rgba(0, 0, 0, 0.38), 0 0 0 0.5px rgba(255, 255, 255, 0.18);

        &.playing,
        &.playing:hover,
        &:hover {
          transform: none;
        }

        .cover-img-square {
          display: block;
        }
      }
    }

    .song-info-area,
    .mobile-song-info-area {
      width: min(100%, 430px);
      min-width: 0;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.45rem;
      text-align: center;
    }

    .mobile-song-info-area {
      display: flex;
    }

    .song-title-large {
      width: 100%;
      max-width: 100%;
      min-width: 0;
      font-size: clamp(1.45rem, 6.6vw, 2.15rem) !important;
      line-height: 1.12;
      letter-spacing: -0.04em;
      color: rgba(255, 255, 255, 0.96);
      text-shadow: 0 3px 18px rgba(0, 0, 0, 0.36);

      &.text-scroll-container {
        white-space: normal;
        overflow: visible;
      }

      .text-scroll-wrapper {
        display: block;
        width: 100%;
        min-width: 0;

        &.animate-scroll {
          animation: none;
        }
      }

      .text-scroll-item {
        width: 100%;
        min-width: 0;
        padding-right: 0;
        display: block;
        overflow: visible;
        overflow-wrap: anywhere;
        white-space: normal;
      }

      .text-scroll-item + .text-scroll-item {
        display: none;
      }
    }

    .song-meta-large {
      width: 100%;
      max-width: 100%;
      min-width: 0;
      justify-content: center;
      flex-wrap: wrap;
      gap: 0.35rem 0.5rem;
      font-size: clamp(0.86rem, 3.8vw, 0.98rem) !important;
      line-height: 1.35;
      opacity: 0.78;
      color: rgba(255, 255, 255, 0.72);
      overflow: visible;

      .artist,
      .album {
        min-width: 0;
        max-width: 100%;
        overflow: visible;
        overflow-wrap: anywhere;
        text-overflow: clip;
        white-space: normal;
      }

      .artist.singer-link {
        min-height: 32px;
        display: inline-flex;
        align-items: center;
      }

      .divider {
        flex: 0 0 auto;
      }
    }

    .right {
      width: 100dvw;
      max-width: 100dvw;
      min-width: 0;
      min-height: min(62dvh, 520px);
      max-height: min(68dvh, 560px);
      margin: 0;
      padding: 0;
      flex: 0 1 min(68dvh, 560px);
      overflow: hidden;
      border-radius: 0;
      background: rgba(0, 0, 0, 0.1);
      border: none;
      box-shadow: none;
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
      mask: none;
      opacity: 0;
      pointer-events: none;
      position: fixed;
      left: 0;
      right: auto;
      top: calc(var(--mobile-safe-top) + var(--mobile-page-top-gutter) + var(--mobile-touch-target) + 24px);
      bottom: calc(var(--play-bottom-height) + var(--mobile-safe-bottom) + 22px);
      transition: opacity var(--motion-duration-standard) var(--motion-ease-standard), transform var(--motion-duration-standard) var(--motion-ease-standard);
      transform: translateY(12px);

      .lyric-empty {
        height: 100%;
        min-height: min(62dvh, 520px);
        transform: none;

        span {
          color: rgba(255, 255, 255, 0.58);
        }
      }

      :deep(.lyric-player) {
        width: 100%;
        height: 100%;
        min-height: min(62dvh, 520px);
        transform: none;
        --amll-lyric-player-font-size: clamp(24px, 7vw, 34px);
        --amll-lp-font-size: clamp(24px, 7vw, 34px);
      }
    }

    &.mobile-show-lyrics {
      .left {
        opacity: 0;
        transform: scale(0.96);
        pointer-events: none;
      }

      .right {
        width: 100dvw !important;
        max-width: 100dvw;
        left: 0;
        right: auto;
        opacity: 1;
        pointer-events: auto;
        transform: translateY(0);
      }
    }
  }

  .float-action {
    right: var(--mobile-page-gutter);
    bottom: calc(var(--play-bottom-height) + var(--mobile-safe-bottom) + 12px);

    .skin-btn {
      width: var(--mobile-touch-target);
      height: var(--mobile-touch-target);
      min-width: var(--mobile-touch-target);
      min-height: var(--mobile-touch-target);
      border-radius: 50%;
      background: rgba(0, 0, 0, 0.22);
      border-color: rgba(255, 255, 255, 0.18);
      touch-action: manipulation;

      &[data-tooltip]::after {
        display: none;
      }
    }

    .settings-panel {
      width: calc(100vw - var(--mobile-page-gutter) * 2);
      max-height: calc(100dvh - var(--play-bottom-height) - var(--mobile-safe-bottom) - 120px);
      right: 0;
      bottom: calc(var(--mobile-touch-target) + 10px);
      padding: 16px;
      border-radius: var(--mobile-card-radius);
      overflow-y: auto;
      -webkit-overflow-scrolling: touch;
    }
  }

  .audio-visualizer-container {
    bottom: calc(var(--play-bottom-height) + var(--mobile-safe-bottom));
    height: 38px;
    opacity: 0.42;
  }

  @media (max-height: 700px) {
    .playbox {
      --mobile-art-size: min(70vw, 300px, 36dvh);
      gap: 0.65rem;
      padding-top: calc(var(--mobile-safe-top) + var(--mobile-page-top-gutter) + var(--mobile-touch-target) + 8px) !important;

      .left,
      .cover-layout-container {
        gap: 0.65rem;
      }

      .song-title-large {
        font-size: clamp(1.28rem, 6vw, 1.75rem) !important;
      }

      .song-meta-large {
        font-size: 0.84rem !important;
      }

      .right {
        min-height: min(58dvh, 460px);
        max-height: none;
        flex-basis: min(64dvh, 500px);
        top: calc(var(--mobile-safe-top) + var(--mobile-page-top-gutter) + var(--mobile-touch-target) + 18px);
        bottom: calc(var(--play-bottom-height) + var(--mobile-safe-bottom) + 16px);

        .lyric-empty,
        :deep(.lyric-player) {
          min-height: min(58dvh, 460px);
        }
      }
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .cd-container,
    .vinyl-record::after,
    .label-shine {
      animation: none !important;
    }
  }
}
</style>

<style lang="scss">
.full-play {
  --user-lyric-fw: v-bind(lyricFontWeight) !important;
  .lyric-player {
    [class*='Line' i] {
      &, & *, span, div { font-weight: var(--user-lyric-fw) !important; }
    }
    [class*='emphasize' i] { font-weight: var(--user-lyric-fw) !important; }
    [class*='romanWord' i] {
      font-size: 0.5em !important;
      font-weight: var(--user-lyric-fw) !important;
      line-height: 1 !important;
    }
  }
}
</style>
