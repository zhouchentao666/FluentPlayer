<script lang="ts" setup>
import { computed } from 'vue'
import { OpenImageFile } from '@bridge/app'
import type { AppSettings, WindowEffect } from '../../composables/useConfig'
import SettingCard from './SettingCard.vue'
import SettingRow from './SettingRow.vue'
import SegmentedControl from './SegmentedControl.vue'
import SettingSlider from './SettingSlider.vue'

const props = defineProps<{
  settings: AppSettings
}>()

const emit = defineEmits<{
  update: [partial: Partial<AppSettings>]
}>()

const effects = [
  { value: 'none', label: '无' },
  { value: 'acrylic', label: '亚克力' },
  { value: 'custom-image', label: '自定义图片' },
  { value: 'song-color', label: '歌曲背景' },
] as const

const fileName = computed(() => {
  const path = props.settings.customImagePath
  if (!path) return '未选择图片'
  const sep = path.includes('\\') ? '\\' : '/'
  return path.split(sep).pop() || path
})

function updateEffect(value: string) {
  emit('update', { windowEffect: value as WindowEffect })
}

async function chooseImage() {
  const path = await OpenImageFile()
  if (path) emit('update', { customImagePath: path })
}

function clearImage() {
  emit('update', { customImagePath: '' })
}
</script>

<template>
  <SettingCard title="窗口特效">
    <SettingRow label="特效类型" description="选择窗口背景效果">
      <SegmentedControl
        :options="effects"
        :model-value="settings.windowEffect"
        @update:model-value="updateEffect"
      />
    </SettingRow>

    <template v-if="settings.windowEffect === 'custom-image'">
      <SettingRow label="背景图片" description="选择一张图片作为窗口背景">
        <div class="image-row">
          <span class="file-name">{{ fileName }}</span>
          <button class="file-btn" @click="chooseImage">选择图片</button>
          <button v-if="settings.customImagePath" class="file-btn" @click="clearImage">清除</button>
        </div>
      </SettingRow>
      <SettingRow label="遮罩透明度">
        <SettingSlider
          :model-value="settings.customImageOpacity"
          @update:model-value="value => emit('update', { customImageOpacity: value })"
        />
      </SettingRow>
      <SettingRow label="模糊程度">
        <SettingSlider
          :min="0"
          :max="80"
          :model-value="settings.customImageBlur"
          @update:model-value="value => emit('update', { customImageBlur: value })"
        />
      </SettingRow>
    </template>

    <template v-if="settings.windowEffect === 'song-color'">
      <SettingRow label="遮罩透明度" description="亮色模式下遮罩的透明度">
        <SettingSlider
          :model-value="settings.songColorOpacity"
          @update:model-value="value => emit('update', { songColorOpacity: value })"
        />
      </SettingRow>
      <SettingRow label="模糊程度">
        <SettingSlider
          :min="0"
          :max="80"
          :model-value="settings.songColorBlur"
          @update:model-value="value => emit('update', { songColorBlur: value })"
        />
      </SettingRow>
    </template>
  </SettingCard>
</template>

<style scoped>
.image-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.file-name {
  font-size: 13px;
  color: var(--fluent-text-secondary);
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-btn {
  padding: 6px 12px;
  border: 1px solid var(--fluent-border);
  border-radius: 6px;
  background: var(--fluent-bg-hover);
  color: inherit;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.18s ease;
  white-space: nowrap;
}

.file-btn:hover {
  background: var(--fluent-bg-active);
}
</style>
