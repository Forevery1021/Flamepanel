/**
 * E2E 后端进程管理（Doc/23 §3.2 T1-2）。
 *
 * 与 vitest（单元/组件层）的关键差异：E2E 断言的是**真实进程 + 真实浏览器**下的
 * 用户可见行为。历史两轮 P0 缺陷（#84 RBAC 种子只在 InMemory 生效、R-1 表格静默
 * 空白）都恰好落在 vitest 与 InMemory 的双重盲区里——本文件即为此引入：
 * 必须跑 `OP_DATABASE_BACKEND=sqlite` 的真实二进制。
 *
 * 环境变量：
 * - `E2E_BACKEND_BIN`：二进制路径（默认 `target/debug/flame-kernel`，相对**仓库根**）；
 * - `E2E_BACKEND_PORT`：监听端口（默认 18080，避开本地 dev 的 8080）。
 */

import { spawn, type ChildProcess } from 'node:child_process'
import { existsSync, mkdirSync, rmSync } from 'node:fs'
import { resolve } from 'node:path'

/** `frontend/e2e/helpers` → 仓库根（frontend/e2e/helpers → ../../..） */
export const REPO_ROOT = resolve(import.meta.dirname, '../../..')

/** E2E 运行期状态目录（临时数据：SQLite 库/密钥/证书/沙箱根） */
export const E2E_STATE_DIR = resolve(REPO_ROOT, '.e2e-state')

/** 沙箱根（文件管理器白名单；E2E 不碰文件，仅保证启动参数合法） */
const SANDBOX_DIR = resolve(E2E_STATE_DIR, 'files')

/** 数据库文件（每次运行由 global-setup 删除，保证「全新安装」起点） */
const DB_PATH = resolve(E2E_STATE_DIR, 'app.db')

/** 默认后端端口（避开 dev server 的 8080） */
export function backendPort(): number {
  return Number(process.env.E2E_BACKEND_PORT ?? 18080)
}

export function backendBaseUrl(): string {
  return `http://127.0.0.1:${backendPort()}`
}

/** 二进制路径（优先显式指定；否则用 debug 构建，缺失时给出可执行的补救命令） */
export function backendBin(): string {
  const explicit = process.env.E2E_BACKEND_BIN
  if (explicit) return resolve(REPO_ROOT, explicit)
  return resolve(REPO_ROOT, 'target/debug/flame-kernel')
}

export interface BackendHandle {
  pid: number
  stop: () => Promise<void>
}

/**
 * 启动真实内核（SQLite 模式）并等待 `/health` 就绪。
 *
 * 注意：**不设 `OP_ADMIN_PASSWORD`**——那会切到「无人值守」模式并禁用向导，
 * 黄金路径①（向导全流程）将无从走通。向导所需的 admin 由用例在页面里创建。
 */
export async function startBackend(): Promise<BackendHandle> {
  const bin = backendBin()
  if (!existsSync(bin)) {
    throw new Error(
      `E2E 需要真实后端二进制：${bin} 不存在。\n` +
        `请先构建：cargo build --package flame-kernel（仓库根执行）`,
    )
  }
  mkdirSync(SANDBOX_DIR, { recursive: true })

  const child = spawn(bin, [], {
    // 工作目录＝E2E 状态目录，**不能**用仓库根：内核把 jwt_secret.key /
    // bootstrap_token.key / secret_encryption.key 写在 **CWD 相对**的 `./data` 下
    // （`config::loader` 的持久化路径），与 `OP_DATABASE_URL` 无关。跑在仓库根会把
    // 密钥文件丢进 `<repo>/data/`——该目录**不在 .gitignore 内**，一次 `git add -A`
    // 就会把签名密钥提交进仓库。改到 .e2e-state 后，所有散落产物都落在已忽略目录内。
    cwd: E2E_STATE_DIR,
    env: {
      ...process.env,
      OP_PORT: String(backendPort()),
      OP_HOST: '127.0.0.1',
      OP_DATABASE_BACKEND: 'sqlite',
      OP_DATABASE_URL: `sqlite:${DB_PATH}?mode=rwc`,
      OP_FILE_ROOT: SANDBOX_DIR,
      OP_TERMINAL_CWD: SANDBOX_DIR,
      RUST_LOG: process.env.RUST_LOG ?? 'warn',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  })

  const logs: string[] = []
  child.stdout?.on('data', (d: Buffer) => logs.push(d.toString()))
  child.stderr?.on('data', (d: Buffer) => logs.push(d.toString()))

  try {
    await waitForHealth(child, logs)
  } catch (e) {
    await stopChild(child)
    throw e
  }

  return { pid: child.pid ?? -1, stop: () => stopChild(child) }
}

/** 轮询 `/health` 直到 200（或进程提前退出/超时） */
async function waitForHealth(child: ChildProcess, logs: string[]): Promise<void> {
  const deadline = Date.now() + 30_000
  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(`E2E 后端启动即退出（code=${child.exitCode}）:\n${logs.join('')}`)
    }
    try {
      const res = await fetch(`${backendBaseUrl()}/health`)
      if (res.ok) return
    } catch {
      // 尚未监听，继续等
    }
    await new Promise((r) => setTimeout(r, 250))
  }
  throw new Error(`E2E 后端 30s 内未就绪:\n${logs.join('')}`)
}

async function stopChild(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null) return
  child.kill('SIGTERM')
  const deadline = Date.now() + 5_000
  while (child.exitCode === null && Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 100))
  }
  if (child.exitCode === null) child.kill('SIGKILL')
}

/** 清空运行期状态目录（global-teardown 复用） */
export function cleanStateDir(): void {
  rmSync(E2E_STATE_DIR, { recursive: true, force: true })
}
