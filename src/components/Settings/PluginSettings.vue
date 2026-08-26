<template>
  <div class="plugin-section">
    <div class="section-header">
      <h2>{{ t('settings.plugin.title') }}</h2>
      <div class="header-actions">
        <t-button theme="primary" @click="showAddDialog = true">
          <template #icon><t-icon name="add" /></template> {{ t('settings.plugin.addPlugin') }}
        </t-button>
        <t-button theme="default" @click="doRefresh">
          <template #icon><t-icon name="refresh" /></template> {{ t('common.refresh') }}
        </t-button>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="store.loading && store.plugins.length === 0" class="state-block loading-state">
      <t-loading size="medium" />
      <span>{{ t('settings.plugin.loading') }}</span>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="state-block error-state">
      <t-icon name="error-circle" style="font-size: 48px; color: var(--td-error-color)" />
      <p>{{ t('settings.plugin.loadError') }}</p>
      <p class="error-msg">{{ error }}</p>
      <t-button theme="default" size="small" @click="doRefresh">
        <template #icon><t-icon name="refresh" /></template> {{ t('common.retry') }}
      </t-button>
    </div>

    <!-- Empty -->
    <div v-else-if="store.plugins.length === 0" class="state-block empty-state">
      <t-icon name="app" style="font-size: 48px" />
      <p>{{ t('settings.plugin.noPlugins') }}</p>
      <p class="hint">{{ t('settings.plugin.noPluginsHint') }}</p>
    </div>

    <!-- Plugin List -->
    <div v-else class="plugin-list">
      <div
        v-for="plugin in store.plugins"
        :key="plugin.plugin_id"
        class="plugin-card"
        :class="{ selected: store.isSelected(plugin.plugin_id) }"
      >
        <div class="plugin-info">
          <h3>
            {{ plugin.plugin_info.name }}
            <span class="version">v{{ plugin.plugin_info.version }}</span>
            <span v-if="store.isSelected(plugin.plugin_id)" class="tag tag-current">{{ t('settings.plugin.currentUsing') }}</span>
            <span v-if="store.isServicePlugin(plugin)" class="tag tag-service">{{ t('settings.plugin.servicePlugin') }}</span>
          </h3>
          <p class="author">{{ t('settings.plugin.author', { name: plugin.plugin_info.author }) }}</p>
          <p class="description">{{ plugin.plugin_info.description || t('settings.plugin.noDescription') }}</p>
          <div v-if="plugin.supported_sources.length > 0" class="sources">
            <span class="source-label">{{ t('settings.plugin.supportedSources') }}</span>
            <span v-for="source in plugin.supported_sources" :key="source.name" class="source-tag">
              {{ source.name }}
            </span>
          </div>
        </div>
        <div class="plugin-actions">
          <t-button
            theme="default"
            size="small"
            @click="viewLogs(plugin)"
          >
            <template #icon><t-icon name="view-list" /></template> {{ t('settings.plugin.logs') }}
          </t-button>
          <t-button
            v-if="store.isServicePlugin(plugin)"
            theme="default"
            size="small"
            @click="openConfig(plugin)"
          >
            <template #icon><t-icon name="setting" /></template> {{ t('settings.plugin.config') }}
          </t-button>
          <t-button
            v-if="store.isServicePlugin(plugin)"
            theme="primary"
            size="small"
            @click="openImport(plugin)"
          >
            <template #icon><t-icon name="download" /></template> {{ t('settings.plugin.importPlaylist') }}
          </t-button>
          <t-button
            v-if="!store.isSelected(plugin.plugin_id) && !store.isServicePlugin(plugin)"
            theme="primary"
            size="small"
            @click="doSelect(plugin)"
          >
            <template #icon><t-icon name="check" /></template> {{ t('settings.plugin.use') }}
          </t-button>
          <t-button
            theme="danger"
            size="small"
            variant="outline"
            @click="confirmUninstall(plugin)"
          >
            <template #icon><t-icon name="delete" /></template> {{ t('settings.plugin.uninstall') }}
          </t-button>
        </div>
      </div>
    </div>

    <!-- Add Plugin (merged: type + method in one dialog) -->
    <t-dialog
      v-model:visible="showAddDialog"
      :header="t('settings.plugin.addPlugin')"
      :confirm-btn="{ content: t('common.confirm') }"
      :cancel-btn="{ content: t('common.cancel') }"
      attach="body"
      @confirm="doImport"
    >
      <div class="add-plugin-form">
        <div class="form-section">
          <p class="dialog-tip">{{ t('settings.plugin.pluginTypeTip') }}</p>
          <t-radio-group v-model="addPluginType" variant="primary-filled">
            <t-radio-button value="cr">{{ t('settings.plugin.cranPlugin') }}</t-radio-button>
            <t-radio-button value="lx">{{ t('settings.plugin.lxPlugin') }}</t-radio-button>
          </t-radio-group>
        </div>
        <div class="form-section">
          <p class="dialog-tip">{{ t('settings.plugin.selectImportMethod') }}</p>
          <t-radio-group v-model="importMethod" variant="primary-filled">
            <t-radio-button value="local">{{ t('settings.plugin.localImport') }}</t-radio-button>
            <t-radio-button value="online">{{ t('settings.plugin.onlineImport') }}</t-radio-button>
          </t-radio-group>
          <div v-if="importMethod === 'online'" class="online-section">
            <t-input
              v-model="onlineUrl"
              :placeholder="t('settings.plugin.onlineUrlPlaceholder')"
              size="large"
            />
            <p class="dialog-tip">{{ t('settings.plugin.onlineUrlTip') }}</p>
          </div>
          <div v-else class="dialog-tip">
            {{ t('settings.plugin.localImportTip') }}
          </div>
        </div>
      </div>
    </t-dialog>

    <!-- Log Dialog -->
    <t-dialog
      v-model:visible="logDialogVisible"
      :close-btn="false"
      :footer="false"
      attach="body"
      width="80%"
      :style="{ maxWidth: '900px' }"
      class="log-dialog"
    >
      <template #header>
        <div class="log-dialog-header">
          <div class="log-title">
            <t-icon name="view-list" />
            {{ t('settings.plugin.pluginLog', { name: currentLogName }) }}
          </div>
          <div class="log-actions">
            <t-button size="small" variant="outline" :loading="logsLoading" @click="refreshLogs">
              {{ t('common.refresh') }}
            </t-button>
          </div>
          <div class="mac-controls">
            <div class="mac-btn close" @click="logDialogVisible = false" />
            <div class="mac-btn minimize" />
            <div class="mac-btn maximize" />
          </div>
        </div>
      </template>
      <div class="console-container">
        <div class="console-header">
          <div class="console-info">
            <span class="console-prompt">$</span>
            <span class="console-path">~/plugins/{{ currentLogName }}</span>
            <span class="console-time">{{ formatTime(new Date()) }}</span>
          </div>
        </div>
        <div ref="logContentRef" class="console-content">
          <div v-if="logsLoading" class="console-loading">
            <t-loading size="small" />
            <span>{{ t('settings.plugin.loadingLogs') }}</span>
          </div>
          <div v-else-if="logsError" class="console-error">
            <span>{{ t('settings.plugin.loadLogsFailed', { msg: logsError }) }}</span>
          </div>
          <div v-else-if="logs.length === 0" class="console-empty">
            <span>{{ t('settings.plugin.noLogs') }}</span>
          </div>
          <div v-else class="log-entries">
            <div
              v-for="(log, i) in logs"
              :key="i"
              class="log-entry"
              :class="getLogLevel(log)"
            >
              <span class="log-ts">{{ formatLogTime(i) }}</span>
              <span class="log-content">{{ log }}</span>
            </div>
          </div>
        </div>
      </div>
    </t-dialog>

    <!-- Config Dialog -->
    <t-dialog
      v-model:visible="configDialogVisible"
      :header="t('settings.plugin.configTitle', { name: configPluginName })"
      :confirm-btn="{ content: t('common.save') }"
      :cancel-btn="{ content: t('common.cancel') }"
      attach="body"
      @confirm="doSaveConfig"
    >
      <div class="config-form">
        <div v-for="field in configSchema" :key="field.key" class="config-field">
          <label class="config-label">
            {{ field.label }}
            <span v-if="field.required" class="required">*</span>
          </label>
          <t-input
            v-if="field.type === 'text'"
            v-model="configValues[field.key]"
            :placeholder="field.placeholder || ''"
          />
          <t-input
            v-else-if="field.type === 'password'"
            v-model="configValues[field.key]"
            type="password"
            :placeholder="field.placeholder || ''"
          />
          <t-input-number
            v-else-if="field.type === 'number'"
            v-model="configValues[field.key]"
            :placeholder="field.placeholder || ''"
            style="width: 100%"
          />
          <t-select
            v-else-if="field.type === 'select'"
            v-model="configValues[field.key]"
            :placeholder="field.placeholder || t('settings.plugin.selectPlaceholder')"
          >
            <t-option
              v-for="opt in field.options"
              :key="opt.value"
              :value="opt.value"
              :label="opt.label"
            />
          </t-select>
        </div>
        <div class="config-test">
          <t-button
            theme="default"
            size="small"
            :loading="configTesting"
            @click="doTestConnection"
          >
            {{ t('settings.plugin.testConnection') }}
          </t-button>
          <span
            v-if="configTestResult"
            class="test-result"
            :class="{ success: configTestResult.success, fail: !configTestResult.success }"
          >
            {{ configTestResult.message }}
          </span>
        </div>
      </div>
    </t-dialog>

    <!-- Import Playlist -->
    <ImportPlaylist
      v-model:visible="importDialogVisible"
      :plugin-id="importPluginId"
      :plugin-name="importPluginName"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { MessagePlugin, DialogPlugin } from 'tdesign-vue-next'
