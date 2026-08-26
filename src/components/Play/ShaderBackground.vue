<script lang="ts" setup>
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue'
import { useGlobalPlayStatusStore } from '@/store/GlobalPlayStatus'
import DefaultCover from '@/assets/images/Default.jpg'
import CoverImage from '@/assets/images/cover.png'

const props = defineProps<{
  coverImage: string
}>()

const canvasRef = ref<HTMLCanvasElement | null>(null)
let gl: WebGLRenderingContext | null = null
let program: WebGLProgram | null = null
let vertexBuffer: WebGLBuffer | null = null
let vertexShader: WebGLShader | null = null
let fragmentShader: WebGLShader | null = null
let uTimeLocation: WebGLUniformLocation | null = null
let uColorLocation: WebGLUniformLocation | null = null
let animationFrameId: number | null = null
const startTime = Date.now()
const dominantColor = ref({ r: 0.3, g: 0.3, b: 0.5 })

const actualCoverImage = computed(() => {
  if (
    props.coverImage.includes('@assets/images/Default.jpg') ||
    props.coverImage.includes('@/assets/images/Default.jpg')
  ) {
    return DefaultCover
  } else if (
    props.coverImage.includes('@assets/images/cover.png') ||
    props.coverImage.includes('@/assets/images/cover.png')
  ) {
    return CoverImage
  }
  return props.coverImage
})

const globalPlayStatus = useGlobalPlayStatusStore()

watch(() => globalPlayStatus.player.coverDetail.ColorObject, (color) => {
  if (color) {
    dominantColor.value = {
      r: color.r / 255,
      g: color.g / 255,
      b: color.b / 255
    }
  }
}, { immediate: true })

// Vertex shader
const vertexShaderSource = `
  attribute vec2 a_position;
  varying vec2 v_texCoord;

  void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
    v_texCoord = a_position * 0.5 + 0.5;
  }
`

// Fragment shader - FBM fluid effect
const fragmentShaderSource = `
  precision highp float;
  varying vec2 v_texCoord;
  uniform float u_time;
  uniform vec3 u_color;

  float random(vec2 st) {
    return fract(sin(dot(st.xy, vec2(12.9898, 78.233))) * 43758.5453123);
  }

  float noise(vec2 st) {
    vec2 i = floor(st);
    vec2 f = fract(st);
    float a = random(i);
    float b = random(i + vec2(1.0, 0.0));
    float c = random(i + vec2(0.0, 1.0));
    float d = random(i + vec2(1.0, 1.0));
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(a, b, u.x) +
           (c - a) * u.y * (1.0 - u.x) +
           (d - b) * u.x * u.y;
  }

  float fbm(vec2 st) {
    float value = 0.0;
    float amplitude = 0.5;
    float frequency = 0.6;
    for (int i = 0; i < 4; i++) {
      value += amplitude * noise(st * frequency);
      frequency *= 1.8;
      amplitude *= 0.6;
    }
    return smoothstep(0.2, 0.8, value);
  }

  vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
  }

  void main() {
    vec2 st = v_texCoord;
    float time = u_time * 0.20;

    vec2 q = vec2(
      fbm(st + vec2(0.0, time * 0.3)),
      fbm(st + vec2(time * 0.2, 0.0))
    );

    vec2 r = vec2(
      fbm(st + 2.0 * q + vec2(1.7, 9.2) + time * 0.1),
      fbm(st + 2.0 * q + vec2(8.3, 2.8) + time * 0.08)
    );

    float f = fbm(st + r * 0.7);

    vec3 baseColor = u_color;
    float maxComp = max(max(baseColor.r, baseColor.g), baseColor.b);
    float minComp = min(min(baseColor.r, baseColor.g), baseColor.b);
    float delta = maxComp - minComp;

    float hue = 0.0;
    if (delta > 0.0) {
      if (maxComp == baseColor.r) {
        hue = (baseColor.g - baseColor.b) / delta + (baseColor.g < baseColor.b ? 6.0 : 0.0);
      } else if (maxComp == baseColor.g) {
        hue = (baseColor.b - baseColor.r) / delta + 2.0;
      } else {
        hue = (baseColor.r - baseColor.g) / delta + 4.0;
      }
      hue /= 6.0;
    }

    float saturation = maxComp == 0.0 ? 0.0 : delta / maxComp;
    float value = maxComp;

    saturation = min(saturation * 1.0, 1.0);
    value = min(value * 1.3, 1.0);

    vec3 color1 = hsv2rgb(vec3(hue, saturation * 0.9, min(value * 1.1, 1.0)));
    vec3 color2 = hsv2rgb(vec3(mod(hue + 0.05, 1.0), min(saturation * 1.3, 1.0), min(value * 1.2, 1.0)));
    vec3 color3 = hsv2rgb(vec3(mod(hue + 0.1, 1.0), min(saturation * 1.1, 1.0), min(value * 1.15, 1.0)));
    vec3 color4 = hsv2rgb(vec3(mod(hue - 0.05, 1.0), min(saturation * 1.2, 1.0), min(value * 1.25, 1.0)));

    float t1 = smoothstep(0.0, 1.0, f);
    float t2 = sin(f * 3.14) * 0.5 + 0.5;
    float t3 = cos(f * 2.0 + time * 0.5) * 0.5 + 0.5;
    float t4 = sin(f * 4.0 + time * 0.3) * 0.5 + 0.5;

    vec3 color5 = hsv2rgb(vec3(mod(hue + 0.15, 1.0), min(saturation * 1.4, 1.0), min(value * 1.3, 1.0)));
    vec3 color6 = hsv2rgb(vec3(mod(hue - 0.15, 1.0), min(saturation * 1.3, 1.0), min(value * 1.2, 1.0)));

    vec3 colorMix1 = mix(color1, color2, t1);
    vec3 colorMix2 = mix(color3, color4, t2);
    vec3 colorMix3 = mix(color5, color6, t4);

    vec3 color = mix(
      mix(colorMix1, colorMix2, t3),
      colorMix3,
      sin(f * 2.5 + time * 0.4) * 0.5 + 0.5
    );

    color += 0.15 * sin(f * 8.0 + time) * vec3(1.0);

    float ripple1 = sin(st.x * 12.0 + time * 0.8) * sin(st.y * 12.0 + time * 0.7) * 0.06;
    float ripple2 = sin(st.x * 8.0 - time * 0.6) * sin(st.y * 8.0 - time * 0.5) * 0.05;
    float ripple3 = sin(st.x * 15.0 + time * 0.4) * sin(st.y * 15.0 + time * 0.3) * 0.04;
    color += vec3(ripple1 + ripple2 + ripple3);

    float glow = smoothstep(0.3, 0.7, f);
    color = mix(color, vec3(1.0), glow * 0.12);

    float vignette = smoothstep(0.0, 0.7, 0.5 - length(st - 0.5));
    color = mix(color, color * 1.2, vignette * 0.3);

    vec2 pixel = vec2(1.0) / vec2(800.0, 600.0);
    float blur = 0.0;
    blur += f * 0.5;
    blur += fbm(st + pixel * vec2(1.0, 0.0)) * 0.125;
    blur += fbm(st + pixel * vec2(-1.0, 0.0)) * 0.125;
    blur += fbm(st + pixel * vec2(0.0, 1.0)) * 0.125;
    blur += fbm(st + pixel * vec2(0.0, -1.0)) * 0.125;
    color = mix(color, mix(color1, color4, 0.5), (blur - f) * 0.2);

    color = clamp(color, 0.0, 1.0);
    gl_FragColor = vec4(color, 1.0);
  }
`

