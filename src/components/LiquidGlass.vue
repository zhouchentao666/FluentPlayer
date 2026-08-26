<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, useId, type CSSProperties } from 'vue'
import { ShaderDisplacementGenerator, fragmentShaders } from './lib/shader-utils'
import { displacementMap, polarDisplacementMap, prominentDisplacementMap } from './lib/displacement-maps'

type GlassMode = 'standard' | 'polar' | 'prominent' | 'shader'

type MousePoint = { x: number; y: number }

const generateShaderDisplacementMap = (width: number, height: number): string => {
  const generator = new ShaderDisplacementGenerator({
    width,
    height,
    fragment: fragmentShaders.liquidGlass,
  })
  const dataUrl = generator.updateShader()
  generator.destroy()
  return dataUrl
}

const getMap = (mode: GlassMode, shaderMapUrl?: string) => {
  switch (mode) {
    case 'standard':
      return displacementMap
    case 'polar':
      return polarDisplacementMap
    case 'prominent':
      return prominentDisplacementMap
    case 'shader':
      return shaderMapUrl || displacementMap
    default:
      throw new Error(`Invalid mode: ${mode}`)
  }
}

const props = withDefaults(
  defineProps<{
    displacementScale?: number
    blurAmount?: number
    saturation?: number
    aberrationIntensity?: number
    elasticity?: number
    cornerRadius?: number
    globalMousePos?: MousePoint
    mouseOffset?: MousePoint
    mouseContainer?: HTMLElement | null
    padding?: string
    overLight?: boolean
    mode?: GlassMode
    style?: CSSProperties
    contentStyle?: CSSProperties
  }>(),
  {
    displacementScale: 70,
    blurAmount: 0.0625,
    saturation: 140,
    aberrationIntensity: 2,
    elasticity: 0.15,
    cornerRadius: 999,
    mouseContainer: null,
    padding: '24px 32px',
    overLight: false,
    mode: 'standard',
    style: () => ({}),
    contentStyle: () => ({}),
  },
)

const emit = defineEmits<{
  click: [event: MouseEvent]
}>()

const glassRef = ref<HTMLDivElement | null>(null)
const isHovered = ref(false)
const isActive = ref(false)
const glassSize = ref({ width: 270, height: 69 })
const internalGlobalMousePos = ref<MousePoint>({ x: 0, y: 0 })
const internalMouseOffset = ref<MousePoint>({ x: 0, y: 0 })
const shaderMapUrl = ref('')

const vueId = useId()
const filterId = `glass-filter-${vueId}`

const isFirefox = typeof navigator !== 'undefined' && navigator.userAgent.toLowerCase().includes('firefox')
let mouseTrackingTarget: HTMLElement | null = null
let resizeObserver: ResizeObserver | null = null

const globalMousePos = computed(() => props.globalMousePos || internalGlobalMousePos.value)
const mouseOffset = computed(() => props.mouseOffset || internalMouseOffset.value)

const handleMouseMove = (e: MouseEvent) => {
  const container = mouseTrackingTarget || props.mouseContainer || glassRef.value
  if (!container) return

  const rect = container.getBoundingClientRect()
  const centerX = rect.left + rect.width / 2
  const centerY = rect.top + rect.height / 2

  internalMouseOffset.value = {
    x: ((e.clientX - centerX) / rect.width) * 100,
    y: ((e.clientY - centerY) / rect.height) * 100,
  }

  internalGlobalMousePos.value = {
    x: e.clientX,
    y: e.clientY,
  }
}

const cleanupMouseTracking = () => {
  if (!mouseTrackingTarget) return

  mouseTrackingTarget.removeEventListener('mousemove', handleMouseMove)
  mouseTrackingTarget = null
}

const setupMouseTracking = () => {
  if (props.globalMousePos && props.mouseOffset) return

  const container = props.mouseContainer || glassRef.value
  if (!container || mouseTrackingTarget === container) return

  cleanupMouseTracking()
  container.addEventListener('mousemove', handleMouseMove)
  mouseTrackingTarget = container
}

