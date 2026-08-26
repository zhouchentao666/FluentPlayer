<template>
  <Teleport to="body">
    <Transition name="glass-fade">
      <div v-if="visible" class="liquid-glass-overlay" @click.self="handleClose">
        <div class="overlay-drag-bar" data-tauri-drag-region />
        <LiquidGlass
          class="liquid-glass-panel"
          :corner-radius="cornerRadius"
          :displacement-scale="48"
          :blur-amount="0.08"
          :saturation="180"
          :aberration-intensity="1.5"
          padding="0"
          mode="standard"
          :content-style="liquidGlassContentStyle"
          @click.stop
        >
          <div class="liquid-glass-panel__content">
            <!-- Header -->
          <div class="glass-header" data-tauri-drag-region>
            <div class="glass-title-group">
              <div class="glass-icon">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" />
                </svg>
              </div>
              <div class="glass-title-text">
                <h2 class="glass-title">{{ t('backup.backupAndRestore') }}</h2>
                <div class="glass-status">
                  <span :class="['status-indicator', { connected: store.isConnected }]" />
                  <span class="status-label">{{ store.statusText }}</span>
                </div>
              </div>
            </div>
            <button class="glass-close-btn" @click="handleClose">
              <i class="iconfont icon-a-quxiaoguanbi" />
            </button>
          </div>

          <!-- Tabs -->
          <div class="glass-tabs">
            <button
              :class="['glass-tab', { active: activeTab === 'recommend' }]"
              @click="activeTab = 'recommend'"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
              </svg>
              <span>{{ t('backup.tabRecommend') }}</span>
            </button>
            <button
              :class="['glass-tab', { active: activeTab === 'config' }]"
              @click="activeTab = 'config'"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
              <span>{{ t('backup.tabConfig') }}</span>
            </button>
            <button
              :class="['glass-tab', { active: activeTab === 'ops' }]"
              :disabled="!store.isConnected"
              @click="activeTab = 'ops'"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
              </svg>
              <span>{{ t('backup.tabOps') }}</span>
            </button>
          </div>

          <!-- Tab: Recommend -->
          <div v-show="activeTab === 'recommend'" class="glass-content">
            <div class="recommend-intro">
              {{ t('backup.recommendIntro') }}
            </div>

            <div class="recommend-list">
              <div class="recommend-card" @click="openLink('https://www.hi168.com')">
                <div class="recommend-card-header">
                  <div class="recommend-badge free">{{ t('backup.free') }}</div>
                  <div class="recommend-card-info">
                    <h3>{{ t('backup.hi168.title') }}</h3>
                    <span class="recommend-url">{{ t('backup.hi168.url') }}</span>
                  </div>
                </div>
                <div class="recommend-features">
                  <div class="feature-tag"><b>500 GB</b> {{ t('backup.hi168.storage') }}</div>
                  <div class="feature-tag"><b>10 GB</b> {{ t('backup.hi168.traffic') }}</div>
                  <div class="feature-tag">{{ t('backup.hi168.s3Compatible') }}</div>
                </div>
                <p class="recommend-desc">{{ t('backup.hi168.desc') }}</p>
              </div>

              <div class="recommend-card" @click="openLink('https://data.cstcloud.cn/')">
                <div class="recommend-card-header">
                  <div class="recommend-badge free">{{ t('backup.free') }}</div>
                  <div class="recommend-card-info">
                    <h3>{{ t('backup.cstcloud.title') }}</h3>
                    <span class="recommend-url">{{ t('backup.cstcloud.url') }}</span>
                  </div>
                </div>
                <div class="recommend-features">
                  <div class="feature-tag"><b>20 GB</b> {{ t('backup.cstcloud.storage') }}</div>
                  <div class="feature-tag">{{ t('backup.cstcloud.s3Compatible') }}</div>
                  <div class="feature-tag">{{ t('backup.cstcloud.research') }}</div>
                </div>
                <p class="recommend-desc">{{ t('backup.cstcloud.desc') }}</p>
              </div>
            </div>

            <div class="glass-hint">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10" /><line x1="12" y1="16" x2="12" y2="12" /><line x1="12" y1="8" x2="12.01" y2="8" />
              </svg>
              <span>{{ t('backup.recommendHint') }}</span>
            </div>
          </div>

          <!-- Tab: Config -->
          <div v-show="activeTab === 'config'" class="glass-content">
            <div class="form-grid">
              <div class="form-item full">
                <label>{{ t('backup.endpoint') }}</label>
                <input v-model="store.config.endpoint" type="text" placeholder="https://s3.amazonaws.com" spellcheck="false" />
              </div>
              <div class="form-item">
                <label>{{ t('backup.region') }}</label>
                <input v-model="store.config.region" type="text" placeholder="auto" />
              </div>
              <div class="form-item">
                <label>{{ t('backup.bucket') }}</label>
                <input v-model="store.config.bucket" type="text" placeholder="my-bucket" />
              </div>
              <div class="form-item">
                <label>{{ t('backup.accessKeyId') }}</label>
                <input v-model="store.config.accessKeyId" type="password" placeholder="AKIA..." />
              </div>
              <div class="form-item">
                <label>{{ t('backup.secretAccessKey') }}</label>
                <input v-model="store.config.secretAccessKey" type="password" placeholder="••••••••" />
              </div>
            </div>

            <div v-if="store.errorMessage" class="glass-error">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10" /><line x1="15" y1="9" x2="9" y2="15" /><line x1="9" y1="9" x2="15" y2="15" />
              </svg>
              {{ store.errorMessage }}
            </div>

            <button
              :class="['glass-btn', 'primary', { loading: store.isConnecting }]"
              :disabled="store.isConnecting || !isConfigValid"
              @click="handleTestConnection"
            >
              <template v-if="store.isConnecting">
                <span class="glass-spinner" />
                {{ t('backup.connecting') }}
              </template>
              <template v-else-if="store.isConnected">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
                {{ t('backup.connectedRetest') }}
              </template>
              <template v-else>
                {{ t('backup.testConnection') }}
              </template>
            </button>
          </div>

          <!-- Tab: Ops -->
          <div v-show="activeTab === 'ops'" class="glass-content">
            <!-- Password & Settings -->
            <div class="glass-field-group">
              <div class="form-item">
                <label>{{ t('backup.backupPassword') }}</label>
                <div class="input-wrapper">
                  <input
                    v-model="store.backupPassword"
                    :type="showBackupPwd ? 'text' : 'password'"
                    :placeholder="t('backup.encryptBackupPlaceholder')"
                    autocomplete="new-password"
                  />
                  <button class="input-toggle" @click="showBackupPwd = !showBackupPwd">
                    <svg v-if="showBackupPwd" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/><path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                    <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                  </button>
                </div>
              </div>
              <div class="form-item">
                <label>{{ t('backup.maxBackups') }}</label>
                <input
                  :value="store.maxBackups"
                  type="number"
                  min="1"
                  max="100"
                  placeholder="10"
                  @input="store.maxBackups = Math.max(1, Math.min(100, parseInt(($event.target as HTMLInputElement)?.value, 10) || 10))"
                  @change="store.saveConfig()"
                />
              </div>
            </div>

            <div class="ops-card">
              <div class="ops-card-header">
                <div class="ops-icon backup">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                </div>
                <div class="ops-card-text">
                  <h3>{{ t('backup.backupToCloud') }}</h3>
                  <p>{{ t('backup.backupToCloudDesc') }}</p>
                  <span v-if="store.lastBackupTime" class="last-time">
                    {{ t('backup.lastBackup') }}{{ formatTime(store.lastBackupTime) }}
                  </span>
                </div>
              </div>
              <button
                :class="['glass-btn', 'accent', { loading: store.isBackingUp }]"
                :disabled="store.isBackingUp || !store.backupPassword"
                @click="handleBackup"
              >
                <template v-if="store.isBackingUp">
                  <span class="glass-spinner" />
                  {{ t('backup.backingUp') }}
                </template>
                <template v-else>
                  {{ t('backup.encryptBackup') }}
                </template>
              </button>
            </div>

            <div class="ops-card">
              <div class="ops-card-header">
                <div class="ops-icon restore">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
                  </svg>
                </div>
                <div class="ops-card-text">
                  <h3>{{ t('backup.restoreFromCloud') }}</h3>
                  <p>{{ t('backup.restoreFromCloudDesc') }}</p>
                </div>
              </div>
              <div class="restore-field">
                <div class="input-wrapper">
                  <input
                    v-model="restorePassword"
                    :type="showRestorePwd ? 'text' : 'password'"
                    :placeholder="t('backup.restorePassword')"
                    autocomplete="current-password"
                  />
                  <button class="input-toggle" @click="showRestorePwd = !showRestorePwd">
                    <svg v-if="showRestorePwd" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/><path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                    <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                  </button>
                </div>
              </div>
              <div class="restore-actions">
                <button
                  :class="['glass-btn', 'outline', { loading: store.isRestoring }]"
                  :disabled="store.isRestoring || !restorePassword"
                  @click="handleRestore('merge')"
                >
                  <template v-if="store.isRestoring">
                    <span class="glass-spinner" />
                    {{ t('backup.restoring') }}
                  </template>
                  <template v-else>
                    {{ t('backup.mergeRestore') }}
                  </template>
                </button>
                <button
                  :class="['glass-btn', 'danger', { loading: store.isRestoring }]"
                  :disabled="store.isRestoring || !restorePassword"
                  @click="handleRestore('overwrite')"
                >
                  {{ t('backup.overwriteRestore') }}
                </button>
              </div>
            </div>

            <div v-if="store.errorMessage" class="glass-error">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10" /><line x1="15" y1="9" x2="9" y2="15" /><line x1="9" y1="9" x2="15" y2="15" />
              </svg>
              {{ store.errorMessage }}
            </div>

            <div class="glass-hint">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10" /><line x1="12" y1="16" x2="12" y2="12" /><line x1="12" y1="8" x2="12.01" y2="8" />
              </svg>
              <span>{{ t('backup.opsHint') }}</span>
            </div>
          </div>
          </div>
        </LiquidGlass>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, type CSSProperties } from 'vue'
