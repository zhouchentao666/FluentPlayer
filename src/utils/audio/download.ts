import { NotifyPlugin, MessagePlugin } from 'tdesign-vue-next'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { useSettingsStore } from '@/store/Settings'
import { computed, createApp, defineComponent, h, onMounted, onUnmounted, ref, toRaw, type CSSProperties, type PropType } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { musicSdk } from '@/services/musicSdk'
import PluginRunner from '@/utils/plugin/PluginRunner'
import LiquidGlass from '@/components/LiquidGlass.vue'
import {
  getQualityDisplayName,
  buildQualityFormats,
  compareQuality,
  calculateBestQuality,
  filterByPluginQualities
} from '@/utils/quality'
import i18n from '@/locales'

interface MusicItem {
  singer: string
  name: string
  albumName: string
  albumId: number
  source: string
  interval: string
  songmid: number
  img: string
  lrc: null | string
  types: Array<{ type: string; size: string }>
  _types: Record<string, any>
  typeUrl: Record<string, any>
}

function extractLyricText(result: any): string {
  if (!result) return ''
  if (typeof result === 'string') return result
  if (typeof result?.data === 'string') return result.data
  const obj = typeof result === 'object' ? result : typeof result?.data === 'object' ? result.data : null
  if (!obj) return ''
  return obj.lxlyric || obj.crlyric || obj.lyric || obj.lrc || obj.tlyric || obj.rlyric || ''
}

function getCleanSongInfo(songInfo: any) {
  return JSON.parse(JSON.stringify(toRaw(songInfo)))
}

function getLikelyServicePluginId(songInfo: any): string | undefined {
  const keys = ['_servicePluginId', 'servicePluginId']
  for (const key of keys) {
    const val = songInfo?.[key]
    if (typeof val === 'string' && val) return val
  }
  return undefined
}

async function fetchTtmlLyricText(source: string, songId: string | number): Promise<string> {
  const ttmlSource = source === 'wy' ? 'ncm' : source === 'tx' ? 'qq' : ''
  if (!ttmlSource) return ''

  const url = `https://amll-ttml-db.stevexmh.net/${ttmlSource}/${songId}`
  const proxyResponse = await (window as any).api?.httpProxy?.(url, { method: 'GET', timeout: 10000 })
  const statusCode = Number(proxyResponse?.statusCode || 0)
  if (statusCode >= 400) return ''
  const body = proxyResponse?.body
  return typeof body === 'string' && body.length >= 100 ? body : ''
}

async function fetchSdkLyricText(source: string, songInfo: MusicItem): Promise<string> {
  try {
    const result = await (window as any).api?.music?.requestSdk?.('getLyric', {
      source,
      songInfo: getCleanSongInfo(songInfo),
      grepLyricInfo: true,
      useStrictMode: false
    })
    return extractLyricText(result)
  } catch {
    return ''
  }
}

async function fetchServicePluginLyricText(pluginId: string | undefined, songInfo: MusicItem): Promise<string> {
  if (!pluginId) return ''
  try {
    const result = await PluginRunner.getLyric(pluginId, songInfo.source || 'kw', getCleanSongInfo(songInfo))
    return extractLyricText(result)
  } catch {
    return ''
  }
}

async function resolveDownloadLyricText(pluginId: string | undefined, songInfo: MusicItem): Promise<string> {
  const source = songInfo.source || 'kw'
  const songId = songInfo.songmid

  if (source === 'wy' || source === 'tx') {
    const ttml = await fetchTtmlLyricText(source, songId)
    if (ttml) return ttml
    return fetchSdkLyricText(source, songInfo)
  }

  const servicePluginId = getLikelyServicePluginId(songInfo) || pluginId
  const serviceLyric = await fetchServicePluginLyricText(servicePluginId, songInfo)
  if (serviceLyric) return serviceLyric

  return fetchSdkLyricText(source, songInfo)
}


type DownloadQualityOption = { type: string; size?: string }

const DOWNLOAD_QUALITY_DIALOG_STYLE_ID = 'download-quality-liquid-glass-dialog-style'

const downloadQualityLiquidGlassContentStyle: CSSProperties = {
  color: 'var(--td-text-color-primary)',
  font: 'inherit',
  lineHeight: 'normal',
  textShadow: 'none'
}

