import '@bridge/runtime'
import { createApp } from 'vue'
import App from './App.vue'
import DesktopLyricApp from './DesktopLyricApp.vue'
import './style.css'

const params = new URLSearchParams(window.location.search)
let root = App
if (params.get('desktopLyric') === '1') {
  root = DesktopLyricApp
}

createApp(root).mount('#app')
