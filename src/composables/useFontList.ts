import { onMounted, ref } from 'vue'

// 兜底字体列表（当浏览器 Font Access API 不可用或被拒绝时使用）
const FALLBACK_FONTS = [
  'system-ui',
  'sans-serif',
  'serif',
  'monospace',
  'PingFang SC',
  'Microsoft YaHei',
  'Microsoft YaHei UI',
  'SimHei',
  'SimSun',
  'KaiTi',
  'Songti SC',
  'Heiti SC',
  'Source Han Sans SC',
  'Noto Sans CJK SC',
  'Noto Serif CJK SC',
  'Arial',
  'Segoe UI',
  'Tahoma',
  'Helvetica',
  'Times New Roman',
  'Georgia',
  'Courier New',
  'Consolas',
  'Menlo',
  'Monaco',
]

/**
 * 获取系统所有可用字体。优先使用 Font Access API (window.queryLocalFonts)，
 * 不可用或被拒绝时回退到内置列表。
 */
export function useFontList() {
  const fonts = ref<string[]>(FALLBACK_FONTS)
  const loading = ref(true)

  onMounted(async () => {
    try {
      const queryLocalFonts = (window as unknown as {
        queryLocalFonts?: () => Promise<{ family: string }[]>
      }).queryLocalFonts
      if (typeof queryLocalFonts === 'function') {
        const all = await queryLocalFonts()
        const set = new Set<string>()
        for (const f of all) {
          if (f.family) set.add(f.family)
        }
        if (set.size > 0) {
          fonts.value = Array.from(set).sort((a, b) => a.localeCompare(b, 'zh-Hans-CN'))
        }
      }
    } catch {
      // 权限被拒或环境不支持，保留兜底列表
    } finally {
      loading.value = false
    }
  })

  return { fonts, loading }
}