import LiquidGlass from '@/components/LiquidGlass.vue'
import { useS3BackupStore } from '@/store/S3Backup'

defineProps<{ visible: boolean }>()
const emit = defineEmits<{ (e: 'update:visible', val: boolean): void }>()

const { t } = useI18n()
const store = useS3BackupStore()
const activeTab = ref<'recommend' | 'config' | 'ops'>('recommend')
const restorePassword = ref('')
const showBackupPwd = ref(false)
const showRestorePwd = ref(false)

// Responsive corner-radius for mobile
const isMobile = ref(false)
const mobileMql = typeof window !== 'undefined' ? window.matchMedia('(max-width: 768px)') : null

const onMobileChange = (e: MediaQueryListEvent | MediaQueryList) => {
  isMobile.value = e.matches
}

onMounted(() => {
  if (mobileMql) {
    onMobileChange(mobileMql)
    mobileMql.addEventListener('change', onMobileChange)
  }
})

onUnmounted(() => {
  if (mobileMql) {
    mobileMql.removeEventListener('change', onMobileChange)
  }
})

const cornerRadius = computed(() => {
  if (!isMobile.value) return 22
  const cssVal = getComputedStyle(document.documentElement).getPropertyValue('--mobile-card-radius')?.trim()
  if (cssVal) {
    const num = parseFloat(cssVal)
    if (Number.isFinite(num)) return num
  }
  return 18
})

