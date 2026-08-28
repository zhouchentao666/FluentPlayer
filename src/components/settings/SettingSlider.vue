<script lang="ts" setup>
import WinUISlider from './WinUISlider.vue'

withDefaults(
  defineProps<{
    label?: string
    min?: number
    max?: number
    step?: number
    modelValue: number
  }>(),
  {
    label: '',
    min: 0,
    max: 100,
    step: 1,
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
}>()

function handleInput(value: number) {
  emit('update:modelValue', value)
}
</script>

<template>
  <div class="slider-row">
    <span v-if="label" class="slider-label">{{ label }}</span>
    <WinUISlider
      class="slider-input"
      :model-value="modelValue"
      :min="min"
      :max="max"
      :step="step"
      :aria-label="label || 'slider'"
      @update:model-value="handleInput"
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
}

.slider-value {
  font-size: 12px;
  color: var(--fluent-text-secondary);
  min-width: 28px;
  text-align: right;
}
</style>