const calculateDirectionalScale = computed(() => {
  if (!globalMousePos.value.x && !globalMousePos.value.y) return 'scale(1)'
  if (!glassRef.value) return 'scale(1)'

  const rect = glassRef.value.getBoundingClientRect()
  const pillCenterX = rect.left + rect.width / 2
  const pillCenterY = rect.top + rect.height / 2

  const deltaX = globalMousePos.value.x - pillCenterX
  const deltaY = globalMousePos.value.y - pillCenterY

  const edgeDistanceX = Math.max(0, Math.abs(deltaX) - glassSize.value.width / 2)
  const edgeDistanceY = Math.max(0, Math.abs(deltaY) - glassSize.value.height / 2)
  const edgeDistance = Math.sqrt(edgeDistanceX * edgeDistanceX + edgeDistanceY * edgeDistanceY)

  const activationZone = 200
  if (edgeDistance > activationZone) return 'scale(1)'

  const fadeInFactor = 1 - edgeDistance / activationZone

  const centerDistance = Math.sqrt(deltaX * deltaX + deltaY * deltaY)
  if (centerDistance === 0) return 'scale(1)'

  const normalizedX = deltaX / centerDistance
  const normalizedY = deltaY / centerDistance

  const stretchIntensity = Math.min(centerDistance / 300, 1) * props.elasticity * fadeInFactor

  const scaleX = 1 + Math.abs(normalizedX) * stretchIntensity * 0.3 - Math.abs(normalizedY) * stretchIntensity * 0.15
  const scaleY = 1 + Math.abs(normalizedY) * stretchIntensity * 0.3 - Math.abs(normalizedX) * stretchIntensity * 0.15

  return `scaleX(${Math.max(0.8, scaleX)}) scaleY(${Math.max(0.8, scaleY)})`
})

const calculateFadeInFactor = computed(() => {
  if (!globalMousePos.value.x && !globalMousePos.value.y) return 0
  if (!glassRef.value) return 0

  const rect = glassRef.value.getBoundingClientRect()
  const pillCenterX = rect.left + rect.width / 2
  const pillCenterY = rect.top + rect.height / 2

  const edgeDistanceX = Math.max(0, Math.abs(globalMousePos.value.x - pillCenterX) - glassSize.value.width / 2)
  const edgeDistanceY = Math.max(0, Math.abs(globalMousePos.value.y - pillCenterY) - glassSize.value.height / 2)
  const edgeDistance = Math.sqrt(edgeDistanceX * edgeDistanceX + edgeDistanceY * edgeDistanceY)

  const activationZone = 200
  return edgeDistance > activationZone ? 0 : 1 - edgeDistance / activationZone
})

const calculateElasticTranslation = computed(() => {
  if (!glassRef.value) return { x: 0, y: 0 }

  const fadeIn = calculateFadeInFactor.value
  const rect = glassRef.value.getBoundingClientRect()
  const pillCenterX = rect.left + rect.width / 2
  const pillCenterY = rect.top + rect.height / 2

  return {
    x: (globalMousePos.value.x - pillCenterX) * props.elasticity * 0.1 * fadeIn,
    y: (globalMousePos.value.y - pillCenterY) * props.elasticity * 0.1 * fadeIn,
  }
})

const transformStyle = computed(() => {
  const translation = calculateElasticTranslation.value
  const scale = isActive.value ? 'scale(0.96)' : calculateDirectionalScale.value
  return `translate(${translation.x}px, ${translation.y}px) ${scale}`
})

const baseStyle = computed<CSSProperties>(() => ({
  ...props.style,
  transform: transformStyle.value,
  transition: 'transform ease-out 0.2s',
}))

const glassBodyStyle = computed<CSSProperties>(() => ({
  borderRadius: `${props.cornerRadius}px`,
  display: 'inline-flex',
  alignItems: 'center',
  gap: '24px',
  padding: props.padding,
  overflow: 'hidden',
  transition: 'all 0.2s ease-in-out',
  boxShadow: glassBoxShadow.value,
  boxSizing: 'border-box',
}))

