import { computed, createApp, defineComponent, h, onMounted, onUnmounted, ref, type CSSProperties, type PropType } from 'vue'
import LiquidGlass from '@/components/LiquidGlass.vue'
import i18n from '@/locales'

interface LiquidGlassConfirmOptions {
  title: string
  body: string
  confirmText?: string
  cancelText?: string
  confirmTheme?: 'primary' | 'danger'
  icon?: 'play' | 'shuffle' | 'info'
}

const LIQUID_GLASS_CONFIRM_STYLE_ID = 'liquid-glass-confirm-dialog-style'

const liquidGlassConfirmContentStyle: CSSProperties = {
  color: 'var(--td-text-color-primary)',
  font: 'inherit',
  lineHeight: 'normal',
  textShadow: 'none'
}

function ensureLiquidGlassConfirmStyle() {
  if (typeof document === 'undefined' || document.getElementById(LIQUID_GLASS_CONFIRM_STYLE_ID)) return

  const style = document.createElement('style')
  style.id = LIQUID_GLASS_CONFIRM_STYLE_ID
  style.textContent = `
.liquid-glass-confirm-dialog.liquid-glass-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
  -webkit-backdrop-filter: blur(var(--glass-blur-overlay)) saturate(140%);
  animation: liquid-glass-confirm-overlay-in var(--motion-duration-quick) var(--motion-ease-standard);
}

.liquid-glass-confirm-dialog .overlay-drag-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 38px;
  z-index: 2;
}

.liquid-glass-confirm-dialog .liquid-glass-panel {
  width: min(420px, calc(100vw - 32px));
  max-width: 100%;
  flex: 0 0 auto;
  animation: liquid-glass-confirm-panel-in var(--motion-duration-standard) var(--motion-ease-out);
}

.liquid-glass-confirm-dialog .liquid-glass-panel__content {
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

.liquid-glass-confirm-dialog .glass-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
  flex-shrink: 0;
}

.liquid-glass-confirm-dialog .glass-title-group {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.liquid-glass-confirm-dialog .glass-icon {
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

.liquid-glass-confirm-dialog .glass-icon svg {
  color: var(--td-brand-color, #0052d9);
  filter: drop-shadow(0 0 3px rgba(100, 140, 255, 0.25));
}

.liquid-glass-confirm-dialog .glass-title-text {
  min-width: 0;
}

.liquid-glass-confirm-dialog .glass-title {
  margin: 0;
  color: var(--td-text-color-primary);
  font-size: 17px;
  font-weight: 600;
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.liquid-glass-confirm-dialog .glass-close-btn {
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

.liquid-glass-confirm-dialog .glass-close-btn:hover {
  background: rgba(255, 80, 80, 0.15);
  border-color: rgba(255, 80, 80, 0.25);
  color: var(--td-error-color, #d54941);
}

.liquid-glass-confirm-dialog .glass-close-btn .iconfont {
  font-size: 13px;
}

.liquid-glass-confirm-dialog .confirm-content {
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.liquid-glass-confirm-dialog .confirm-content::-webkit-scrollbar {
  display: none;
}

.liquid-glass-confirm-dialog .confirm-message {
  margin: 0;
  color: var(--td-text-color-secondary);
  font-size: 14px;
  line-height: 1.7;
  word-break: break-word;
}

.liquid-glass-confirm-dialog .confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 24px;
  flex-shrink: 0;
}

.liquid-glass-confirm-dialog .glass-btn {
  appearance: none;
  min-width: 92px;
  min-height: 38px;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-text-color-primary);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  margin: 0;
  padding: 9px 18px;
  font-size: 14px;
  font-weight: 600;
  line-height: 1.25;
  text-align: center;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard), border-color var(--motion-duration-quick) var(--motion-ease-standard), color var(--motion-duration-quick) var(--motion-ease-standard), box-shadow var(--motion-duration-quick) var(--motion-ease-standard), opacity var(--motion-duration-quick) var(--motion-ease-standard), transform var(--motion-duration-quick) var(--motion-ease-standard);
}

.liquid-glass-confirm-dialog .glass-btn:hover {
  background: color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
  border-color: color-mix(in srgb, var(--td-text-color-primary) 16%, transparent);
  box-shadow: var(--glass-shadow-control);
  transform: translateY(-1px);
}

.liquid-glass-confirm-dialog .glass-btn:active {
  transform: translateY(0);
}

.liquid-glass-confirm-dialog .glass-btn.primary {
  background: color-mix(in srgb, var(--td-brand-color) 16%, transparent);
  border-color: color-mix(in srgb, var(--td-brand-color) 32%, transparent);
  color: var(--td-brand-color, #0052d9);
}

.liquid-glass-confirm-dialog .glass-btn.primary:hover {
  background: color-mix(in srgb, var(--td-brand-color) 22%, transparent);
  border-color: color-mix(in srgb, var(--td-brand-color) 45%, transparent);
}

.liquid-glass-confirm-dialog .glass-btn.danger {
  background: color-mix(in srgb, var(--td-error-color, #d54941) 14%, transparent);
  border-color: color-mix(in srgb, var(--td-error-color, #d54941) 30%, transparent);
  color: var(--td-error-color, #d54941);
}

.liquid-glass-confirm-dialog .glass-btn.danger:hover {
  background: color-mix(in srgb, var(--td-error-color, #d54941) 20%, transparent);
  border-color: color-mix(in srgb, var(--td-error-color, #d54941) 45%, transparent);
}

@keyframes liquid-glass-confirm-overlay-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes liquid-glass-confirm-panel-in {
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
  .liquid-glass-confirm-dialog.liquid-glass-overlay {
    align-items: flex-end;
    justify-content: center;
    padding: calc(var(--mobile-safe-top) + 12px) var(--mobile-page-gutter) calc(var(--mobile-safe-bottom) + 12px);
    background: var(--mobile-scrim);
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
  }

  .liquid-glass-confirm-dialog .overlay-drag-bar {
    display: none;
  }

  .liquid-glass-confirm-dialog .liquid-glass-panel {
    width: min(440px, 100%);
    max-height: min(82dvh, 680px);
    display: flex;
  }

  .liquid-glass-confirm-dialog .liquid-glass-panel .glass {
    max-height: inherit;
  }

  .liquid-glass-confirm-dialog .liquid-glass-panel .liquid-glass__content {
    max-height: inherit;
    overflow: hidden;
  }

  .liquid-glass-confirm-dialog .liquid-glass-panel__content {
    border-radius: var(--mobile-card-radius);
    padding: 20px 16px calc(var(--mobile-safe-bottom) + 16px);
    max-height: min(82dvh, 680px);
  }

  .liquid-glass-confirm-dialog .liquid-glass-panel__content::before {
    content: '';
    display: block;
    width: 38px;
    height: 4px;
    border-radius: 999px;
    background: rgba(120, 120, 128, 0.36);
    margin: -8px auto 12px;
    flex-shrink: 0;
  }

  .liquid-glass-confirm-dialog .glass-header {
    margin-bottom: 14px;
  }

  .liquid-glass-confirm-dialog .glass-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
  }

  .liquid-glass-confirm-dialog .glass-close-btn,
  .liquid-glass-confirm-dialog .glass-btn {
    min-width: var(--mobile-touch-target);
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .liquid-glass-confirm-dialog .confirm-content {
    flex: 1 1 auto;
    min-height: 0;
    max-height: max(120px, calc(min(82dvh, 680px) - 170px - var(--mobile-safe-bottom)));
    -webkit-overflow-scrolling: touch;
  }

  .liquid-glass-confirm-dialog .confirm-actions {
    flex-direction: column-reverse;
    gap: 8px;
    margin-top: 20px;
  }

  .liquid-glass-confirm-dialog .glass-btn {
    width: 100%;
  }
}

@media (prefers-reduced-motion: reduce) {
  .liquid-glass-confirm-dialog.liquid-glass-overlay,
  .liquid-glass-confirm-dialog .liquid-glass-panel,
  .liquid-glass-confirm-dialog .glass-btn {
    animation: none !important;
    transition: none !important;
    transform: none !important;
  }
}
`
  document.head.appendChild(style)
}

