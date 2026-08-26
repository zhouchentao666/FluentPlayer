<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import { Loading as TLoading } from 'tdesign-vue-next'
import { LocalUserDetailStore } from '@/store/LocalUserDetail'
import { storeToRefs } from 'pinia'
import { useSettingsStore } from '@/store/Settings'

const { t } = useI18n()

const userStore = LocalUserDetailStore()
const settingsStore = useSettingsStore()
const { userInfo } = storeToRefs(userStore)
const { settings } = storeToRefs(settingsStore)

const ball = ref<HTMLElement | null>(null)
const ballClass = ref('hidden-right')
const showAskWindow = ref(false)
const inputText = ref('')
const messages = ref<
  Array<{ id: number; type: 'user' | 'ai' | 'loading' | 'error'; content: string; html?: string }>
>([])
let nextMsgId = 0
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement | null>(null)
let timer: number | null = null

const isDragging = ref(false)
const hasDragged = ref(false)
const ballPosition = ref({ x: 0, y: 0 })
const isOnLeft = ref(false)
const dragOffset = ref({ x: 0, y: 0 })
const windowSize = ref({ width: 0, height: 0 })

const isFloatBallVisible = ref(settings.value.showFloatBall !== false)
const isHovering = ref(false)

const showBall = () => {
  ballClass.value = ''
  clearTimer()
}

const closeBall = (e: MouseEvent) => {
  e.stopPropagation()
  isFloatBallVisible.value = false
  settingsStore.updateSettings({ showFloatBall: false })
}

const handleMouseEnter = () => {
  isHovering.value = true
  showBall()
}

const handleMouseLeave = () => {
  isHovering.value = false
  startAutoHide()
}

const startAutoHide = () => {
  clearTimer()
  timer = window.setTimeout(() => {
    ballClass.value = isOnLeft.value ? 'hidden-left' : 'hidden-right'
  }, 3000)
}

const clearTimer = () => {
  if (timer) {
    clearTimeout(timer)
    timer = null
  }
}

const getTouchXY = (e: TouchEvent) => {
  const t = e.touches[0] || e.changedTouches[0]
  return { clientX: t.clientX, clientY: t.clientY }
}

const startDrag = (clientX: number, clientY: number) => {
  if (showAskWindow.value) return

  isDragging.value = true
  hasDragged.value = false
  clearTimer()

  const rect = ball.value?.getBoundingClientRect()
  if (rect) {
    dragOffset.value = {
      x: clientX - rect.left,
      y: clientY - rect.top
    }
  }
}

const moveDrag = (clientX: number, clientY: number) => {
  if (!isDragging.value) return
  hasDragged.value = true

  const x = clientX - dragOffset.value.x
  const y = clientY - dragOffset.value.y

  const maxX = windowSize.value.width - 120
  const maxY = windowSize.value.height - 176
  const minY = 90

  ballPosition.value = {
    x: Math.max(0, Math.min(x, maxX)),
    y: Math.max(minY, Math.min(y, maxY))
  }
}

const endDrag = () => {
  if (!isDragging.value) return

  isDragging.value = false

  if (hasDragged.value) {
    const centerX = ballPosition.value.x + 60
    const screenCenter = windowSize.value.width / 2

    if (centerX < screenCenter) {
      ballPosition.value.x = 6
      isOnLeft.value = true
      ballClass.value = 'hidden-left'
    } else {
      ballPosition.value.x = windowSize.value.width - 106
      isOnLeft.value = false
      ballClass.value = 'hidden-right'
    }
    saveBallPosition()
    clearTimer()
    startAutoHide()
  }
}

const handleMouseDown = (e: MouseEvent) => {
  startDrag(e.clientX, e.clientY)
  document.addEventListener('mousemove', handleMouseMove)
  document.addEventListener('mouseup', handleMouseUp)
  e.preventDefault()
}

const handleMouseMove = (e: MouseEvent) => {
  moveDrag(e.clientX, e.clientY)
}

