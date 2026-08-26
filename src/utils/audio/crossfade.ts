import { reactive } from 'vue'

/**
 * 对外暴露的 reactive 状态，供 UI (hint, progress bar) 订阅显示。
 * active: 交叉淡化 gain 包络已启动
 * scheduled: 已进入观察窗口（尾段，但尚未触发淡化）
 * fadeStart / fadeDuration: 当前曲目内的过渡起始时间 & 时长
 * markStart / markEnd: 进度条标记范围（显示过渡可能发生的区段）
 * fadeInMarkEnd: 交叉淡化完成后新歌曲开头的淡入区间结束位置（秒）
 */
export const crossfadeState = reactive({
  active: false,
  scheduled: false,
  fadeStart: 0,
  fadeDuration: 0,
  markStart: 0,
  markEnd: 0,
  fadeInMarkEnd: 0
})
