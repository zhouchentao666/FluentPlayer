<script lang="ts" setup>
import { computed } from 'vue'
import { useDominantColors } from '../../composables/useDominantColors'
import PlayerDetailDynamicBackground from './PlayerDetailDynamicBackground.vue'

const props = defineProps<{
  coverUrl: string | null
  active: boolean
  backgroundMode?: 'static' | 'dynamic'
  hasLyrics?: boolean
}>()

const { dominantColors } = useDominantColors(computed(() => props.coverUrl))
</script>

<template>
  <div class="background-container">
    <!-- Dark base -->
    <div class="base-bg"></div>

    <template v-if="backgroundMode === 'dynamic'">
      <PlayerDetailDynamicBackground
        :cover-url="coverUrl"
        :active="active"
        :has-lyrics="hasLyrics"
      />
    </template>

    <template v-else>
      <!-- Dominant color wash -->
      <div
        class="color-layer"
        :style="{ backgroundColor: dominantColors[0] }"
      ></div>

      <!-- Blurred cover as main background -->
      <div v-if="coverUrl" class="cover-bg-layer">
        <img
          :src="coverUrl"
          class="cover-bg-img"
          draggable="false"
          decoding="async"
        />
        <div class="cover-overlay"></div>
      </div>

      <!-- Soft radial gradients from palette -->
      <div
        class="gradient-1"
        :style="{ background: `radial-gradient(circle at 24% 16%, ${dominantColors[1] || dominantColors[0]}22 0%, transparent 52%)` }"
      ></div>
      <div
        class="gradient-2"
        :style="{ background: `radial-gradient(circle at 78% 84%, ${dominantColors[2] || dominantColors[0]}18 0%, transparent 62%)` }"
      ></div>

      <!-- Vignette / edge darkening -->
      <div class="edge-gradient-h"></div>
      <div class="edge-gradient-v"></div>
    </template>
  </div>
</template>

<style scoped>
.background-container {
  position: absolute;
  inset: 0;
  z-index: 0;
  overflow: hidden;
  pointer-events: none;
  user-select: none;
}

.base-bg {
  position: absolute;
  inset: 0;
  background: #0a0a0a;
  z-index: 0;
}

.color-layer {
  position: absolute;
  inset: 0;
  opacity: 0.12;
  transition: background-color 1200ms;
  z-index: 0;
}

.cover-bg-layer {
  position: absolute;
  inset: 0;
  z-index: 1;
  overflow: hidden;
}

.cover-bg-img {
  position: absolute;
  inset: -10%;
  width: 120%;
  height: 120%;
  object-fit: cover;
  filter: blur(48px) brightness(0.55) saturate(1.4);
  transform: scale(1.05);
}

.cover-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  backdrop-filter: blur(6px);
}

.gradient-1 {
  position: absolute;
  inset: 0;
  z-index: 2;
}

.gradient-2 {
  position: absolute;
  inset: 0;
  z-index: 3;
}

.edge-gradient-h {
  position: absolute;
  inset: 0;
  z-index: 18;
  background: linear-gradient(to right, rgba(0, 0, 0, 0.2), transparent 25%, transparent 75%, rgba(0, 0, 0, 0.2));
}

.edge-gradient-v {
  position: absolute;
  inset: 0;
  z-index: 20;
  background: linear-gradient(to bottom, rgba(0, 0, 0, 0.15), transparent 35%, rgba(0, 0, 0, 0.45));
}
</style>