import { usePluginStore } from '@/store/plugin'
import type { LoadedPlugin, PluginConfigField } from '@/store/plugin'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import ImportPlaylist from '@/components/ServicePlugin/ImportPlaylist.vue'

const { t } = useI18n()
const store = usePluginStore()
const userStore = LocalUserDetailStore()
const error = ref<string | null>(null)

// Add plugin flow
const showAddDialog = ref(false)
const addPluginType = ref<'cr' | 'lx'>('cr')
const importMethod = ref<'local' | 'online'>('local')
const onlineUrl = ref('')

// Log dialog
const logDialogVisible = ref(false)
const currentLogId = ref('')
const currentLogName = ref('')
const logs = ref<string[]>([])
const logsLoading = ref(false)
const logsError = ref<string | null>(null)
const logContentRef = ref<HTMLElement>()

// Config dialog
const configDialogVisible = ref(false)
const configPluginId = ref('')
const configPluginName = ref('')
const configSchema = ref<PluginConfigField[]>([])
const configValues = ref<Record<string, any>>({})
const configTesting = ref(false)
const configTestResult = ref<{ success: boolean; message: string } | null>(null)

// Import dialog
const importDialogVisible = ref(false)
const importPluginId = ref('')
const importPluginName = ref('')

onMounted(() => {
  doRefresh()
})

