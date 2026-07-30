import api from './client'
import type { ServerNode, PaginatedResponse } from '@/types'

export function listNodes(page = 1, pageSize = 20) {
  return api.get<PaginatedResponse<ServerNode>>('/nodes', { params: { page, page_size: pageSize } })
}

export function createNode(node: ServerNode) {
  return api.post<number>('/nodes', { node })
}

export function deleteNode(id: number) {
  return api.delete(`/nodes/${id}`)
}
