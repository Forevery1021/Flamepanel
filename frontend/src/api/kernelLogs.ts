import api from './client'
import type {
  KernelLogEntry,
  KernelLogOverview,
  KernelLogStats,
  LevelCounts,
  LogFileInfo,
  LogLevel,
  LogPage,
} from '@/api/generated'

export type {
  KernelLogEntry,
  KernelLogOverview,
  KernelLogStats,
  LevelCounts,
  LogFileInfo,
  LogLevel,
  LogPage,
}

/** 五级（按严重程度升序；与后端 `domain::kernel_log::LogLevel::ALL` 同序） */
export const LOG_LEVELS: readonly LogLevel[] = ['trace', 'debug', 'info', 'warn', 'error'] as const

export interface KernelLogQuery {
  file?: string
  levels?: LogLevel[]
  q?: string
  target?: string
  limit?: number
  before?: number | null
}

/** 概览：启用状态、目录、当前级别、保留期、文件列表 */
export function getKernelLogOverview() {
  return api.get<KernelLogOverview>('/kernel-logs')
}

/** 倒序分页（最新在前）；`before` 为上一页的 `next_before` */
export function listKernelLogEntries(query: KernelLogQuery) {
  const params: Record<string, string | number> = {}
  if (query.file) params.file = query.file
  if (query.levels?.length) params.levels = query.levels.join(',')
  if (query.q?.trim()) params.q = query.q.trim()
  if (query.target?.trim()) params.target = query.target.trim()
  if (query.limit) params.limit = query.limit
  if (query.before != null) params.before = query.before
  return api.get<LogPage>('/kernel-logs/entries', { params })
}

/** 单文件五级计数 */
export function getKernelLogStats(file: string) {
  return api.get<KernelLogStats>('/kernel-logs/stats', { params: { file } })
}

/** 下载原始文件（Blob，调用方负责触发保存） */
export function downloadKernelLogFile(name: string) {
  return api.get<Blob>(`/kernel-logs/files/${encodeURIComponent(name)}/download`, {
    responseType: 'blob',
    timeout: 0,
  })
}

/** 删除历史文件（当前文件后端拒绝） */
export function deleteKernelLogFile(name: string) {
  return api.delete<{ ok: boolean }>(`/kernel-logs/files/${encodeURIComponent(name)}`)
}

/** 运行时调级（立即生效，重启回到 OP_LOG_LEVEL） */
export function setKernelLogLevel(level: LogLevel) {
  return api.put<{ level: LogLevel }>('/kernel-logs/level', { level })
}
