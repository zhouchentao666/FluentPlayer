<script setup lang="ts">
import { ref, computed, onMounted } from "vue"
import { useOnlineSources } from "@online/store"
import { toast } from "../../composables/useToast"

const emit = defineEmits<{ (e: "back"): void }>()

const online = useOnlineSources()

const sources = computed(() => online.state.scripts)
const enabledCount = computed(() => online.state.scripts.filter((s) => s.enabled).length)
const totalCount = computed(() => online.state.scripts.length)

const importUrl = ref("")
const importBusy = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)

function fmtPlatforms(s?: Record<string, unknown>): string {
  if (!s) return "通用"
  const keys = Object.keys(s)
  return keys.length ? keys.join(" / ") : "通用"
}

function triggerFile() {
  fileInput.value?.click()
}

async function onPickFile(e: Event) {
  const input = e.target as HTMLInputElement
  const files = input.files ? Array.from(input.files) : []
  if (files.length === 0) return
  let okCount = 0
  let failCount = 0
  const errors: string[] = []
  for (const file of files) {
    try {
      const text = await file.text()
      await online.importScript(text)
      okCount += 1
    } catch (err) {
      failCount += 1
      errors.push(`${file.name}: ${(err as Error).message}`)
    }
  }
  if (okCount > 0) {
    toast(`已导入 ${okCount} 个音源${failCount > 0 ? `，${failCount} 个失败` : ""}`, failCount > 0 ? "warning" : "success")
  } else if (errors.length > 0) {
    toast(errors[0], "error")
  }
  input.value = ""
}

async function onImportUrl() {
  const u = importUrl.value.trim()
  if (!u) return
  importBusy.value = true
  try {
    await online.importScriptFromUrl(u)
    toast("音源已导入", "success")
    importUrl.value = ""
  } catch (err) {
    toast((err as Error).message, "error")
  } finally {
    importBusy.value = false
  }
}

function toggle(s: { id: string }) {
  online.toggleEnabled(s.id)
}
function remove(s: { id: string }) {
  online.removeScript(s.id)
}
function move(s: { id: string }, dir: -1 | 1) {
  const arr = online.state.scripts
  const idx = arr.findIndex((x) => x.id === s.id)
  if (idx < 0) return
  online.reorderScripts(idx, idx + dir)
}

onMounted(() => {
  if (!online.state.initialized) online.initOnlineSources().catch(() => {})
})
</script>

<template>
  <div class="osources-view">
    <div class="osources-topbar">
      <button class="back-btn" title="返回在线音乐" @click="emit('back')">
        <svg viewBox="0 0 16 16" width="16" height="16"><path d="M10 3L5 8l5 5" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
        返回
      </button>
    </div>
    <div class="osources-head">
      <h2 class="title">自定义音源</h2>
      <div class="osources-summary">已启用 {{ enabledCount }} / {{ totalCount }} 个音源</div>
      <div class="import-row">
        <button class="ghost" @click="triggerFile">导入脚本文件</button>
        <input
          ref="fileInput"
          type="file"
          accept=".js,.txt"
          multiple
          style="display: none"
          @change="onPickFile"
        />
        <input v-model="importUrl" class="url-input" placeholder="粘贴音源脚本链接…" @keyup.enter="onImportUrl" />
        <button class="ghost" :disabled="importBusy" @click="onImportUrl">
          {{ importBusy ? "导入中…" : "从链接导入" }}
        </button>
      </div>
    </div>

    <div v-if="online.state.error" class="src-error">{{ online.state.error }}</div>

    <div v-if="sources.length === 0" class="state">
      暂无自定义音源。导入 LX 格式音源脚本后，即可在播放 / 搜索中自动使用。
    </div>

    <div v-else class="src-list">
      <div v-for="s in sources" :key="s.id" class="src-item">
        <div class="src-main">
          <div class="src-name">{{ s.name }}</div>
          <div class="src-meta">
            <span v-if="s.author">作者：{{ s.author }}</span>
            <span v-if="s.version">版本：{{ s.version }}</span>
            <span>平台：{{ fmtPlatforms(s.sources as Record<string, unknown>) }}</span>
          </div>
        </div>
        <div class="src-actions">
          <button class="toggle" :class="{ on: s.enabled }" @click="toggle(s)">
            {{ s.enabled ? "已启用" : "已停用" }}
          </button>
          <button class="order" @click="move(s, -1)" title="上移">↑</button>
          <button class="order" @click="move(s, 1)" title="下移">↓</button>
          <button class="del" title="删除" @click="remove(s)">删除</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.osources-view {
  padding: 22px 28px;
  max-width: 880px;
  margin: 0 auto;
}
.osources-topbar {
  margin-bottom: 8px;
}
.back-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  border-radius: 10px;
  border: 1px solid var(--fluent-border);
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 13px;
  cursor: pointer;
}
.back-btn:hover {
  background: var(--fluent-bg-hover);
}
.osources-head {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 20px;
}
.title {
  font-size: 20px;
  font-weight: 700;
  color: var(--fluent-text);
  margin: 0;
}
.osources-summary {
  font-size: 13px;
  color: var(--fluent-text-secondary);
}
.import-row {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  align-items: center;
}
.ghost {
  height: 36px;
  padding: 0 16px;
  border: 1px solid var(--fluent-border);
  border-radius: 10px;
  background: var(--fluent-bg-card);
  color: var(--fluent-text);
  font-size: 13px;
  cursor: pointer;
}
.ghost:hover {
  background: var(--fluent-bg-hover);
}
.ghost:disabled {
  opacity: 0.5;
  cursor: default;
}
.url-input {
  flex: 1;
  min-width: 220px;
  height: 36px;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid var(--fluent-input-border);
  background: var(--fluent-input-bg);
  color: var(--fluent-text);
  outline: none;
  font-size: 13px;
}
.url-input:focus {
  border-color: var(--fluent-accent);
}
.src-error {
  color: #f87171;
  font-size: 13px;
  margin-bottom: 12px;
}
.src-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.src-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  border-radius: 12px;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
}
.src-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--fluent-text);
}
.src-meta {
  display: flex;
  gap: 14px;
  flex-wrap: wrap;
  margin-top: 4px;
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.src-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}
.toggle {
  height: 30px;
  padding: 0 12px;
  border-radius: 8px;
  border: 1px solid var(--fluent-border);
  background: transparent;
  color: var(--fluent-text-secondary);
  font-size: 12px;
  cursor: pointer;
}
.toggle.on {
  background: var(--fluent-accent);
  border-color: var(--fluent-accent);
  color: #fff;
}
.order {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  border: 1px solid var(--fluent-border);
  background: transparent;
  color: var(--fluent-text-secondary);
  cursor: pointer;
  font-size: 14px;
}
.order:hover {
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
}
.del {
  border: 1px solid var(--fluent-border);
  background: transparent;
  color: var(--fluent-text-secondary);
  padding: 6px 12px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
}
.del:hover {
  color: #f87171;
  border-color: #f87171;
}
.state {
  color: var(--fluent-text-secondary);
  padding: 30px 0;
  text-align: center;
  font-size: 14px;
}
</style>
