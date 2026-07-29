import '@bridge/runtime'
import { createApp } from 'vue'
import App from './App.vue'
import EditorApp from './EditorApp.vue'
import DesktopLyricApp from './DesktopLyricApp.vue'
import './style.css'

const params = new URLSearchParams(window.location.search)
let root = App
if (params.get('editor') === '1') {
  root = EditorApp
} else if (params.get('desktopLyric') === '1') {
  root = DesktopLyricApp
}

createApp(root).mount('#app')
