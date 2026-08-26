<template>
  <div class="user-capsule-container">
    <n-dropdown
      v-if="authStore.isAuthenticated && authStore.user"
      :options="userOpt"
      placement="bottom-end"
      trigger="hover"
      @select="handleMenuSelect"
    >
      <div class="user-capsule logged-in">
        <t-avatar
          v-if="authStore.user.picture"
          :image="authStore.user.picture"
          size="small"
          style="margin-right: 4px"
        />
        <t-avatar
          v-else
          size="small"
          style="margin-right: 4px; background: rgba(125,125,125,0.2); color: inherit"
        >{{ userName.charAt(0) }}</t-avatar>
        <span class="user-name">{{ userName }}</span>
      </div>
    </n-dropdown>

    <div v-else class="user-capsule" @click="handleLogin">
      <t-avatar size="small" style="margin-right: 4px; background: rgba(125,125,125,0.2); color: inherit">
        <template #icon><i class="iconfont icon-weidenglu" /></template>
      </t-avatar>
      <span class="user-name">{{ t('common.notLoggedIn') }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, h, type Component } from 'vue'
import { useRouter } from 'vue-router'
import { NIcon } from 'naive-ui'
import { useAuthStore } from '@/store/Auth'
import displayName from '@/utils/auth/displayName'

const authStore = useAuthStore()
const router = useRouter()
const { t } = useI18n()

const userName = computed(() => displayName(authStore.user))

const renderIcon = (icon: Component) => () => h(NIcon, null, { default: () => h(icon) })

const userOpt = computed(() => [
  { label: t('common.userMenu.myInfo'), key: 'myInfo', icon: renderIcon(h('i', { class: 'iconfont icon-gerenzhongxin' })) },
  { type: 'divider', key: 'd1' },
  { label: t('common.userMenu.logout'), key: 'logout', icon: renderIcon(h('i', { class: 'iconfont icon-tuichu' })) }
])

const handleLogin = () => authStore.login()

const handleMenuSelect = (key: string | number) => {
  if (key === 'logout') authStore.logout()
  else if (key === 'myInfo') router.push('/home/profile')
}
</script>

<style scoped>
.user-capsule {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.15rem 0.6rem 0.15rem 0.4rem;
  border-radius: 999px;
  cursor: pointer;
  transition: background-color 0.2s;
  height: 2rem;
  box-sizing: border-box;
}

.user-capsule:hover {
  background: rgba(125, 125, 125, 0.2);
}

.user-capsule.logged-in {
  background: rgba(125, 125, 125, 0.1);
  border: 1px solid rgba(125, 125, 125, 0.2);
}

.user-name {
  font-size: 0.8rem;
  max-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
  color: var(--td-text-color-primary);
  line-height: 1;
}
</style>
