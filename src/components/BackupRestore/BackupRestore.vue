<template>
  <div class="backup-restore-btn" @click="showDialog = true">
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" />
    </svg>
    <span class="btn-label">{{ t('backup.backup') }}</span>
    <span v-if="s3Store.isConnected" class="status-dot" />
  </div>
  <S3ConfigDialog v-model:visible="showDialog" />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useS3BackupStore } from '@/store/S3Backup'
import S3ConfigDialog from './S3ConfigDialog.vue'

const { t } = useI18n()
const s3Store = useS3BackupStore()
const showDialog = ref(false)
</script>

<style scoped>
.backup-restore-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 4px 10px 4px 8px;
  border-radius: 999px;
  cursor: pointer;
  transition: background 0.15s;
  height: 30px;
  box-sizing: border-box;
  position: relative;
  color: var(--td-text-color-secondary);

  &:hover {
    background: rgba(125, 125, 125, 0.12);
    color: var(--td-text-color-primary);
  }

  &:active {
    background: rgba(125, 125, 125, 0.18);
  }
}

.btn-label {
  font-size: 12px;
  font-weight: 500;
  line-height: 1;
  white-space: nowrap;
}

.status-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--td-success-color, #2ba471);
  box-shadow: 0 0 4px rgba(43, 164, 113, 0.5);
  position: absolute;
  top: 5px;
  right: 7px;
}
</style>
