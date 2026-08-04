import api from './client'
import type { ServerNode, PaginatedResponse } from '@/types'

export function listNodes(page = 1, pageSize = 20) {
  return api.get<PaginatedResponse<ServerNode>>('/nodes', { params: { page, page_size: pageSize } })
}

export function createNode(node: ServerNode) {
  return api.post<number>('/nodes', { node })
}

export function updateNode(id: number, node: ServerNode) {
  return api.put<ServerNode>(`/nodes/${id}`, { node })
}

export function deleteNode(id: number) {
  return api.delete(`/nodes/${id}`)
}

export function nodeStatus(id: number) {
  return api.get<{ id: number; status: string }>(`/nodes/${id}/status`)
}

export function nodeMetrics(id: number) {
  return api.get<{
    cpu_usage?: number
    memory_usage_percent?: number
    disk_usage_percent?: number
    load_one?: number
  }>(`/nodes/${id}/metrics`)
}
