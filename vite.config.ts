import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],
  resolve: {
    alias: {
      '@bridge': fileURLToPath(new URL('./src/bridge', import.meta.url)),
      '@online': fileURLToPath(new URL('./src/online', import.meta.url)),
    },
  },

  // Vite options tailored for Tauri development
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    // 双端共用：根据构建目标选择兼容的 JS 目标
    // - Windows/Linux 桌面 → chrome105
    // - macOS 桌面 / iOS → safari15
    // - Android → chrome105（Android System WebView 基于 Chromium）
    target: ['android', 'windows', 'linux'].includes(process.env.TAURI_ENV_PLATFORM || '')
      ? 'chrome105'
      : 'safari15',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}))