const handleMouseUp = () => {
  document.removeEventListener('mousemove', handleMouseMove)
  document.removeEventListener('mouseup', handleMouseUp)
  endDrag()
}

const handleTouchStart = (e: TouchEvent) => {
  const { clientX, clientY } = getTouchXY(e)
  startDrag(clientX, clientY)
  document.addEventListener('touchmove', handleTouchMove, { passive: false })
  document.addEventListener('touchend', handleTouchEnd)
  document.addEventListener('touchcancel', handleTouchEnd)
}

const handleTouchMove = (e: TouchEvent) => {
  e.preventDefault()
  const { clientX, clientY } = getTouchXY(e)
  moveDrag(clientX, clientY)
}

const handleTouchEnd = () => {
  document.removeEventListener('touchmove', handleTouchMove)
  document.removeEventListener('touchend', handleTouchEnd)
  document.removeEventListener('touchcancel', handleTouchEnd)
  endDrag()
}

const checkAPIKey = async (): Promise<boolean> => {
  if (!userInfo.value.deepseekAPIkey) {
    const errorMessage = t('ai.configRequired')
    messages.value.push({
      id: nextMsgId++,
      type: 'error',
      content: errorMessage,
      html: DOMPurify.sanitize(await marked(errorMessage))
    })
    return false
  }
  clearErrorMessages()
  return true
}

const createWelcomeMessage = () => t('ai.greeting')

const clearErrorMessages = () => {
  messages.value = messages.value.filter((msg) => msg.type !== 'error')
}

const handleBallClick = async (e?: MouseEvent | TouchEvent) => {
  if (hasDragged.value) {
    hasDragged.value = false
    e?.preventDefault()
    return
  }

  clearTimer()
  showAskWindow.value = true

  if (!(await checkAPIKey())) {
    return
  }

  if (messages.value.length === 0) {
    const welcomeContent = createWelcomeMessage()
    messages.value.push({
      id: nextMsgId++,
      type: 'ai',
      content: welcomeContent,
      html: DOMPurify.sanitize(await marked(welcomeContent))
    })
  }
}

const closeAskWindow = () => {
  showAskWindow.value = false
  startAutoHide()
}

const generateStreamId = () => {
  return 'stream_' + Date.now() + '_' + Math.random().toString(36).substring(2, 9)
}

const sendMessage = async () => {
  if (!inputText.value.trim() || isLoading.value) return

  if (!(await checkAPIKey())) {
    return
  }

  const userMessage = inputText.value
  inputText.value = ''
  isLoading.value = true

  messages.value.push({
    id: nextMsgId++,
    type: 'user',
    content: userMessage,
    html: DOMPurify.sanitize(await marked(userMessage))
  })
  scrollToBottom()

  const aiMessageIndex = messages.value.length
  messages.value.push({
    id: nextMsgId++,
    type: 'loading',
    content: t('ai.thinking'),
    html: ''
  })
  scrollToBottom()

  const streamId = generateStreamId()
  let aiContent = ''

  try {
    const handleStreamChunk = async (data: { streamId: string; chunk: string }) => {
      if (data.streamId === streamId) {
        aiContent += data.chunk
        const existingId = messages.value[aiMessageIndex]?.id ?? nextMsgId++
        messages.value[aiMessageIndex] = {
          id: existingId,
          type: 'ai',
          content: aiContent,
          html: DOMPurify.sanitize(await marked(aiContent))
        }
      }
    }

    const handleStreamEnd = (data: { streamId: string }) => {
      if (data.streamId === streamId) {
        isLoading.value = false
        ;(window as any).api?.ai?.removeStreamListeners?.()
      }
    }

    const handleStreamError = async (data: { streamId: string; error: string }) => {
      if (data.streamId === streamId) {
        console.error('AI流式响应错误:', data.error)
        if (!aiContent) {
          messages.value[aiMessageIndex] = {
            id: messages.value[aiMessageIndex]?.id ?? nextMsgId++,
            type: 'error',
            content: t('ai.sendFailed', { message: data.error }),
            html: DOMPurify.sanitize(await marked(t('ai.sendFailed', { message: data.error })))
          }
        }
        isLoading.value = false
        ;(window as any).api?.ai?.removeStreamListeners?.()
      }
    }

    ;(window as any).api?.ai?.onStreamChunk?.(handleStreamChunk)
    ;(window as any).api?.ai?.onStreamEnd?.(handleStreamEnd)
    ;(window as any).api?.ai?.onStreamError?.(handleStreamError)

    await (window as any).api?.ai?.askStream?.(userMessage, streamId)
  } catch (error: any) {
    console.error('AI流式API调用失败:', error)
    if (!aiContent) {
      const errorMessage = t('ai.sendFailed', { message: (error as Error).message || t('ai.unknownError') })
      messages.value[aiMessageIndex] = {
        id: messages.value[aiMessageIndex]?.id ?? nextMsgId++,
        type: 'error',
        content: errorMessage,
        html: DOMPurify.sanitize(await marked(errorMessage))
      }
    }
    isLoading.value = false
    ;(window as any).api?.ai?.removeStreamListeners?.()
  }

  scrollToBottom()
}

