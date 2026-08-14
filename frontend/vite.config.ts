import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import UnoCSS from 'unocss/vite'
import AutoImport from 'unplugin-auto-import/vite'
import { resolve } from 'path'

export default defineConfig({
  plugins: [
    vue(),
    UnoCSS(),
    AutoImport({
      imports: ['vue', 'vue-router'],
      dts: 'src/auto-imports.d.ts',
    }),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
      '/ws': {
        target: 'http://localhost:8080',
        ws: true,
      },
    },
  },
  build: {
    chunkSizeWarningLimit: 900,
    rollupOptions: {
      output: {
        // 重型依赖独立分包：首屏不加载（echarts 仅 Dashboard/Health、xterm 仅 Terminal）
        manualChunks(id: string) {
          if (id.includes('node_modules/echarts')) return 'echarts'
          if (id.includes('@xterm')) return 'xterm'
          if (id.includes('node_modules/openvue')) return 'openvue'
          if (id.includes('node_modules/vue-i18n')) return 'vue-i18n'
        },
      },
    },
  },
})
