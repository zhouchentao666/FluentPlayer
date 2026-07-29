<script lang="ts" setup>
export type PlayMode = 'sequential' | 'single' | 'reverse' | 'stop' | 'shuffle'

const props = defineProps<{
  isPlaying: boolean
  playMode: PlayMode
}>()

const emit = defineEmits<{
  (e: 'toggle-play'): void
  (e: 'prev'): void
  (e: 'next'): void
  (e: 'cycle-mode'): void
  (e: 'toggle-queue'): void
}>()

const modeConfig: Record<PlayMode, { icon: string; title: string }> = {
  sequential: {
    icon: 'M7 7h10v2H9v2.5L5.5 8 9 4.5V7zm10 10H7v-2h8v-2.5l3.5 3.5-3.5 3.5V17z',
    title: '顺序播放',
  },
  single: {
    icon: 'M7 7h10v2H9v2.5L5.5 8 9 4.5V7zm10 10H7v-2h8v-2.5l3.5 3.5-3.5 3.5V17z M12 13c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z',
    title: '单曲循环',
  },
  reverse: {
    icon: 'M17 7H7v2h8v2.5l3.5-3.5L15 4.5V7zM7 17h10v-2H9v-2.5L5.5 16 9 19.5V17z',
    title: '逆序播放',
  },
  stop: {
    icon: 'M8 8h8v8H8V8z',
    title: '播完就停',
  },
  shuffle: {
    icon: 'M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.46 20 9.5V4h-5.5zm.33 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z',
    title: '随机播放',
  },
}
</script>

<template>
  <div class="controls">
    <button
      class="side-btn"
      :title="modeConfig[playMode].title"
      @click="emit('cycle-mode')"
    >
      <svg viewBox="0 0 24 24" fill="currentColor">
        <path :d="modeConfig[playMode].icon" />
      </svg>
    </button>

    <button class="control-btn" title="上一首" @click="emit('prev')">
      <svg viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6V6zm3.5 6l8.5 6V6l-8.5 6z" /></svg>
    </button>
    <button class="play-btn" :title="isPlaying ? '暂停' : '播放'" @click="emit('toggle-play')">
      <svg v-if="isPlaying" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" /></svg>
      <svg v-else viewBox="0 0 24 24" fill="currentColor"><path d="M8.3 5v14l11-7z" /></svg>
    </button>
    <button class="control-btn" title="下一首" @click="emit('next')">
      <svg viewBox="0 0 24 24" fill="currentColor"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" /></svg>
    </button>

    <button class="side-btn" title="播放列表" @click="emit('toggle-queue')">
      <svg viewBox="0 0 24 24" fill="currentColor">
        <path d="M4 6h16v2H4V6zm0 5h16v2H4v-2zm0 5h16v2H4v-2z" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.controls {
  display: flex;
  align-items: center;
  gap: 12px;
}

.control-btn,
.side-btn {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: inherit;
  cursor: pointer;
  transition: background 0.18s ease, transform 0.1s ease, color 0.18s ease;
}

.control-btn:hover,
.side-btn:hover {
  background: var(--fluent-bg-hover);
}

.control-btn:active,
.side-btn:active {
  transform: scale(0.95);
}

.control-btn svg,
.side-btn svg {
  width: 22px;
  height: 22px;
}

.side-btn {
  color: var(--fluent-text-secondary);
}

.side-btn:hover {
  color: var(--fluent-text);
}

.play-btn {
  width: 42px;
  height: 42px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 50%;
  background: var(--fluent-bg-active);
  color: inherit;
  cursor: pointer;
  transition: background 0.18s ease, transform 0.1s ease;
}

.play-btn:hover {
  background: var(--fluent-accent);
  color: #fff;
}

.play-btn:active {
  transform: scale(0.95);
}

.play-btn svg {
  width: 24px;
  height: 24px;
}
</style>
