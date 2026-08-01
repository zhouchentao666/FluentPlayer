<template>
  <div class="win-scroll-viewer" :class="{ 'is-horizontal': horizontal, 'hide-scroll': hideScroll }">
    <slot />
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'WinScrollViewer',
  props: {
    horizontal: { type: Boolean, default: false },
    hideScroll: { type: Boolean, default: false },
  },
});
</script>

<style scoped>
.win-scroll-viewer {
  overflow: auto;
  -webkit-overflow-scrolling: touch;
}
.win-scroll-viewer.hide-scroll {
  scrollbar-width: none;
}
.win-scroll-viewer.hide-scroll::-webkit-scrollbar {
  display: none;
}
</style>
