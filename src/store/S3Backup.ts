import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import i18n from '@/locales'

export interface S3Config {
  endpoint: string
  region: string
  accessKeyId: string
  secretAccessKey: string
  bucket: string
}

export interface BackupData {
  version: number
  timestamp: string
  settings: any
}

type RestoreMode = 'overwrite' | 'merge'

const STORAGE_KEY = 's3BackupConfig'

export const useS3BackupStore = defineStore('s3Backup', () => {
  const config = ref<S3Config>({
    endpoint: '',
    region: 'auto',
    accessKeyId: '',
    secretAccessKey: '',
    bucket: ''
  })

  const isConnected = ref(false)
  const isConnecting = ref(false)
  const isBackingUp = ref(false)
  const isRestoring = ref(false)
  const lastBackupTime = ref<string | null>(null)
  const errorMessage = ref<string | null>(null)
  const backupPassword = ref('')
  const maxBackups = ref(10)

  const statusText = computed(() => {
    if (isConnecting.value) return i18n.global.t('backup.connecting')
    if (isBackingUp.value) return i18n.global.t('backup.backingUp')
    if (isRestoring.value) return i18n.global.t('backup.restoring')
    if (isConnected.value) return i18n.global.t('backup.connected')
    return i18n.global.t('backup.disconnected')
  })

  function loadConfig() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY)
      if (saved) {
        const parsed = JSON.parse(saved) as Partial<S3Config>
        config.value = {
          endpoint: parsed.endpoint || '',
          region: parsed.region || 'auto',
          accessKeyId: parsed.accessKeyId || '',
          secretAccessKey: '',
          bucket: parsed.bucket || ''
        }
        if (parsed.secretAccessKey) {
          localStorage.setItem(STORAGE_KEY, JSON.stringify({
            endpoint: config.value.endpoint,
            region: config.value.region,
            accessKeyId: config.value.accessKeyId,
            bucket: config.value.bucket
          }))
        }
      }
      const time = localStorage.getItem('lastBackupTime')
      if (time) lastBackupTime.value = time
      const mb = localStorage.getItem('maxBackups')
      if (mb) maxBackups.value = parseInt(mb, 10) || 10
    } catch (e) {
      console.error('加载 S3 配置失败:', e)
    }
  }

  function saveConfig() {
    const persistedConfig = {
      endpoint: config.value.endpoint,
      region: config.value.region,
      accessKeyId: config.value.accessKeyId,
      bucket: config.value.bucket
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(persistedConfig))
    localStorage.setItem('maxBackups', String(maxBackups.value))
  }

  function toApiConfig(): Record<string, string> {
    return {
      endpoint: config.value.endpoint,
      region: config.value.region || 'auto',
      accessKeyId: config.value.accessKeyId,
      secretAccessKey: config.value.secretAccessKey,
      bucket: config.value.bucket
    }
  }

  async function testConnection(): Promise<boolean> {
    isConnecting.value = true
    errorMessage.value = null
    try {
      await (window as any).api.s3.testConnection(toApiConfig())
      isConnected.value = true
      saveConfig()
      return true
    } catch (e: any) {
      isConnected.value = false
      errorMessage.value = e?.message || e?.toString() || i18n.global.t('backup.connectFailed')
      return false
    } finally {
      isConnecting.value = false
    }
  }

  async function backup(): Promise<boolean> {
    if (!isConnected.value) {
      errorMessage.value = i18n.global.t('backup.connectFirst')
      return false
    }
    if (!backupPassword.value) {
      errorMessage.value = i18n.global.t('backup.setPassword')
      return false
    }

    isBackingUp.value = true
    errorMessage.value = null
    try {
      const settings = JSON.parse(localStorage.getItem('appSettings') || '{}')

      const result = await (window as any).api.s3.backup(
        toApiConfig(),
        settings,
        backupPassword.value,
        maxBackups.value
      )

      lastBackupTime.value = result.timestamp
      localStorage.setItem('lastBackupTime', result.timestamp)
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || e?.toString() || i18n.global.t('backup.backupFailed')
      return false
    } finally {
      isBackingUp.value = false
    }
  }

  async function restore(mode: RestoreMode = 'overwrite', password: string): Promise<boolean> {
    if (!isConnected.value) {
      errorMessage.value = i18n.global.t('backup.connectFirst')
      return false
    }
    if (!password) {
      errorMessage.value = i18n.global.t('backup.inputRestorePassword')
      return false
    }

    isRestoring.value = true
    errorMessage.value = null
    try {
      const result = await (window as any).api.s3.restore(toApiConfig(), password, mode)
      const data: BackupData = result.data

      if (mode === 'overwrite') {
        localStorage.setItem('appSettings', JSON.stringify(data.settings))
      } else {
        const localSettings = JSON.parse(localStorage.getItem('appSettings') || '{}')
        const mergedSettings = { ...data.settings, ...localSettings }
        localStorage.setItem('appSettings', JSON.stringify(mergedSettings))
      }

      return true
    } catch (e: any) {
      errorMessage.value = e?.message || e?.toString() || i18n.global.t('backup.restoreFailed')
      return false
    } finally {
      isRestoring.value = false
    }
  }

  // Initialize
  loadConfig()
  if (config.value.endpoint && config.value.accessKeyId && config.value.secretAccessKey) {
    testConnection()
  }

  return {
    config,
    isConnected,
    isConnecting,
    isBackingUp,
    isRestoring,
    lastBackupTime,
    errorMessage,
    statusText,
    backupPassword,
    maxBackups,
    testConnection,
    backup,
    restore,
    saveConfig
  }
})