async function doRefresh() {
  error.value = null
  try {
    await store.initialize()
  } catch (e: any) {
    error.value = e.message || t('settings.plugin.unknownError')
  }
}

// ==================== Add Plugin ====================

async function doImport() {
  try {
    showAddDialog.value = false
    if (importMethod.value === 'local') {
      const result = await store.selectAndAdd(addPluginType.value)
      if (!result) return // canceled
      MessagePlugin.success(t('settings.plugin.installSuccess', { name: result.plugin_info.name }))
    } else {
      if (!onlineUrl.value.trim()) {
        MessagePlugin.warning(t('settings.plugin.enterUrl'))
        showAddDialog.value = true
        return
      }
      try { new URL(onlineUrl.value) } catch {
        MessagePlugin.warning(t('settings.plugin.invalidUrl'))
        showAddDialog.value = true
        return
      }
      const result = await store.downloadAndAdd(onlineUrl.value.trim(), addPluginType.value)
      MessagePlugin.success(t('settings.plugin.installSuccess', { name: result.plugin_info.name }))
    }
    onlineUrl.value = ''
  } catch (e: any) {
    const msg = typeof e === 'string' ? e : (e.message || t('settings.plugin.unknownError'))
    MessagePlugin.error(t('settings.plugin.installFailed', { msg }))
  }
}

