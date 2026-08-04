import api from './client'
import type { ProcessEntry } from '@/types'

export function listTopProcesses() {
  return api.get<ProcessEntry[]>('/metrics/processes')
}