const scrollToBottom = () => {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

watch(
  () => userInfo.value.deepseekAPIkey,
  async (newKey, oldKey) => {
    if (!oldKey && newKey) {
      clearErrorMessages()
      if (showAskWindow.value && messages.value.length === 0) {
        const welcomeContent = createWelcomeMessage()
        messages.value.push({
          id: nextMsgId++,
          type: 'ai',
          content: welcomeContent,
          html: DOMPurify.sanitize(await marked(welcomeContent))
        })
        scrollToBottom()
      }
    }
  },
  { immediate: false }
)

watch(
  () => settings.value.showFloatBall,
  (newValue) => {
    isFloatBallVisible.value = newValue
  },
  { immediate: true }
)

const updateWindowSize = () => {
  windowSize.value = {
    width: window.innerWidth,
    height: window.innerHeight
  }
}

const saveBallPosition = () => {
  const positionData = {
    x: ballPosition.value.x,
    y: ballPosition.value.y,
    isOnLeft: isOnLeft.value
  }
  localStorage.setItem('floatBallPosition', JSON.stringify(positionData))
}

const loadBallPosition = () => {
  try {
    const savedPosition = localStorage.getItem('floatBallPosition')
    if (savedPosition) {
      const positionData = JSON.parse(savedPosition)
      ballPosition.value = {
        x: positionData.x,
        y: positionData.y
      }
      isOnLeft.value = positionData.isOnLeft
    } else {
      setDefaultPosition()
    }
  } catch (error) {
    console.error('加载悬浮球位置失败:', error)
    setDefaultPosition()
  }
}

const setDefaultPosition = () => {
  updateWindowSize()
  ballPosition.value = {
    x: windowSize.value.width - 126,
    y: windowSize.value.height - 176
  }
  isOnLeft.value = false
}

const initBallPosition = () => {
  updateWindowSize()
  loadBallPosition()
}

const handleResize = () => {
  updateWindowSize()
  const maxX = windowSize.value.width - 120
  const maxY = windowSize.value.height - 176
  const minY = 90

  if (!isOnLeft.value) {
    ballPosition.value.x = windowSize.value.width - 106
  }

  ballPosition.value.x = Math.max(0, Math.min(ballPosition.value.x, maxX))
  ballPosition.value.y = Math.max(minY, Math.min(ballPosition.value.y, maxY))
}

onMounted(() => {
  initBallPosition()
  startAutoHide()
  handleResize()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  clearTimer()
  document.removeEventListener('mousemove', handleMouseMove)
  document.removeEventListener('mouseup', handleMouseUp)
  document.removeEventListener('touchmove', handleTouchMove)
  document.removeEventListener('touchend', handleTouchEnd)
  document.removeEventListener('touchcancel', handleTouchEnd)
  window.removeEventListener('resize', handleResize)
  saveBallPosition()
})
</script>

<template>
  <div>
    <transition name="ball-fade" appear>
      <div
        v-show="!showAskWindow && isFloatBallVisible"
        ref="ball"
        class="float-ball-container"
        :class="{ dragging: isDragging }"
        :style="{
          left: ballPosition.x - 10 + 'px',
          top: ballPosition.y - 10 + 'px',
          right: 'auto',
          bottom: 'auto'
        }"
        @mouseenter="handleMouseEnter"
        @mouseleave="handleMouseLeave"
        @mousedown="handleMouseDown"
        @touchstart="handleTouchStart"
      >
        <div
          class="float-ball"
          :class="[ballClass, { hovering: isHovering }]"
          @click="handleBallClick"
        >
          <video autoplay muted loop src="../../assets/videos/AI.mp4" />
        </div>
        <div v-show="isHovering" class="close-ball-btn" @click="closeBall" @mousedown.stop>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
            <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </div>
      </div>
    </transition>

    <transition name="glass-fade" appear>
      <div v-show="showAskWindow" class="chat-overlay" @click.self="closeAskWindow">
        <div
          class="chat-panel"
          :class="{ 'on-left': isOnLeft }"
          :style="{
            left: isOnLeft ? ballPosition.x + 120 + 'px' : 'auto',
            right: isOnLeft ? 'auto' : windowSize.width - ballPosition.x + 20 + 'px',
            bottom: Math.max(20, 176) + 'px'
          }"
        >
          <div class="glass-border-glow"></div>
          <div class="glass-light-sweep"></div>
          <div class="glass-ambient"></div>

          <div class="chat-header">
            <div class="header-indicator"></div>
            <h3>{{ t('ai.title') }}</h3>
            <button class="close-btn" @click="closeAskWindow">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
          <div class="chat-content">
            <div ref="messagesContainer" class="chat-messages">
              <div
                v-for="message in messages"
                :key="message.id"
                class="message"
                :class="message.type"
              >
                <div v-if="message.type === 'loading'" class="message-content loading-content">
                  <t-loading size="small" />
                  <span class="loading-text">{{ message.content }}</span>
                </div>
                <div v-else class="message-content" v-html="message.html || DOMPurify.sanitize(message.content)"></div>
              </div>
            </div>
          </div>
          <div class="input-area">
            <input
              v-model="inputText"
              :placeholder="t('ai.inputPlaceholder')"
              class="message-input"
              @keyup.enter="sendMessage"
            />
            <button class="send-btn" :disabled="!inputText.trim() || isLoading" @click="sendMessage">
              <svg v-if="!isLoading" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="22" y1="2" x2="11" y2="13" /><polygon points="22 2 15 22 11 13 2 9 22 2" />
              </svg>
              <t-loading v-else size="small" />
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
/* ==================== Float Ball ==================== */
.float-ball-container {
  position: fixed;
  width: 120px;
  height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard),
              transform var(--motion-duration-quick) var(--motion-ease-standard);
  user-select: none;
  touch-action: none;
}

