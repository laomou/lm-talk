import { createApp } from 'vue'
import App from './App.vue'
import './style.css'
import { router } from './router'
import { registerPwa } from './pwa'

createApp(App).use(router).mount('#app')
void registerPwa()
