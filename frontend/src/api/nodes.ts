import api from './client'
import type { ServerNode } from '@/types'

export function listNodes() {
  return api.get<ServerNode[]>('/nodes')
}

export function createNode(node: ServerNode) {
  return api.post<number>('/nodes', { node })
}

export function deleteNode(id: number) {
  return api.delete(`/nodes/${id}`)
}