// ==================== Select / Uninstall ====================

function doSelect(plugin: LoadedPlugin) {
  store.selectPlugin(plugin)

  const { plugin_id, plugin_info, supported_sources } = plugin

  if (!supported_sources || supported_sources.length === 0) {
    userStore.userInfo.pluginId = plugin_id
    userStore.userInfo.pluginName = plugin_info.name
    userStore.userInfo.supportedSources = {}
    userStore.userInfo.selectSources = ''
    userStore.userInfo.selectQuality = ''
    MessagePlugin.success(t('settings.plugin.selectPluginNoSource', { name: plugin_info.name }))
    return
  }

  // Convert supported_sources array to object keyed by source_id
  const supportedSourcesForStore: Record<string, any> = {}
  for (const src of supported_sources) {
    const key = src.source_id || src.name
    supportedSourcesForStore[key] = {
      name: src.name,
      type: t('settings.plugin.sourceType'),
      qualitys: src.qualities,
    }
  }

  // Preserve previous source selection or use first available
  let selectSources: string
  const prevSource = userStore.userInfo.selectSources as string
  if (prevSource && supportedSourcesForStore[prevSource]) {
    selectSources = prevSource
  } else {
    selectSources = Object.keys(supportedSourcesForStore)[0]
  }

  // Preserve previous quality or use highest available
  let selectQuality: string
  const qualitys: string[] = supportedSourcesForStore[selectSources]?.qualitys || []
  const prevQuality = userStore.userInfo.selectQuality as string
  if (prevQuality && qualitys.includes(prevQuality)) {
    selectQuality = prevQuality
  } else {
    selectQuality = qualitys.length > 0 ? qualitys[qualitys.length - 1] : ''
  }

  userStore.userInfo.pluginId = plugin_id
  userStore.userInfo.pluginName = plugin_info.name
  userStore.userInfo.supportedSources = userStore.mergeBuiltInSources(userStore.userInfo, supportedSourcesForStore)
  userStore.userInfo.selectSources = selectSources
  userStore.userInfo.selectQuality = selectQuality

  MessagePlugin.success(t('settings.plugin.selectPlugin', { name: plugin_info.name }))
}

function confirmUninstall(plugin: LoadedPlugin) {
  const dialog = DialogPlugin.confirm({
    header: t('settings.plugin.confirmUninstallTitle'),
    body: t('settings.plugin.confirmUninstallBody', { name: plugin.plugin_info.name }),
    confirmBtn: { content: t('settings.plugin.confirmUninstallBtn'), theme: 'danger' },
    cancelBtn: { content: t('common.cancel') },
    onConfirm: async () => {
      try {
        await store.uninstallPlugin(plugin.plugin_id)
        MessagePlugin.success(t('settings.plugin.uninstallSuccess', { name: plugin.plugin_info.name }))
        dialog.destroy()
      } catch (e: any) {
        MessagePlugin.error(t('settings.plugin.uninstallFailed', { msg: e.message }))
      }
    }
  })
}

// ==================== Logs ====================

async function viewLogs(plugin: LoadedPlugin) {
  currentLogId.value = plugin.plugin_id
  currentLogName.value = plugin.plugin_info.name
  logDialogVisible.value = true
  await loadLogs()
}

async function loadLogs() {
  if (!currentLogId.value) return
  logsLoading.value = true
  logsError.value = null
  try {
    logs.value = await store.getPluginLog(currentLogId.value)
    await nextTick()
    if (logContentRef.value) {
      logContentRef.value.scrollTop = logContentRef.value.scrollHeight
    }
  } catch (e: any) {
    logsError.value = e.message
    logs.value = []
  } finally {
    logsLoading.value = false
  }
}