const defaultContentStyle = computed<CSSProperties>(() => ({
  position: 'relative',
  zIndex: 1,
  color: '#fff',
  font: '500 20px/1 system-ui',
  textShadow: props.overLight ? '0px 2px 12px rgba(0, 0, 0, 0)' : '0px 2px 12px rgba(0, 0, 0, 0.4)',
  transition: 'all 150ms ease-in-out',
}))

const borderGradient1 = computed(() =>
  `linear-gradient(${135 + mouseOffset.value.x * 1.2}deg, rgba(255, 255, 255, 0.0) 0%, rgba(255, 255, 255, ${0.12 + Math.abs(mouseOffset.value.x) * 0.008}) ${Math.max(10, 33 + mouseOffset.value.y * 0.3)}%, rgba(255, 255, 255, ${0.4 + Math.abs(mouseOffset.value.x) * 0.012}) ${Math.min(90, 66 + mouseOffset.value.y * 0.4)}%, rgba(255, 255, 255, 0.0) 100%)`,
)

const borderGradient2 = computed(() =>
  `linear-gradient(${135 + mouseOffset.value.x * 1.2}deg, rgba(255, 255, 255, 0.0) 0%, rgba(255, 255, 255, ${0.32 + Math.abs(mouseOffset.value.x) * 0.008}) ${Math.max(10, 33 + mouseOffset.value.y * 0.3)}%, rgba(255, 255, 255, ${0.6 + Math.abs(mouseOffset.value.x) * 0.012}) ${Math.min(90, 66 + mouseOffset.value.y * 0.4)}%, rgba(255, 255, 255, 0.0) 100%)`,
)

const displacementMapUrl = computed(() => getMap(props.mode, shaderMapUrl.value))

const backdropFilterStyle = computed<CSSProperties>(() => ({
  filter: isFirefox ? undefined : `url(#${filterId})`,
  backdropFilter: `blur(${(props.overLight ? 12 : 4) + props.blurAmount * 32}px) saturate(${props.saturation}%)`,
}))

const updateGlassSize = () => {
  if (!glassRef.value) return

  const rect = glassRef.value.getBoundingClientRect()
  const width = Math.round(rect.width)
  const height = Math.round(rect.height)

  if (width === glassSize.value.width && height === glassSize.value.height) return
  glassSize.value = { width, height }
}

const glassBoxShadow = computed(() =>
  props.overLight
    ? '0px 16px 70px rgba(0, 0, 0, 0.75)'
    : '0px 12px 40px rgba(0, 0, 0, 0.25)',
)

watch(() => props.mode, () => {
  if (props.mode === 'shader') {
    shaderMapUrl.value = generateShaderDisplacementMap(glassSize.value.width, glassSize.value.height)
  }
})

watch([() => glassSize.value.width, () => glassSize.value.height], () => {
  if (props.mode === 'shader') {
    shaderMapUrl.value = generateShaderDisplacementMap(glassSize.value.width, glassSize.value.height)
  }
})

watch([() => props.globalMousePos, () => props.mouseOffset], ([newGlobal, newOffset]) => {
  if (newGlobal && newOffset) {
    cleanupMouseTracking()
  } else {
    setupMouseTracking()
  }
})

watch(() => props.mouseContainer, () => {
  cleanupMouseTracking()
  setupMouseTracking()
})

onMounted(() => {
  updateGlassSize()
  setupMouseTracking()

  if (typeof ResizeObserver !== 'undefined' && glassRef.value) {
    resizeObserver = new ResizeObserver(updateGlassSize)
    resizeObserver.observe(glassRef.value)
  }

  window.addEventListener('resize', updateGlassSize)
})

onUnmounted(() => {
  cleanupMouseTracking()
  resizeObserver?.disconnect()
  window.removeEventListener('resize', updateGlassSize)
})
</script>

