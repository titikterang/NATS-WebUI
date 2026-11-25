/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// `npm run dev` serves the UI with hot reload and proxies the API and websockets to the backend.
// Override the backend with NATS_WEBUI_BACKEND=http://host:port.
const backend = process.env.NATS_WEBUI_BACKEND ?? 'http://127.0.0.1:8080'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': { target: backend, ws: true },
      // Anchored: a plain '/client' prefix would also catch the SPA route /clients/<id>.
      '^/client/': { target: backend, ws: true },
      '/healthz': { target: backend },
    },
  },
  build: {
    outDir: 'dist',
    chunkSizeWarningLimit: 1500,
    rollupOptions: {
      output: {
        // Keep the large UI libraries in their own cacheable chunks.
        manualChunks(id: string) {
          if (id.includes('/node_modules/element-plus/')) return 'element-plus'
          if (id.includes('/node_modules/apexcharts/') || id.includes('/node_modules/vue3-apexcharts/')) return 'charts'
          return undefined
        },
      },
    },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
})
