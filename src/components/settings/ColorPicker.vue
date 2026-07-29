<script lang="ts" setup>
defineProps<{
  colors: string[]
  modelValue: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()
</script>

<template>
  <div class="color-picker">
    <button
      v-for="color in colors"
      :key="color"
      :class="['color-dot', { active: modelValue === color }]"
      :style="{ background: color }"
      @click="emit('update:modelValue', color)"
    />
  </div>
</template>

<style scoped>
.color-picker {
  display: flex;
  gap: 8px;
}

.color-dot {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: transform 0.15s ease, border-color 0.15s ease;
}

.color-dot.active {
  border-color: var(--fluent-text);
  transform: scale(1.1);
}
</style>
