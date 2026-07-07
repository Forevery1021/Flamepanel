import api from './client'
import type { ServerNode } from '@/types'

export function listNodes() {
  return api.get<ServerNode[]>('/nodes')
}

export function createNode(node: ServerNode) {
  return api.post<number>('/nodes', { node })
}