function createShader(gl: WebGLRenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type)!
  gl.shaderSource(shader, source)
  gl.compileShader(shader)
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader)
    gl.deleteShader(shader)
    throw new Error('着色器编译错误: ' + info)
  }
  return shader
}

function createProgram(gl: WebGLRenderingContext, vs: WebGLShader, fs: WebGLShader): WebGLProgram {
  const prog = gl.createProgram()!
  gl.attachShader(prog, vs)
  gl.attachShader(prog, fs)
  gl.linkProgram(prog)
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(prog)
    gl.deleteProgram(prog)
    throw new Error('程序链接错误: ' + info)
  }
  return prog
}

function initWebGL() {
  if (!canvasRef.value) return
  gl = canvasRef.value.getContext('webgl')
  if (!gl) { console.error('无法初始化WebGL'); return }

  vertexShader = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource)
  fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource)
  program = createProgram(gl, vertexShader, fragmentShader)

  vertexBuffer = gl.createBuffer()
  gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer)
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW)

  const pos = gl.getAttribLocation(program, 'a_position')
  uTimeLocation = gl.getUniformLocation(program, 'u_time')
  uColorLocation = gl.getUniformLocation(program, 'u_color')
  gl.enableVertexAttribArray(pos)
  gl.vertexAttribPointer(pos, 2, gl.FLOAT, false, 0, 0)
  gl.useProgram(program)
  resizeCanvas()
  startRenderLoop()
}

function resizeCanvas() {
  if (!canvasRef.value || !gl) return
  const c = canvasRef.value
  const w = c.clientWidth, h = c.clientHeight
  if (c.width !== w || c.height !== h) {
    c.width = w
    c.height = h
    gl.viewport(0, 0, c.width, c.height)
  }
}

function render() {
  if (!gl || !program || !uTimeLocation || !uColorLocation) return
  const t = (Date.now() - startTime) / 1000
  gl.uniform1f(uTimeLocation, t)
  gl.uniform3f(
    uColorLocation,
    dominantColor.value.r,
    dominantColor.value.g,
    dominantColor.value.b
  )
  gl.clearColor(0, 0, 0, 0)
  gl.clear(gl.COLOR_BUFFER_BIT)
  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4)
  animationFrameId = requestAnimationFrame(render)
}

function startRenderLoop() {
  if (animationFrameId !== null) cancelAnimationFrame(animationFrameId)
  animationFrameId = requestAnimationFrame(render)
}

function stopRenderLoop() {
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId)
    animationFrameId = null
  }
}

onMounted(async () => {
  window.addEventListener('resize', resizeCanvas)
  initWebGL()
})

function disposeWebGL() {
  stopRenderLoop()
  if (gl) {
    if (vertexBuffer) { gl.deleteBuffer(vertexBuffer); vertexBuffer = null }
    if (program) { gl.deleteProgram(program); program = null }
    if (vertexShader) { gl.deleteShader(vertexShader); vertexShader = null }
    if (fragmentShader) { gl.deleteShader(fragmentShader); fragmentShader = null }
    uTimeLocation = null
    uColorLocation = null
    gl = null
  }
}

onBeforeUnmount(() => {
  window.removeEventListener('resize', resizeCanvas)
  disposeWebGL()
})
</script>

<template>
  <canvas ref="canvasRef" class="shader-background"></canvas>
</template>

<style scoped>
.shader-background {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: -1;
}
</style>
