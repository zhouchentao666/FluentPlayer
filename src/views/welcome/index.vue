<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import * as THREE from 'three'

const router = useRouter()
const { t } = useI18n()
const containerRef = ref<HTMLDivElement>()
const countdown = ref(0)
const showSkip = ref(false)

interface SceneProfile {
  isMobile: boolean
  textScale: number
  textParticleStep: number
  bgParticleCount: number
  bgSpread: number
  bokehCount: number
  bokehSpread: number
  bokehMinScale: number
  bokehMaxScale: number
  startCameraZ: number
  targetCameraZ: number
  pixelRatioLimit: number
  textParticleSize: number
  bgParticleSize: number
}

let renderer: THREE.WebGLRenderer | null = null
let particleTexture: THREE.CanvasTexture | null = null
let animationId = 0
let clock: THREE.Clock
let convergeProgress = 0
let textData: { targetPos: number[]; startPos: number[]; pColors: number[] }
let textParticles: THREE.Points
let bgParticles: THREE.Points
let bokehGroup: THREE.Group
let camera: THREE.PerspectiveCamera
let sceneProfile: SceneProfile
let textGeometryScale = 1

// 倒计时控制
let countdownTimer: ReturnType<typeof setInterval> | null = null
const COUNTDOWN_SECONDS = 3

function getSceneProfile(width: number, height: number): SceneProfile {
  const shortSide = Math.min(width, height)
  const isMobile = width <= 768
  const narrowScale = Math.min(1, Math.max(0.72, shortSide / 430))

  if (!isMobile) {
    return {
      isMobile,
      textScale: 0.08,
      textParticleStep: 2,
      bgParticleCount: 800,
      bgSpread: 80,
      bokehCount: 15,
      bokehSpread: 60,
      bokehMinScale: 5,
      bokehMaxScale: 20,
      startCameraZ: 45,
      targetCameraZ: 22,
      pixelRatioLimit: 2,
      textParticleSize: 0.2,
      bgParticleSize: 0.4
    }
  }

  return {
    isMobile,
    textScale: 0.04 * narrowScale,
    textParticleStep: shortSide <= 360 ? 3 : 2,
    bgParticleCount: shortSide <= 360 ? 360 : 480,
    bgSpread: 58,
    bokehCount: 9,
    bokehSpread: 42,
    bokehMinScale: 4,
    bokehMaxScale: 12,
    startCameraZ: 52,
    targetCameraZ: 30,
    pixelRatioLimit: 1.5,
    textParticleSize: shortSide <= 360 ? 0.18 : 0.19,
    bgParticleSize: 0.34
  }
}

function getTextObjectScale(profile: SceneProfile): number {
  return profile.isMobile ? profile.textScale / textGeometryScale : 1
}

function startCountdown() {
  showSkip.value = true
  countdown.value = COUNTDOWN_SECONDS
  countdownTimer = setInterval(() => {
    countdown.value--
    if (countdown.value <= 0) {
      clearCountdown()
      router.replace('/home')
    }
  }, 1000)
}

function clearCountdown() {
  if (countdownTimer) {
    clearInterval(countdownTimer)
    countdownTimer = null
  }
}

function skipToHome() {
  clearCountdown()
  router.replace('/home')
}

function createCircleTexture(): THREE.CanvasTexture {
  const canvas = document.createElement('canvas')
  canvas.width = 64
  canvas.height = 64
  const ctx = canvas.getContext('2d')!
  const gradient = ctx.createRadialGradient(32, 32, 0, 32, 32, 32)
  gradient.addColorStop(0, 'rgba(255,255,255,1)')
  gradient.addColorStop(0.3, 'rgba(255,255,255,0.8)')
  gradient.addColorStop(1, 'rgba(255,255,255,0)')
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, 64, 64)
  return new THREE.CanvasTexture(canvas)
}

