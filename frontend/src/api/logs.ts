import api from './client'
import type { OperationLog, LogEntry, PaginatedResponse } from '@/types'

export function listOperationLogs(page = 1, pageSize = 20) {
  return api.get<PaginatedResponse<OperationLog>>('/operation-logs', { params: { page, page_size: pageSize } })
}

export function listSystemLogs(page = 1, pageSize = 20) {
  return api.get<PaginatedResponse<LogEntry>>('/logs', { params: { page, page_size: pageSize } })
}