const liquidGlassContentStyle: CSSProperties = {
  color: 'var(--td-text-color-primary)',
  font: 'inherit',
  lineHeight: 'normal',
  textShadow: 'none',
}

const isConfigValid = computed(() =>
  store.config.endpoint &&
  store.config.accessKeyId &&
  store.config.secretAccessKey &&
  store.config.bucket
)

function handleClose() {
  emit('update:visible', false)
}

function openLink(url: string) {
  window.open(url, '_blank')
}

async function handleTestConnection() {
  const ok = await store.testConnection()
  if (ok) activeTab.value = 'ops'
}

async function handleBackup() {
  await store.backup()
}

async function handleRestore(mode: 'overwrite' | 'merge') {
  await store.restore(mode, restorePassword.value)
  window.location.reload()
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso)
    return d.toLocaleString('zh-CN', { hour12: false })
  } catch {
    return iso
  }
}
</script>

<style lang="scss" scoped>
// ========================================
// Liquid Glass Design — S3 Config Dialog
// ========================================

// --- Overlay ---
.liquid-glass-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
  -webkit-backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
}

// --- Overlay Top Drag Bar ---
.overlay-drag-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 38px;
  z-index: 2;
}

// --- Glass Panel ---
.liquid-glass-panel {
  width: min(500px, calc(100vw - 32px));
  max-width: 100%;
  flex: 0 0 auto;
}