function getTextParticleData(
  text: string,
  profile: SceneProfile
): { targetPos: number[]; startPos: number[]; pColors: number[] } {
  const tCanvas = document.createElement('canvas')
  const tWidth = 600
  const tHeight = 300
  tCanvas.width = tWidth
  tCanvas.height = tHeight
  const ctx = tCanvas.getContext('2d', { willReadFrequently: true })!

  ctx.fillStyle = '#ffffff'
  ctx.font = '900 160px Arial, sans-serif'
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.fillText(text, tWidth / 2, tHeight / 2)

  const imgData = ctx.getImageData(0, 0, tWidth, tHeight).data
  const targetPos: number[] = []
  const startPos: number[] = []
  const pColors: number[] = []

  const textColorCore = new THREE.Color('#1a1a1a')
  const textColorHighlight = new THREE.Color('#00d26a')
  const step = profile.textParticleStep

  for (let y = 0; y < tHeight; y += step) {
    for (let x = 0; x < tWidth; x += step) {
      const index = (y * tWidth + x) * 4
      if (imgData[index + 3] > 128) {
        const px = (x - tWidth / 2) * profile.textScale
        const py = -(y - tHeight / 2) * profile.textScale
        const pz = (Math.random() - 0.5) * 2

        targetPos.push(px, py, pz)

        startPos.push(
          px + (Math.random() - 0.5) * profile.bgSpread,
          py + (Math.random() - 0.5) * profile.bgSpread,
          pz + (Math.random() - 0.5) * profile.bgSpread
        )

        const c = Math.random() > 0.4 ? textColorHighlight : textColorCore
        pColors.push(c.r, c.g, c.b)
      }
    }
  }
  return { targetPos, startPos, pColors }
}

