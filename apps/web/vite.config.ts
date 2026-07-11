import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

const base = process.env.GITHUB_PAGES_BASE || '/'

export default defineConfig({
  base,
  plugins: [vue()],
  server: {
    port: 5173
  }
})
