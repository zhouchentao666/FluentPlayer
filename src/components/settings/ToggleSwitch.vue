<script lang="ts" setup>
defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()
</script>

<template>
  <button
    :class="['toggle', { on: modelValue }]"
    @click="emit('update:modelValue', !modelValue)"
  >
    <span class="toggle-thumb"></span>
  </button>
</template>

<style scoped>
.toggle {
  width: 40px;
  height: 22px;
  border: none;
  border-radius: 11px;
  background: var(--fluent-border);
  position: relative;
  cursor: pointer;
  transition: background 0.2s ease;
}

.toggle.on {
  background: var(--fluent-accent);
}

.toggle-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: white;
  transition: transform 0.2s ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
}

.toggle.on .toggle-thumb {
  transform: translateX(18px);
}
</style>
