import api from './client'
import type { ScheduledTask } from '@/api/generated'
import type { Page } from '@/api/generated'

export type { ScheduledTask } from '@/api/generated'

export interface ScheduledTaskInput {
  name: string
  command: string
  schedule?: string
  enabled?: boolean
}

export function listScheduledTasks(page = 1, pageSize = 20) {
  return api.get<Page<ScheduledTask>>('/scheduled-tasks', {
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