.liquid-glass-panel__content {
  position: relative;
  width: 100%;
  overflow: hidden;
  border-radius: 22px;
  padding: 28px;
}

// ==================
// Header
// ==================
.glass-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 18px;
}

.glass-title-group {
  display: flex;
  align-items: center;
  gap: 12px;
}

.glass-icon {
  width: 42px;
  height: 42px;
  border-radius: 13px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, rgba(var(--td-brand-color-rgb, 0, 82, 204), 0.18), rgba(140, 80, 255, 0.12));
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  box-shadow: 0 3px 10px color-mix(in srgb, var(--td-brand-color) 12%, transparent);

  svg {
    color: var(--td-brand-color, #0052d9);
    filter: drop-shadow(0 0 3px rgba(100, 140, 255, 0.25));
  }
}

.glass-title-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.glass-title {
  font-size: 17px;
  font-weight: 600;
  margin: 0;
  color: var(--td-text-color-primary);
  line-height: 1.2;
}

.glass-status {
  display: flex;
  align-items: center;
  gap: 5px;
}

.status-indicator {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--td-text-color-disabled);
  transition: background-color var(--motion-duration-standard) var(--motion-ease-standard), border-color var(--motion-duration-standard) var(--motion-ease-standard), color var(--motion-duration-standard) var(--motion-ease-standard), box-shadow var(--motion-duration-standard) var(--motion-ease-standard), opacity var(--motion-duration-standard) var(--motion-ease-standard), transform var(--motion-duration-standard) var(--motion-ease-standard);

  &.connected {
    background: var(--td-success-color, #2ba471);
    box-shadow: 0 0 6px rgba(43, 164, 113, 0.5);
  }
}

.status-label {
  font-size: 11px;
  color: var(--td-text-color-primary);
  opacity: 0.55;
}

.glass-close-btn {
  width: 30px;
  height: 30px;
  border-radius: 9px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 3%, transparent);
  color: var(--td-text-color-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);

  &:hover {
    background: rgba(255, 80, 80, 0.15);
    border-color: rgba(255, 80, 80, 0.25);
    color: var(--td-error-color, #d54941);
  }

  .iconfont { font-size: 13px; }
}

// ==================
// Tabs
// ==================
.glass-tabs {
  display: flex;
  gap: 3px;
  padding: 3px;
  border-radius: 11px;
  background: color-mix(in srgb, var(--td-text-color-primary) 5%, transparent);
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 5%, transparent);
  margin-bottom: 18px;
}

.glass-tab {
  flex: 1;
  padding: 7px 14px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--td-text-color-secondary);
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 5px;

  svg { flex-shrink: 0; }

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--td-text-color-primary) 3%, transparent);
    color: var(--td-text-color-primary);
  }

  &.active {
    background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
    color: var(--td-text-color-primary);
    box-shadow: 0 1px 4px color-mix(in srgb, var(--td-text-color-primary) 6%, transparent), inset 0 1px 0 color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  }

  &:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
}