function createScene(width: number, height: number) {
  sceneProfile = getSceneProfile(width, height)
  const scene = new THREE.Scene()

  renderer = new THREE.WebGLRenderer({
    alpha: true,
    antialias: true,
    powerPreference: 'high-performance'
  })
  renderer.setSize(width, height)
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, sceneProfile.pixelRatioLimit))

  camera = new THREE.PerspectiveCamera(50, width / height, 0.1, 1000)
  camera.position.z = sceneProfile.startCameraZ

  particleTexture = createCircleTexture()

  // 背景粒子
  const bgParticleCount = sceneProfile.bgParticleCount
  const bgGeometry = new THREE.BufferGeometry()
  const bgPositions = new Float32Array(bgParticleCount * 3)
  const bgColors = new Float32Array(bgParticleCount * 3)

  const colorBrand = new THREE.Color('#00d26a')
  const colorLight = new THREE.Color('#a3e8b8')
  const colorGray = new THREE.Color('#e0e8e4')

  for (let i = 0; i < bgParticleCount; i++) {
    bgPositions[i * 3] = (Math.random() - 0.5) * sceneProfile.bgSpread
    bgPositions[i * 3 + 1] = (Math.random() - 0.5) * sceneProfile.bgSpread
    bgPositions[i * 3 + 2] = (Math.random() - 0.5) * sceneProfile.bgSpread

    const randColor = Math.random()
    let c = colorGray
    if (randColor > 0.7) c = colorLight
    if (randColor > 0.9) c = colorBrand

    bgColors[i * 3] = c.r
    bgColors[i * 3 + 1] = c.g
    bgColors[i * 3 + 2] = c.b
  }

  bgGeometry.setAttribute('position', new THREE.BufferAttribute(bgPositions, 3))
  bgGeometry.setAttribute('color', new THREE.BufferAttribute(bgColors, 3))

  const bgMaterial = new THREE.PointsMaterial({
    size: sceneProfile.bgParticleSize,
    vertexColors: true,
    map: particleTexture,
    transparent: true,
    opacity: sceneProfile.isMobile ? 0.42 : 0.5,
    blending: THREE.NormalBlending,
    depthWrite: false
  })
  bgParticles = new THREE.Points(bgGeometry, bgMaterial)
  scene.add(bgParticles)

  // 前景散景
  bokehGroup = new THREE.Group()
  for (let i = 0; i < sceneProfile.bokehCount; i++) {
    const mat = new THREE.SpriteMaterial({
      map: particleTexture,
      transparent: true,
      opacity: Math.random() * 0.2 + 0.05,
      blending: THREE.NormalBlending,
      color: Math.random() > 0.5 ? 0x00d26a : 0x00e88a
    })
    const sprite = new THREE.Sprite(mat)
    sprite.position.set(
      (Math.random() - 0.5) * sceneProfile.bokehSpread,
      (Math.random() - 0.5) * sceneProfile.bokehSpread,
      15 + Math.random() * 20
    )
    const scale =
      Math.random() * (sceneProfile.bokehMaxScale - sceneProfile.bokehMinScale) +
      sceneProfile.bokehMinScale
    sprite.scale.set(scale, scale, 1)
    sprite.userData = { vy: Math.random() * 0.03 + 0.01, rx: Math.random() * 0.02 }
    bokehGroup.add(sprite)
  }
  scene.add(bokehGroup)

  // 粒子文字
  textData = getTextParticleData('Mio', sceneProfile)
  textGeometryScale = sceneProfile.textScale
  const textGeometry = new THREE.BufferGeometry()
  textGeometry.setAttribute(
    'position',
    new THREE.BufferAttribute(new Float32Array(textData.startPos), 3)
  )
  textGeometry.setAttribute(
    'color',
    new THREE.BufferAttribute(new Float32Array(textData.pColors), 3)
  )

  const textMaterial = new THREE.PointsMaterial({
    size: sceneProfile.textParticleSize,
    vertexColors: true,
    map: particleTexture,
    transparent: true,
    opacity: 0.9,
    blending: THREE.NormalBlending,
    depthWrite: false
  })

  textParticles = new THREE.Points(textGeometry, textMaterial)
  textParticles.scale.setScalar(getTextObjectScale(sceneProfile))
  textParticles.position.y = sceneProfile.isMobile ? 1.6 : 1
  scene.add(textParticles)

  clock = new THREE.Clock()
  convergeProgress = 0

  let countdownStarted = false

  function animate() {
    animationId = requestAnimationFrame(animate)
    const delta = clock.getDelta()
    const elapsedTime = clock.getElapsedTime()

    bgParticles.rotation.y = elapsedTime * 0.05
    bokehGroup.children.forEach((sprite) => {
      const ud = sprite.userData
      sprite.position.y += ud.vy
      sprite.position.x += Math.sin(elapsedTime + sprite.position.y) * ud.rx
      if (sprite.position.y > 40) sprite.position.y = -40
    })

    convergeProgress += (1 - convergeProgress) * delta * 1.5

    const textPosArray = textParticles.geometry.attributes.position.array
    for (let i = 0; i < textData.targetPos.length / 3; i++) {
      const ix = i * 3
      const iy = i * 3 + 1
      const iz = i * 3 + 2

      textPosArray[ix] = THREE.MathUtils.lerp(
        textPosArray[ix],
        textData.targetPos[ix],
        convergeProgress * 0.03
      )
      textPosArray[iy] = THREE.MathUtils.lerp(
        textPosArray[iy],
        textData.targetPos[iy],
        convergeProgress * 0.03
      )
      textPosArray[iz] = THREE.MathUtils.lerp(
        textPosArray[iz],
        textData.targetPos[iz],
        convergeProgress * 0.03
      )

      if (convergeProgress > 0.9) {
        textPosArray[ix] += Math.sin(elapsedTime * 1.5 + iy) * 0.001
        textPosArray[iy] += Math.cos(elapsedTime * 1.5 + ix) * 0.001
      }
    }
    textParticles.geometry.attributes.position.needsUpdate = true

    const textYOffset = sceneProfile.isMobile ? 1.6 : 1
    textParticles.scale.setScalar(getTextObjectScale(sceneProfile))
    textParticles.position.y = textYOffset + Math.sin(elapsedTime * 1.2) * 0.15

    camera.position.z = THREE.MathUtils.lerp(
      camera.position.z,
      sceneProfile.targetCameraZ + Math.sin(elapsedTime * 0.3) * 1.0,
      0.02
    )
    camera.lookAt(0, 0, 0)

    renderer!.render(scene, camera)

    // 动画完成后启动倒计时
    if (!countdownStarted && convergeProgress > 0.95) {
      countdownStarted = true
      startCountdown()
    }
  }

  animate()
}

