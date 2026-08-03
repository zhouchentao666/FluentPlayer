import { ref } from 'vue'
import { Version } from '@bridge/app'
import { toast } from './useToast'

export const UPDATE_REPO = 'zhouchentao666/FluentPlayer'

function normalizeVersion(v: string): string {
  return v.replace(/^v/i, '').trim()
}

/**
 * 从字符串中提取版本号（支持 1.0.1、v1.0.1、1.0.1(test)、1.0.1（test）等格式）
 */
function extractVersion(v: string): string {
  const normalized = normalizeVersion(v)
  // 匹配 x.x.x 格式的版本号，后面可跟括号内的任意内容
  const match = normalized.match(/^(\d+(?:\.\d+)*)/)
  return match ? match[1] : normalized
}

function compareVersion(a: string, b: string): number {
  const pa = extractVersion(a).split('.').map((n) => parseInt(n, 10) || 0)
  const pb = extractVersion(b).split('.').map((n) => parseInt(n, 10) || 0)
  const len = Math.max(pa.length, pb.length)
  for (let i = 0; i < len; i++) {
    const x = pa[i] ?? 0
    const y = pb[i] ?? 0
    if (x !== y) return x - y
  }
  return 0
}

const appVersionSingleton = ref<string>('0.0.0')

// 单例状态：App.vue（渲染更新弹窗）与 Settings.vue（触发手动检查）共享同一份，
// 避免非单例导致的状态分裂、手动检查与弹窗行为不一致。
const latestVersion = ref('')
const showUpdate = ref(false)
const checking = ref(false)

export function useUpdater() {
  const appVersion = appVersionSingleton

  async function ensureAppVersion() {
    if (!appVersion.value || appVersion.value === '0.0.0') {
      try {
        const v = await Version()
        if (v) appVersion.value = v
      } catch {
        // 忽略，使用默认值
      }
    }
  }

  // showToast=true 时（手动检查）给出明确反馈；false 时（启动自动检查）静默。
  async function checkForUpdates(showToast = false) {
    if (checking.value) return
    checking.value = true
    try {
      await ensureAppVersion()
      const controller = new AbortController()
      const timer = setTimeout(() => controller.abort(), 10000)
      let res: Response
      try {
        res = await fetch(`https://api.github.com/repos/${UPDATE_REPO}/releases/latest`, {
          headers: { Accept: 'application/vnd.github+json' },
          signal: controller.signal,
        })
      } finally {
        clearTimeout(timer)
      }
      if (!res.ok) {
        if (showToast) toast('检查更新失败（GitHub 接口暂不可用）', 'warning')
        return
      }
      const data = await res.json()
      // 优先使用 tag_name，如果没有则尝试 name 字段
      let tag = data?.tag_name
      if (typeof tag !== 'string' || !tag) {
        tag = data?.name
      }
      if (typeof tag !== 'string' || !tag) {
        if (showToast) toast('检查更新失败：接口返回异常', 'warning')
        return
      }
      if (compareVersion(tag, appVersion.value) > 0) {
        latestVersion.value = extractVersion(tag)
        showUpdate.value = true
        if (showToast) toast(`发现新版本 v${latestVersion.value}`, 'success')
      } else if (showToast) {
        toast('已是最新版本', 'success')
      }
    } catch (err) {
      if (showToast) {
        const msg = err instanceof DOMException && err.name === 'AbortError'
          ? '检查更新超时'
          : '检查更新失败（网络错误）'
        toast(msg, 'warning')
      }
    } finally {
      checking.value = false
    }
  }

  return { appVersion, latestVersion, showUpdate, checking, checkForUpdates }
}