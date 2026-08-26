/**
 * 背景渲染配置类型定义
 */

/**
 * 背景渲染预设类型
 */
export type BackgroundRenderPreset = 'auto' | 'performance' | 'quality' | 'custom'

/**
 * 单个场景的背景渲染配置
 */
export interface BackgroundRenderConfig {
  /** 预设模式 */
  preset: BackgroundRenderPreset
  /** 是否启用背景效果 */
  enabled: boolean
  /** 是否启用音频响应（随低频跳动） */
  audioResponse: boolean
  /** 渲染缩放比例 (0.1-1.0)，值越大效果越精细但性能消耗越大 */
  renderScale: number
  /** 流动速度 (0.1-5.0)，控制背景动画速度 */
  flowSpeed: number
  /** 静态模式，开启后背景几乎静止 */
  staticMode: boolean
  /** FPS 限制 (15-60)，控制帧率上限 */
  fps: number
}

/**
 * 完整的背景渲染设置
 */
export interface BackgroundRenderSettings {
  /** FullPlay 页面配置 */
  fullPlay: BackgroundRenderConfig
}

/**
 * 预设配置常量
 */
export const BACKGROUND_PRESETS: Record<Exclude<BackgroundRenderPreset, 'auto' | 'custom'>, BackgroundRenderConfig> = {
  /** 性能模式 - 低配置设备友好 */
  performance: {
    preset: 'performance',
    enabled: true,
    audioResponse: false,
    renderScale: 0.3,
    flowSpeed: 0.5,
    staticMode: true,
    fps: 15
  },
  /** 质量模式 - 高配置设备 */
  quality: {
    preset: 'quality',
    enabled: true,
    audioResponse: true,
    renderScale: 0.8,
    flowSpeed: 1.5,
    staticMode: false,
    fps: 60
  }
}

/**
 * 默认背景渲染配置
 */
export const DEFAULT_BACKGROUND_RENDER_SETTINGS: BackgroundRenderSettings = {
  fullPlay: {
    preset: 'auto',
    enabled: true,
    audioResponse: true,
    renderScale: 0.5,
    flowSpeed: 1.0,
    staticMode: false,
    fps: 30
  }
}