function ensureDownloadQualityDialogStyle() {
  if (typeof document === 'undefined' || document.getElementById(DOWNLOAD_QUALITY_DIALOG_STYLE_ID)) return

  const style = document.createElement('style')
  style.id = DOWNLOAD_QUALITY_DIALOG_STYLE_ID
  style.textContent = `
.download-quality-dialog.liquid-glass-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
  -webkit-backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
  animation: download-quality-overlay-in var(--motion-duration-quick) var(--motion-ease-standard);
}

.download-quality-dialog .overlay-drag-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 38px;
  z-index: 2;
}

.download-quality-dialog .liquid-glass-panel {
  width: min(400px, calc(100vw - 32px));
  max-width: 100%;
  flex: 0 0 auto;
  animation: download-quality-glass-in var(--motion-duration-standard) var(--motion-ease-out);
}

.download-quality-dialog .liquid-glass-panel__content {
  position: relative;
  width: 100%;
  max-height: min(calc(100vh - 64px), 680px);
  overflow: hidden;
  border-radius: 22px;
  padding: 28px;
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
}

.download-quality-dialog .glass-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
  flex-shrink: 0;
}

.download-quality-dialog .glass-title-group {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.download-quality-dialog .glass-icon {
  width: 42px;
  height: 42px;
  border-radius: 13px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  background: linear-gradient(135deg, rgba(var(--td-brand-color-rgb, 0, 82, 204), 0.18), rgba(140, 80, 255, 0.12));
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  box-shadow: 0 3px 10px color-mix(in srgb, var(--td-brand-color) 12%, transparent);
}

.download-quality-dialog .glass-icon svg {
  color: var(--td-brand-color, #0052d9);
  filter: drop-shadow(0 0 3px rgba(100, 140, 255, 0.25));
}

.download-quality-dialog .glass-title-text {
  min-width: 0;
}

.download-quality-dialog .glass-title {
  margin: 0;
  color: var(--td-text-color-primary);
  font-size: 17px;
  font-weight: 600;
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.download-quality-dialog .glass-close-btn {
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
  flex-shrink: 0;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
}

.download-quality-dialog .glass-close-btn:hover {
  background: rgba(255, 80, 80, 0.15);
  border-color: rgba(255, 80, 80, 0.25);
  color: var(--td-error-color, #d54941);
}

.download-quality-dialog .glass-close-btn .iconfont {
  font-size: 13px;
}

.download-quality-dialog .quality-selector {
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.download-quality-dialog .quality-list {
  max-height: min(52vh, 420px);
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  scrollbar-width: none;
  -ms-overflow-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-right: 2px;
}

.download-quality-dialog .quality-list::-webkit-scrollbar {
  display: none;
}

.download-quality-dialog .quality-item {
  width: 100%;
  appearance: none;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  border-radius: 13px;
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-text-color-primary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin: 0;
  padding: 12px 14px;
  text-align: left;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
}

.download-quality-dialog .quality-item:hover {
  background: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
  border-color: var(--td-brand-color, #0052d9);
  box-shadow: var(--glass-shadow-control);
  transform: translateX(3px);
}

.download-quality-dialog .quality-item:active {
  transform: translateX(0);
}

.download-quality-dialog .quality-item.active {
  background: color-mix(in srgb, var(--td-brand-color) 14%, transparent);
  border-color: color-mix(in srgb, var(--td-brand-color) 30%, transparent);
  box-shadow: inset 0 1px 0 color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
}

.download-quality-dialog .quality-item.active:hover {
  background: color-mix(in srgb, var(--td-brand-color) 18%, transparent);
}

.download-quality-dialog .quality-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.download-quality-dialog .quality-name {
  color: var(--td-text-color-primary);
  font-size: 14px;
  font-weight: 500;
  line-height: 1.3;
}

.download-quality-dialog .quality-item.active .quality-name {
  color: var(--td-brand-color, #0052d9);
}

.download-quality-dialog .quality-code {
  color: var(--td-text-color-secondary);
  font-size: 12px;
  line-height: 1.25;
}

.download-quality-dialog .quality-size {
  color: var(--td-text-color-secondary);
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 500;
  line-height: 1.25;
  white-space: nowrap;
}

@keyframes download-quality-overlay-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes download-quality-glass-in {
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
  .download-quality-dialog.liquid-glass-overlay {
    align-items: flex-end;
    justify-content: center;
    padding: calc(var(--mobile-safe-top) + 12px) var(--mobile-page-gutter) calc(var(--mobile-safe-bottom) + 12px);
    background: var(--mobile-scrim);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
  }

  .download-quality-dialog .overlay-drag-bar {
    display: none;
  }

  .download-quality-dialog .liquid-glass-panel {
    width: min(440px, 100%);
    max-height: min(82dvh, 680px);
    display: flex;
  }

  .download-quality-dialog .liquid-glass-panel .glass {
    max-height: inherit;
  }

  .download-quality-dialog .liquid-glass-panel .liquid-glass__content {
    max-height: inherit;
    overflow: hidden;
  }

  .download-quality-dialog .liquid-glass-panel__content {
    border-radius: var(--mobile-card-radius);
    padding: 20px 16px calc(var(--mobile-safe-bottom) + 16px);
    max-height: min(82dvh, 680px);
  }

  .download-quality-dialog .liquid-glass-panel__content::before {
    content: '';
    display: block;
    width: 38px;
    height: 4px;
    border-radius: 999px;
    background: rgba(120, 120, 128, 0.36);
    margin: -8px auto 12px;
    flex-shrink: 0;
  }

  .download-quality-dialog .glass-header {
    margin-bottom: 14px;
  }

  .download-quality-dialog .glass-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
  }

  .download-quality-dialog .glass-close-btn {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .download-quality-dialog .quality-selector {
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
  }

  .download-quality-dialog .quality-list {
    max-height: max(120px, calc(min(82dvh, 680px) - 118px - var(--mobile-safe-bottom)));
    min-height: 0;
    -webkit-overflow-scrolling: touch;
  }

  .download-quality-dialog .quality-item {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-card-radius-small);
    touch-action: manipulation;
  }
}

@media (prefers-reduced-motion: reduce) {
  .download-quality-dialog.liquid-glass-overlay,
  .download-quality-dialog .liquid-glass-panel,
  .download-quality-dialog .quality-item {
    animation: none !important;
    transition: none !important;
    transform: none !important;
  }
}
`
  document.head.appendChild(style)
}

