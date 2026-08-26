import { resolve } from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import { TDesignResolver } from '@tdesign-vue-next/auto-import-resolver'
import { NaiveUiResolver } from 'unplugin-vue-components/resolvers'
import VueI18nPlugin from '@intlify/unplugin-vue-i18n/vite'

const host = process.env.TAURI_DEV_HOST

export default defineConfig(async () => ({
  plugins: [
    vue(),
    AutoImport({
      resolvers: [
        TDesignResolver({
          library: 'vue-next'
        })
      ],
      imports: [
        'vue',
        {
          'naive-ui': ['useDialog', 'useMessage', 'useNotification', 'useLoadingBar'],
          'vue-i18n': ['useI18n']
        }
      ],
      dts: 'src/auto-imports.d.ts'
    }),
    Components({
      resolvers: [
        TDesignResolver({
          library: 'vue-next'
        }),
        NaiveUiResolver()
      ],
      dts: 'src/components.d.ts'
    }),
    VueI18nPlugin({
      include: [
        resolve(__dirname, 'src/locales/*/common.ts'),
        resolve(__dirname, 'src/locales/*/music.ts'),
        resolve(__dirname, 'src/locales/*/settings.ts'),
        resolve(__dirname, 'src/locales/*/play.ts'),
        resolve(__dirname, 'src/locales/*/download.ts'),
        resolve(__dirname, 'src/locales/*/auth.ts'),
        resolve(__dirname, 'src/locales/*/backup.ts'),
        resolve(__dirname, 'src/locales/*/plugin.ts'),
        resolve(__dirname, 'src/locales/*/error.ts'),
        resolve(__dirname, 'src/locales/*/quality.ts'),
        resolve(__dirname, 'src/locales/*/ai.ts')
      ],
      fullInstall: false
    })
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@renderer': resolve(__dirname, 'src'),
      '@assets': resolve(__dirname, 'src/assets'),
      '@components': resolve(__dirname, 'src/components'),
      '@services': resolve(__dirname, 'src/services'),
      '@types': resolve(__dirname, 'src/types'),
      '@store': resolve(__dirname, 'src/store')
    }
  },
  base: './',
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**']
    }
  },
  build: {
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-vue': ['vue', 'vue-router', 'pinia'],
          'vendor-tdesign': ['tdesign-vue-next', 'tdesign-icons-vue-next'],
          'vendor-utils': ['lodash', 'axios', 'marked', 'dompurify'],
          'vendor-lyrics': ['@applemusic-like-lyrics/lyric', '@applemusic-like-lyrics/vue', '@lrc-player/core', '@lrc-player/parse'],
          'vendor-three': ['three']
        }
      }
    }
  }
}))