function handleResize() {
  if (!renderer || !camera || !containerRef.value) return
  const width = containerRef.value.clientWidth
  const height = containerRef.value.clientHeight
  sceneProfile = getSceneProfile(width, height)
  camera.aspect = width / height
  camera.position.z = Math.max(camera.position.z, sceneProfile.targetCameraZ)
  camera.updateProjectionMatrix()
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, sceneProfile.pixelRatioLimit))
  renderer.setSize(width, height)
  if (textParticles) {
    textParticles.scale.setScalar(getTextObjectScale(sceneProfile))
    ;(textParticles.material as THREE.PointsMaterial).size = sceneProfile.textParticleSize
  }
  if (bgParticles) {
    ;(bgParticles.material as THREE.PointsMaterial).size = sceneProfile.bgParticleSize
  }
}

function cleanup() {
  clearCountdown()
  if (animationId) {
    cancelAnimationFrame(animationId)
    animationId = 0
  }
  if (renderer) {
    renderer.domElement.remove()
    renderer.dispose()
    renderer = null
  }
  if (particleTexture) {
    particleTexture.dispose()
    particleTexture = null
  }
  // Dispose geometries and materials
  if (bgParticles) {
    bgParticles.geometry.dispose()
    ;(bgParticles.material as THREE.Material).dispose()
  }
  if (textParticles) {
    textParticles.geometry.dispose()
    ;(textParticles.material as THREE.Material).dispose()
  }
  if (bokehGroup) {
    bokehGroup.children.forEach((child) => {
      if (child instanceof THREE.Sprite) {
        child.material.dispose()
      }
    })
  }
  window.removeEventListener('resize', handleResize)
}

onMounted(() => {
  if (!containerRef.value) return
  const width = containerRef.value.clientWidth
  const height = containerRef.value.clientHeight
  createScene(width, height)
  containerRef.value.appendChild(renderer!.domElement)
  window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
  cleanup()
})
</script>

<template>
  <div class="splash-container">
    <div ref="containerRef" class="webgl-canvas"></div>
    <div class="ui-layer">
      <div class="equalizer">
        <div class="bar"></div>
        <div class="bar"></div>
        <div class="bar"></div>
        <div class="bar"></div>
        <div class="bar"></div>
      </div>
      <transition name="fade">
        <button v-if="showSkip" class="skip-section" type="button" @click="skipToHome">
          <span class="skip-text">{{ t('music.welcome.skip') }} <span class="countdown-num">{{ countdown }}s</span></span>
        </button>
      </transition>
    </div>
  </div>
</template>

<style scoped>
.splash-container {
  width: 100vw;
  max-width: 100vw;
  height: 100dvh;
  min-height: 100dvh;
  position: relative;
  overflow: hidden;
  background: radial-gradient(circle at center, #ffffff 0%, #eef3f0 100%);
  box-sizing: border-box;
  overscroll-behavior: none;
}

.webgl-canvas {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 1;
  overflow: hidden;
  touch-action: none;
}

.webgl-canvas :deep(canvas) {
  display: block;
  width: 100% !important;
  height: 100% !important;
}

.ui-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 2;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  align-items: center;
  pointer-events: none;
  padding-bottom: calc(60px + env(safe-area-inset-bottom, 0px));
  box-sizing: border-box;
}