// ==================
// Recommend Tab
// ==================
.recommend-intro {
  font-size: 12.5px;
  color: var(--td-text-color-primary);
  opacity: 0.65;
  margin-bottom: 14px;
  line-height: 1.5;
}

.recommend-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.recommend-card {
  padding: 14px;
  border-radius: 13px;
  background: color-mix(in srgb, var(--td-bg-color-component) 30%, transparent);
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  cursor: pointer;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);

  &:hover {
    background: color-mix(in srgb, var(--td-bg-color-component) 45%, transparent);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  }

  &:active {
    transform: translateY(0);
  }
}

.recommend-card-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.recommend-badge {
  font-size: 10px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 6px;
  letter-spacing: 0.02em;
  flex-shrink: 0;

  &.free {
    background: rgba(43, 164, 113, 0.15);
    color: var(--td-success-color, #2ba471);
    border: 1px solid rgba(43, 164, 113, 0.2);
  }
}

.recommend-card-info {
  h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--td-text-color-primary);
    margin: 0;
    line-height: 1.3;
  }

  .recommend-url {
    font-size: 11px;
    color: var(--td-brand-color, #0052d9);
    opacity: 0.8;
  }
}

.recommend-features {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 8px;
}

.feature-tag {
  font-size: 11px;
  padding: 3px 8px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--td-text-color-primary) 4%, transparent);
  color: var(--td-text-color-primary);
  opacity: 0.7;

  b {
    font-weight: 600;
    opacity: 1;
  }
}

.recommend-desc {
  font-size: 12px;
  color: var(--td-text-color-primary);
  opacity: 0.5;
  margin: 0;
  line-height: 1.5;
}

// ==================
// Content
// ==================
.glass-content {
  animation: content-in var(--motion-duration-quick) var(--motion-ease-out);
}

@keyframes content-in {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}

// ==================
// Form
// ==================
.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-bottom: 14px;
}

