<script setup lang="ts">
const { t } = useI18n()
</script>

<template>
  <div class="page profile-page">
    <h2>{{ t('auth.profile') }}</h2>
  </div>
</template>

<style scoped>
.profile-page {
  height: 100%;
  padding: 20px;
  box-sizing: border-box;
}

.profile-page h2 {
  margin: 0;
}

@media (max-width: 768px) {
  .profile-page {
    min-width: 0;
    padding: var(--mobile-page-top-gutter) var(--mobile-page-gutter) 0;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  .profile-page h2 {
    font-size: clamp(2rem, 9vw, 2.6rem);
    line-height: 1.1;
    letter-spacing: -0.04em;
  }
}
</style>
