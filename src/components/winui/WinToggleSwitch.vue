<template>
  <label
    class="win-toggle-switch"
    :class="[
      `theme-${theme}`,
      { 'is-on': modelValue, 'is-off': !modelValue, 'is-disabled': disabled }
    ]"
  >
    <input
      class="toggle-switch-input"
      type="checkbox"
      role="switch"
      :checked="modelValue"
      :disabled="disabled"
      :aria-checked="modelValue"
      @change="onToggle"
    />
    <span class="toggle-switch-track" aria-hidden="true">
      <span class="toggle-switch-thumb" />
    </span>
    <WinTextBlock v-if="content || $slots.default" class="toggle-switch-content">
      <slot>{{ content }}</slot>
    </WinTextBlock>
  </label>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import WinTextBlock from './WinTextBlock.vue';

export default defineComponent({
  name: 'WinToggleSwitch',
  components: { WinTextBlock },
  props: {
    modelValue: { type: Boolean, default: false },
    disabled: { type: Boolean, default: false },
    content: { type: String, default: '' },
    theme: { type: String, default: 'light' },
  },
  emits: ['update:modelValue', 'change'],
  setup(props, { emit }) {
    const onToggle = (e: Event) => {
      const checked = (e.target as HTMLInputElement).checked;
      emit('update:modelValue', checked);
      emit('change', checked);
    };
    return { onToggle };
  },
});
</script>

<style scoped>
.win-toggle-switch {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  user-select: none;
  font-family: var(--fluent-font-family, 'Segoe UI', sans-serif);
}
.win-toggle-switch.is-disabled {
  cursor: not-allowed;
  opacity: 0.6;
}
.toggle-switch-input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
  margin: 0;
  padding: 0;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}
.toggle-switch-track {
  position: relative;
  display: inline-block;
  width: 40px;
  height: 20px;
  background: var(--fluent-border, #d1d1d1);
  border-radius: 10px;
  transition: background 0.2s ease;
  flex: none;
}
.win-toggle-switch.theme-light .toggle-switch-track,
.win-toggle-switch.theme-dark .toggle-switch-track {
  background: var(--fluent-border, #d1d1d1);
}
.toggle-switch-track {
  background: var(--ctrl-border, #d1d1d1);
}
.is-on .toggle-switch-track {
  background: var(--fluent-accent, #0078d4);
}
.toggle-switch-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  background: #fff;
  border-radius: 50%;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  transition: transform 0.2s ease;
}
.is-on .toggle-switch-thumb {
  transform: translateX(20px);
}
.toggle-switch-content {
  font-size: var(--fluent-font-size, 14px);
  color: var(--text-color, #323130);
}
</style>
