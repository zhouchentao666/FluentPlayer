<script lang="ts" setup>
defineProps<{
  label?: string
  min?: number
  max?: number
  modelValue: number
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
}>()

function handleInput(event: Event) {
  emit('update:modelValue', Number((event.target as HTMLInputElement).value))
}
</script>

<template>
  <div class="slider-row">
    <span v-if="label" class="slider-label">{{ label }}</span>
    <input
      type="range"
      class="slider-input"
      :min="min ?? 0"
      :max="max ?? 100"
      :value="modelValue"
      @input="handleInput"
    />
    <span class="slider-value">{{ modelValue }}</span>
  </div>
</template>

<style scoped>
.slider-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 220px;
}

.slider-label {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
}

.slider-input {
  flex: 1;
  accent-color: var(--fluent-accent);
  cursor: pointer;
}

.slider-value {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  min-width: 28px;
  text-align: right;
}
</style>
