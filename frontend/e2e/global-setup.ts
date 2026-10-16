/**
 * E2E 全局准备（Doc/23 §3.2 T1-2）。
 *
 * 目标：让三条黄金路径**串行**复用同一套真实后端 + 真实前端产物，且每次运行
 * 都从「全新安装」这一唯一可复现的起点开始——向导只在空库上可走通一次。
 *
 * 做法：
 * 1. 清理上一轮的临时状态目录（SQLite 库 / 密钥 / 证书 / 沙箱根）；
 * 2. 由 `helpers/backend.ts` 启动真实 `flame-kernel` 二进制（SQLite 模式），
 *    等 `/health` 就绪；进程 pid 经 `process.env` 传给 global-teardown；
 * 3. 前置断言后端是**全新库**（`setup status=in_progress`），否则后续用例会在
 *    「向导已被禁用」的库上产生误导性失败。
 */

import { mkdirSync, rmSync } from 'node:fs'
import { E2E_STATE_DIR, backendBaseUrl, startBackend } from './helpers/backend'

export default async function globalSetup() {
  rmSync(E2E_STATE_DIR, { recursive: true, force: true })
  mkdirSync(E2E_STATE_DIR, { recursive: true })

  const backend = await startBackend()
  process.env.E2E_BACKEND_PID = String(backend.pid)

  const res = await fetch(`${backendBaseUrl()}/api/setup/status`)
  if (!res.ok) throw new Error(`E2E 后端 ${backendBaseUrl()} 未就绪: HTTP ${res.status}`)
  const body = (await res.json()) as { status?: string }
  if (body.status !== 'in_progress') {
    throw new Error(
      `E2E 需要全新库：期望 setup status=in_progress，实际 ${body.status}。` +
        `请清理 .e2e-state 后重跑。`,
    )
  }
}
