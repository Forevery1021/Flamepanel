import type { ProcessRow, ProcessSignal, ProtectedReason } from '@/api/toolbox'
import { formatUptime } from '@/utils/inspection'

/**
 * S-12 进程管理纯函数：状态归一 / 配色 / 运行时长 / 用户选项 / 信号说明 key。
 * 视图与弹窗只做编排，所有可测判断都在这里。
 */

export type Tone = 'success' | 'warning' | 'danger' | 'info' | 'neutral'

/** 已知状态（sysinfo `ProcessStatus` 的 Debug 名归一后的 i18n 后缀） */
export const PROCESS_STATES = [
  'run',
  'sleep',
  'idle',
  'disk_sleep',
  'stop',
  'tracing',
  'zombie',
  'dead',
  'other',
] as const
export type ProcessState = (typeof PROCESS_STATES)[number]

/** 后端状态串（`Run` / `Sleep` / `UninterruptibleDiskSleep` / `Unknown(0)` …）→ 归一状态 */
export function normalizeState(status: string): ProcessState {
  switch (status) {
    case 'Run':
    case 'Waking':
      return 'run'
    case 'Sleep':
      return 'sleep'
    case 'Idle':
    case 'Parked':
      return 'idle'
    case 'UninterruptibleDiskSleep':
    case 'LockBlocked':
      return 'disk_sleep'
    case 'Stop':
      return 'stop'
    case 'Tracing':
      return 'tracing'
    case 'Zombie':
      return 'zombie'
    case 'Dead':
    case 'Wakekill':
      return 'dead'
    default:
      return 'other'
  }
}

export function stateTone(state: ProcessState): Tone {
  switch (state) {
    case 'run':
      return 'success'
    case 'zombie':
    case 'dead':
      return 'danger'
    case 'disk_sleep':
    case 'stop':
    case 'tracing':
      return 'warning'
    default:
      return 'neutral'
  }
}

/** CPU 占用配色：≥ 80% 危险、≥ 40% 警告（多核进程可超过 100%） */
export function cpuTone(cpu: number): Tone {
  if (cpu >= 80) return 'danger'
  if (cpu >= 40) return 'warning'
  return 'neutral'
}

/** CPU 数值：< 10 保留 1 位小数，否则取整 */
export function formatCpu(cpu: number): string {
  if (!Number.isFinite(cpu) || cpu <= 0) return '0'
  return cpu < 10 ? cpu.toFixed(1) : String(Math.round(cpu))
}

/** 内存占比（相对全部进程常驻内存合计），用于行内细条；合计为 0 时返回 0 */
export function memoryShare(bytes: number, totalBytes: number): number {
  if (!totalBytes || bytes <= 0) return 0
  return Math.min(100, (bytes / totalBytes) * 100)
}

/** 运行时长：`started_at`（Unix 秒）→ `3d 4h` / `2h 5m` / `5m`；未来时间（时钟漂移）按 0 计 */
export function formatRunTime(startedAt: number, nowMs: number = Date.now()): string {
  if (!startedAt) return '—'
  const secs = Math.max(0, Math.floor(nowMs / 1000) - startedAt)
  return formatUptime(secs)
}

/** 保护原因 → i18n key */
export function protectedI18nKey(reason: ProtectedReason): string {
  return `processes.protected.${reason}`
}

/** 信号 → 说明 i18n key */
export function signalI18nKey(signal: ProcessSignal): string {
  return `processes.signal.${signal}`
}

/** 当前结果中出现过的用户（去重、排序），用于用户筛选下拉 */
export function userOptions(rows: Pick<ProcessRow, 'user'>[], selected?: string): string[] {
  const set = new Set<string>()
  for (const r of rows) if (r.user) set.add(r.user)
  // 已选用户即使当前页没有也保留，避免下拉值\"凭空消失\"
  if (selected) set.add(selected)
  return [...set].sort((a, b) => a.localeCompare(b))
}

/**
 * 结束进程失败 → 专用提示 key（其余交给通用错误文案）。
 * 404 进程已退出 / 409 pid 被复用 / 400 受保护；500 且后端提示权限不足 → 提权指引。
 */
export function killErrorKey(err: unknown): string | null {
  const e = err as { status?: number; message?: string } | null
  switch (e?.status) {
    case 404:
      return 'processes.kill.gone'
    case 409:
      return 'processes.kill.reused'
    case 400:
      return 'processes.kill.protected'
    case 403:
      // 系统层权限不足（EPERM / sudo 需要密码）才有专用提示；RBAC 403 交给通用文案
      return e?.message?.includes('insufficient privilege') ? 'processes.kill.privilege' : null
    default:
      return null
  }
}
