import { reactive, readonly } from "vue"
import { httpFetch } from "@online/lib/http"
import { parseScriptMeta } from "@online/lib/lxApi"
import { sourceRunner, bindSourceRegistry } from "@online/lib/sourceRunner"
import { loadSourceScripts, saveSourceScripts } from "@online/lib/sources/persist"
import type { SourceScript } from "@online/types/source"

interface SourceState {
  scripts: SourceScript[]
  isLoading: boolean
  error: string | null
  initialized: boolean
}

const state = reactive<SourceState>({
  scripts: [],
  isLoading: false,
  error: null,
  initialized: false,
})

function generateId(): string {
  return `user_api_${Math.random().toString(36).slice(2)}_${Date.now()}`
}

function persist() {
  saveSourceScripts(state.scripts.map((s) => ({ ...s })))
}

/** 导入音源脚本（文本）。重复导入相同 URL / 内容会原位更新。 */
async function importScript(rawScript: string, url?: string): Promise<"added" | "updated"> {
  const meta = parseScriptMeta(rawScript)
  const existing = state.scripts.find(
    (s) => (url && s.url === url) || s.rawScript === rawScript
  )
  const script: SourceScript = {
    id: existing?.id ?? generateId(),
    ...meta,
    rawScript,
    enabled: existing?.enabled ?? true,
    url,
  }

  state.isLoading = true
  state.error = null
  try {
    const sources = await sourceRunner.loadScript(script)
    if (sources) script.sources = sources as SourceScript["sources"]
    const rest = existing
      ? state.scripts.filter((x) => x.id !== existing.id)
      : state.scripts.slice()
    state.scripts = [script, ...rest]
    persist()
    state.isLoading = false
    return existing ? "updated" : "added"
  } catch (err) {
    state.isLoading = false
    state.error = (err as Error).message
    throw err
  }
}

/** 从链接导入音源脚本。 */
async function importScriptFromUrl(url: string): Promise<"added" | "updated"> {
  state.isLoading = true
  state.error = null
  let rawScript: string
  try {
    const res = await httpFetch(url, {
      method: "GET",
      headers: { "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36" },
    })
    if (!res.ok) throw new Error(`下载失败（HTTP ${res.status}）`)
    rawScript = await res.text()
  } catch (err) {
    const message = `下载音源脚本失败：${(err as Error).message}`
    state.isLoading = false
    state.error = message
    throw new Error(message)
  }

  if (!rawScript.trim() || /^\s*</.test(rawScript)) {
    const message = "链接返回的内容不是有效的音源脚本"
    state.isLoading = false
    state.error = message
    throw new Error(message)
  }

  return importScript(rawScript, url)
}

function removeScript(id: string): void {
  sourceRunner.unloadScript(id)
  state.scripts = state.scripts.filter((x) => x.id !== id)
  persist()
}

async function toggleEnabled(id: string): Promise<void> {
  const script = state.scripts.find((s) => s.id === id)
  if (!script) return
  const willEnable = !script.enabled
  script.enabled = willEnable
  persist()

  if (willEnable) {
    state.isLoading = true
    state.error = null
    try {
      await sourceRunner.loadScript({ ...script })
      state.isLoading = false
    } catch (err) {
      state.isLoading = false
      state.error = `加载音源「${script.name}」失败：${(err as Error).message}`
    }
  } else {
    sourceRunner.unloadScript(id)
  }
}

function reorderScripts(from: number, to: number): void {
  const n = state.scripts.length
  if (from === to || from < 0 || to < 0 || from >= n || to >= n) return
  const scripts = state.scripts.slice()
  const [moved] = scripts.splice(from, 1)
  scripts.splice(to, 0, moved)
  state.scripts = scripts
  persist()
}

function setScriptSources(id: string, sources: unknown): void {
  const s = state.scripts.find((x) => x.id === id)
  if (s) s.sources = sources as SourceScript["sources"]
}

function clearError(): void {
  state.error = null
}

/** 启动时载入本地保存的脚本并逐个初始化启用的音源。 */
async function initOnlineSources(): Promise<void> {
  if (state.initialized) return
  state.initialized = true
  const scripts = await loadSourceScripts()
  state.scripts = scripts
  for (const script of scripts) {
    if (!script.enabled) continue
    try {
      await sourceRunner.loadScript({ ...script })
    } catch {
      // 单个音源初始化失败不影响其它音源
    }
  }
}

bindSourceRegistry({
  getScripts: () => state.scripts.map((s) => ({ ...s })),
  setScriptSources,
})

export function useOnlineSources() {
  return {
    state: readonly(state) as SourceState,
    importScript,
    importScriptFromUrl,
    removeScript,
    toggleEnabled,
    reorderScripts,
    clearError,
    initOnlineSources,
    sourceRunner,
  }
}