.float-ball {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  overflow: hidden;
  padding: 2px;
  background-image: linear-gradient(135deg, var(--td-brand-color), #ff6600);
  transition: transform var(--motion-duration-quick) var(--motion-ease-out);
  box-shadow: 0 4px 16px color-mix(in srgb, var(--td-brand-color) 30%, transparent);
}

.float-ball-container.dragging {
  transition: none;
  z-index: 10001;
}

.float-ball-container.dragging .float-ball {
  cursor: grabbing;
}

.float-ball.hidden-right {
  transform: translateX(66px);
}

.float-ball.hidden-left {
  transform: translateX(-66px);
}

video {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  overflow: hidden;
}

.close-ball-btn {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 32px;
  height: 32px;
  background: var(--td-error-color);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  z-index: 10002;
  color: #fff;
  transition: transform var(--motion-duration-instant) var(--motion-ease-out),
              background-color var(--motion-duration-instant) var(--motion-ease-standard);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--td-error-color) 35%, transparent);
}

.close-ball-btn:hover {
  background: var(--td-error-color-hover);
  transform: scale(1.1);
  box-shadow: 0 4px 12px color-mix(in srgb, var(--td-error-color) 45%, transparent);
}

.float-ball.hovering {
  transform: scale(1.05);
}

/* ==================== Chat Overlay ==================== */
.chat-overlay {
  position: fixed;
  inset: 0;
  z-index: 10001;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ==================== Chat Panel (Liquid Glass) ==================== */
.chat-panel {
  position: fixed;
  width: 420px;
  max-height: min(80vh, 560px);
  border-radius: 22px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  z-index: 1;

  background: linear-gradient(
    165deg,
    color-mix(in srgb, var(--td-bg-color-container) 78%, transparent) 0%,
    color-mix(in srgb, var(--td-bg-color-container) 62%, transparent) 35%,
    color-mix(in srgb, var(--td-bg-color-container) 70%, transparent) 100%
  );
  backdrop-filter: blur(var(--glass-blur-panel)) saturate(200%);
  -webkit-backdrop-filter: blur(var(--glass-blur-panel)) saturate(200%);

  border: 1.5px solid color-mix(in srgb, var(--td-text-color-primary) 18%, transparent);
  box-shadow:
    var(--glass-shadow-panel),
    inset 0 2px 0 color-mix(in srgb, var(--td-text-color-primary) 12%, transparent),
    inset 0 -1px 0 color-mix(in srgb, var(--td-text-color-primary) 4%, transparent);
}

.chat-panel.on-left {
  transform-origin: left bottom;
}

.chat-panel:not(.on-left) {
  transform-origin: right bottom;
}

/* ==================== Glass Decorations ==================== */
.glass-border-glow {
  position: absolute;
  inset: 0;
  border-radius: 22px;
  padding: 1.5px;
  background: conic-gradient(
    from var(--border-angle, 0deg),
    transparent 0%,
    rgba(120, 180, 255, 0.45) 7%,
    rgba(180, 120, 255, 0.35) 14%,
    rgba(255, 120, 180, 0.3) 22%,
    transparent 34%,
    transparent 66%,
    rgba(120, 255, 200, 0.35) 74%,
    rgba(180, 255, 120, 0.3) 82%,
    rgba(120, 180, 255, 0.45) 94%,
    transparent 100%
  );
  mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  mask-composite: exclude;
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  animation: rotate-border 12s linear infinite;
  pointer-events: none;
  z-index: 0;

  @property --border-angle {
    syntax: '<angle>';
    initial-value: 0deg;
    inherits: false;
  }

  @keyframes rotate-border {
    to { --border-angle: 360deg; }
  }
}

.glass-light-sweep {
  position: absolute;
  top: 0;
  left: 0;
  width: 55%;
  height: 100%;
  transform: translateX(-120%);
  background: linear-gradient(
    108deg,
    transparent 35%,
    color-mix(in srgb, var(--td-text-color-primary) 6%, transparent) 44%,
    color-mix(in srgb, var(--td-text-color-primary) 10%, transparent) 50%,
    color-mix(in srgb, var(--td-text-color-primary) 6%, transparent) 56%,
    transparent 65%
  );
  animation: light-sweep 9s ease-in-out infinite;
  pointer-events: none;
  z-index: 0;
  border-radius: 22px;

  @keyframes light-sweep {
    0%, 100% { transform: translateX(-120%); }
    50% { transform: translateX(320%); }
  }
}

.glass-ambient {
  position: absolute;
  top: -30%;
  right: -20%;
  width: 70%;
  height: 70%;
  background: radial-gradient(ellipse, color-mix(in srgb, var(--td-brand-color) 10%, transparent) 0%, transparent 70%);
  pointer-events: none;
  z-index: 0;
}

/* ==================== Chat Header ==================== */
.chat-header {
  position: relative;
  z-index: 1;
  padding: 18px 20px 14px;
  display: flex;
  align-items: center;
  gap: 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
}

.header-indicator {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--td-brand-color);
  box-shadow: 0 0 8px color-mix(in srgb, var(--td-brand-color) 40%, transparent);
  flex-shrink: 0;
}

