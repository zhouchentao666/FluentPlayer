<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { MessagePlugin } from 'tdesign-vue-next'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const songmid = ref((route.query.songmid as string) || '')
const loading = ref(false)
const saving = ref(false)

const form = ref({
  name: '',
  singer: '',
  albumName: '',
  year: 0,
  filePath: ''
})

const fetchTags = async () => {
  if (!songmid.value) return
  loading.value = true
  try {
    const res = await (window as any).api?.localMusic?.getTags?.(songmid.value, false)
    if (res?.success && res.data) {
      form.value.name = res.data.name || ''
      form.value.singer = res.data.singer || ''
      form.value.albumName = res.data.albumName || ''
      form.value.year = res.data.year || 0
      form.value.filePath = res.data.path || ''
    }
  } catch (e) { console.error('获取标签失败:', e) }
  finally { loading.value = false }
}

const saveTags = async () => {
  if (!form.value.filePath) { MessagePlugin.warning(t('music.tagEditor.noPath')); return }
  saving.value = true
  try {
    await (window as any).api?.localMusic?.writeTags?.(
      form.value.filePath,
      { name: form.value.name, singer: form.value.singer, albumName: form.value.albumName, year: form.value.year },
      {}
    )
    MessagePlugin.success(t('music.tagEditor.saveSuccess'))
  } catch (e) { console.error('保存标签失败:', e); MessagePlugin.error(t('music.tagEditor.saveFailed')) }
  finally { saving.value = false }
}

onMounted(() => fetchTags())
</script>

<template>
  <div class="tag-editor-container">
    <div class="editor-header">
      <t-button variant="text" @click="router.back()">
        <template #icon><i class="iconfont icon-xiangzuo"></i></template>
        {{ t('common.back') }}
      </t-button>
      <h2>{{ t('music.tagEditor.title') }}</h2>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="loading-spinner"></div><p>{{ t('common.loading') }}</p>
    </div>
    <div v-else class="editor-form">
      <div class="form-item">
        <label>{{ t('music.tagEditor.songName') }}</label>
        <t-input v-model="form.name" :placeholder="t('music.tagEditor.songName')" />
      </div>
      <div class="form-item">
        <label>{{ t('music.tagEditor.artist') }}</label>
        <t-input v-model="form.singer" :placeholder="t('music.tagEditor.artist')" />
      </div>
      <div class="form-item">
        <label>{{ t('music.tagEditor.album') }}</label>
        <t-input v-model="form.albumName" :placeholder="t('music.tagEditor.album')" />
      </div>
      <div class="form-item">
        <label>{{ t('music.tagEditor.year') }}</label>
        <t-input-number v-model="form.year" :min="0" :max="2099" :placeholder="t('music.tagEditor.year')" />
      </div>
      <div class="form-item">
        <label>{{ t('music.tagEditor.filePath') }}</label>
        <div class="file-path">{{ form.filePath }}</div>
      </div>
      <div class="form-actions">
        <t-button theme="primary" :loading="saving" @click="saveTags">{{ t('music.tagEditor.saveTag') }}</t-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tag-editor-container { width: 100%; height: 100%; display: flex; flex-direction: column; overflow: hidden; padding: 20px; box-sizing: border-box; }
.editor-header { display: flex; align-items: center; gap: 12px; flex-shrink: 0; margin-bottom: 20px; }
.editor-header h2 { font-size: 20px; font-weight: 600; color: var(--td-text-color-primary); margin: 0; }
.editor-form { max-width: 600px; }
.form-item { margin-bottom: 16px; }
.form-item label { display: block; font-size: 13px; color: var(--td-text-color-secondary); margin-bottom: 6px; }
.file-path { font-size: 12px; color: var(--td-text-color-placeholder); word-break: break-all; padding: 8px; background: var(--td-bg-color-page); border-radius: 6px; }
.form-actions { margin-top: 24px; }
.loading-state { display: flex; flex-direction: column; align-items: center; padding: 60px; }
.loading-spinner { width: 40px; height: 40px; border: 3px solid var(--td-bg-color-component); border-top-color: var(--td-brand-color); border-radius: 50%; will-change: transform; animation: spin 1s linear infinite; margin-bottom: 12px; }
.loading-state p { color: var(--td-text-color-secondary); }

@media (max-width: 768px) {
  .tag-editor-container {
    min-width: 0;
    padding: var(--mobile-page-top-gutter) var(--mobile-page-gutter) 0;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  .editor-header {
    align-items: flex-start;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 18px;
  }

  .editor-header :deep(.t-button) {
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }

  .editor-header h2 {
    font-size: clamp(2rem, 9vw, 2.6rem);
    line-height: 1.1;
    letter-spacing: -0.04em;
  }

  .editor-form {
    width: 100%;
    max-width: none;
    padding-bottom: 12px;
  }

  .form-item {
    margin-bottom: 14px;
  }

  .form-item label {
    font-size: 14px;
  }

  .form-item :deep(.t-input),
  .form-item :deep(.t-input-number) {
    width: 100%;
    min-height: var(--mobile-touch-target);
  }

  .file-path {
    padding: 12px;
    border-radius: var(--mobile-card-radius-small);
    line-height: 1.5;
  }

  .form-actions :deep(.t-button) {
    width: 100%;
    min-height: var(--mobile-touch-target);
    border-radius: var(--mobile-control-radius);
    touch-action: manipulation;
  }
}

@media (prefers-reduced-motion: reduce) {
  .loading-spinner {
    animation: none;
  }
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