.equalizer {
  display: flex;
  gap: 6px;
  height: 30px;
  align-items: flex-end;
  opacity: 0;
  animation: fadeIn 1s ease-out 2.8s forwards;
}

.bar {
  width: 5px;
  background: linear-gradient(to top, #00d26a, #00d28a);
  border-radius: 3px;
  box-shadow: 0 4px 8px rgba(0, 210, 106, 0.2);
  will-change: transform; animation: eqPulse 1s ease-in-out infinite alternate;
  transform-origin: bottom;
}

.bar:nth-child(1) { height: 15px; animation-duration: 0.7s; }
.bar:nth-child(2) { height: 25px; animation-duration: 0.5s; animation-delay: 0.2s; }
.bar:nth-child(3) { height: 10px; animation-duration: 0.8s; animation-delay: 0.4s; }
.bar:nth-child(4) { height: 30px; animation-duration: 0.6s; animation-delay: 0.1s; }
.bar:nth-child(5) { height: 20px; animation-duration: 0.9s; animation-delay: 0.5s; }

@keyframes fadeIn {
  to { opacity: 1; }
}

@keyframes eqPulse {
  0% { transform: scaleY(0.4); }
  100% { transform: scaleY(1.3); }
}

.skip-section {
  margin-top: 1.5rem;
  min-height: 44px;
  padding: 0 1.2rem;
  border: 0;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.06);
  backdrop-filter: blur(8px);
  cursor: pointer;
  pointer-events: auto;
  transition: background 0.2s;
  user-select: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  touch-action: manipulation;
  appearance: none;
  font: inherit;
}

.skip-section:hover {
  background: rgba(0, 0, 0, 0.12);
}

.skip-text {
  font-size: 0.85rem;
  color: #555;
  font-weight: 500;
  letter-spacing: 0.5px;
}

.countdown-num {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
  color: #00d26a;
}

.fade-enter-active {
  transition: opacity 0.6s ease;
}

.fade-enter-from {
  opacity: 0;
}

@media (max-width: 768px) {
  .splash-container {
    width: 100dvw;
    max-width: 100dvw;
  }

  .ui-layer {
    padding: 0 var(--mobile-page-gutter) calc(var(--mobile-safe-bottom) + 48px);
  }

  .equalizer {
    height: 26px;
    gap: 5px;
  }

  .bar {
    width: 4px;
  }

  .skip-section {
    min-height: var(--mobile-touch-target);
    padding: 0 18px;
    background: var(--mobile-glass-bg, rgba(255, 255, 255, 0.72));
    border: 0.5px solid var(--mobile-glass-border, rgba(0, 0, 0, 0.08));
    backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-backdrop-filter: saturate(var(--mobile-glass-saturate)) blur(var(--mobile-glass-blur));
    -webkit-tap-highlight-color: transparent;
  }

  .skip-section:active {
    background: color-mix(in srgb, var(--td-brand-color, #00d26a) 14%, var(--mobile-glass-bg, rgba(255, 255, 255, 0.72)));
  }

  .skip-text {
    font-size: 0.9rem;
  }
}

@media (max-width: 360px) {
  .ui-layer {
    padding-right: max(12px, env(safe-area-inset-right, 0px));
    padding-bottom: calc(var(--mobile-safe-bottom) + 36px);
    padding-left: max(12px, env(safe-area-inset-left, 0px));
  }

  .skip-section {
    margin-top: 1.1rem;
    padding: 0 16px;
  }
}

@media (prefers-reduced-motion: reduce) {
  .bar {
    animation: none;
  }
}

/* 暗色模式 */
@media (prefers-color-scheme: dark) {
  .splash-container {
    background: radial-gradient(circle at center, #1a1a1a 0%, #0d0d0d 100%);
  }

  .skip-section {
    background: rgba(255, 255, 255, 0.08);
  }

  .skip-section:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .skip-text {
    color: #aaa;
  }
}
</style>
