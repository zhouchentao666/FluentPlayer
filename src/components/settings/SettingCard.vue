<script lang="ts" setup>
defineProps<{
  title: string
}>()
</script>

<template>
  <section class="setting-card">
    <h2>{{ title }}</h2>
    <slot />
  </section>
</template>

<style scoped>
.setting-card {
  padding: 20px;
  border-radius: 12px;
  background: var(--fluent-bg-card);
  border: 1px solid var(--fluent-border);
  backdrop-filter: blur(40px) saturate(125%);
  -webkit-backdrop-filter: blur(40px) saturate(125%);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.08);
}

.setting-card h2 {
  margin: 0 0 16px;
  font-size: 16px;
  font-weight: 600;
}
</style>
