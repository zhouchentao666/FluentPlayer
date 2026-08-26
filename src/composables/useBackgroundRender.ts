import { ref, computed, watch, type Ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  BackgroundRender as CoreBackgroundRender,
  PixiRenderer
} from '@applemusic-like-lyrics/core'
import type { BackgroundRenderConfig } from '@/types/background'

interface UseBackgroundRenderOptions {
  /** 容器元素引用 */
  container: Ref<HTMLDivElement | null>
  /** 是否启用 */
  enabled: Ref<boolean>
  /** 配置对象 */
  config: Ref<BackgroundRenderConfig>
  /** 封面图片 URL */
  coverImage: Ref<string>
  /** 是否有歌词 */
  hasLyric: Ref<boolean>
  /** 是否正在播放 */
  isPlaying: Ref<boolean>
}

/**
 * 背景渲染 Composable
 * 封装 AMLL BackgroundRender 的初始化、配置更新和音频响应逻辑
 */
export function useBackgroundRender(options: UseBackgroundRenderOptions) {
  const {
    container,
    enabled,
    config,
    coverImage,
    hasLyric,
    isPlaying
  } = options

  // 背景渲染器实例
  const bgRender = ref<CoreBackgroundRender<any> | undefined>(undefined)
  const isInitialized = ref(false)

  // 音频响应相关
  let spectrumUnlisten: UnlistenFn | null = null

  /**
   * 初始化背景渲染器
   */
  const init = async () => {
    if (!container.value || !enabled.value) return

    // 清理旧实例
    if (bgRender.value) {
      dispose()
    }

    try {
      // 创建新实例
      bgRender.value = CoreBackgroundRender.new(PixiRenderer)
      const canvas = bgRender.value.getElement()

      // 设置样式
      canvas.style.position = 'absolute'
      canvas.style.top = '0'
      canvas.style.left = '0'
      canvas.style.width = '100%'
      canvas.style.height = '100%'
      canvas.style.zIndex = '-1'

      // 添加到容器
      container.value.appendChild(canvas)

      // 应用初始配置
      applyConfig()

      // 设置封面
      await bgRender.value.setAlbum(coverImage.value, false)

      // 设置是否有歌词
      bgRender.value.setHasLyric(hasLyric.value)

      // 启动渲染
      bgRender.value.resume()

      isInitialized.value = true

      // 如果启用了音频响应且正在播放，启动它
      if (config.value.audioResponse && isPlaying.value) {
        startAudioResponse()
      }
    } catch (error) {
      console.error('[useBackgroundRender] 初始化失败:', error)
    }
  }

  /**
   * 应用配置到背景渲染器
   */
  const applyConfig = () => {
    if (!bgRender.value) return

    bgRender.value.setRenderScale(config.value.renderScale)
    bgRender.value.setFlowSpeed(config.value.flowSpeed)
    bgRender.value.setStaticMode(config.value.staticMode)
    bgRender.value.setFPS(config.value.fps)
  }

  /**
   * 更新封面
   */
  const updateCover = async (imageUrl: string) => {
    if (!bgRender.value) return

    try {
      // 清理旧纹理
      const renderer = bgRender.value as any
      const oldTexture = renderer.curContainer?.children?.[0]?.texture

      await bgRender.value.setAlbum(imageUrl, false)

      // 延迟清理旧纹理以避免闪烁
      if (oldTexture) {
        setTimeout(() => {
          if (oldTexture.baseTexture && !oldTexture.baseTexture.destroyed) {
            try {
              oldTexture.destroy(true)
            } catch (e) {
              console.warn('[useBackgroundRender] 清理旧纹理失败:', e)
            }
          }
        }, 2000)
      }
    } catch (error) {
      console.error('[useBackgroundRender] 更新封面失败:', error)
      // 降级到默认封面
      try {
        await bgRender.value.setAlbum('/src/assets/images/Default.jpg', false)
      } catch (e) {
        console.error('[useBackgroundRender] 设置默认封面也失败:', e)
      }
    }
  }

  /**
   * 启动音频响应效果
   */
  const startAudioResponse = async () => {
    if (!config.value.audioResponse || spectrumUnlisten || !bgRender.value) return

    try {
      spectrumUnlisten = await listen('player:spectrum', (event: any) => {
        if (!bgRender.value || !isPlaying.value) return

        const { bands } = event.payload
        if (bands && Array.isArray(bands) && bands.length > 0) {
          // 提取低频能量（前 10 个频段）
          const lowFreqBands = bands.slice(0, 10)
          const avgLowFreq = lowFreqBands.reduce((sum: number, val: number) => sum + val, 0) / lowFreqBands.length

          // 转换为 0-1 范围（-80dB 到 0dB）
          const normalizedVolume = Math.max(0, Math.min(1, (avgLowFreq + 80) / 80))

          // 应用到背景渲染器
          bgRender.value.setLowFreqVolume(normalizedVolume)
        }
      })
    } catch (error) {
      console.error('[useBackgroundRender] 启动音频响应失败:', error)
    }
  }

  /**
   * 停止音频响应效果
   */
  const stopAudioResponse = () => {
    if (spectrumUnlisten) {
      spectrumUnlisten()
      spectrumUnlisten = null
    }
    // 重置低频音量
    bgRender.value?.setLowFreqVolume(0)
  }

  /**
   * 暂停渲染
   */
  const pause = () => {
    bgRender.value?.pause()
  }

  /**
   * 恢复渲染
   */
  const resume = () => {
    bgRender.value?.resume()
  }

  /**
   * 清理资源
   */
  const dispose = () => {
    // 停止音频响应
    stopAudioResponse()

    if (bgRender.value) {
      const canvas = bgRender.value.getElement()
      canvas?.parentNode?.removeChild(canvas)
      bgRender.value.dispose()
      bgRender.value = undefined
    }

    isInitialized.value = false
  }

  // === 监听器 ===

  // 监听封面变化
  watch(
    () => coverImage.value,
    (newImage) => {
      if (isInitialized.value) {
        updateCover(newImage)
      }
    }
  )

  // 监听是否有歌词变化
  watch(
    () => hasLyric.value,
    (hasLyric) => {
      if (bgRender.value) {
        bgRender.value.setHasLyric(hasLyric)
      }
    }
  )

  // 监听配置变化
  watch(
    [
      () => config.value.renderScale,
      () => config.value.flowSpeed,
      () => config.value.staticMode,
      () => config.value.fps,
      () => config.value.audioResponse
    ],
    () => {
      if (isInitialized.value) {
        applyConfig()

        // 如果音频响应配置变化，需要重启
        if (config.value.audioResponse && isPlaying.value && !spectrumUnlisten) {
          startAudioResponse()
        } else if (!config.value.audioResponse && spectrumUnlisten) {
          stopAudioResponse()
        }
      }
    }
  )

  // 监听播放状态
  watch(
    () => isPlaying.value,
    (playing) => {
      if (config.value.audioResponse && isInitialized.value) {
        if (playing) {
          startAudioResponse()
        } else {
          stopAudioResponse()
        }
      }
    }
  )

  // 监听启用状态
  watch(
    () => enabled.value,
    (isEnabled) => {
      if (isEnabled && !isInitialized.value) {
        init()
      } else if (!isEnabled && isInitialized.value) {
        dispose()
      }
    }
  )

  // 监听窗口可见性（失焦时降低资源消耗）
  if (typeof document !== 'undefined') {
    watch(
      () => document.visibilityState,
      (state) => {
        if (!isInitialized.value) return

        if (state === 'hidden') {
          // 窗口失焦时暂停或降低 FPS
          pause()
        } else {
          // 窗口聚焦时恢复
          resume()
        }
      }
    )
  }

  return {
    isInitialized,
    init,
    pause,
    resume,
    dispose,
    bgRender
  }
}
