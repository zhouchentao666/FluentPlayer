<script setup lang="ts">
import { toasts } from '../composables/useToast'
</script>

<template>
  <Teleport to="body">
    <TransitionGroup name="toast" tag="div" class="toast-container">
      <div v-for="t in toasts" :key="t.id" class="toast-item" :class="`toast-${t.type}`">
        {{ t.message }}
      </div>
    </TransitionGroup>
  </Teleport>
</template>

<style scoped>
.toast-container {
  position: fixed;
  top: 48px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 9999;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  pointer-events: none;
}
.toast-item {
  max-width: 420px;
  padding: 9px 16px;
  border-radius: 10px;
  font-size: 13px;
  color: #fff;
  background: rgba(40, 40, 46, 0.92);
  border: 1px solid rgba(255, 255, 255, 0.1);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  backdrop-filter: blur(12px);
  word-break: break-all;
}
.toast-error { background: rgba(160, 46, 52, 0.92); }
.toast-success { background: rgba(38, 120, 70, 0.92); }
.toast-enter-active, .toast-leave-active { transition: all 0.25s ease; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translateY(-8px); }
</style>