const DownloadQualityDialog = defineComponent({
  name: 'DownloadQualityDialog',
  props: {
    title: { type: String, required: true },
    userQuality: { type: String, required: true },
    qualityOptions: { type: Array as PropType<DownloadQualityOption[]>, required: true }
  },
  emits: {
    select: (qualityType: string) => typeof qualityType === 'string' && qualityType.length > 0,
    close: () => true
  },
  setup(props, { emit }) {
    const isMobile = ref(false)
    let mobileMql: MediaQueryList | null = null

    const onMobileChange = (event: MediaQueryListEvent | MediaQueryList) => {
      isMobile.value = event.matches
    }

    const onKeydown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.preventDefault()
      emit('close')
    }

    onMounted(() => {
      if (typeof window !== 'undefined') {
        mobileMql = window.matchMedia('(max-width: 768px)')
        onMobileChange(mobileMql)
        mobileMql.addEventListener('change', onMobileChange)
      }
      if (typeof document !== 'undefined') {
        document.addEventListener('keydown', onKeydown)
      }
    })

    onUnmounted(() => {
      if (mobileMql) {
        mobileMql.removeEventListener('change', onMobileChange)
      }
      if (typeof document !== 'undefined') {
        document.removeEventListener('keydown', onKeydown)
      }
    })

    const cornerRadius = computed(() => {
      if (!isMobile.value || typeof document === 'undefined') return 22
      const cssVal = getComputedStyle(document.documentElement).getPropertyValue('--mobile-card-radius')?.trim()
      if (cssVal) {
        const num = parseFloat(cssVal)
        if (Number.isFinite(num)) return num
      }
      return 18
    })

    const closeLabel = computed(() => i18n.global.t('common.close'))

    const handleOverlayClick = (event: MouseEvent) => {
      if (event.target === event.currentTarget) emit('close')
    }

    const renderQualityItem = (quality: DownloadQualityOption) => {
      const active = quality.type === props.userQuality
      return h(
        'button',
        {
          key: quality.type,
          type: 'button',
          class: ['quality-item', { active }],
          role: 'option',
          'aria-selected': active ? 'true' : 'false',
          onClick: () => emit('select', quality.type)
        },
        [
          h('div', { class: 'quality-info' }, [
            h('div', { class: 'quality-name' }, getQualityDisplayName(quality.type)),
            h('div', { class: 'quality-code' }, quality.type.toUpperCase())
          ]),
          h('div', { class: 'quality-size' }, quality.size || '')
        ]
      )
    }

    return () =>
      h(
        'div',
        {
          class: 'download-quality-dialog liquid-glass-overlay',
          onClick: handleOverlayClick
        },
        [
          h('div', { class: 'overlay-drag-bar', 'data-tauri-drag-region': '' }),
          h(
            LiquidGlass,
            {
              class: 'liquid-glass-panel',
              cornerRadius: cornerRadius.value,
              displacementScale: 48,
              blurAmount: 0.08,
              saturation: 180,
              aberrationIntensity: 1.5,
              padding: '0',
              mode: 'standard',
              contentStyle: downloadQualityLiquidGlassContentStyle,
              role: 'dialog',
              'aria-modal': 'true',
              'aria-label': props.title,
              onClick: (event: MouseEvent) => event.stopPropagation()
            },
            {
              default: () =>
                h('div', { class: 'liquid-glass-panel__content' }, [
                  h('div', { class: 'glass-header', 'data-tauri-drag-region': '' }, [
                    h('div', { class: 'glass-title-group' }, [
                      h('div', { class: 'glass-icon', 'aria-hidden': 'true' }, [
                        h(
                          'svg',
                          {
                            width: '22',
                            height: '22',
                            viewBox: '0 0 24 24',
                            fill: 'none',
                            stroke: 'currentColor',
                            'stroke-width': '1.8',
                            'stroke-linecap': 'round',
                            'stroke-linejoin': 'round'
                          },
                          [
                            h('path', { d: 'M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4' }),
                            h('polyline', { points: '7 10 12 15 17 10' }),
                            h('line', { x1: '12', y1: '15', x2: '12', y2: '3' })
                          ]
                        )
                      ]),
                      h('div', { class: 'glass-title-text' }, [
                        h('h2', { class: 'glass-title' }, props.title)
                      ])
                    ]),
                    h(
                      'button',
                      {
                        type: 'button',
                        class: 'glass-close-btn',
                        'aria-label': closeLabel.value,
                        onClick: () => emit('close')
                      },
                      [h('i', { class: 'iconfont icon-a-quxiaoguanbi', 'aria-hidden': 'true' })]
                    )
                  ]),
                  h('div', { class: 'quality-selector' }, [
                    h(
                      'div',
                      {
                        class: 'quality-list',
                        role: 'listbox',
                        'aria-label': props.title
                      },
                      props.qualityOptions.map(renderQualityItem)
                    )
                  ])
                ])
            }
          )
        ]
      )
  }
})

