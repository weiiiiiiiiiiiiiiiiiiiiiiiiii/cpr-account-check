import { createApp } from 'vue'
import App from './App.vue'
import './style.css'

function syncTheme() { document.documentElement.dataset.theme = window.codexProxyPlugin?.theme || 'light' }
async function bootstrap() {
  if (import.meta.env.DEV && new URLSearchParams(location.search).has('demo')) {
    const { installDemo } = await import('./preview')
    installDemo()
  }
  syncTheme()
  window.addEventListener('codex-proxy-themechange', syncTheme)
  createApp(App).mount('#app')
}
void bootstrap()