<template>
  <div
    ref="glassRef"
    class="liquid-glass"
    :class="{ 'liquid-glass--clickable': overLight }"
    :style="baseStyle"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
    @mousedown="isActive = true"
    @mouseup="isActive = false"
    @click="emit('click', $event)"
  >
    <div
      class="liquid-glass__shade"
      :style="{
        opacity: overLight ? 0.2 : 0,
        borderRadius: `${cornerRadius}px`,
      }"
    />
    <div
      class="liquid-glass__shade liquid-glass__shade--overlay"
      :style="{
        opacity: overLight ? 1 : 0,
        borderRadius: `${cornerRadius}px`,
      }"
    />

    <svg class="liquid-glass__filter" :style="{ width: `${glassSize.width}px`, height: `${glassSize.height}px` }" aria-hidden="true">
      <defs>
        <radialGradient :id="`${filterId}-edge-mask`" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stop-color="black" stop-opacity="0" />
          <stop :offset="`${Math.max(30, 80 - aberrationIntensity * 2)}%`" stop-color="black" stop-opacity="0" />
          <stop offset="100%" stop-color="white" stop-opacity="1" />
        </radialGradient>
        <filter :id="filterId" x="-35%" y="-35%" width="170%" height="170%" color-interpolation-filters="sRGB">
          <feImage
            x="0" y="0" width="100%" height="100%"
            result="DISPLACEMENT_MAP"
            :href="displacementMapUrl"
            preserveAspectRatio="xMidYMid slice"
          />

          <feColorMatrix
            in="DISPLACEMENT_MAP"
            type="matrix"
            values="0.3 0.3 0.3 0 0
                    0.3 0.3 0.3 0 0
                    0.3 0.3 0.3 0 0
                    0 0 0 1 0"
            result="EDGE_INTENSITY"
          />
          <feComponentTransfer in="EDGE_INTENSITY" result="EDGE_MASK">
            <feFuncA type="discrete" :tableValues="`0 ${aberrationIntensity * 0.05} 1`" />
          </feComponentTransfer>

          <feOffset in="SourceGraphic" dx="0" dy="0" result="CENTER_ORIGINAL" />

          <feDisplacementMap
            in="SourceGraphic"
            in2="DISPLACEMENT_MAP"
            :scale="displacementScale * (mode === 'shader' ? 1 : -1)"
            xChannelSelector="R"
            yChannelSelector="B"
            result="RED_DISPLACED"
          />
          <feColorMatrix
            in="RED_DISPLACED"
            type="matrix"
            values="1 0 0 0 0
                    0 0 0 0 0
                    0 0 0 0 0
                    0 0 0 1 0"
            result="RED_CHANNEL"
          />

          <feDisplacementMap
            in="SourceGraphic"
            in2="DISPLACEMENT_MAP"
            :scale="displacementScale * ((mode === 'shader' ? 1 : -1) - aberrationIntensity * 0.05)"
            xChannelSelector="R"
            yChannelSelector="B"
            result="GREEN_DISPLACED"
          />
          <feColorMatrix
            in="GREEN_DISPLACED"
            type="matrix"
            values="0 0 0 0 0
                    0 1 0 0 0
                    0 0 0 0 0
                    0 0 0 1 0"
            result="GREEN_CHANNEL"
          />

          <feDisplacementMap
            in="SourceGraphic"
            in2="DISPLACEMENT_MAP"
            :scale="displacementScale * ((mode === 'shader' ? 1 : -1) - aberrationIntensity * 0.1)"
            xChannelSelector="R"
            yChannelSelector="B"
            result="BLUE_DISPLACED"
          />
          <feColorMatrix
            in="BLUE_DISPLACED"
            type="matrix"
            values="0 0 0 0 0
                    0 0 0 0 0
                    0 0 1 0 0
                    0 0 0 1 0"
            result="BLUE_CHANNEL"
          />

          <feBlend in="GREEN_CHANNEL" in2="BLUE_CHANNEL" mode="screen" result="GB_COMBINED" />
          <feBlend in="RED_CHANNEL" in2="GB_COMBINED" mode="screen" result="RGB_COMBINED" />

          <feGaussianBlur
            in="RGB_COMBINED"
            :stdDeviation="Math.max(0.1, 0.5 - aberrationIntensity * 0.1)"
            result="ABERRATED_BLURRED"
          />

          <feComposite in="ABERRATED_BLURRED" in2="EDGE_MASK" operator="in" result="EDGE_ABERRATION" />

          <feComponentTransfer in="EDGE_MASK" result="INVERTED_MASK">
            <feFuncA type="table" tableValues="1 0" />
          </feComponentTransfer>
          <feComposite in="CENTER_ORIGINAL" in2="INVERTED_MASK" operator="in" result="CENTER_CLEAN" />

          <feComposite in="EDGE_ABERRATION" in2="CENTER_CLEAN" operator="over" />
        </filter>
      </defs>
    </svg>

    <div class="glass" :style="glassBodyStyle">
      <span
        class="glass__warp"
        :style="{
          ...backdropFilterStyle,
          position: 'absolute',
          inset: '0',
        }"
      />

      <div class="liquid-glass__content" :style="[defaultContentStyle, contentStyle]">
        <slot />
      </div>
    </div>

    <span
      class="liquid-glass__border liquid-glass__border--screen"
      :style="{
        borderRadius: `${cornerRadius}px`,
        background: borderGradient1,
      }"
    />

    <span
      class="liquid-glass__border liquid-glass__border--overlay"
      :style="{
        borderRadius: `${cornerRadius}px`,
        background: borderGradient2,
      }"
    />

    <div
      class="liquid-glass__highlight liquid-glass__highlight--soft"
      :style="{
        borderRadius: `${cornerRadius}px`,
        opacity: isHovered || isActive ? 0.5 : 0,
      }"
    />
    <div
      class="liquid-glass__highlight liquid-glass__highlight--active"
      :style="{
        borderRadius: `${cornerRadius}px`,
        opacity: isActive ? 0.5 : 0,
      }"
    />
    <div
      class="liquid-glass__highlight liquid-glass__highlight--hover"
      :style="{
        borderRadius: `${cornerRadius}px`,
        opacity: isHovered ? 0.4 : isActive ? 0.8 : 0,
      }"
    />
  </div>