function refreshLogs() { loadLogs() }

function getLogLevel(log: string): string {
  const l = log.toLowerCase()
  if (l.includes('error') || l.includes('错误')) return 'log-error'
  if (l.includes('warn') || l.includes('警告')) return 'log-warn'
  if (l.includes('info') || l.includes('信息')) return 'log-info'
  if (l.includes('debug') || l.includes('调试')) return 'log-debug'
  return 'log-default'
}

function formatTime(date: Date): string {
  return date.toLocaleTimeString('zh-CN', { hour12: false })
}

function formatLogTime(index: number): string {
  const now = new Date()
  const t = new Date(now.getTime() - (logs.value.length - index - 1) * 1000)
  return t.toLocaleTimeString('zh-CN', { hour12: false })
}

// ==================== Config ====================

async function openConfig(plugin: LoadedPlugin) {
  configPluginId.value = plugin.plugin_id
  configPluginName.value = plugin.plugin_info.name
  configTestResult.value = null
  try {
    configSchema.value = await store.getConfigSchema(plugin.plugin_id)
    const saved = await store.getConfig(plugin.plugin_id)
    const values: Record<string, any> = {}
    for (const field of configSchema.value) {
      values[field.key] = saved[field.key] ?? field.default ?? ''
    }
    configValues.value = values
    configDialogVisible.value = true
  } catch (e: any) {
    MessagePlugin.error(t('settings.plugin.getPluginConfigFailed', { msg: e.message }))
  }
}

async function doSaveConfig() {
  try {
    for (const field of configSchema.value) {
      if (field.required && !configValues.value[field.key]) {
        MessagePlugin.warning(t('settings.plugin.requiredField', { label: field.label }))
        return
      }
    }
    await store.saveConfig(configPluginId.value, JSON.parse(JSON.stringify(configValues.value)))
    MessagePlugin.success(t('settings.plugin.configSaved'))
    configDialogVisible.value = false
  } catch (e: any) {
    MessagePlugin.error(t('settings.plugin.saveConfigFailed', { msg: e.message }))
  }
}

async function doTestConnection() {
  configTesting.value = true
  configTestResult.value = null
  try {
    await store.saveConfig(configPluginId.value, JSON.parse(JSON.stringify(configValues.value)))
    configTestResult.value = await store.testConnection(configPluginId.value)
    if (configTestResult.value.success) {
      MessagePlugin.success(configTestResult.value.message)
    } else {
      MessagePlugin.error(configTestResult.value.message)
    }
  } catch (e: any) {
    configTestResult.value = { success: false, message: e.message }
    MessagePlugin.error(t('settings.plugin.testConnectionFailed', { msg: e.message }))
  } finally {
    configTesting.value = false
  }
}

// ==================== Import Playlist ====================

function openImport(plugin: LoadedPlugin) {
  importPluginId.value = plugin.plugin_id
  importPluginName.value = plugin.plugin_info.name
  importDialogVisible.value = true
}
</script>

<style scoped lang="scss">
.plugin-section {
  height: 100%;
  color: var(--td-text-color-primary);
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
  flex-shrink: 0;

  h2 {
    font-size: 24px;
    font-weight: 600;
    border-left: 8px solid var(--td-brand-color-3);
    padding-left: 12px;
    border-radius: 8px;
    line-height: 1.5em;
    margin: 0;
    color: var(--td-text-color-primary);
  }
}

.header-actions {
  display: flex;
  gap: 12px;
}

// State blocks
.state-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 0;
  gap: 12px;
  background: var(--settings-feature-bg, var(--td-bg-color-container));
  border: 1px solid var(--settings-feature-border, transparent);
  border-radius: 12px;
  color: var(--td-text-color-secondary);
}

.state-block p {
  margin: 0;
  color: var(--td-text-color-primary);
}

.error-msg {
  color: var(--td-error-color);
  font-size: 14px;
  max-width: 80%;
  text-align: center;
}

.hint {
  font-size: 14px;
  color: var(--td-text-color-placeholder);
}

