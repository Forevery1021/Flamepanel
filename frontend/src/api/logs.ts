import api from './client'
import type { OperationLog, LogEntry, PaginatedResponse } from '@/types'

export function listOperationLogs(page = 1, pageSize = 20, action?: string) {
  const params: Record<string, string | number> = { page, page_size: pageSize }
  if (action) params.action = action
  return api.get<PaginatedResponse<OperationLog>>('/operation-logs', { params })
}

export function listSystemLogs(page = 1, pageSize = 20) {
  return api.get<PaginatedResponse<LogEntry>>('/logs', { params: { page, page_size: pageSize } })
}