const iconPaths: Record<NonNullable<LiquidGlassConfirmOptions['icon']>, string[]> = {
  play: ['M8 5v14l11-7z'],
  shuffle: [
    'M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41z',
    'M14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.46 20 9.5V4h-5.5z',
    'M14.83 13.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z'
  ],
  info: ['M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z', 'M12 16v-4', 'M12 8h.01']
}

const LiquidGlassConfirmDialog = defineComponent({
  name: 'LiquidGlassConfirmDialog',
  props: {
    title: { type: String, required: true },
    body: { type: String, required: true },
    confirmText: { type: String, required: true },
    cancelText: { type: String, required: true },
    confirmTheme: { type: String as PropType<'primary' | 'danger'>, default: 'primary' },
    icon: { type: String as PropType<NonNullable<LiquidGlassConfirmOptions['icon']>>, default: 'info' }
  },
  emits: {
    confirm: () => true,
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

    const renderIcon = () => {
      const paths = iconPaths[props.icon] || iconPaths.info
      const fill = props.icon === 'play' || props.icon === 'shuffle' ? 'currentColor' : 'none'
      const stroke = props.icon === 'info' ? 'currentColor' : 'none'

      return h(
        'svg',
        {
          width: '22',
          height: '22',
          viewBox: '0 0 24 24',
          fill,
          stroke,
          'stroke-width': props.icon === 'info' ? '1.8' : undefined,
          'stroke-linecap': props.icon === 'info' ? 'round' : undefined,
          'stroke-linejoin': props.icon === 'info' ? 'round' : undefined
        },
        paths.map((d) => h('path', { d }))
      )
    }

    return () =>
      h(
        'div',
        {
          class: 'liquid-glass-confirm-dialog liquid-glass-overlay',
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
              contentStyle: liquidGlassConfirmContentStyle,
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
                      h('div', { class: 'glass-icon', 'aria-hidden': 'true' }, [renderIcon()]),
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
                  h('div', { class: 'confirm-content' }, [
                    h('p', { class: 'confirm-message' }, props.body)
                  ]),
                  h('div', { class: 'confirm-actions' }, [
                    h(
                      'button',
                      {
                        type: 'button',
                        class: 'glass-btn outline',
                        onClick: () => emit('close')
                      },
                      props.cancelText
                    ),
                    h(
                      'button',
                      {
                        type: 'button',
                        class: ['glass-btn', props.confirmTheme],
                        onClick: () => emit('confirm')
                      },
                      props.confirmText
                    )
                  ])
                ])
            }
          )
        ]
      )
  }
})

export function createLiquidGlassConfirm(options: LiquidGlassConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    if (typeof document === 'undefined') {
      resolve(false)
      return
    }

    ensureLiquidGlassConfirmStyle()

    const mountTarget = document.createElement('div')
    mountTarget.className = 'liquid-glass-confirm-dialog-host'
    document.body.appendChild(mountTarget)

    let settled = false
    let app: ReturnType<typeof createApp> | null = null

    const cleanup = () => {
      app?.unmount()
      app = null
      mountTarget.remove()
    }

    const settle = (value: boolean) => {
      if (settled) return
      settled = true
      cleanup()
      resolve(value)
    }

    try {
      app = createApp(LiquidGlassConfirmDialog, {
        title: options.title,
        body: options.body,
        confirmText: options.confirmText || i18n.global.t('common.confirm'),
        cancelText: options.cancelText || i18n.global.t('common.cancel'),
        confirmTheme: options.confirmTheme || 'primary',
        icon: options.icon || 'info',
        onConfirm: () => settle(true),
        onClose: () => settle(false)
      })
      app.mount(mountTarget)
    } catch (error) {
      console.error('创建 LiquidGlass 确认弹窗失败:', error)
      settle(false)
    }
  })
}
