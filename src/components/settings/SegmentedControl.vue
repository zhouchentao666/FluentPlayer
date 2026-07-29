<script lang="ts" setup>
interface Option {
  readonly value: string
  readonly label: string
}

defineProps<{
  options: readonly Option[]
  modelValue: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()
</script>

<template>
  <div class="segmented-control">
    <button
      v-for="opt in options"
      :key="opt.value"
      :class="['segment', { active: modelValue === opt.value }]"
      @click="emit('update:modelValue', opt.value)"
    >
      {{ opt.label }}
    </button>
  </div>
</template>

<style scoped>
.segmented-control {
  display: flex;
  padding: 3px;
  gap: 3px;
  border-radius: 8px;
  background: var(--fluent-bg-hover);
}

.segment {
  border: none;
  background: transparent;
  color: inherit;
  padding: 5px 12px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.18s ease, box-shadow 0.18s ease;
}

.segment.active {
  background: var(--fluent-bg-card);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}
</style>
