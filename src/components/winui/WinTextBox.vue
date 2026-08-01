<template>
  <input
    class="win-text-box"
    :value="modelValue"
    :placeholder="placeholder"
    :disabled="disabled"
    :type="type"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    @change="$emit('change', ($event.target as HTMLInputElement).value)"
  />
</template>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'WinTextBox',
  props: {
    modelValue: { type: [String, Number], default: '' },
    placeholder: { type: String, default: '' },
    disabled: { type: Boolean, default: false },
    type: { type: String, default: 'text' },
  },
  emits: ['update:modelValue', 'change'],
});
</script>

<style scoped>
.win-text-box {
  width: 100%;
  padding: 6px 10px;
  border: 1px solid var(--fluent-border, #d1d1d1);
  border-radius: 4px;
  background: var(--card-bg, #fff);
  color: var(--text-color, #323130);
  font-size: var(--fluent-font-size, 14px);
  outline: none;
  transition: border-color 0.15s ease;
}
.win-text-box:focus {
  border-color: var(--fluent-accent, #0078d4);
}
.win-text-box:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