</template>

<style scoped>
.liquid-glass {
  position: relative;
  display: inline-flex;
  isolation: isolate;
  vertical-align: middle;
}

.liquid-glass--clickable {
  cursor: pointer;
}

.liquid-glass__shade,
.liquid-glass__filter,
.liquid-glass__border,
.liquid-glass__highlight {
  position: absolute;
  inset: 0;
  pointer-events: none;
  transition: all 0.2s ease-out;
}

.liquid-glass__shade {
  z-index: 0;
  background: #000;
}

.liquid-glass__shade--overlay {
  mix-blend-mode: overlay;
}

.liquid-glass__filter {
  z-index: 0;
  overflow: visible;
}

.glass {
  position: relative;
  isolation: isolate;
  z-index: 1;
  width: 100%;
}

.glass__warp {
  border-radius: inherit;
  pointer-events: none;
}

.liquid-glass__border {
  z-index: 2;
  padding: 1.5px;
  box-shadow: 0 0 0 0.5px rgba(255, 255, 255, 0.5) inset, 0 1px 3px rgba(255, 255, 255, 0.25) inset, 0 1px 4px rgba(0, 0, 0, 0.35);
  -webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
}

.liquid-glass__border--screen {
  opacity: 0.2;
  mix-blend-mode: screen;
}

.liquid-glass__border--overlay {
  mix-blend-mode: overlay;
}

.liquid-glass__highlight {
  z-index: 3;
  mix-blend-mode: overlay;
}

.liquid-glass__highlight--soft,
.liquid-glass__highlight--active {
  background-image: radial-gradient(circle at 50% 0%, rgba(255, 255, 255, 0.5) 0%, rgba(255, 255, 255, 0) 50%);
}

.liquid-glass__highlight--active {
  background-image: radial-gradient(circle at 50% 0%, rgba(255, 255, 255, 1) 0%, rgba(255, 255, 255, 0) 80%);
}

.liquid-glass__highlight--hover {
  background-image: radial-gradient(circle at 50% 0%, rgba(255, 255, 255, 1) 0%, rgba(255, 255, 255, 0) 100%);
}

.liquid-glass__content {
  box-sizing: border-box;
  width: 100%;
}
</style>
