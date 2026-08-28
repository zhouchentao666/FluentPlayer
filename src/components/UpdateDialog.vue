<script lang="ts" setup>
import { openUrl } from '@tauri-apps/plugin-opener'

const props = defineProps<{
  currentVersion: string
  latestVersion: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const GITHUB_URL = 'https://github.com/zhouchentao666/FluentPlayer/releases'
const PAN_URL = 'https://1823125279.share.123pan.cn/123pan/Gk8wjv-qcgXd'

async function openGithub() {
  try {
    await openUrl(GITHUB_URL)
  } catch {
    // 忽略打开失败
  }
  emit('close')
}

async function openPan() {
  try {
    await openUrl(PAN_URL)
  } catch {
    // 忽略打开失败
  }
  emit('close')
}
</script>

<template>
  <div class="update-mask" @click.self="emit('close')">
    <div class="update-dialog glass">
      <div class="update-icon">⬆</div>
      <h2 class="update-title">发现新版本</h2>
      <p class="update-text">
        当前版本 <b>{{ props.currentVersion }}</b>，最新版本
        <b class="update-latest">{{ props.latestVersion }}</b>
      </p>
      <p class="update-sub">选择下载渠道获取最新安装包：</p>
      <div class="update-actions">
        <button class="btn-primary" @click="openGithub">Github</button>
        <button class="btn-secondary" @click="openPan">123云盘</button>
        <button class="btn-ghost" @click="emit('close')">取消</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.update-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}
.update-dialog {
  width: 380px;
  max-width: calc(100vw - 48px);
  padding: 28px 28px 24px;
  border-radius: 16px;
  text-align: center;
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.35);
}
.update-icon {
  width: 56px;
  height: 56px;
  margin: 0 auto 14px;
  border-radius: 50%;
  background: var(--fluent-accent, #0078d4);
  color: #fff;
  font-size: 26px;
  line-height: 56px;
  text-align: center;
}
.update-title {
  margin: 0 0 10px;
  font-size: 22px;
  font-weight: 600;
}
.update-text {
  margin: 0 0 4px;
  font-size: 14px;
  color: var(--fluent-text);
}
.update-latest {
  color: var(--fluent-accent, #0078d4);
}
.update-sub {
  margin: 0 0 20px;
  font-size: 13px;
  color: var(--fluent-text-secondary, #888);
}
.update-actions {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.btn-primary,
.btn-secondary,
.btn-ghost {
  height: 40px;
  border-radius: 10px;
  font-size: 14px;
  cursor: pointer;
  border: 1px solid transparent;
  transition: background 0.18s ease, border-color 0.18s ease;
}
.btn-primary {
  background: var(--fluent-accent, #0078d4);
  color: #fff;
}
.btn-primary:hover {
  filter: brightness(1.08);
}
.btn-secondary {
  background: var(--fluent-bg-hover, rgba(255, 255, 255, 0.08));
  color: var(--fluent-text);
  border-color: var(--fluent-border, rgba(255, 255, 255, 0.12));
}
.btn-secondary:hover {
  background: var(--fluent-bg-active, rgba(255, 255, 255, 0.14));
}
.btn-ghost {
  background: transparent;
  color: var(--fluent-text-secondary, #888);
}
.btn-ghost:hover {
  color: var(--fluent-text);
}
</style>
