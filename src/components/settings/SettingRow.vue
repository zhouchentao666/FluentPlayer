<script lang="ts" setup>
defineProps<{
  label: string
  description?: string
}>()
</script>

<template>
  <div class="setting-row">
    <div class="setting-info">
      <div class="setting-label">{{ label }}</div>
      <div v-if="description" class="setting-desc">{{ description }}</div>
    </div>
    <slot />
  </div>
</template>

<style scoped>
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0;
  gap: 16px;
}

.setting-row + .setting-row {
  border-top: 1px solid var(--fluent-border);
}

.setting-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.setting-label {
  font-size: 14px;
  font-weight: 500;
}

.setting-desc {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
</style>
