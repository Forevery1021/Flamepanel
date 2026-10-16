import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import UnoCSS from 'unocss/vite'
import AutoImport from 'unplugin-auto-import/vite'
import { resolve } from 'path'
import { cspPlugin } from './scripts/csp.ts'

const rootDir = import.meta.dirname

export default defineConfig({
  plugins: [
    vue(),
    UnoCSS(),
    // Doc/33 P-6：CSP 单一来源（meta 注入 + dist/csp.conf）
    cspPlugin(),
    AutoImport({
      imports: ['vue', 'vue-router'],
      dts: 'src/auto-imports.d.ts',
    }),
  ],
  resolve: {
    alias: {
      '@': resolve(rootDir, 'src'),
    },
  },
  server: {
    port: 5173,
    // 反代目标可覆盖：E2E（T1-2）用 `vite preview` 提供构建产物，并把 /api、/ws
    // 指到 global-setup 启动的真实内核（非默认 8080），避免与本地 dev 实例串台。
    // `vite preview` 复用 `server.proxy`（见 Vite resolvePreviewOptions）。
    proxy: {
      '/api': {
        target: process.env.VITE_PROXY_TARGET ?? 'http://localhost:8080',
        changeOrigin: true,
      },
      '/ws': {
        target: process.env.VITE_PROXY_TARGET ?? 'http://localhost:8080',
        ws: true,
      },
    },
  },
  build: {
    chunkSizeWarningLimit: 900,
    rollupOptions: {
      output: {
        // 重型依赖独立分包：首屏不加载（echarts 仅 Dashboard/Health、xterm 仅 Terminal）
        // openvue 不再整包聚合——多数视图只用少数组件，整包 826KB 会被首屏拖入，
        // 交给 Rollup 按实际 import 树摇后自动分包，视图级懒加载才真正生效
        manualChunks(id: string) {
          if (id.includes('node_modules/echarts')) return 'echarts'
          if (id.includes('@xterm')) return 'xterm'
          if (id.includes('node_modules/vue-i18n')) return 'vue-i18n'
        },
      },
    },
  },
})
