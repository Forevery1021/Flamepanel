import type { HostDisk, HostInterface } from '@/api/toolbox'

/** 服务器与浏览器时钟偏差超过该秒数时提示（TOTP / 证书 / 定时任务都依赖准确时间） */
export const CLOCK_SKEW_WARN_SECS = 30

/** 工具箱入口：按此顺序展示，实际是否可见由路由 `meta.perm` 与当前角色决定 */
export const TOOL_ROUTE_NAMES = [
  'Processes',
  'Inspection',
  'Terminal',
  'Firewall',
  'ScheduledTasks',
  'SystemLogs',
  'Health',
] as const

/** 已用百分比（0–100，整数）；total 为 0 时返回 0 */
export function usedPercent(total: number, available: number): number {
  if (!total || total <= 0) return 0
  const used = Math.max(0, total - Math.min(available, total))
  return Math.round((used / total) * 100)
}

/** 用量色调：≥ 90 danger，≥ 75 warning，否则 success */
export function usageTone(percent: number): 'success' | 'warning' | 'danger' {
  if (percent >= 90) return 'danger'
  if (percent >= 75) return 'warning'
  return 'success'
}

/**
 * 服务器时钟相对浏览器的偏差（秒，正数 = 服务器快）。
 * `receivedAtMs` 为浏览器收到响应的时刻；网络往返通常远小于提示阈值，忽略不计。
 */
export function clockSkewSeconds(serverNowIso: string, receivedAtMs: number): number {
  const server = Date.parse(serverNowIso)
  if (Number.isNaN(server)) return 0
  return Math.round((server - receivedAtMs) / 1000)
}

/** 是否需要提示时钟偏差 */
export function isClockSkewed(skewSecs: number): boolean {
  return Math.abs(skewSecs) > CLOCK_SKEW_WARN_SECS
}

/** 网卡筛选：默认隐藏回环 / 容器虚拟网卡（服务端已把物理网卡排在前面） */
export function visibleInterfaces(list: HostInterface[], showVirtual: boolean): HostInterface[] {
  return showVirtual ? list : list.filter((i) => !i.virtual_iface)
}

/** 磁盘已用百分比 */
export function diskUsedPercent(disk: Pick<HostDisk, 'total_bytes' | 'available_bytes'>): number {
  return usedPercent(disk.total_bytes, disk.available_bytes)
}

/** 负载 → `0.12 / 0.34 / 0.56` */
export function formatLoad(load: number[]): string {
  if (!load.length) return '—'
  return load.map((v) => v.toFixed(2)).join(' / ')
}
