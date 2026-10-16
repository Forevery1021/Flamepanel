/**
 * E2E 全局收尾（Doc/23 §3.2 T1-2）：停掉 global-setup 启动的真实后端进程。
 *
 * 进程 pid 经 `process.env.E2E_BACKEND_PID` 由 setup 传递（Playwright 的
 * globalSetup / globalTeardown 默认在同一进程运行，环境变量通道足够）。
 * 用例失败中断时进程可能已被清理，故对 `kill` 失败静默。
 */

import { cleanStateDir } from './helpers/backend'

export default async function globalTeardown() {
  const pid = Number(process.env.E2E_BACKEND_PID ?? 0)
  if (pid > 0) {
    try {
      process.kill(pid, 'SIGTERM')
    } catch {
      // 进程已退出
    }
  }
  // 默认清理临时状态（库/密钥/证书）；排障时设 E2E_KEEP_STATE=1 保留
  if (process.env.E2E_KEEP_STATE !== '1') cleanStateDir()
}
