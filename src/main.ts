import './assets/base.css'
import './assets/icon_font/iconfont.css'
import './assets/icon_font/iconfont.js'
import './assets/main.css'

import App from './App.vue'
import { createApp } from 'vue'
import router from './router'
import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'
import i18n from './locales'
import LogtoClient from '@logto/browser'
import { performanceTelemetry } from '@/utils/performanceMonitor'

// IPC adapter layer
import './bridge'

// 全局 fetch 拦截器：图片请求直连优先，失败后代理兜底，绕过 WebView CORS
import './utils/cors-proxy'
import { installImageProxyFallback } from '@/utils/imageProxy'

// Initialize Logto client (skip in desktop lyric window)
import config from './config'
if (!window.location.hash.includes('/desktop-lyric')) {
  config.instance = new LogtoClient({
    appId: config.appId,
    endpoint: config.endpoint,
  })
}

if (import.meta.env.DEV) {
  performanceTelemetry.startMemorySampling()
}

installImageProxyFallback()

const app = createApp(App)

// 过滤 TDesign TPopup 已知的 slot 警告（TPopupTrigger 在 render 外调用 slot）
app.config.warnHandler = (msg) => {
  if (typeof msg === 'string' && msg.includes('invoked outside of the render function')) return
  console.warn(msg)
}

const pinia = createPinia()
pinia.use(piniaPluginPersistedstate)
app.use(pinia)
app.use(i18n)
app.use(router)
app.mount('#app')
