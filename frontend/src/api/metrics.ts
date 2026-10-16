import api from './client'
import type { MetricsHistoryResponse, ProcessEntry } from '@/api/generated'

export type { MetricsHistoryPoint, MetricsHistoryResponse } from '@/api/generated'

export function listTopProcesses() {
  return api.get<ProcessEntry[]>('/metrics/processes')
}

/** S-07 监控历史范围：≤ 24h 为 1 min 点，7d / 30d 为 1 h 点（服务端 `step_secs` 给出桶宽） */
export const METRICS_HISTORY_RANGES = ['1h', '6h', '24h', '7d', '30d'] as const
export type MetricsHistoryRange = (typeof METRICS_HISTORY_RANGES)[number]

/**
 * S-07 监控历史降采样曲线（需 `node:read`）。
 * 刚启动或历史不足时 `points` 少于窗口理论点数，前端按 `bucket_start` 绘制即可。
 */
export function getMetricsHistory(range: MetricsHistoryRange = '24h') {
  return api.get<MetricsHistoryResponse>('/metrics/history', { params: { range } })
}
