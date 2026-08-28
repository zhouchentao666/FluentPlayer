<script lang="ts" setup>
import { ref, nextTick, onMounted } from 'vue'

const emit = defineEmits<{
  (e: 'confirm', name: string): void
  (e: 'cancel'): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const name = ref('')

onMounted(() => {
  nextTick(() => inputRef.value?.focus())
})

function confirm() {
  emit('confirm', name.value.trim())
}

function cancel() {
  emit('cancel')
}
</script>

<template>
  <div class="create-row">
    <input
      ref="inputRef"
      v-model="name"
      type="text"
      placeholder="歌单名称"
      @keydown.enter="confirm"
      @keydown.esc="cancel"
      @blur="confirm"
    />
  </div>
</template>

<style scoped>
.create-row {
  padding: 4px 8px;
}

.create-row input {
  width: 100%;
  box-sizing: border-box;
  padding: 7px 10px;
  border: 1px solid var(--fluent-input-border);
  border-radius: 8px;
  background: var(--fluent-input-bg);
  color: inherit;
  font-size: 13px;
  outline: none;
}

.create-row input::placeholder {
  color: var(--fluent-text-secondary);
}
</style>
