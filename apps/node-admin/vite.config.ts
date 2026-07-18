import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import pkg from './package.json'

const base = process.env.GITHUB_PAGES_BASE || '/'
const buildRef = process.env.BUILD_REF || process.env.GITHUB_SHA?.slice(0, 12) || 'dev'

export default defineConfig({
  base,
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
    __BUILD_REF__: JSON.stringify(buildRef),
  },
  plugins: [vue()],
  server: {
    port: 5174,
  },
})
