import api from './client'
import type { PaginatedResponse } from '@/types'

export interface ScheduledTask {
  id: number
  name: string
  command: string
  schedule: string
  enabled: boolean
  last_status: string
  last_output: string
  last_run_at: string | null
  next_run_at: string | null
  created_at: string
  updated_at: string
}

export interface ScheduledTaskInput {
  name: string
  command: string
  schedule?: string
  enabled?: boolean
}

export function listScheduledTasks(page = 1, pageSize = 20) {
  return api.get<PaginatedResponse<ScheduledTask>>('/scheduled-tasks', {
    params: { page, page_size: pageSize },
  })
}

export function createScheduledTask(input: ScheduledTaskInput) {
  return api.post<ScheduledTask>('/scheduled-tasks', input)
}

export function updateScheduledTask(id: number, input: Partial<ScheduledTaskInput>) {
  return api.put<ScheduledTask>(`/scheduled-tasks/${id}`, input)
}

export function deleteScheduledTask(id: number) {
  return api.delete(`/scheduled-tasks/${id}`)
}

export function runScheduledTask(id: number) {
  return api.post<ScheduledTask>(`/scheduled-tasks/${id}/run`)
}

export function toggleScheduledTask(id: number, enabled: boolean) {
  return api.post<ScheduledTask>(`/scheduled-tasks/${id}/toggle`, { enabled })
}
