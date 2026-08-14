import api from './client'
import type { ServerNode, RemoteFileEntry, RemoteExecResult, BatchExecItem } from '@/types'
import type { Page } from '@/api/generated'

export function listNodes(page = 1, pageSize = 20) {
  return api.get<Page<ServerNode>>('/nodes', { params: { page, page_size: pageSize } })
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

// ── Stage5 多节点远程调用 ──────────────────────────────────────

export function remoteExecute(id: number, command: string, timeoutSecs?: number) {
  return api.post<RemoteExecResult>(`/nodes/${id}/execute`, { command, timeout_secs: timeoutSecs })
}

/**
 * Phase A1：调用 Agent 动作枚举（白名单安全动作）。
 * Agent 侧拒绝非白名单命令，返回 `{status:"ok",data:{...}}` 或 `{status:"err",data:{code,message}}`。
 */
export function remoteAction(id: number, action: string, params: Record<string, unknown> = {}) {
  return api.post<{ status: string; data?: unknown }>(`/nodes/${id}/action`, { action, params })
}

export function batchExecute(nodeIds: number[], command: string, timeoutSecs?: number) {
  return api.post<{ items: BatchExecItem[] }>('/nodes/batch-execute', {
    node_ids: nodeIds,
    command,
    timeout_secs: timeoutSecs,
  })
}

export function remoteListFiles(id: number, path: string) {
  return api.get<RemoteFileEntry[]>(`/nodes/${id}/files`, { params: { path } })
}

export function remoteDownloadFile(id: number, path: string) {
  return api.get<{ node_id: number; path: string; size: number; content_base64: string }>(
    `/nodes/${id}/files/download`,
    { params: { path } },
  )
}

export function remoteUploadFile(id: number, path: string, contentBase64: string) {
  return api.post<{ node_id: number; path: string; size: number }>(`/nodes/${id}/files/upload`, {
    path,
    content_base64: contentBase64,
  })
}

export function registerNode(payload: {
  name: string
  host: string
  ip_address: string
  agent_port: number
  auth_token: string
}) {
  return api.post<{ id: number }>('/nodes/register', payload)
}