.chat-header h3 {
  margin: 0;
  flex: 1;
  font-size: 16px;
  font-weight: 600;
  color: var(--td-text-color-primary);
}

.close-btn {
  background: none;
  border: none;
  color: var(--td-text-color-secondary);
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  transition: background-color var(--motion-duration-instant) var(--motion-ease-standard),
              color var(--motion-duration-instant) var(--motion-ease-standard);
}

.close-btn:hover {
  background-color: color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  color: var(--td-text-color-primary);
}

/* ==================== Chat Content ==================== */
.chat-content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.chat-messages {
  flex: 1;
  padding: 16px 20px;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
}

.message {
  margin-bottom: 12px;
  display: flex;
}

.message.user {
  justify-content: flex-end;
}

.message.ai,
.message.loading,
.message.error {
  justify-content: flex-start;
}

.message-content {
  max-width: 85%;
  padding: 10px 14px;
  border-radius: 18px;
  font-size: 14px;
  line-height: 1.5;
  word-break: break-word;
}

.message.user .message-content {
  background: color-mix(in srgb, var(--td-brand-color) 85%, transparent);
  color: #fff;
  border-bottom-right-radius: 4px;
}

.message.ai .message-content {
  background: color-mix(in srgb, var(--td-bg-color-component) 90%, transparent);
  color: var(--td-text-color-primary);
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
  border-bottom-left-radius: 4px;
}

