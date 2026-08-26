<template>
  <t-config-provider :global-config="tdGlobalConfig">
    <n-config-provider :theme-overrides="themeOverrides" :locale="naiveLocale" :date-locale="naiveDateLocale">
      <n-message-provider><n-dialog-provider><nglobal-style />
        <GlobalAudio />
        <AIChat />
        <slot />
      </n-dialog-provider></n-message-provider>
    </n-config-provider>
  </t-config-provider>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { NConfigProvider, NMessageProvider, NDialogProvider, NGlobalStyle as NglobalStyle, dateEnUS, dateZhCN, enUS, zhCN } from 'naive-ui'
import type { GlobalConfigProvider } from 'tdesign-vue-next/es/config-provider/type'
import tdesignZhCN from 'tdesign-vue-next/es/locale/zh_CN'
import tdesignEnUS from 'tdesign-vue-next/es/locale/en_US'
import { useDynamicSongTheme } from '@/composables/useDynamicSongTheme'
import GlobalAudio from '@/components/Play/GlobalAudio.vue'
import { useSettingsStore } from '@/store/Settings'
import { setI18nLocale, type ResolvedLocale } from '@/locales/runtime'

const settingsStore = useSettingsStore()
const { settings } = storeToRefs(settingsStore)
const activeLocale = ref<ResolvedLocale>('zh-CN')
const themeVersion = ref(0)

const syncLocale = async () => {
  activeLocale.value = await settingsStore.resolveLocale() as ResolvedLocale
  setI18nLocale(activeLocale.value)
}

watch(() => settings.value.language, syncLocale, { immediate: true })

useDynamicSongTheme({
  onThemeChange: () => {
    themeVersion.value++
  }
})

const naiveLocale = computed(() => activeLocale.value === 'zh-CN' ? zhCN : enUS)
const naiveDateLocale = computed(() => activeLocale.value === 'zh-CN' ? dateZhCN : dateEnUS)
const tdGlobalConfig = computed<GlobalConfigProvider>(() => activeLocale.value === 'zh-CN' ? { ...tdesignZhCN } as unknown as GlobalConfigProvider : { ...tdesignEnUS } as unknown as GlobalConfigProvider)

const themeOverrides = computed(() => {
  void themeVersion.value
  const docEl = document.documentElement
  const computedStyle = getComputedStyle(docEl)
  const primary = (computedStyle.getPropertyValue('--td-brand-color') || '').trim()
  const mainColor = primary || '#2ba55b'
  return {
    common: {
      primaryColor: mainColor,
      primaryColorHover: mainColor,
      primaryColorPressed: mainColor
    }
  }
})
</script>
