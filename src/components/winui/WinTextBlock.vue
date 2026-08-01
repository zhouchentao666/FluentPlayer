<template>
  <component
    :is="tag"
    class="win-text-block"
    :class="[`text-${align}`, `text-${wrap}`, `text-${transform}`, { 'text-truncate': truncate }]"
  >
    <slot />
  </component>
</template>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'WinTextBlock',
  props: {
    tag: { type: String, default: 'span' },
    align: { type: String, default: 'left' },
    wrap: { type: String, default: 'wrap' },
    transform: { type: String, default: 'none' },
    truncate: { type: Boolean, default: false },
  },
});
</script>

<style scoped>
.win-text-block {
  font-size: var(--fluent-font-size, 14px);
  line-height: 1.4;
  color: var(--text-color, inherit);
}
</style>
