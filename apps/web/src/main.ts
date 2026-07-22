import { createApp } from 'vue'
import App from './App.vue'
import './style.css'
import { router } from './router'
import { registerPwa } from './pwa'
import { i18n } from './i18n'

createApp(App).use(router).use(i18n).mount('#app')
void registerPwa()
