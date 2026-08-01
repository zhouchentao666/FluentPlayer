<template>
  <WinToggleSwitch
    :model-value="modelValue"
    :disabled="disabled"
    :content="content"
    :theme="theme"
    @update:model-value="$emit('update:modelValue', $event)"
    @change="$emit('change', $event)"
  >
    <slot />
  </WinToggleSwitch>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import WinToggleSwitch from '../winui/WinToggleSwitch.vue';

export default defineComponent({
  name: 'ToggleSwitch',
  components: { WinToggleSwitch },
  props: {
    modelValue: { type: Boolean, default: false },
    disabled: { type: Boolean, default: false },
    content: { type: String, default: '' },
    theme: { type: String, default: 'light' },
  },
  emits: ['update:modelValue', 'change'],
});
</script>