export function createQualityDialog(
  songInfoOrTypes: MusicItem | Array<{ type: string; size?: string }>,
  userQuality: string,
  title: string = i18n.global.t('download.selectQuality'),
  pluginQualities?: string[]
): Promise<string | null> {
  return new Promise((resolve) => {
    let types: Array<{ type: string; size?: string }> = []
    if (Array.isArray(songInfoOrTypes)) {
      types = songInfoOrTypes
    } else {
      types = songInfoOrTypes.types || []
    }

    const availableQualities = filterByPluginQualities(buildQualityFormats(types), pluginQualities)
    const qualityOptions = [...availableQualities]

    qualityOptions.sort((a, b) => compareQuality(a.type, b.type))

    if (typeof document === 'undefined') {
      resolve(null)
      return
    }

    ensureDownloadQualityDialogStyle()

    const mountTarget = document.createElement('div')
    mountTarget.className = 'download-quality-dialog-host'
    document.body.appendChild(mountTarget)

    let settled = false
    let app: ReturnType<typeof createApp> | null = null

    const cleanup = () => {
      app?.unmount()
      app = null
      mountTarget.remove()
    }

    const settle = (value: string | null) => {
      if (settled) return
      settled = true
      cleanup()
      resolve(value)
    }

    try {
      app = createApp(DownloadQualityDialog, {
        title,
        userQuality,
        qualityOptions,
        onSelect: (qualityType: string) => settle(qualityType),
        onClose: () => settle(null)
      })
      app.mount(mountTarget)
    } catch (error) {
      console.error('创建下载音质选择弹窗失败:', error)
      settle(null)
    }
  })
}

