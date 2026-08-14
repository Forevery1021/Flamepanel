import api from './client'
import type { OperationLog, LogEntry } from '@/types'
import type { Page } from '@/api/generated'

export function listOperationLogs(page = 1, pageSize = 20, action?: string) {
  const params: Record<string, string | number> = { page, page_size: pageSize }
  if (action) params.action = action
  return api.get<Page<OperationLog>>('/operation-logs', { params })
}

export function listSystemLogs(page = 1, pageSize = 20) {
  return api.get<Page<LogEntry>>('/logs', { params: { page, page_size: pageSize } })
}
