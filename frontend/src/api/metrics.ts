import api from './client'
import type { ProcessEntry } from '@/api/generated'

export function listTopProcesses() {
  return api.get<ProcessEntry[]>('/metrics/processes')
}