async function downloadSingleSong(songInfo: MusicItem): Promise<void> {
  try {
    const localUserDetail = LocalUserDetailStore()
    const userQuality =
      (localUserDetail.userInfo.sourceQualityMap || {})[toRaw(songInfo).source as any] ||
      (localUserDetail.userSource.quality as string)

    const pluginQualities = (localUserDetail.userInfo.supportedSources || {})[toRaw(songInfo).source as string]?.qualitys
    const selectedQuality = await createQualityDialog(songInfo, userQuality, i18n.global.t('download.selectQuality'), pluginQualities)
    if (!selectedQuality) return

    let quality = selectedQuality as string
    const calculatedQuality = calculateBestQuality(songInfo.types, quality)
    if (calculatedQuality && calculatedQuality !== quality) {
      quality = calculatedQuality
      MessagePlugin.warning(i18n.global.t('download.qualityUnavailable', { name: getQualityDisplayName(quality) }))
    }

    const tip = MessagePlugin.success(i18n.global.t('download.gettingUrl') + songInfo.name)

    let rawUrl = ''
    if (songInfo.source === 'subsonic') {
      rawUrl = await musicSdk.getMusicUrl(toRaw(songInfo) as any, quality)
    } else {
      const pluginId = localUserDetail.userSource.pluginId
      if (!pluginId) {
        MessagePlugin.error(i18n.global.t('play.noSourcePlugin'))
        ;(await tip).close()
        return
      }
      rawUrl = await PluginRunner.getMusicUrl(
        pluginId,
        songInfo.source || 'kw',
        toRaw(songInfo) as any,
        quality
      )
    }

    ;(await tip).close()

    if (!rawUrl || typeof rawUrl !== 'string') {
      MessagePlugin.error(i18n.global.t('download.getUrlFailed'))
      return
    }

    const settingsStore = useSettingsStore()
    const template = settingsStore.settings.filenameTemplate || '%t - %s'
    const safeName = `${songInfo.name} - ${songInfo.singer}`.replace(/[\/\\:*?"<>|]/g, '_')

    const dirs = await invoke<{ cacheDir: string; downloadDir: string }>('get_directories')
    const downloadDir = dirs?.downloadDir || ''
    const urlExtension = rawUrl.match(/\.(mp3|flac|m4a|aac|ogg|opus|wav)(?:[?#/]|$)/i)?.[1]?.toLowerCase()
    const extension = urlExtension || (['flac', 'flac24bit', 'hires', 'master', 'atmos', 'atmos_plus'].includes(quality) ? 'flac' : 'mp3')
    const filePath = downloadDir ? `${downloadDir}/${safeName}.${extension}` : `${safeName}.${extension}`

    await (window as any).api?.download?.addTask(
      {
        name: songInfo.name,
        singer: songInfo.singer,
        songmid: songInfo.songmid,
        source: songInfo.source,
        img: songInfo.img,
        albumName: songInfo.albumName
      },
      rawUrl,
      filePath,
      null,
      quality,
      1
    )

    // 监听下载完成后写入标签
    const tagOptions = (settingsStore.settings.tagWriteOptions ?? {}) as Record<string, any>
    const needsTags = tagOptions.basicInfo || tagOptions.cover || tagOptions.lyrics || tagOptions.downloadLyrics
    if (needsTags) {
      const unlisten = await listen('download:task-completed', async (event: any) => {
        const task = event.payload
        if (task.file_path !== filePath && task.filePath !== filePath) return
        unlisten()

        try {
          const songInfoForTags = { ...toRaw(songInfo) }

          if (tagOptions.lyrics || tagOptions.downloadLyrics) {
            songInfoForTags.lrc = songInfo.source === 'subsonic'
              ? await musicSdk.getLyric(toRaw(songInfo) as any)
              : await resolveDownloadLyricText(localUserDetail.userSource.pluginId, toRaw(songInfo) as MusicItem)
          }

          await invoke('local_music__write_tags', {
            filePath,
            songInfo: songInfoForTags,
            tagWriteOptions: tagOptions
          })
        } catch (e) {
          console.warn('标签写入失败:', e)
        }
      })
    }

    await NotifyPlugin.success({
      title: i18n.global.t('download.addedToQueue'),
      content: `${songInfo.name} ${i18n.global.t('download.queueViewTip')}`
    })
  } catch (error: any) {
    console.error('下载失败:', error)
    const msg = error?.message || ''
    if (msg.includes('歌曲正在下载中')) {
      await NotifyPlugin.warning({ title: i18n.global.t('common.tip'), content: i18n.global.t('download.duplicateDownload') })
    } else if (msg.includes('歌曲已下载完成')) {
      await NotifyPlugin.info({ title: i18n.global.t('common.tip'), content: i18n.global.t('download.alreadyDownloaded') })
    } else {
      await NotifyPlugin.error({ title: i18n.global.t('download.downloadFailed'), content: msg || i18n.global.t('download.unknownError') })
    }
  }
}

export { downloadSingleSong }