// Plugin list
.plugin-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.plugin-card {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 20px;
  border-radius: 12px;
  background: var(--plugins-card-bg, var(--td-bg-color-container));
  box-shadow: var(--plugins-card-shadow, var(--theme-card-shadow));
  transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease, box-shadow 0.3s ease, opacity 0.3s ease, transform 0.3s ease;
  border: 2px solid transparent;

  &.selected {
    border-color: var(--td-brand-color);
    background: var(--td-brand-color-1);
  }
}

.plugin-info {
  flex: 1;
  margin-right: 20px;
  color: var(--td-text-color-primary);

  h3 {
    margin: 0 0 8px 0;
    font-size: 18px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 10px;
    line-height: 1.4;
    color: var(--td-text-color-primary);
  }
}

.version {
  font-size: 12px;
  color: var(--td-text-color-placeholder);
  font-weight: 500;
  background: var(--td-bg-color-secondarycontainer);
  padding: 2px 8px;
  border-radius: 6px;
}

.tag {
  padding: 3px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  color: var(--td-text-color-anti);

  &.tag-current {
    background: linear-gradient(135deg, var(--td-brand-color-5), var(--td-brand-color-6));
    box-shadow: var(--theme-shadow-light);
  }

  &.tag-service {
    background: linear-gradient(135deg, var(--td-brand-color-4), var(--td-brand-color-5));
    box-shadow: var(--theme-shadow-light);
  }
}

.author {
  margin: 0 0 4px 0;
  font-size: 14px;
  color: var(--td-text-color-secondary);
}

.description {
  margin: 0 0 8px 0;
  font-size: 14px;
  color: var(--td-text-color-secondary);
  line-height: 1.5;
}

.sources {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}

.source-label {
  font-size: 13px;
  color: var(--td-text-color-secondary);
  font-weight: 500;
}

.source-tag {
  background: linear-gradient(135deg, var(--td-brand-color-4), var(--td-brand-color-5));
  color: var(--td-text-color-anti);
  padding: 3px 10px;
  border-radius: 10px;
  font-size: 12px;
  font-weight: 500;
}

.plugin-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 120px;
}

// Dialog tips
.dialog-tip {
  font-size: 13px;
  color: var(--td-text-color-secondary);
  margin-bottom: 16px;
  line-height: 1.5;
}

.add-plugin-form {
  display: flex;
  flex-direction: column;
  gap: 20px;

  .form-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
}

.online-section {
  margin-top: 16px;
}

// Config form
.config-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 8px 0;
  color: var(--td-text-color-primary);
}

.config-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.config-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--td-text-color-primary);

  .required {
    color: var(--td-error-color);
    margin-left: 2px;
  }
}

.config-test {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 8px;
  padding-top: 12px;
  border-top: 1px solid var(--td-border-level-1-color);

  .test-result {
    font-size: 13px;
    &.success { color: var(--td-success-color); }
    &.fail { color: var(--td-error-color); }
  }
}

// Log dialog
.log-dialog-header {
  display: flex;
  align-items: center;
  padding: 12px 20px;
  background: var(--td-bg-color-secondarycontainer);
  width: 100%;

  .log-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    font-size: 14px;
    flex: 1;
    color: var(--td-text-color-primary);
  }

  .log-actions {
    display: flex;
    gap: 8px;
    margin-right: 12px;
  }

  .mac-controls {
    display: flex;
    gap: 8px;

    .mac-btn {
      width: 12px;
      height: 12px;
      border-radius: 50%;
      cursor: pointer;

      &.close { background: var(--plugins-mac-close); &:hover { background: var(--plugins-mac-close); filter: brightness(0.92); } }
      &.minimize { background: var(--plugins-mac-minimize); &:hover { background: var(--plugins-mac-minimize); filter: brightness(0.92); } }
      &.maximize { background: var(--plugins-mac-maximize); &:hover { background: var(--plugins-mac-maximize); filter: brightness(1.08); } }
    }
  }
}

