<script setup lang="ts">
defineProps<{
  modelValue: string
  options: { id: string; name: string }[]
}>()
const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()
</script>

<template>
  <div class="platform-tabs">
    <button
      v-for="opt in options"
      :key="opt.id"
      class="platform-tab"
      :class="{ active: opt.id === modelValue }"
      @click="emit('update:modelValue', opt.id)"
    >
      {{ opt.name }}
    </button>
  </div>
</template>

<style scoped>
.platform-tabs {
  display: inline-flex;
  gap: 6px;
  padding: 4px;
  border-radius: 12px;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
  flex-wrap: wrap;
}
.platform-tab {
  border: none;
  background: transparent;
  color: var(--fluent-text-secondary);
  padding: 6px 14px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}
.platform-tab:hover {
  background: var(--fluent-bg-hover);
  color: var(--fluent-text);
}
.platform-tab.active {
  background: var(--fluent-accent);
  color: #fff;
}
</style>
