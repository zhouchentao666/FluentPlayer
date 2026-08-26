<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { useSettingsStore } from '@/store/Settings'

interface Props {
  controlStyle?: 'traffic-light' | 'windows' | boolean
  showSettings?: boolean
  showBack?: boolean
  showAccount?: boolean
  title?: string
  color?: string
}

const props = withDefaults(defineProps<Props>(), {
  controlStyle: false,
  showSettings: true,
  showBack: false,
  showAccount: false,
  title: '',
  color: ''
})

const { t } = useI18n()

const router = useRouter()
const userStore = LocalUserDetailStore()
const { userInfo } = storeToRefs(userStore)
const settingsStore = useSettingsStore()

const controlsClass = computed(() => {
  if (props.controlStyle !== false) {
    return `title-controls ${props.controlStyle}`
  } else {
    return userInfo.value.topBarStyle ? 'title-controls traffic-light' : 'title-controls windows'
  }
})

const handleMinimize = async () => {
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow().minimize()
}

const handleMaximize = async () => {
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow().toggleMaximize()
}

const handleClose = async () => {
  if (!settingsStore.settings.hasConfiguredCloseBehavior) {
    showCloseDialog.value = true
    return
  }

  if (settingsStore.settings.closeToTray) {
    handleMiniMode()
  } else {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow().close()
  }
}

const showCloseDialog = ref(false)
const rememberChoice = ref(true)

const handleCloseChoice = async (toTray: boolean) => {
  if (rememberChoice.value) {
    settingsStore.updateSettings({
      closeToTray: toTray,
      hasConfiguredCloseBehavior: true
    })
  }
  showCloseDialog.value = false
  if (toTray) {
    handleMiniMode()
  } else {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow().close()
  }
}

const handleMiniMode = async () => {
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow().hide()
}

const handleSettings = () => {
  router.push('/settings')
}

const handleBack = () => {
  router.back()
}
</script>

<template>
  <div :class="controlsClass" data-tauri-drag-region>
    <div class="left" data-tauri-drag-region>
      <div class="back-box">
        <t-button
          v-if="showBack"
          shape="circle"
          theme="default"
          variant="text"
          class="control-btn back-btn"
          :title="t('common.titleBar.back')"
          @click="handleBack"
        >
          <i class="iconfont icon-xiangzuo"></i>
        </t-button>
      </div>
      <div class="title-box">
        <p>{{ title }}</p>
      </div>
    </div>

    <div class="window-controls">
      <slot name="before-settings" />

      <t-button
        v-if="showSettings"
        shape="circle"
        theme="default"
        variant="text"
        class="control-btn settings-btn"
        :title="t('common.titleBar.settings')"
        @click="handleSettings"
      >
        <i class="iconfont icon-shezhi"></i>
      </t-button>

      <div
        class="separator"
        :style="color ? 'background: ' + color : ''"
      ></div>

      <t-button
        shape="circle"
        theme="default"
        variant="text"
        class="control-btn minimize-btn"
        :title="t('common.titleBar.minimize')"
        @click="handleMinimize"
      >
        <i
          v-if="controlsClass.includes('windows')"
          class="iconfont icon-zuixiaohua"
        ></i>
        <div v-else class="traffic-light minimize"></div>
      </t-button>

      <t-button
        shape="circle"
        theme="default"
        variant="text"
        class="control-btn maximize-btn"
        :title="t('common.titleBar.maximize')"
        @click="handleMaximize"
      >
        <i
          v-if="controlsClass.includes('windows')"
          class="iconfont icon-a-tingzhiwukuang"
        ></i>
        <div v-else class="traffic-light maximize"></div>
      </t-button>

      <t-button
        shape="circle"
        theme="default"
        variant="text"
        class="control-btn close-btn"
        :title="t('common.titleBar.close')"
        @click="handleClose"
      >
        <i
          v-if="controlsClass.includes('windows')"
          class="iconfont icon-a-quxiaoguanbi"
        ></i>
        <div v-else class="traffic-light close"></div>
      </t-button>
    </div>

    <t-dialog v-model:visible="showCloseDialog" :header="t('common.titleBar.closeTip')" :close-btn="true" placement="top">
      <div>{{ t('common.titleBar.closeQuestion') }}</div>
      <div style="margin-top: 10px">
        <t-checkbox v-model="rememberChoice">{{ t('common.titleBar.rememberChoice') }}</t-checkbox>
      </div>
      <template #footer>
        <t-button theme="default" @click="handleCloseChoice(false)">
          {{ t('common.titleBar.exitDirectly') }}
        </t-button>
        <t-button theme="primary" @click="handleCloseChoice(true)">
          {{ t('common.titleBar.minimizeToTray') }}
        </t-button>
      </template>
    </t-dialog>
  </div>
</template>

<style lang="scss" scoped>
.title-controls {
  display: flex;
  align-items: center;
  width: 100%;
  gap: 0.25rem;

  .control-btn {
    width: 2.25rem;
    height: 2.25rem;
    min-width: 2.25rem;
    padding: 0;
    border: none;
    background: transparent;

    .iconfont {
      font-size: 1.125rem;
      color: var(--td-text-color-secondary);
    }

    &:hover .iconfont {
      color: var(--td-text-color-primary);
    }
  }

  .left {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex: 1;
    min-height: 20px;

    .back-box {
      display: flex;
      align-items: center;
      gap: 0.25rem;

      .back-btn {
        margin-right: 0.5rem;

        &:hover {
          background-color: var(--td-bg-color-component-hover);
        }
      }
    }

    .title-box {
      flex: 1;

      p {
        margin: 0;
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--td-text-color-primary);
        line-height: 1.2;
      }
    }
  }

  .settings-btn {
    &:hover {
      background-color: var(--td-bg-color-component-hover);
    }
  }

  .window-controls {
    display: flex;
    align-items: center;
    gap: 0.125rem;
    flex-shrink: 0;
  }

  .separator {
    width: 1px;
    height: 1rem;
    background: var(--td-border-level-1-color);
    margin: 0 3px;
    border-radius: 2px;
  }

  .minimize-btn:hover,
  .maximize-btn:hover {
    background-color: var(--td-bg-color-component-hover);
  }

  .close-btn:hover {
    background-color: #e81123;

    .iconfont {
      color: #fff !important;
    }
  }
}

@media (max-width: 768px) {
  .window-controls .separator,
  .window-controls .minimize-btn,
  .window-controls .maximize-btn,
  .window-controls .close-btn {
    display: none;
  }
}

// Windows style
.title-controls.windows {
  .control-btn {
    border-radius: 0.25rem;
  }
}

// Traffic light style (macOS)
.title-controls.traffic-light {
  .control-btn {
    border-radius: 50%;
    width: 2.25rem;
    height: 2.25rem;
    min-width: 2.25rem;
  }

  .traffic-light {
    width: 1rem;
    height: 1rem;
    border-radius: 50%;

    &.close {
      background-color: #ff5f57;

      &:hover {
        background-color: #ff3b30;
      }
    }

    &.minimize {
      background-color: #ffbd2e;

      &:hover {
        background-color: #ff9500;
      }
    }

    &.maximize {
      background-color: #28ca42;

      &:hover {
        background-color: #30d158;
      }
    }
  }

  .close-btn:hover {
    background-color: transparent;
  }

  .minimize-btn:hover,
  .maximize-btn:hover {
    background-color: transparent;
  }
}
</style>