.form-item {
  display: flex;
  flex-direction: column;
  gap: 5px;

  &.full { grid-column: 1 / -1; }

  label {
    font-size: 12px;
    font-weight: 600;
    color: var(--td-text-color-primary);
    opacity: 0.75;
    letter-spacing: 0.01em;
    padding-left: 2px;
  }

  input {
    width: 100%;
    padding: 9px 12px;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
    background: color-mix(in srgb, var(--td-bg-color-component) 50%, transparent);
    color: var(--td-text-color-primary);
    font-size: 13px;
    outline: none;
    transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
    box-sizing: border-box;

    &::placeholder {
      color: var(--td-text-color-placeholder);
    }

    &:focus {
      border-color: var(--td-brand-color, #0052d9);
      background: color-mix(in srgb, var(--td-bg-color-component) 65%, transparent);
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
    }
  }
}

// ==================
// Buttons
// ==================
.glass-btn {
  width: 100%;
  padding: 10px 18px;
  border-radius: 11px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-text-color-primary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;

  &:hover:not(:disabled) {
    background: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
    transform: translateY(-1px);
    box-shadow: var(--glass-shadow-control);
  }

  &:active:not(:disabled) {
    transform: translateY(0);
    box-shadow: none;
  }

  &:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  &.primary {
    background: var(--td-brand-color, #0052d9);
    border-color: var(--td-brand-color, #0052d9);
    color: #fff;

    &:hover:not(:disabled) {
      background: var(--td-brand-color-hover, #4787f0);
      border-color: var(--td-brand-color-hover, #4787f0);
      box-shadow: 0 4px 16px color-mix(in srgb, var(--td-brand-color) 30%, transparent);
    }

    &:active:not(:disabled) {
      background: var(--td-brand-color-active, #003cab);
    }
  }

  &.accent {
    background: linear-gradient(135deg, rgba(43, 164, 113, 0.2), rgba(0, 130, 200, 0.16));
    border-color: rgba(43, 164, 113, 0.25);
    color: var(--td-success-color, #2ba471);

    &:hover:not(:disabled) {
      background: linear-gradient(135deg, rgba(43, 164, 113, 0.28), rgba(0, 130, 200, 0.22));
      box-shadow: 0 3px 16px rgba(43, 164, 113, 0.15);
    }
  }

  &.danger {
    background: rgba(213, 73, 65, 0.12);
    border-color: rgba(213, 73, 65, 0.2);
    color: var(--td-error-color, #d54941);

    &:hover:not(:disabled) {
      background: rgba(213, 73, 65, 0.2);
      box-shadow: 0 3px 16px rgba(213, 73, 65, 0.12);
    }
  }

  &.outline {
    background: transparent;
    border-color: color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
    color: var(--td-text-color-primary);

    &:hover:not(:disabled) {
      background: color-mix(in srgb, var(--td-text-color-primary) 3%, transparent);
      border-color: color-mix(in srgb, var(--td-text-color-primary) 14%, transparent);
    }
  }
}

// ==================
// Glass Field Group (ops tab top)
// ==================
.glass-field-group {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 10px;
  margin-bottom: 12px;

  .form-item:last-child {
    width: 120px;

    input {
      text-align: center;
    }
  }
}

.input-wrapper {
  position: relative;

  input {
    padding-right: 36px !important;
  }
}

.input-toggle {
  position: absolute;
  right: 6px;
  top: 50%;
  transform: translateY(-50%);
  background: none;
  border: none;
  padding: 4px;
  cursor: pointer;
  opacity: 0.5;
  display: flex;
  align-items: center;
  justify-content: center;

  &:hover { opacity: 0.8; }

  svg { color: var(--td-text-color-secondary); }
}

.restore-field {
  .input-wrapper input {
    width: 100%;
    padding: 9px 12px 9px 36px;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
    background: color-mix(in srgb, var(--td-bg-color-component) 50%, transparent);
    color: var(--td-text-color-primary);
    font-size: 13px;
    outline: none;
    transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
    box-sizing: border-box;

    &::placeholder { color: var(--td-text-color-placeholder); }

    &:focus {
      border-color: var(--td-brand-color, #0052d9);
      background: color-mix(in srgb, var(--td-bg-color-component) 65%, transparent);
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
    }
  }
}

// ==================
// Ops Cards
// ==================
.ops-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px;
  border-radius: 14px;
  background: color-mix(in srgb, var(--td-bg-color-component) 25%, transparent);
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 5%, transparent);

  & + & {
    margin-top: 10px;
  }
}

.ops-card-header {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.ops-card-text {
  flex: 1;
  min-width: 0;

  h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--td-text-color-primary);
    margin: 0 0 2px;
  }

  p {
    font-size: 12px;
    color: var(--td-text-color-primary);
    opacity: 0.65;
    margin: 0;
    line-height: 1.4;
  }
}

.ops-icon {
  width: 38px;
  height: 38px;
  border-radius: 11px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;

  &.backup {
    background: rgba(43, 164, 113, 0.1);
    border: 1px solid rgba(43, 164, 113, 0.15);
    svg { color: var(--td-success-color, #2ba471); }
  }

  &.restore {
    background: rgba(0, 82, 204, 0.1);
    border: 1px solid rgba(0, 82, 204, 0.15);
    svg { color: var(--td-brand-color, #0052d9); }
  }
}

.last-time {
  font-size: 11px;
  color: var(--td-text-color-primary);
  opacity: 0.5;
  margin-top: 3px;
  display: block;
}

.restore-actions {
  display: flex;
  gap: 8px;

  .glass-btn {
    flex: 1;
  }
}

// ==================
// Error / Hint
// ==================
.glass-error {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 9px 12px;
  border-radius: 9px;
  background: rgba(213, 73, 65, 0.08);
  border: 1px solid rgba(213, 73, 65, 0.15);
  color: var(--td-error-color, #d54941);
  font-size: 12px;
  margin-top: 10px;

  svg { flex-shrink: 0; }
}

.glass-hint {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  padding: 9px 12px;
  border-radius: 9px;
  background: rgba(0, 82, 204, 0.06);
  border: 1px solid rgba(0, 82, 204, 0.12);
  color: var(--td-text-color-primary);
  opacity: 0.7;
  font-size: 11.5px;
  margin-top: 12px;
  line-height: 1.5;

  svg {
    flex-shrink: 0;
    margin-top: 1px;
    color: var(--td-brand-color, #0052d9);
    opacity: 0.7;
  }

  b {
    font-weight: 600;
    color: var(--td-text-color-primary);
  }
}

// ==================
// Spinner
// ==================
.glass-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  border-top-color: currentColor;
  border-radius: 50%;
  will-change: transform;
  animation: spin 0.6s linear infinite;
  display: inline-block;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

// ==================
// Transition
// ==================
.glass-fade-enter-active .liquid-glass-panel {
  animation: glass-in var(--motion-duration-standard) var(--motion-ease-out);
}
.glass-fade-leave-active .liquid-glass-panel {
  animation: glass-in var(--motion-duration-quick) var(--motion-ease-out) reverse;
}
.glass-fade-enter-active,
.glass-fade-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard);
}
.glass-fade-enter-from,
.glass-fade-leave-to {
  opacity: 0;
}

@keyframes glass-in {
  from {
    opacity: 0;
    transform: scale(0.96) translateY(8px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@media (max-width: 768px) {
  .liquid-glass-overlay {
    align-items: flex-end;
    justify-content: center;
    padding: calc(var(--mobile-safe-top) + 12px) var(--mobile-page-gutter) calc(var(--mobile-safe-bottom) + 12px);
    background: var(--mobile-scrim);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
  }

  .overlay-drag-bar {
    display: none;
  }

  .liquid-glass-panel {
    width: min(440px, 100%);
    max-height: min(82dvh, 680px);
    display: flex;
  }

  :deep(.glass) {
    height: 100%;
  }

  :deep(.liquid-glass__content) {
    height: 100%;
    overflow: hidden;
  }

  .liquid-glass-panel__content {
    border-radius: var(--mobile-card-radius);
    padding: 20px 16px calc(var(--mobile-safe-bottom) + 16px);
    height: 100%;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  .liquid-glass-panel__content::before {
    content: '';
    display: block;
    width: 38px;
    height: 4px;
    border-radius: 999px;
    background: rgba(120, 120, 128, 0.36);
    margin: -8px auto 12px;
  }

  .glass-header {
    margin-bottom: 14px;
  }

  .glass-title-group {
    min-width: 0;
  }

  .glass-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
  }

  .glass-close-btn,
  .input-toggle {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .glass-tabs {
    gap: 4px;
    margin-bottom: 14px;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
  }

  .glass-tab {
    min-width: max-content;
    min-height: var(--mobile-touch-target);
    padding: 0 12px;
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .form-grid,
  .glass-field-group {
    grid-template-columns: 1fr;
  }

  .glass-field-group .form-item:last-child {
    width: 100%;
  }

  .form-item input,
  .restore-field .input-wrapper input {
    min-height: var(--mobile-touch-target);
    font-size: 16px;
  }

  .input-toggle {
    right: 0;
    padding: 0;
  }

  .recommend-card,
  .ops-card {
    border-radius: var(--mobile-card-radius-small);
  }

  .recommend-card {
    min-height: var(--mobile-touch-target);
    touch-action: manipulation;
  }

  .glass-btn {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .restore-actions {
    flex-direction: column;
  }

  .glass-hint {
    line-height: 1.55;
  }
}

@media (prefers-reduced-motion: reduce) {
  .glass-spinner {
    animation: none !important;
  }

  .liquid-glass-panel {
    animation: none !important;
    transition: none !important;
    transform: none !important;
  }
}
</style>
