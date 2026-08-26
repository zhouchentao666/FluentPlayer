<template>
  <div class="home">
    <HomeLayout>
      <template #body>
        <router-view v-slot="{ Component }">
          <Transition :name="routeDirection === 'forward' ? 'route-slide-forward' : 'route-slide-backward'" mode="out-in">
            <KeepAlive exclude="list">
              <component :is="Component" :key="$route.name" />
            </KeepAlive>
          </Transition>
        </router-view>
      </template>
    </HomeLayout>
    <PlayMusic />
    <PluginUpdateNoticeDialog />
  </div>
</template>

<script setup lang="ts">
import HomeLayout from '@/components/layout/HomeLayout.vue'
import PlayMusic from '@/components/Play/PlayMusic.vue'
import PluginUpdateNoticeDialog from '@/components/Plugin/PluginUpdateNoticeDialog.vue'
import { onMounted } from 'vue'
import { usePluginStore } from '@/store/plugin'
import PluginRunner from '@/utils/plugin/PluginRunner'
import { routeDirection } from '@/router/index'

defineOptions({ name: 'HomeView' })

// Session flag: only trigger update check once per app session
let updateCheckDone = false

onMounted(async () => {
  if (updateCheckDone) return
  updateCheckDone = true

  try {
    const pluginStore = usePluginStore()
    if (pluginStore.plugins.length === 0) {
      await pluginStore.initialize()
    }

    for (const plugin of pluginStore.plugins) {
      try {
        await PluginRunner.reloadPlugin(plugin.plugin_id)
      } catch {
        // Reload failure is non-critical
      }
    }
  } catch (e) {
    console.warn('[HomeView] 插件更新检查失败:', e)
  }
})
</script>

<style scoped>
.home {
  height: 100%;
  overflow: hidden;
}

.route-slide-forward-enter-active,
.route-slide-forward-leave-active,
.route-slide-backward-enter-active,
.route-slide-backward-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard),
              transform var(--motion-duration-quick) var(--motion-ease-standard);
}

.route-slide-forward-enter-from {
  opacity: 0;
  transform: translateX(20px);
}
.route-slide-forward-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

.route-slide-backward-enter-from {
  opacity: 0;
  transform: translateX(-20px);
}
.route-slide-backward-leave-to {
  opacity: 0;
  transform: translateX(20px);
}
</style>
