<script lang="ts" setup>
const props = defineProps<{
  reflectionUrl: string
}>()
</script>

<template>
  <transition name="reflection-reveal" appear>
    <div v-if="reflectionUrl" class="reflection-wrapper">
      <div class="reflection-glass">
        <img :src="reflectionUrl" class="reflection-img" draggable="false" decoding="async" />
      </div>
    </div>
  </transition>
</template>

<style scoped>
.reflection-wrapper {
  position: absolute;
  top: calc(100% + 2px);
  left: 0;
  width: 100%;
  height: 65%;
  pointer-events: none;
  z-index: 10;
  border-radius: inherit;
  overflow: hidden;
  perspective: 1500px;
  transform-origin: top;
  transform: rotateX(40deg) skewX(-18deg) scale(1.01);
  opacity: 0.2;
}

.reflection-glass {
  position: absolute;
  inset: 0;
  border-radius: inherit;
  overflow: hidden;
  -webkit-mask-image: linear-gradient(
    to bottom,
    black 0%,
    rgba(0, 0, 0, 0.5) 30%,
    transparent 85%
  );
  mask-image: linear-gradient(
    to bottom,
    black 0%,
    rgba(0, 0, 0, 0.5) 30%,
    transparent 85%
  );
}

.reflection-img {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  aspect-ratio: 1;
  object-fit: cover;
  transform: scaleY(-1);
}

.reflection-reveal-enter-active,
.reflection-reveal-appear-active {
  transition:
    transform 560ms cubic-bezier(0.22, 1, 0.36, 1) 220ms,
    opacity 420ms ease-out 220ms,
    filter 560ms cubic-bezier(0.22, 1, 0.36, 1) 220ms;
}

.reflection-reveal-leave-active {
  transition:
    transform 220ms cubic-bezier(0.4, 0, 0.2, 1),
    opacity 180ms ease-in,
    filter 220ms cubic-bezier(0.4, 0, 0.2, 1);
}

.reflection-reveal-enter-from,
.reflection-reveal-appear-from,
.reflection-reveal-leave-to {
  opacity: 0;
  filter: blur(10px);
}

.reflection-reveal-enter-from,
.reflection-reveal-appear-from {
  transform: translateY(-18px) rotateX(58deg) skewX(-22deg) scale(0.96);
}

.reflection-reveal-leave-to {
  transform: translateY(-10px) rotateX(48deg) skewX(-20deg) scale(0.985);
}
</style>
