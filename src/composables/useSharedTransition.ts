import { ref } from 'vue'

// 配角元素交错阶段（0=全隐, 1=顶栏, 2=歌曲信息+控件, 3=歌词区域）
// 封面平移动画已改为纯 CSS 驱动，天然支持打断，无需 FLIP 逻辑
export const staggerPhase = ref(0)

export function useSharedTransition() {
  return {
    staggerPhase,
  }
}