.message.loading .message-content {
  background: color-mix(in srgb, var(--td-bg-color-component) 60%, transparent);
  color: var(--td-text-color-secondary);
  font-style: italic;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
}

.loading-content {
  display: flex;
  align-items: center;
  gap: 8px;
}

.loading-text {
  font-size: 14px;
  color: var(--td-text-color-secondary);
}

.message.error .message-content {
  background: color-mix(in srgb, var(--td-error-color) 12%, transparent);
  color: var(--td-error-color);
  border: 1px solid color-mix(in srgb, var(--td-error-color) 25%, transparent);
}

/* ==================== Input Area ==================== */
.input-area {
  position: relative;
  z-index: 1;
  padding: 14px 16px;
  border-top: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
  display: flex;
  gap: 10px;
  align-items: center;
}

.message-input {
  flex: 1;
  padding: 10px 16px;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 15%, transparent);
  border-radius: 999px;
  outline: none;
  font-size: 14px;
  background: color-mix(in srgb, var(--td-bg-color-component) 80%, transparent);
  color: var(--td-text-color-primary);
  transition: border-color var(--motion-duration-quick) var(--motion-ease-standard),
              box-shadow var(--motion-duration-quick) var(--motion-ease-standard);
}

.message-input::placeholder {
  color: var(--td-text-color-placeholder);
}

.message-input:focus {
  border-color: var(--td-brand-color);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--td-brand-color) 15%, transparent);
}

.send-btn {
  width: 40px;
  height: 40px;
  padding: 0;
  background: var(--td-brand-color);
  color: #fff;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background-color var(--motion-duration-quick) var(--motion-ease-standard),
              opacity var(--motion-duration-quick) var(--motion-ease-standard);
}

.send-btn:hover:not(:disabled) {
  background: var(--td-brand-color-hover);
}

.send-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* ==================== Transitions ==================== */
.ball-fade-enter-active,
.ball-fade-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard),
              transform var(--motion-duration-quick) var(--motion-ease-standard);
}

.ball-fade-enter-from,
.ball-fade-leave-to {
  opacity: 0;
  transform: scale(0.8) translateX(56px);
}

