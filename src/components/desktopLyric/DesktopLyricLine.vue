<script setup lang="ts">
import { computed } from 'vue'
import type { LyricLine, LyricWord } from '@applemusic-like-lyrics/core'
import type { RenderLine } from '../../composables/useDesktopLyricRenderer'
import type { DesktopLyricConfig } from '../../composables/useConfig'

const props = defineProps<{
  line: RenderLine
  config: DesktopLyricConfig
  index: number
  showYrc: boolean
  yrcData: LyricLine[]
  getYrcStyle: (word: LyricWord, lyricIndex: number) => { backgroundPositionX: string }
  getScrollStyle: (line: RenderLine) => { transform: string; willChange?: string }
  getPlainText: (words: LyricWord[]) => string
  setLineRef: (el: Element | null, key: string) => void
  setContentRef: (el: Element | null, key: string) => void
}>()

const isYrc = computed(() => {
  return props.showYrc && props.yrcData.length > 0 && (props.line.line?.words?.length || 0) > 1
})

const playedColor = computed(() => props.config.mainColor)
const unplayedColor = computed(() => props.config.unplayedColor || 'rgba(255,255,255,0.5)')
</script>

<template>
  <div
    :ref="(el) => setLineRef(el as Element, line.key)"
    :class="[
      'dl-line',
      config.position,
      {
        active: line.active,
        'is-yrc': isYrc,
        'has-mask': config.textBackgroundMask,
        'is-next': !line.active && config.isDoubleLine,
        'align-left': config.position === 'both' && line.index % 2 === 0,
        'align-right': config.position === 'both' && line.index % 2 !== 0,
      }
    ]"
    :style="{
      color: line.active ? playedColor : unplayedColor,
      top: index === 0 ? '0px' : `${config.fontSize * 1.9}px`,
      fontSize: index > 0 ? '0.8em' : '1em',
    }"
  >
    <span
      v-if="isYrc"
      :ref="(el) => setContentRef(el as Element, line.key)"
      class="dl-scroll-content"
      :style="getScrollStyle(line)"
    >
      <span class="dl-content">
        <span
          v-for="(text, textIndex) in line.line.words"
          :key="textIndex"
          :class="['dl-text', { 'end-space': text.word.endsWith(' ') || text.startTime === 0 }]"
        >
          <span
            class="dl-word"
            :style="{
              fontWeight: config.fontWeight,
              backgroundImage: `linear-gradient(to right, ${playedColor} 50%, ${unplayedColor} 50%)`,
              textShadow: 'none',
              filter: `drop-shadow(0 0 1px ${config.shadowColor}) drop-shadow(0 0 2px ${config.shadowColor})`,
              ...getYrcStyle(text, line.index)
            }"
          >
            {{ text.word }}
          </span>
        </span>
      </span>
    </span>
    <span
      v-else
      :ref="(el) => setContentRef(el as Element, line.key)"
      class="dl-scroll-content"
      :style="[getScrollStyle(line), { fontWeight: config.fontWeight }]"
    >
      {{ getPlainText(line.line.words || []) }}
    </span>
  </div>
</template>

<style scoped>
.dl-line {
  position: absolute;
  width: 100%;
  left: 0;
  line-height: normal;
  padding: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition:
    top 0.6s cubic-bezier(0.55, 0, 0.1, 1),
    font-size 0.6s cubic-bezier(0.55, 0, 0.1, 1),
    color 0.6s cubic-bezier(0.55, 0, 0.1, 1),
    opacity 0.6s cubic-bezier(0.55, 0, 0.1, 1),
    transform 0.6s cubic-bezier(0.55, 0, 0.1, 1);
  will-change: top, font-size, transform;
  transform-origin: left center;
}
.dl-line.center {
  text-align: center;
  transform-origin: center center;
}
.dl-line.right {
  text-align: right;
  transform-origin: right center;
}
.dl-line.both.align-right {
  text-align: right;
  transform-origin: right center;
}
.dl-line.both.align-left {
  text-align: left;
  transform-origin: left center;
}
.dl-line.has-mask .dl-scroll-content {
  background-color: var(--mask-bg);
  border-radius: 6px;
  padding: 2px 8px;
  display: inline-block;
}
.dl-scroll-content {
  display: inline-block;
  white-space: nowrap;
  will-change: transform;
}
.dl-line.is-yrc .dl-content {
  display: inline-flex;
  flex-wrap: nowrap;
}
.dl-line.is-yric .dl-text {
  position: relative;
  display: inline-block;
}
.dl-word {
  display: inline-block;
  background-clip: text;
  -webkit-background-clip: text;
  color: transparent;
  background-size: 200% 100%;
  background-repeat: no-repeat;
  background-position-x: 100%;
  will-change: background-position-x;
}
.dl-text.end-space {
  margin-right: 5vh;
}
.dl-text.end-space:last-child {
  margin-right: 0;
}
.dl-line.is-yrc.center .dl-content {
  justify-content: center;
}
.dl-line.is-yrc.right .dl-content {
  justify-content: flex-end;
}
.dl-line.is-yrc.both.align-right .dl-content {
  justify-content: flex-end;
}
</style>
