import api from './client'
import type { OperationLog, LogEntry } from '@/types'

export function listOperationLogs() {
  return api.get<OperationLog[]>('/operation-logs')
}

export function listSystemLogs() {
  return api.get<LogEntry[]>('/logs')
}
