import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(import.meta.dirname, 'src'),
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    // T1-1：组件测试基建——补齐 matchMedia / ResizeObserver / IntersectionObserver
    setupFiles: ['src/test/setup.ts'],
    // T1-3：构建期工具（frontend/scripts/*.mjs）的纯函数单测与源码同列，共用 vitest 出口
    include: ['src/**/*.spec.ts', 'src/**/*.test.ts', 'scripts/**/*.spec.mjs'],
    coverage: {
      provider: 'v8',
      // T1-3 覆盖率门禁（Doc/23 §3.2；原问题 P2-2「白名单仅 7 个文件」）。
      //
      // 口径：从「7 个文件白名单」放宽为「全量手写源码减下列排除项」，
      // 分母由 545 行 → 1689 行（3.1 倍），不再有"白名单外靠人肉"的灰区。
      // 排除项各自的理由：
      // - `src/views/**`（约 4.5k 行）：真实浏览器层已由 Playwright E2E 覆盖
      //   （Doc/09 §4.5，跑真内核 + 真产物 + 真 Chromium）；其可测逻辑将在
      //   T2-4 巨型视图拆分时迁入 `composables/`，届时自动回到本口径内。
      // - `src/api/generated/**`：openapi-typescript 生成物，非手写代码。
      // - `src/locales/**`：静态文案数据，无分支语义。
      // - `src/main.ts` / `src/App.vue`：应用引导与根组件，无单测语义（E2E 首屏即覆盖）。
      // - 测试自身（`__tests__/`、`*.spec.ts`、`*.d.ts`）与 `src/test/**`：测试基建不参与被测量。
      include: ['src/**/*.{vue,ts}'],
      exclude: [
        'src/**/__tests__/**',
        'src/**/*.spec.ts',
        'src/**/*.d.ts',
        'src/App.vue',
        'src/main.ts',
        'src/api/generated/**',
        'src/locales/**',
        'src/views/**',
        'src/test/**',
      ],
      reporter: ['text', 'json-summary'],
      // 阈值只降不升即 CI 红（vitest thresholds 语义为"下限"）。
      thresholds: {
        // 全局地板：**2026-10-02 batch-10 上调 lines/statements 60 → 70、branches/functions
        // 68/62 → 70/65**（Doc/30 M-UX「覆盖率地板 70/65」收尾）。
        // 背景：中型视图拆分最后一站（Files / Databases）把文件浏览、批量操作、安装表单
        // 迁入 `utils/fileBrowser.ts`、`utils/databases.ts`、`composables/useFile{Browser,Ops}.ts`、
        // `useResponsiveLayout.ts` 与 `components/{files,databases}/**`，均落在统计区内且带单测。
        // 实测（batch-10 全量 1721 例）：81.41/76.74/72.13/82.7
        // （statements/branches/functions/lines）；上一档记录为 73.83/70.37/63.96/75.1。
        // 逐项余量（均大于 §7.15 记录的 ±0.4 点抖动一个数量级）：
        // - lines / statements 60 → **70**——实测 82.7 / 81.4，余量 12.7 / 11.4；
        // - branches 68 → **70**——实测 76.7，余量 6.7；
        // - functions 62 → **65**——实测 72.1，余量 7.1。
        // 不一次贴近实测：新增代码的函数 / 分支密度高于存量均值，留出正常迭代空间。
        lines: 70,
        statements: 70,
        branches: 70,
        functions: 65,
        // 高水位资产单独锁（不随全局地板松绑）——这两处是全站复用面最大、
        // 回归代价最高的代码，且 T1-1/T0-2 已把它们实测到 95%+：
        // 组件封装层供所有视图消费；权限指令是安全边界。
        'src/components/ui/**': {
          lines: 90,
          statements: 90,
          branches: 84,
          functions: 90,
        },
        'src/directives/**': {
          lines: 95,
          statements: 95,
          branches: 90,
          functions: 95,
        },
      },
    },
  },
})
