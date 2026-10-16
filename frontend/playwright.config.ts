import { defineConfig, devices } from '@playwright/test'
import { backendBaseUrl } from './e2e/helpers/backend'

/**
 * E2E 配置（Doc/23 §3.2 T1-2）。
 *
 * 设计取舍：
 * - **真实后端 + 真实产物**：`webServer` 起 `vite preview`（构建产物），`/api` 经
 *   `VITE_PROXY_TARGET` 反代到 global-setup 启动的真实 `flame-kernel`（SQLite 模式）。
 *   vitest 与 InMemory 双重盲区曾让两个 P0 缺陷（#84 RBAC 种子、R-1 表格空白）
 *   长期不可见，E2E 的价值恰在此。
 * - **串行（`workers: 1`、`fullyParallel: false`）**：三条黄金路径共享同一套后端，
 *   且向导只在空库上可走通一次——用例存在资源上的先后依赖（setup → 登录 → CRUD）。
 *   这是有意为之，不是配置疏漏。
 * - **固定 zh-CN**：用例用可见译文定位（用户视角），语言固定可避免 `.locale` 分叉。
 * - **不引 pnpm**：与仓库纪律一致（CI 以 package-lock.json 为准）。
 *
 * 运行前置：`cargo build --package flame-kernel`（提供真实二进制）。
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  // 刻意不用重试：本套件在**一份全新数据库**上串行运行，而向导只在空库上可走通
  // 一次——重试向导用例必然二次失败（库已初始化），只会用第二条失败掩盖首因。
  // 套件本身不依赖外部服务（无 Docker/MySQL/外网），确定性强于一般 E2E。
  retries: 0,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  timeout: 60_000,
  expect: { timeout: 10_000 },

  globalSetup: './e2e/global-setup.ts',
  globalTeardown: './e2e/global-teardown.ts',

  use: {
    baseURL: process.env.E2E_BASE_URL ?? 'http://127.0.0.1:4173',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    locale: 'zh-CN',
    timezoneId: 'Asia/Shanghai',
  },

  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],

  webServer: {
    // 两点关键：
    // 1. `--host 127.0.0.1` 不可省——`vite preview` 默认只绑 localhost，双栈机器上
    //    可能落到 `[::1]`，此时 `127.0.0.1` 的探活永远失败（webServer 超时）；
    // 2. **构建与预览串在同一条命令里**——`vite preview` 只服务 `dist/`，产物陈旧
    //    时 E2E 实际测的是旧代码（假绿）；并进来后每次运行都测当前源码。
    command: `npm run build && npm run preview -- --host 127.0.0.1 --port 4173 --strictPort`,
    url: 'http://127.0.0.1:4173',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    env: {
      // 前端产物里 `/api`、`/ws` 的反代目标指向 global-setup 启动的真实后端
      VITE_PROXY_TARGET: backendBaseUrl(),
      VITE_E2E: '1',
    },
  },
})
