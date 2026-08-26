<template>
  <Provider v-if="!$route.path.includes('desktop-lyric')">
    <div class="app-route-stage">
      <router-view v-slot="{ Component }">
        <component :is="Component" class="vt-page" />
      </router-view>
    </div>
  </Provider>
  <router-view v-else />
  <GlobalContextMenu v-if="!$route.path.includes('desktop-lyric')" />
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import Provider from '@/components/layout/Provider.vue'
import GlobalContextMenu from '@/components/ContextMenu/GlobalContextMenu.vue'
import { useAuthStore } from '@/store/Auth'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { useSettingsStore } from '@/store/Settings'
import { useAppUpdater } from '@/composables/useAppUpdater'
import { setupMediaButtonListener } from '@/utils/audio/globaPlayList'

const router = useRouter()
const authStore = useAuthStore()
const settingsStore = useSettingsStore()
const { checkForUpdate } = useAppUpdater()

async function checkForUpdateOnStartup() {
  if (!settingsStore.settings.autoUpdate) return
  await checkForUpdate()
}

onMounted(async () => {
  if (window.location.hash.includes('/desktop-lyric')) return

  setupMediaButtonListener()
  LocalUserDetailStore().init()
  void checkForUpdateOnStartup()
  const url = new URL(window.location.href)
  const code = url.searchParams.get('code')
  const state = url.searchParams.get('state')

  if (code && state) {
    await authStore.handleCallback(window.location.href)
    window.history.replaceState({}, '', window.location.pathname)
    router.push('/home')
  } else {
    await authStore.init()
  }
})
</script>

<style scoped>
.app-route-stage {
  position: relative;
  height: 100%;
  overflow: hidden;
  background: var(--td-bg-color-page);
}
</style>
