import { computed, ref } from 'vue'

export function useCoverTilt(isExpanded: () => boolean) {
  const tiltRotateX = ref(0)
  const tiltRotateY = ref(0)
  const shineX = ref(50)
  const shineY = ref(50)
  const isHovering = ref(false)

  function handleMouseMove(e: MouseEvent, target: HTMLElement | null) {
    if (!target || !isExpanded()) return
    const rect = target.getBoundingClientRect()
    const x = e.clientX - rect.left
    const y = e.clientY - rect.top
    const centerX = rect.width / 2
    const centerY = rect.height / 2

    const maxRotate = 12
    tiltRotateY.value = ((x - centerX) / centerX) * maxRotate
    tiltRotateX.value = -((y - centerY) / centerY) * maxRotate

    shineX.value = (x / rect.width) * 100
    shineY.value = (y / rect.height) * 100
  }

  function handleMouseEnter() {
    isHovering.value = true
  }

  function handleMouseLeave() {
    isHovering.value = false
    tiltRotateX.value = 0
    tiltRotateY.value = 0
    shineX.value = 50
    shineY.value = 50
  }

  const coverTransform = computed(() => {
    if (!isExpanded() || !isHovering.value) return ''
    return `perspective(1000px) rotateX(${tiltRotateX.value}deg) rotateY(${tiltRotateY.value}deg) scale3d(1.02, 1.02, 1.02)`
  })

  const shadowTransform = computed(() => {
    if (!isExpanded() || !isHovering.value) return ''
    const x = -tiltRotateY.value * 1.6
    const y = tiltRotateX.value * 1.2
    const scaleX = 1 - Math.abs(tiltRotateY.value) * 0.008
    const scaleY = 1 - Math.abs(tiltRotateX.value) * 0.008
    return `translate(${x}px, ${y}px) scale(${scaleX}, ${scaleY})`
  })

  return {
    shineX,
    shineY,
    isHovering,
    coverTransform,
    shadowTransform,
    handleMouseMove,
    handleMouseEnter,
    handleMouseLeave,
  }
}
