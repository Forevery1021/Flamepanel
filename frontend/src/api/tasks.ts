import api from './client'

/** 统一任务状态（后端 OpenAPI 已覆盖） */
export type TaskState = 'pending' | 'running' | 'success' | 'failed' | 'cancelled'

/** 任务类别 */
export type TaskKind = 'install' | 'engine_switch' | 'batch_node' | 'generic'

/** 统一任务记录（后端返回字段） */
export interface TaskRecord {
  id: number
  kind: TaskKind
  name: string
  state: TaskState
  progress: number
  message: string
  created_at: string
  updated_at: string
}

export interface TaskListResponse {
  tasks: TaskRecord[]
  total: number
}

/** 列出全部任务（可按状态过滤） */
export function listTasks(state?: TaskState) {
  return api.get<TaskListResponse>('/tasks', {
    params: state ? { state } : {},
  })
}

/** 查询单个任务 */
export function getTask(id: number) {
  return api.get<TaskRecord>(`/tasks/${id}`)
}

/** 取消任务 */
export function cancelTask(id: number) {
  return api.post<TaskRecord>(`/tasks/${id}/cancel`)
}

/** 清理终态任务 */
export function pruneTasks() {
  return api.post<{ pruned: number }>('/tasks/prune')
}