.console-container {
  background: var(--plugins-console-bg);
  color: var(--plugins-console-text);
  font-family: 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Code', 'Consolas', monospace;
  font-size: 13px;
  line-height: 1.4;
  min-height: 300px;
  display: flex;
  flex-direction: column;
  border-radius: 0 0 12px 12px;
}

.console-header {
  background: var(--plugins-console-header-bg);
  border-bottom: 1px solid var(--plugins-console-border);
  padding: 8px 16px;

  .console-info {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12px;

    .console-prompt { color: var(--plugins-console-prompt); font-weight: bold; }
    .console-path { color: var(--plugins-console-path); }
    .console-time { color: var(--plugins-console-time); margin-left: auto; }
  }
}

.console-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  max-height: 50vh;

  &::-webkit-scrollbar { width: 8px; }
  &::-webkit-scrollbar-track { background: var(--plugins-console-scrollbar-track); }
  &::-webkit-scrollbar-thumb { background: var(--plugins-console-scrollbar-thumb); border-radius: 4px; &:hover { background: var(--plugins-console-scrollbar-thumb-hover); } }
}

.console-loading,
.console-error,
.console-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 40px 0;
  color: var(--plugins-console-time);
}

.console-error { color: var(--plugins-log-error); }

.log-entries {
  .log-entry {
    display: flex;
    margin-bottom: 4px;
    padding: 2px 0;
    border-radius: 3px;

    &:hover { background: var(--plugins-console-header-bg); }

    .log-ts {
      color: var(--plugins-console-time);
      font-size: 11px;
      width: 80px;
      text-align: center;
      flex-shrink: 0;
      margin-right: 12px;
      font-weight: 500;
    }

    .log-content {
      flex: 1;
      word-break: break-all;
      white-space: pre-wrap;
      user-select: text;
    }

    &.log-error .log-content { color: var(--plugins-log-error); }
    &.log-warn .log-content { color: var(--plugins-log-warn); }
    &.log-info .log-content { color: var(--plugins-log-info); }
    &.log-debug .log-content { color: var(--plugins-log-debug); }
    &.log-default .log-content { color: var(--plugins-console-text); }
  }
}

// Responsive
@media (max-width: 768px) {
  .section-header {
    align-items: flex-start;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 14px;

    h2 {
      font-size: 20px;
      border-left-width: 6px;
      padding-left: 10px;
    }
  }

  .header-actions {
    width: 100%;
    gap: 8px;

    .t-button {
      flex: 1;
    }
  }

  .state-block {
    padding: 36px 14px;
    border-radius: 10px;
  }

  .plugin-list {
    gap: 10px;
  }

  .plugin-card {
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border-width: 1px;

    .plugin-info {
      width: 100%;
      min-width: 0;
      margin-right: 0;

      h3 {
        flex-wrap: wrap;
        gap: 6px;
        font-size: 16px;
      }
    }

    .plugin-actions {
      width: 100%;
      min-width: auto;
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }
  }

  .sources {
    align-items: flex-start;
  }

  .config-test {
    align-items: flex-start;
    flex-direction: column;
    gap: 8px;
  }

  .log-dialog-header {
    padding: 10px 12px;

    .log-title {
      min-width: 0;
      font-size: 13px;
    }

    .mac-controls {
      display: none;
    }
  }

  .console-container {
    min-height: 260px;
    font-size: 12px;
  }

  .console-header .console-info {
    gap: 8px;
    overflow: hidden;

    .console-path {
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }

  .console-content {
    padding: 12px;
    max-height: 55vh;
  }

  .log-entries .log-entry {
    flex-direction: column;
    gap: 2px;

    .log-ts {
      width: auto;
      text-align: left;
      margin-right: 0;
    }
  }
}
</style>

<style lang="scss">
.log-dialog {
  .t-dialog {
    border-radius: 12px;
    overflow: hidden;
  }
  .t-dialog__header {
    padding: 0;
    background: var(--td-bg-color-secondarycontainer);
    border-bottom: 1px solid var(--td-border-level-1-color);
  }
  .t-dialog__body {
    padding: 0;
  }
}
</style>