.ball-fade-enter-to,
.ball-fade-leave-from {
  opacity: 1;
  transform: scale(1);
}

.glass-fade-enter-active .chat-panel {
  animation: glass-in var(--motion-duration-standard) var(--motion-ease-out);
}

.glass-fade-leave-active .chat-panel {
  animation: glass-in var(--motion-duration-quick) var(--motion-ease-out) reverse;
}

.glass-fade-enter-active,
.glass-fade-leave-active {
  transition: opacity var(--motion-duration-quick) var(--motion-ease-standard);
}

.glass-fade-enter-from,
.glass-fade-leave-to {
  opacity: 0;
}

@keyframes glass-in {
  from {
    opacity: 0;
    transform: scale(0.96) translateY(8px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

/* ==================== Markdown Content (Theme-aware) ==================== */
.message-content {
  font-family:
    -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Helvetica Neue', Arial, sans-serif;
  color: var(--td-text-color-primary);
  line-height: 1.7;
}

.message-content :deep(h1),
.message-content :deep(h2),
.message-content :deep(h3),
.message-content :deep(h4),
.message-content :deep(h5),
.message-content :deep(h6) {
  margin: 1.2em 0 0.8em 0;
  font-weight: 700;
  color: var(--td-text-color-primary);
  letter-spacing: -0.025em;
}

.message-content :deep(h1) {
  font-size: 1.875rem;
  background: linear-gradient(135deg, var(--td-brand-color) 0%, color-mix(in srgb, var(--td-brand-color) 60%, #764ba2) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.message-content :deep(h2) {
  font-size: 1.5rem;
  border-bottom: 2px solid color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
  padding-bottom: 0.5rem;
}

.message-content :deep(h3) {
  font-size: 1.25rem;
}

.message-content :deep(h4),
.message-content :deep(h5),
.message-content :deep(h6) {
  font-size: 1.125rem;
  color: var(--td-text-color-secondary);
}

.message-content :deep(p) {
  margin: 1em 0;
  line-height: 1.8;
  color: var(--td-text-color-primary);
}

.message-content :deep(ul),
.message-content :deep(ol) {
  margin: 1em 0;
  padding-left: 2em;
}

.message-content :deep(li) {
  margin: 0.5em 0;
  line-height: 1.6;
  color: var(--td-text-color-primary);
}

.message-content :deep(ul li::marker) {
  color: var(--td-brand-color);
}

.message-content :deep(blockquote) {
  margin: 1.5em 0;
  padding: 1.25rem 1.5rem;
  background: color-mix(in srgb, var(--td-brand-color) 8%, transparent);
  border-left: 4px solid var(--td-brand-color);
  border-radius: 0 8px 8px 0;
  font-style: italic;
}

.message-content :deep(code) {
  background: color-mix(in srgb, var(--td-text-color-primary) 6%, transparent);
  color: var(--td-error-color);
  padding: 0.2em 0.4em;
  border-radius: 6px;
  font-family: 'SF Mono', 'Monaco', 'Inconsolata', 'Roboto Mono', 'Consolas', monospace;
  font-size: 0.875em;
  font-weight: 600;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
}

.message-content :deep(pre) {
  background: color-mix(in srgb, var(--td-bg-color-page) 95%, #1a202c);
  color: var(--td-text-color-primary);
  padding: 1.25rem;
  border-radius: 12px;
  overflow-x: auto;
  margin: 1.5em 0;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
  position: relative;
}

.message-content :deep(pre::before) {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, var(--td-brand-color) 0%, color-mix(in srgb, var(--td-brand-color) 50%, #764ba2) 100%);
  border-radius: 12px 12px 0 0;
}

.message-content :deep(pre code) {
  background: none;
  color: inherit;
  padding: 0;
  border: none;
  box-shadow: none;
  font-size: 0.875rem;
  font-weight: 400;
}

.message-content :deep(strong) {
  font-weight: 700;
  background: color-mix(in srgb, var(--td-brand-color) 10%, transparent);
  padding: 0.1em 0.3em;
  border-radius: 4px;
}

.message-content :deep(a) {
  color: var(--td-brand-color);
  text-decoration: none;
  font-weight: 500;
  border-bottom: 2px solid transparent;
  transition: border-color var(--motion-duration-quick) var(--motion-ease-standard),
              color var(--motion-duration-quick) var(--motion-ease-standard);
}

.message-content :deep(a:hover) {
  color: var(--td-brand-color-hover);
  border-bottom-color: var(--td-brand-color);
}

.message-content :deep(table) {
  border-collapse: separate;
  border-spacing: 0;
  width: 100%;
  margin: 1.5em 0;
  background: color-mix(in srgb, var(--td-bg-color-container) 90%, transparent);
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--td-text-color-primary) 12%, transparent);
}

.message-content :deep(th),
.message-content :deep(td) {
  padding: 0.75rem 1rem;
  text-align: left;
  border-bottom: 1px solid color-mix(in srgb, var(--td-text-color-primary) 8%, transparent);
}

.message-content :deep(th) {
  background: color-mix(in srgb, var(--td-text-color-primary) 5%, transparent);
  font-weight: 700;
  color: var(--td-text-color-primary);
  font-size: 0.875rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.message-content :deep(tbody tr:last-child td) {
  border-bottom: none;
}

.message-content :deep(hr) {
  margin: 2rem 0;
  border: none;
  height: 2px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    color-mix(in srgb, var(--td-text-color-primary) 15%, transparent) 50%,
    transparent 100%
  );
}

/* ==================== Mobile (≤768px) ==================== */
@media (max-width: 768px) {
  .chat-overlay {
    align-items: flex-end;
    justify-content: center;
    padding: 0 var(--mobile-page-gutter) calc(var(--mobile-safe-bottom, 0px) + 12px);
  }

  .chat-panel {
    position: relative !important;
    left: auto !important;
    right: auto !important;
    bottom: auto !important;
    width: 100%;
    max-width: 420px;
    max-height: min(76dvh, 600px);
    border-radius: var(--mobile-card-radius, 22px);
    background: color-mix(in srgb, var(--td-bg-color-container) 92%, transparent);
    border-color: color-mix(in srgb, var(--td-text-color-primary) 10%, transparent);
    box-shadow: var(--mobile-surface-shadow, var(--glass-shadow-panel));
    transform-origin: bottom center !important;
  }

  .chat-panel::before {
    content: '';
    width: 38px;
    height: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--td-text-color-primary) 24%, transparent);
    align-self: center;
    margin-bottom: 4px;
    flex-shrink: 0;
  }

  .glass-border-glow,
  .glass-light-sweep,
  .glass-ambient {
    display: none;
  }

  .chat-header {
    padding: 14px 16px 10px;
  }

  .close-btn {
    min-width: var(--mobile-touch-target, 44px);
    min-height: var(--mobile-touch-target, 44px);
    border-radius: var(--mobile-control-radius, 999px);
  }

  .chat-messages {
    padding: 12px 14px;
  }

  .message-content {
    max-width: 90%;
  }

  .input-area {
    padding: 10px 12px calc(var(--mobile-safe-bottom, 0px) + 8px);
  }

  .message-input {
    min-height: var(--mobile-touch-target, 44px);
    font-size: 16px;
  }

  .send-btn {
    width: var(--mobile-touch-target, 44px);
    height: var(--mobile-touch-target, 44px);
  }

  .float-ball.hidden-right {
    transform: translateX(50px);
  }

  .float-ball.hidden-left {
    transform: translateX(-50px);
  }
}

/* ==================== Reduced Motion ==================== */
@media (prefers-reduced-motion: reduce) {
  .glass-border-glow,
  .glass-light-sweep {
    animation: none !important;
  }

  .glass-fade-enter-active .chat-panel,
  .glass-fade-leave-active .chat-panel {
    animation: none !important;
  }
}
</style>
